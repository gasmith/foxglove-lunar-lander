use std::time::Duration;

use foxglove::WebSocketServer;
use foxglove::websocket::Capability;
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
        lander.clear_landing_report();
        landscape.log_static();
        controls.do_reset();
        game_round(lander, &controls).await;
    }
}

async fn game_round(mut lander: Lander, controls: &Controls) {
    let mut landed = false;
    while !controls.is_reset_pending() {
        if !landed {
            lander.step(GAME_STEP_DURATION.as_secs_f32(), controls);
            if lander.has_landed() {
                landed = true;
                lander.log_landing_report();
                lander.stop();
            }
        }
        lander.log();
        tokio::time::sleep(GAME_STEP_DURATION).await;
    }
}
