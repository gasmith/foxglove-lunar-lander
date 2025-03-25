use std::sync::Arc;

use glam::Vec3;
use parking_lot::RwLock;

#[derive(Default, Clone)]
pub struct Controls(Arc<RwLock<Inner>>);
impl Controls {
    pub fn thrust(&self) -> f32 {
        self.0.read().thrust
    }

    pub fn rotation(&self) -> Vec3 {
        self.0.read().rotation
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
        inner.thrust = 0.0;
        inner.rotation = Vec3::ZERO;
    }

    pub fn update(&self, thrust: f32, rotation: Vec3) {
        let mut inner = self.0.write();
        inner.thrust = thrust;
        inner.rotation = rotation;
    }
}

#[derive(Default)]
struct Inner {
    reset: bool,
    thrust: f32,
    rotation: Vec3,
}
