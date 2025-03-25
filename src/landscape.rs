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

static_typed_channel!(LANDSCAPE, "/landscape", SceneUpdate);
static_typed_channel!(LANDSCAPE_FT, "/landscape_ft", FrameTransform);

pub struct Landscape {
    height_map: HeightMap,
    frame_transform: FrameTransform,
    scene_update: SceneUpdate,
    landing_zone: LandingZone,
}
impl Landscape {
    pub fn new<R: Rng>(rng: &mut R) -> Self {
        let mut height_map = HeightMap::new(rng.random());
        height_map.set_random_landing_zone(rng);
        Landscape::from_height_map(height_map)
    }

    fn from_height_map(height_map: HeightMap) -> Self {
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
        let landing_zone = LandingZone::new(height_map.landing_zone());
        Self {
            height_map,
            frame_transform,
            landing_zone,
            scene_update,
        }
    }

    pub fn lander_start_position(&self) -> Vec3 {
        self.height_map.center() - self.height_map.landing_zone() + (Vec3::Z * 100.0)
    }

    pub fn log(&self) {
        LANDSCAPE_FT.log(&self.frame_transform);
        LANDSCAPE.log(&self.scene_update);
        self.landing_zone.log();
    }
}
