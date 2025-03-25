use foxglove::schemas::{Point3, SceneEntity, TriangleListPrimitive};
use glam::Vec3;
use noise::{NoiseFn, Perlin};
use rand::prelude::*;

use crate::{LANDING_ZONE_BLEND_RADIUS, LANDING_ZONE_RADIUS};

pub struct HeightMap {
    width: u32,
    depth: u32,
    landing_zone: Vec3,
    z: Vec<f64>,
}
impl HeightMap {
    pub fn new(seed: u32) -> Self {
        let width = 200;
        let depth = 200;
        let noise_scale = 0.1;
        let z_scale = 2.0;
        let perlin = Perlin::new(seed);
        Self {
            width,
            depth,
            landing_zone: Vec3::ZERO,
            z: (0..width)
                .flat_map(|x| {
                    (0..depth).map(move |y| {
                        z_scale * perlin.get([x as f64 * noise_scale, y as f64 * noise_scale])
                    })
                })
                .collect(),
        }
    }

    pub fn landing_zone(&self) -> Vec3 {
        self.landing_zone
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn depth(&self) -> u32 {
        self.depth
    }

    pub fn center(&self) -> Vec3 {
        Vec3 {
            x: self.width as f32 / 2.0,
            y: self.depth as f32 / 2.0,
            z: 0.0,
        }
    }

    fn random_range<R: Rng>(&self, rng: &mut R, margin: u32, max: u32) -> u32 {
        rng.random_range(margin..max.saturating_sub(margin))
    }

    pub fn set_random_landing_zone<R: Rng>(&mut self, rng: &mut R) {
        let x = self.random_range(rng, LANDING_ZONE_BLEND_RADIUS * 2, self.width);
        let y = self.random_range(rng, LANDING_ZONE_BLEND_RADIUS * 2, self.depth);
        self.set_landing_zone(x, y);
    }

    fn set_landing_zone(&mut self, center_x: u32, center_y: u32) {
        let radius = LANDING_ZONE_RADIUS;
        let blend_radius = LANDING_ZONE_BLEND_RADIUS;
        let center_z = self.get(center_x, center_y);

        for ix in (center_x - blend_radius)..=(center_x + blend_radius) {
            for iy in (center_y - blend_radius)..=(center_y + blend_radius) {
                if ix < self.width && iy < self.depth {
                    let dx = ix as f64 - center_x as f64;
                    let dy = iy as f64 - center_y as f64;
                    let dist = (dx * dx + dy * dy).sqrt();
                    if dist <= blend_radius as f64 {
                        // t=0 within the flat zone, t=1 at the edge of the blend radius.
                        let t = ((dist - (radius as f64)) / 2.0).clamp(0.0, 1.0);
                        let z = (1.0 - t) * center_z + t * self.get(ix, iy);
                        self.set(ix, iy, z);
                    }
                }
            }
        }

        self.landing_zone = Vec3 {
            x: center_x as f32,
            y: center_y as f32,
            z: center_z as f32,
        };
    }

    fn set(&mut self, ix: u32, iy: u32, z: f64) {
        let idx = (ix * self.depth + iy) as usize;
        self.z[idx] = z;
    }

    fn get(&self, ix: u32, iy: u32) -> f64 {
        let idx = (ix * self.depth + iy) as usize;
        self.z[idx]
    }

    fn get_point3(&self, ix: u32, iy: u32) -> Point3 {
        Point3 {
            x: ix as f64,
            y: iy as f64,
            z: self.get(ix, iy),
        }
    }

    pub fn scene_entity(&self) -> SceneEntity {
        SceneEntity {
            frame_id: "landscape".into(),
            triangles: (0..(self.width - 1))
                .map(|ix| TriangleListPrimitive {
                    points: (0..(self.depth - 1))
                        .flat_map(|iy| {
                            vec![
                                // Triangle 1
                                self.get_point3(ix, iy),
                                self.get_point3(ix, iy + 1),
                                self.get_point3(ix + 1, iy),
                                // Triangle 2
                                self.get_point3(ix + 1, iy),
                                self.get_point3(ix, iy + 1),
                                self.get_point3(ix + 1, iy + 1),
                            ]
                        })
                        .collect(),
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        }
    }
}
