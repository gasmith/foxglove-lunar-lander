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

    pub fn rate_of_descent(&self) -> f32 {
        self.0.read().rate_of_descent
    }

    pub fn is_reset_pending(&self) -> bool {
        self.0.read().reset
    }

    pub fn request_reset(&self) {
        self.0.write().reset = true;
    }

    pub fn do_reset(&self) {
        let mut inner = self.0.write();
        inner.reset = false;
        inner.rate_of_descent = 0.0;
        inner.rotate = Vec3::ZERO;
    }

    pub fn update(&self, strafe: Vec2, rotate: Vec3, delta_rate_of_descent: f32) {
        let mut inner = self.0.write();
        inner.strafe = strafe;
        inner.rotate = rotate;
        inner.rate_of_descent += delta_rate_of_descent;
    }
}

#[derive(Default)]
struct Inner {
    reset: bool,
    strafe: Vec2,
    rotate: Vec3,
    rate_of_descent: f32,
}
