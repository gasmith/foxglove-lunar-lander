use foxglove::schemas::{FrameTransform, SceneUpdate, Vector3};
use foxglove::static_typed_channel;
use glam::Vec3;
use rand::prelude::*;

mod height_map;
mod landing_zone;
use height_map::HeightMap;
use landing_zone::LandingZone;

use crate::parameters::Parameters;

static_typed_channel!(LANDSCAPE, "/landscape", SceneUpdate);

pub struct Landscape {
    frame_transform: FrameTransform,
    scene_update: SceneUpdate,
    landing_zone: LandingZone,
    lander_init_position: Vec3,
    hidden: bool,
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
        let lander_init_position =
            height_map.center() - landing_zone_center + (Vec3::Z * params.lander_init_altitude());
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
            lander_init_position,
            hidden: false,
        }
    }

    pub fn set_hidden(&mut self, hidden: bool) {
        self.hidden = hidden;
    }

    pub fn lander_init_position(&self) -> Vec3 {
        self.lander_init_position
    }

    pub fn frame_transforms(&self) -> Vec<FrameTransform> {
        vec![
            self.frame_transform.clone(),
            self.landing_zone.frame_transform(),
        ]
    }

    pub fn log_scene(&self) {
        if self.hidden {
            LANDSCAPE.log(&SceneUpdate::default());
        } else {
            LANDSCAPE.log(&self.scene_update);
            self.landing_zone.log_scene();
        }
    }
}
