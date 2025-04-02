use foxglove::LazyChannel;
use foxglove::schemas::{
    Color, FrameTransform, SceneEntity, SceneEntityDeletion, SceneUpdate, TextPrimitive, Vector3,
};

use crate::landing::LandingStatus;

static BANNER: LazyChannel<SceneUpdate> = LazyChannel::new("/banner");

#[derive(Default)]
pub struct Banner(SceneUpdate);

impl Banner {
    fn new(text: &str, r: f64, g: f64, b: f64) -> Self {
        Self(SceneUpdate {
            entities: vec![SceneEntity {
                id: "banner".into(),
                frame_id: "banner".into(),
                texts: vec![TextPrimitive {
                    pose: None,
                    billboard: true,
                    font_size: 48.0,
                    scale_invariant: true,
                    color: Some(Color { r, g, b, a: 0.75 }),
                    text: text.to_string(),
                }],
                ..Default::default()
            }],
            ..Default::default()
        })
    }
    pub fn press_start() -> Self {
        Self::new("PRESS START", 1.0, 0.0, 1.0)
    }

    pub fn landing_status(status: LandingStatus) -> Self {
        let ((r, g, b), text) = match status {
            LandingStatus::Landed => ((0.0, 1.0, 0.0), "LANDED"),
            LandingStatus::Missed => ((1.0, 1.0, 0.0), "MISSED"),
            LandingStatus::Crashed => ((1.0, 0.0, 0.0), "YOU DIED"),
        };
        Self::new(text, r, g, b)
    }

    pub fn frame_transform(&self) -> FrameTransform {
        FrameTransform {
            parent_frame_id: "lander".into(),
            child_frame_id: "banner".into(),
            translation: Some(Vector3 {
                z: 5.0,
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    pub fn log_scene(&self) {
        BANNER.log(&self.0)
    }

    pub fn clear_scene() {
        BANNER.log(&SceneUpdate {
            deletions: vec![SceneEntityDeletion {
                id: "banner".into(),
                ..Default::default()
            }],
            ..Default::default()
        })
    }
}
