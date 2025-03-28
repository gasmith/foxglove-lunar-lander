use std::sync::Arc;

use foxglove::websocket::{Client, ClientChannel, ServerListener};

use crate::controls::{Controls, GamepadMsg};
use crate::parameters::Parameters;

pub struct Listener {
    params: Arc<Parameters>,
    controls: Arc<Controls>,
}
impl Listener {
    pub fn new(params: Arc<Parameters>, controls: Arc<Controls>) -> Self {
        Self { params, controls }
    }

    pub fn into_listener(self) -> Arc<dyn ServerListener> {
        Arc::new(self)
    }
}
impl ServerListener for Listener {
    fn on_client_advertise(&self, _client: Client, channel: &ClientChannel) {
        // When a client advertises a new joystick, do a hard reset to discard button-press state.
        if channel.schema_name == "sensor_msgs/Joy" {
            self.controls.hard_reset();
        }
    }

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
        self.controls.update_from_msg(&msg);
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
