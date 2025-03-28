use std::time::{Duration, Instant};

use glam::{Vec2, Vec3};
use parking_lot::RwLock;

use crate::gamepad::{Gamepad, GamepadMsg};

pub struct Controls {
    gamepad: Gamepad,
    state: RwLock<State>,
}
impl Controls {
    pub fn new(gamepad: Gamepad) -> Self {
        Self {
            gamepad,
            state: Default::default(),
        }
    }

    pub fn update_from_msg(&self, msg: &GamepadMsg) {
        let mut state = self.state.write();
        state.reset.update(self.gamepad.read_start(msg));
        state.strafe = Vec2 {
            x: self.gamepad.read_strafe_x(msg),
            y: self.gamepad.read_strafe_y(msg),
        };
        // should use Button::with_repeat?
        let yaw = match (
            self.gamepad.read_yaw_left(msg),
            self.gamepad.read_yaw_right(msg),
        ) {
            (true, false) => 0.5,
            (false, true) => -0.5,
            _ => 0.0,
        };
        state.rotate = Vec3 {
            x: self.gamepad.read_pitch(msg),
            y: self.gamepad.read_roll(msg),
            z: yaw,
        };
        state
            .vertical_velocity_up
            .update(self.gamepad.read_vertical_velocity_up(msg));
        state
            .vertical_velocity_down
            .update(self.gamepad.read_vertical_velocity_down(msg));
    }

    pub fn strafe(&self) -> Vec2 {
        self.state.read().strafe
    }

    pub fn rotate(&self) -> Vec3 {
        self.state.read().rotate
    }

    pub fn get_and_reset_vertical_velocity_delta(&self) -> f32 {
        let mut state = self.state.write();
        0.2 * (state.vertical_velocity_up.get_and_reset() as f32
            - state.vertical_velocity_down.get_and_reset() as f32)
    }

    pub fn get_reset_requested(&self) -> bool {
        self.state.read().reset.get() > 0
    }

    /// Resets all values and button-press state.
    pub fn hard_reset(&self) {
        self.reset(true);
    }

    /// Resets all values, but retains button-press state.
    pub fn soft_reset(&self) {
        self.reset(false);
    }

    fn reset(&self, hard: bool) {
        let mut inner = self.state.write();
        inner.reset.reset(hard);
        inner.strafe = Vec2::ZERO;
        inner.rotate = Vec3::ZERO;
        inner.vertical_velocity_up.reset(hard);
        inner.vertical_velocity_down.reset(hard);
    }
}

struct State {
    reset: Button,
    strafe: Vec2,
    rotate: Vec3,
    vertical_velocity_up: Button,
    vertical_velocity_down: Button,
}
impl Default for State {
    fn default() -> Self {
        let repeat = Duration::from_millis(100);
        Self {
            reset: Button::default(),
            strafe: Vec2::default(),
            rotate: Vec3::default(),
            vertical_velocity_up: Button::with_repeater(repeat),
            vertical_velocity_down: Button::with_repeater(repeat),
        }
    }
}

#[derive(Default)]
struct Button {
    pressed: bool,
    count: u32,
    timeout: Option<Duration>,
    deadline: Option<Instant>,
}

impl Button {
    fn with_repeater(timeout: Duration) -> Self {
        Self {
            timeout: Some(timeout),
            ..Default::default()
        }
    }

    fn update(&mut self, pressed: bool) {
        if self.deadline.is_some_and(|t| t > Instant::now()) {
            self.pressed = false;
            self.deadline = None;
        }
        match (pressed, self.pressed) {
            (true, false) => {
                self.pressed = true;
                self.count += 1;
                if let Some(timeout) = self.timeout {
                    self.deadline = Some(Instant::now() + timeout);
                }
            }
            (false, true) => {
                self.pressed = false;
            }
            _ => (),
        }
    }

    fn get(&self) -> u32 {
        self.count
    }

    fn get_and_reset(&mut self) -> u32 {
        let value = self.count;
        self.count = 0;
        value
    }

    fn reset(&mut self, hard: bool) {
        self.count = 0;
        if hard {
            self.pressed = false;
            self.deadline = None;
        }
    }
}
