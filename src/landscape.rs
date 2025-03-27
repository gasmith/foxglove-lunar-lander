use foxglove::{
    schemas::{FrameTransform, SceneUpdate, Vector3},
    static_typed_channel,
};
use glam::Vec3;
use rand::prelude::*;

mod height_map;
mod landing_zone;
use height_map::HeightMap;
use landing_zone::LandingZone;

use crate::parameters::Parameters;

pub const DEFAULT_LANDSCAPE_WIDTH: u32 = 200;
pub const DEFAULT_LANDING_ZONE_MIN_DISTANCE: u32 = 30;
pub const DEFAULT_LANDING_ZONE_MAX_DISTANCE: u32 = 70;
pub const DEFAULT_LANDING_ZONE_RADIUS: u32 = 20;
pub const DEFAULT_LANDER_START_ALTITUDE: u32 = 200;

static_typed_channel!(LANDSCAPE, "/landscape", SceneUpdate);
static_typed_channel!(LANDSCAPE_FT, "/landscape_ft", FrameTransform);

pub struct Landscape {
    frame_transform: FrameTransform,
    scene_update: SceneUpdate,
    landing_zone: LandingZone,
    lander_start_position: Vec3,
}
impl Landscape {
    pub fn new<R: Rng>(rng: &mut R, params: &Parameters) -> Self {
        let mut height_map = HeightMap::new(rng, params.landscape_width());
        let landing_zone_center = height_map.create_random_landing_zone(
            rng,
            params.landing_zone_min_distance(),
            params.landing_zone_max_distance(),
            params.landing_zone_radius(),
        );
        let lander_start_position = height_map.center() - landing_zone_center
            + (Vec3::Z * params.lander_start_altitude() as f32);
        let frame_transform = FrameTransform {
            parent_frame_id: "world".into(),
            child_frame_id: "landscape".into(),
            translation: Some(Vector3 {
                x: f64::from(height_map.width()) / -2.0,
                y: f64::from(height_map.depth()) / -2.0,
                z: 0.0,
            }),
            ..Default::default()
        };
        let scene_update = SceneUpdate {
            entities: vec![height_map.scene_entity()],
            ..Default::default()
        };
        Self {
            frame_transform,
            scene_update,
            landing_zone: landing_zone_center.into(),
            lander_start_position,
        }
    }

    pub fn lander_start_position(&self) -> Vec3 {
        self.lander_start_position
    }

    pub fn log_static(&self) {
        LANDSCAPE_FT.log_static(&self.frame_transform);
        LANDSCAPE.log_static(&self.scene_update);
        self.landing_zone.log_static();
    }
}
