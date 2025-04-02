use foxglove::LazyChannel;
use foxglove::schemas::{
    Color, FrameTransform, SceneEntity, SceneUpdate, SpherePrimitive, Vector3,
};
use glam::Vec3;

use crate::convert::IntoFg;

static LANDING_ZONE: LazyChannel<SceneUpdate> = LazyChannel::new("/landing_zone");

pub struct LandingZone {
    frame_transform: FrameTransform,
    scene_update: SceneUpdate,
}
impl From<Vec3> for LandingZone {
    fn from(value: Vec3) -> Self {
        LandingZone::new(value)
    }
}
impl LandingZone {
    pub fn new(center: Vec3) -> Self {
        LANDING_ZONE.init();
        let frame_transform = FrameTransform {
            parent_frame_id: "landscape".into(),
            child_frame_id: "landing_zone".into(),
            translation: Some(center.into_fg()),
            ..Default::default()
        };
        let scene_update = SceneUpdate {
            entities: vec![SceneEntity {
                id: "landing_zone".into(),
                frame_id: "landing_zone".into(),
                spheres: vec![SpherePrimitive {
                    size: Some(Vector3 {
                        x: 2.0,
                        y: 2.0,
                        z: 1.0,
                    }),
                    color: Some(Color {
                        r: 1.0,
                        g: 1.0,
                        b: 0.0,
                        a: 0.7,
                    }),
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        };
        Self {
            frame_transform,
            scene_update,
        }
    }

    pub fn frame_transform(&self) -> FrameTransform {
        self.frame_transform.clone()
    }

    pub fn log_scene(&self) {
        LANDING_ZONE.log(&self.scene_update);
    }
}
