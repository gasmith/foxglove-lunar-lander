use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Context;
use chrono::Utc;
use foxglove::websocket::Capability;
use foxglove::{McapWriter, WebSocketServer};
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;

mod assets;
mod controls;
mod convert;
mod lander;
mod landscape;
mod listener;
mod message;
mod parameters;

use controls::Controls;
use lander::Lander;
use landscape::Landscape;
use listener::Listener;
use parameters::Parameters;
use tempfile::NamedTempFile;

const GAME_STEP_DURATION: Duration = Duration::from_millis(33);

#[tokio::main]
async fn main() {
    if let Err(e) = fallible_main().await {
        eprintln!("fatal: {e}");
    }
}

async fn fallible_main() -> anyhow::Result<()> {
    let params = Parameters::default();
    let controls = Controls::default();
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

async fn game_loop(recordings_dir: PathBuf, params: Parameters, controls: Controls) {
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

    // Reinitialize state.
    lander.clear_landing_report();
    controls.do_reset();

    // Start recording an mcap file.
    let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
    let recording = NamedTempFile::new_in(recordings_dir).unwrap();
    let mcap_writer = McapWriter::new().create(BufWriter::new(recording)).unwrap();

    // Log landscape once, since it has lots of polygons.
    landscape.log_static();

    // Main game loop.
    while !lander.has_landed() {
        tokio::time::sleep(GAME_STEP_DURATION).await;
        lander.step(GAME_STEP_DURATION.as_secs_f32(), controls);
        lander.log();
        if controls.is_reset_pending() {
            return Ok(());
        }
    }

    // Generate and log a landing report.
    let status = lander.log_landing_report().expect("landed");

    // Finalize the recording.
    let recording = mcap_writer
        .close()
        .context("flush recording data")?
        .into_inner()
        .context("recover named tempfile")?;
    recording
        .persist(recordings_dir.join(format!("lander-{timestamp}-{status:?}.mcap")))
        .context("rename recording file")?;

    // Halt the lander and log while waiting for a reset.
    lander.stop();
    while !controls.is_reset_pending() {
        tokio::time::sleep(GAME_STEP_DURATION).await;
        lander.log();
    }

    Ok(())
}
