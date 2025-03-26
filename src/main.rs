use std::time::Duration;

use foxglove::schemas::{
    Color, FrameTransform, SceneEntity, SceneEntityDeletion, SceneUpdate, TextPrimitive, Vector3,
};
use foxglove::websocket::Capability;
use foxglove::{WebSocketServer, static_typed_channel};
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
use lander::{Lander, LanderStatus};
use landscape::Landscape;
use listener::Listener;
use parameters::Parameters;

static_typed_channel!(BANNER, "/banner", SceneUpdate);
static_typed_channel!(BANNER_FT, "/banner_ft", FrameTransform);

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
    let server = WebSocketServer::new()
        .name("fg-lander")
        .capabilities([Capability::ClientPublish, Capability::Parameters])
        .supported_encodings(["json"])
        .fetch_asset_handler_blocking_fn(assets::fetch_asset)
        .listener(Listener::new(params.clone(), controls.clone()).into_listener())
        .start()
        .await?;
    tokio::task::spawn(game_loop(params, controls));
    tokio::signal::ctrl_c().await.ok();
    server.stop().await;
    Ok(())
}

async fn game_loop(params: Parameters, controls: Controls) {
    loop {
        let seed = params.next_seed();
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        let landscape = Landscape::new(&mut rng, &params);
        let lander = Lander::new(
            landscape.lander_start_position(),
            params.landing_zone_radius(),
        );
        landscape.log_static();
        controls.do_reset();
        clear_banner();
        game_round(lander, &controls).await;
    }
}

async fn game_round(mut lander: Lander, controls: &Controls) {
    let mut status = LanderStatus::Aloft;
    while !controls.is_reset_pending() {
        if matches!(status, LanderStatus::Aloft) {
            lander.step(GAME_STEP_DURATION.as_secs_f32(), controls);
            status = lander.status();
            if !matches!(status, LanderStatus::Aloft) {
                display_banner(status);
                lander.stop();
            }
        }
        lander.log();
        tokio::time::sleep(GAME_STEP_DURATION).await;
    }
}

fn clear_banner() {
    BANNER_FT.log_static(&FrameTransform {
        parent_frame_id: "lander".into(),
        child_frame_id: "banner".into(),
        ..Default::default()
    });
    BANNER.log_static(&SceneUpdate {
        deletions: vec![SceneEntityDeletion {
            id: "banner".into(),
            ..Default::default()
        }],
        ..Default::default()
    });
}

fn display_banner(status: LanderStatus) {
    let (color, text) = match status {
        LanderStatus::Aloft => return,
        LanderStatus::Landed => (
            Color {
                r: 0.0,
                g: 1.0,
                b: 0.0,
                a: 0.75,
            },
            "LANDED".to_string(),
        ),
        LanderStatus::TooFast => (
            Color {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 0.75,
            },
            "TOO FAST".to_string(),
        ),
        LanderStatus::NotLevel => (
            Color {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 0.75,
            },
            "NOT LEVEL".to_string(),
        ),
        LanderStatus::Spinning => (
            Color {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 0.75,
            },
            "SPINNING".to_string(),
        ),
        LanderStatus::Missed => (
            Color {
                r: 1.0,
                g: 1.0,
                b: 0.0,
                a: 0.75,
            },
            "MISSED".to_string(),
        ),
    };
    BANNER_FT.log_static(&FrameTransform {
        parent_frame_id: "lander".into(),
        child_frame_id: "banner".into(),
        translation: Some(Vector3 {
            z: 5.0,
            ..Default::default()
        }),
        ..Default::default()
    });
    BANNER.log_static(&SceneUpdate {
        entities: vec![SceneEntity {
            frame_id: "banner".into(),
            id: "banner".into(),
            texts: vec![TextPrimitive {
                pose: None,
                billboard: true,
                font_size: 48.0,
                scale_invariant: true,
                color: Some(color),
                text,
            }],
            ..Default::default()
        }],
        ..Default::default()
    });
}
