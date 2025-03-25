use std::sync::Arc;

use foxglove::websocket::{Client, ClientChannel, ServerListener};
use glam::Vec3;
use serde::Deserialize;

use crate::controls::Controls;
use crate::parameters::Parameters;

pub struct Listener {
    params: Parameters,
    controls: Controls,
}
impl Listener {
    pub fn new(params: Parameters, controls: Controls) -> Self {
        Self { params, controls }
    }

    pub fn into_listener(self) -> Arc<dyn ServerListener> {
        Arc::new(self)
    }
}
impl ServerListener for Listener {
    fn on_message_data(&self, _client: Client, channel: &ClientChannel, payload: &[u8]) {
        if channel.schema_name != "sensor_msgs/Joy" {
            return;
        }
        let msg: GamepadMsg = match serde_json::from_slice(payload) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("failed to deserialize /joy message: {e}");
                return;
            }
        };
        let thrust = msg.thrust();
        let rotation = msg.rotation();
        if msg.reset() {
            self.controls.request_reset();
        }
        self.controls.update(thrust, rotation);
    }

    fn on_get_parameters(
        &self,
        _client: Client,
        param_names: Vec<String>,
        _request_id: Option<&str>,
    ) -> Vec<foxglove::websocket::Parameter> {
        self.params.get(param_names)
    }

    fn on_set_parameters(
        &self,
        _client: Client,
        params: Vec<foxglove::websocket::Parameter>,
        _request_id: Option<&str>,
    ) -> Vec<foxglove::websocket::Parameter> {
        self.params.set(params)
    }
}

#[derive(Debug, Deserialize)]
struct GamepadMsg {
    axes: Vec<f32>,
    buttons: Vec<f32>,
}

#[allow(dead_code)]
impl GamepadMsg {
    const JOY_LX: usize = 0;
    const JOY_LY: usize = 1;
    const JOY_RX: usize = 2;
    const JOY_RY: usize = 3;
    const BUTTON_X: usize = 0;
    const BUTTON_O: usize = 1;
    const BUTTON_S: usize = 2;
    const BUTTON_T: usize = 3;
    const BUTTON_L1: usize = 4;
    const BUTTON_R1: usize = 5;
    const BUTTON_L2: usize = 6;
    const BUTTON_R2: usize = 7;
    const BUTTON_UP: usize = 12;
    const BUTTON_DOWN: usize = 13;
    const BUTTON_LEFT: usize = 14;
    const BUTTON_RIGHT: usize = 15;
    const BUTTON_PS: usize = 16;
    const JOY_DEAD_ZONE_TOL: f32 = 0.10;

    fn reset(&self) -> bool {
        self.buttons[Self::BUTTON_PS] > 0.0
    }

    fn read_axes(&self, idx: usize) -> f32 {
        let raw = self.axes[idx];
        if raw.abs() < Self::JOY_DEAD_ZONE_TOL {
            0.0
        } else {
            raw.clamp(-1.0, 1.0)
        }
    }

    fn thrust(&self) -> f32 {
        self.read_axes(Self::JOY_LY).max(0.0)
    }

    fn pitch(&self) -> f32 {
        -self.read_axes(Self::JOY_RY)
    }

    fn yaw(&self) -> f32 {
        -self.read_axes(Self::JOY_RX)
    }

    fn roll(&self) -> f32 {
        self.buttons[Self::BUTTON_L2] - self.buttons[Self::BUTTON_R2]
    }

    fn rotation(&self) -> Vec3 {
        Vec3 {
            x: self.pitch(),
            y: self.yaw(),
            z: self.roll(),
        }
    }
}
