use std::{fs::File, io::Read, path::Path};

use anyhow::Context;
use serde::Deserialize;

const DEFAULT_JOYSTICK_DEAD_ZONE: f32 = 0.10;

/// Gamepad configuration.
#[derive(Debug, Deserialize)]
pub struct Gamepad {
    map: GamepadMap,
    joystick_dead_zone: Option<f32>,
}

/// A map of gamepad buttons.
#[derive(Debug, Deserialize)]
struct GamepadMap {
    axis_strafe_x: usize,
    axis_strafe_y: usize,
    axis_roll: usize,
    axis_pitch: usize,
    button_yaw_left: usize,
    button_yaw_right: usize,
    button_vertical_velocity_up: usize,
    button_vertical_velocity_down: usize,
    button_start: usize,
}

impl Gamepad {
    pub fn from_json_file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let file =
            File::open(path).with_context(|| format!("failed to open gamepad config: {path:?}"))?;
        Self::from_json_reader(file)
    }

    fn from_json_reader(r: impl Read) -> anyhow::Result<Self> {
        serde_json::from_reader(r).context("failed to load gamepad config")
    }

    fn read_axis(&self, msg: &GamepadMsg, idx: usize) -> f32 {
        let raw = msg.read_axis(idx);
        let dead_zone = self
            .joystick_dead_zone
            .unwrap_or(DEFAULT_JOYSTICK_DEAD_ZONE);
        if raw.abs() < dead_zone { 0.0 } else { raw }
    }

    pub fn read_strafe_x(&self, msg: &GamepadMsg) -> f32 {
        self.read_axis(msg, self.map.axis_strafe_x)
    }

    pub fn read_strafe_y(&self, msg: &GamepadMsg) -> f32 {
        self.read_axis(msg, self.map.axis_strafe_y)
    }

    pub fn read_pitch(&self, msg: &GamepadMsg) -> f32 {
        -self.read_axis(msg, self.map.axis_pitch)
    }

    pub fn read_roll(&self, msg: &GamepadMsg) -> f32 {
        -self.read_axis(msg, self.map.axis_roll)
    }

    pub fn read_yaw_left(&self, msg: &GamepadMsg) -> bool {
        msg.read_button(self.map.button_yaw_left)
    }

    pub fn read_yaw_right(&self, msg: &GamepadMsg) -> bool {
        msg.read_button(self.map.button_yaw_right)
    }

    pub fn read_vertical_velocity_up(&self, msg: &GamepadMsg) -> bool {
        msg.read_button(self.map.button_vertical_velocity_up)
    }

    pub fn read_vertical_velocity_down(&self, msg: &GamepadMsg) -> bool {
        msg.read_button(self.map.button_vertical_velocity_down)
    }

    pub fn read_start(&self, msg: &GamepadMsg) -> bool {
        msg.read_button(self.map.button_start)
    }
}

/// A message containing gamepad state.
#[derive(Debug, Deserialize)]
pub struct GamepadMsg {
    axes: Vec<f32>,
    buttons: Vec<f32>,
}

#[allow(dead_code)]
impl GamepadMsg {
    fn read_axis(&self, idx: usize) -> f32 {
        self.axes[idx].clamp(-1.0, 1.0)
    }

    fn read_button(&self, idx: usize) -> bool {
        self.buttons[idx] > 0.0
    }
}
