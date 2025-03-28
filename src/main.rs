use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use banner::Banner;
use chrono::Utc;
use controls::Gamepad;
use foxglove::schemas::FrameTransforms;
use foxglove::websocket::Capability;
use foxglove::{McapWriter, WebSocketServer, static_typed_channel};
use landing::LandingReport;
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;

mod assets;
mod banner;
mod controls;
mod convert;
mod lander;
mod landing;
mod landscape;
mod listener;
mod parameters;

use controls::Controls;
use lander::Lander;
use landscape::Landscape;
use listener::Listener;
use parameters::Parameters;
use tempfile::NamedTempFile;

static_typed_channel!(FT, "/ft", FrameTransforms);

const GAME_STEP_DURATION: Duration = Duration::from_millis(33);

#[tokio::main]
async fn main() {
    if let Err(e) = fallible_main().await {
        eprintln!("fatal: {e:?}");
    }
}

async fn fallible_main() -> anyhow::Result<()> {
    let params = Arc::new(Parameters::default());
    let gamepad = Gamepad::from_json_file("gamepad.json")?;
    let controls = Arc::new(Controls::new(gamepad));
    let recordings_dir = PathBuf::from("./recordings");
    if !recordings_dir.exists() {
        std::fs::create_dir_all(&recordings_dir).context("failed to create recordings dir")?;
    }
    let server = WebSocketServer::new()
        .name("fg-lander")
        .capabilities([Capability::ClientPublish, Capability::Parameters])
        .supported_encodings(["json"])
        .fetch_asset_handler_blocking_fn(assets::fetch_asset)
        .listener(Listener::new(params.clone(), controls.clone()).into_listener())
        .start()
        .await
        .context("failed to start websocket server")?;
    tokio::task::spawn(game_loop(recordings_dir, params, controls));
    tokio::signal::ctrl_c().await.ok();
    server.stop().await;
    Ok(())
}

async fn game_loop(recordings_dir: PathBuf, params: Arc<Parameters>, controls: Arc<Controls>) {
    loop {
        if let Err(e) = game_iter(&recordings_dir, &params, &controls).await {
            eprintln!("game aborted: {e}");
        }
    }
}

async fn game_iter(
    recordings_dir: &Path,
    params: &Parameters,
    controls: &Controls,
) -> anyhow::Result<()> {
    // Initialize game state.
    let seed = params.next_seed();
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let landscape = Landscape::new(&mut rng, params);
    let mut lander = Lander::new(
        landscape.lander_init_position(),
        params.lander_init_vertical_velocity(),
        params.lander_init_vertical_velocity_target(),
        params.landing_zone_radius(),
    );

    // Clear state, log scene once.
    LandingReport::clear();
    controls.soft_reset();
    log_scene_static(&landscape, &lander);

    // Print a banner to tell the user to press start and wait.
    let banner = Banner::press_start();
    while !controls.get_reset_requested() {
        log_frame_transforms(&landscape, &lander, Some(&banner));
        banner.log_scene();
        tokio::time::sleep(GAME_STEP_DURATION).await;
    }

    Banner::clear_scene();
    controls.soft_reset();

    // Start recording an mcap file.
    let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
    let recording = NamedTempFile::new_in(recordings_dir).unwrap();
    let mcap_writer = McapWriter::new().create(BufWriter::new(recording)).unwrap();

    // Log landscape and lander once at the beginning of the game.
    log_scene_static(&landscape, &lander);

    // Main game loop.
    while !lander.has_landed() {
        tokio::time::sleep(GAME_STEP_DURATION).await;
        lander.step(GAME_STEP_DURATION.as_secs_f32(), controls);
        log_frame_transforms(&landscape, &lander, None);
        lander.log();
        if controls.get_reset_requested() {
            return Ok(());
        }
    }

    // Generate and log a landing report.
    let report = lander.landing_report().expect("landed");
    let status = report.status();
    let banner = Banner::landing_status(status);
    log_frame_transforms(&landscape, &lander, Some(&banner));
    banner.log_scene();
    report.log();

    // Finalize the recording.
    let recording = mcap_writer
        .close()
        .context("flush recording data")?
        .into_inner()
        .context("recover named tempfile")?;
    recording
        .persist(recordings_dir.join(format!("{status:?}-{timestamp}.mcap")))
        .context("rename recording file")?;

    // Halt the lander and log while waiting for a reset.
    lander.stop();
    while !controls.get_reset_requested() {
        log_frame_transforms(&landscape, &lander, Some(&banner));
        banner.log_scene();
        report.log();
        tokio::time::sleep(GAME_STEP_DURATION).await;
    }

    Ok(())
}

/// Logs frame transforms.
fn log_frame_transforms(landscape: &Landscape, lander: &Lander, banner: Option<&Banner>) {
    let mut transforms = Vec::with_capacity(4);
    transforms.extend(landscape.frame_transforms());
    transforms.push(lander.frame_transform());
    if let Some(banner) = banner {
        transforms.push(banner.frame_transform());
    }
    FT.log(&FrameTransforms { transforms });
}

/// Logs static scene entities.
fn log_scene_static(landscape: &Landscape, lander: &Lander) {
    landscape.log_scene();
    lander.log_scene();
}
