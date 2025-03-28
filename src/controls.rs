use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use glam::{Vec2, Vec3};
use parking_lot::RwLock;

#[derive(Default, Clone)]
pub struct Controls(Arc<RwLock<Inner>>);
impl Controls {
    pub fn strafe(&self) -> Vec2 {
        self.0.read().strafe
    }

    pub fn rotate(&self) -> Vec3 {
        self.0.read().rotate
    }

    pub fn get_and_reset_vertical_velocity_delta(&self) -> f32 {
        let mut inner = self.0.write();
        0.2 * (inner.inc_vertical_velocity.get_and_reset() as f32
            - inner.dec_vertical_velocity.get_and_reset() as f32)
    }

    pub fn get_reset_requested(&self) -> bool {
        self.0.read().reset.get() > 0
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
        let mut inner = self.0.write();
        inner.reset.reset(hard);
        inner.strafe = Vec2::ZERO;
        inner.rotate = Vec3::ZERO;
        inner.inc_vertical_velocity.reset(hard);
        inner.dec_vertical_velocity.reset(hard);
    }

    pub fn update(
        &self,
        reset: bool,
        strafe: Vec2,
        rotate: Vec3,
        inc_vertical_velocity: bool,
        dec_vertical_velocity: bool,
    ) {
        let mut inner = self.0.write();
        inner.reset.update(reset);
        inner.strafe = strafe;
        inner.rotate = rotate;
        inner.inc_vertical_velocity.update(inc_vertical_velocity);
        inner.dec_vertical_velocity.update(dec_vertical_velocity);
    }
}

struct Inner {
    reset: Button,
    strafe: Vec2,
    rotate: Vec3,
    inc_vertical_velocity: Button,
    dec_vertical_velocity: Button,
}
impl Default for Inner {
    fn default() -> Self {
        let repeat = Duration::from_millis(100);
        Self {
            reset: Button::default(),
            strafe: Vec2::default(),
            rotate: Vec3::default(),
            inc_vertical_velocity: Button::with_repeater(repeat),
            dec_vertical_velocity: Button::with_repeater(repeat),
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
