use std::sync::Arc;

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
        0.1 * (inner.inc_vertical_velocity.get_and_reset() as f32
            - inner.dec_vertical_velocity.get_and_reset() as f32)
    }

    pub fn is_reset_pending(&self) -> bool {
        self.0.read().reset
    }

    pub fn do_reset(&self) {
        let mut inner = self.0.write();
        inner.reset = false;
        inner.strafe = Vec2::ZERO;
        inner.rotate = Vec3::ZERO;
        inner.inc_vertical_velocity.reset();
        inner.dec_vertical_velocity.reset();
    }

    pub fn update(
        &self,
        strafe: Vec2,
        rotate: Vec3,
        inc_vertical_velocity: bool,
        dec_vertical_velocity: bool,
        request_reset: bool,
    ) {
        let mut inner = self.0.write();
        inner.strafe = strafe;
        inner.rotate = rotate;
        inner.inc_vertical_velocity.update(inc_vertical_velocity);
        inner.dec_vertical_velocity.update(dec_vertical_velocity);
        inner.reset |= request_reset;
    }
}

#[derive(Default)]
struct Inner {
    reset: bool,
    strafe: Vec2,
    rotate: Vec3,
    inc_vertical_velocity: TapCounter,
    dec_vertical_velocity: TapCounter,
}

#[derive(Default)]
struct TapCounter {
    latch: bool,
    count: u32,
}

impl TapCounter {
    fn update(&mut self, pressed: bool) {
        match (pressed, self.latch) {
            (true, false) => {
                self.latch = true;
                self.count += 1;
            }
            (false, true) => {
                self.latch = false;
            }
            _ => (),
        }
    }

    fn get_and_reset(&mut self) -> u32 {
        let value = self.count;
        self.count = 0;
        value
    }

    fn reset(&mut self) {
        self.count = 0;
        self.latch = false;
    }
}
