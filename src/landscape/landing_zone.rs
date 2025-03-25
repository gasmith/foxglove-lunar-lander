use foxglove::{
    schemas::{Color, FrameTransform, SceneEntity, SceneUpdate, SpherePrimitive, Vector3},
    static_typed_channel,
};
use glam::Vec3;

use crate::convert::IntoFg;

static_typed_channel!(LANDING_ZONE, "/landing_zone", SceneUpdate);
static_typed_channel!(LANDING_ZONE_FT, "/landing_zone_ft", FrameTransform);

pub struct LandingZone {
    frame_transform: FrameTransform,
    scene_update: SceneUpdate,
}
impl LandingZone {
    pub fn new(landing_zone: Vec3) -> Self {
        let frame_transform = FrameTransform {
            parent_frame_id: "landscape".into(),
            child_frame_id: "landing_zone".into(),
            translation: Some(landing_zone.into_fg()),
            ..Default::default()
        };
        let scene_update = SceneUpdate {
            entities: vec![SceneEntity {
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

    pub fn log(&self) {
        LANDING_ZONE_FT.log(&self.frame_transform);
        LANDING_ZONE.log(&self.scene_update);
    }
}
