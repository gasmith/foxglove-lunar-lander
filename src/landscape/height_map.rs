use foxglove::schemas::{Point3, SceneEntity, TriangleListPrimitive};
use glam::{Vec2, Vec3};
use noise::{NoiseFn, Perlin};
use rand::prelude::*;

const DEFAULT_NOISE_SCALE: f64 = 0.1;
const DEFAULT_Z_SCALE: f64 = 2.0;

/// A square map of z values.
pub struct HeightMap {
    width: u32,
    z: Vec<f64>,
}
impl HeightMap {
    pub fn new<R: Rng>(rng: &mut R, width: u32) -> HeightMap {
        let z_scale = DEFAULT_Z_SCALE;
        let noise_scale = DEFAULT_NOISE_SCALE;
        let perlin = Perlin::new(rng.random());
        let z = (0..width)
            .flat_map(|x| {
                (0..width).map(move |y| {
                    z_scale * perlin.get([x as f64 * noise_scale, y as f64 * noise_scale])
                })
            })
            .collect();
        Self { width, z }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn depth(&self) -> u32 {
        self.width
    }

    pub fn center(&self) -> Vec3 {
        let x = self.width as f32 / 2.0;
        Vec3::new(x, x, 0.0)
    }

    fn center2(&self) -> Vec2 {
        let x = self.width as f32 / 2.0;
        Vec2::new(x, x)
    }

    pub fn create_random_landing_zone<R: Rng>(
        &mut self,
        rng: &mut R,
        min_distance: u32,
        max_distance: u32,
        radius: u32,
    ) -> Vec3 {
        let angle = rng.random_range(0.0..std::f32::consts::TAU);
        let len = rng.random_range((min_distance as f32)..(max_distance as f32));
        let delta = Vec2::new(angle.cos(), angle.sin()) * len;
        let vec = delta + self.center2();
        self.create_landing_zone(vec.x as u32, vec.y as u32, radius)
    }

    fn create_landing_zone(&mut self, center_x: u32, center_y: u32, radius: u32) -> Vec3 {
        let blend_radius = radius + 3;
        let center_z = self.get(center_x, center_y);

        for ix in (center_x - blend_radius)..=(center_x + blend_radius) {
            for iy in (center_y - blend_radius)..=(center_y + blend_radius) {
                if ix < self.width && iy < self.width {
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

        Vec3 {
            x: center_x as f32,
            y: center_y as f32,
            z: center_z as f32,
        }
    }

    fn set(&mut self, ix: u32, iy: u32, z: f64) {
        let idx = (ix * self.width + iy) as usize;
        self.z[idx] = z;
    }

    fn get(&self, ix: u32, iy: u32) -> f64 {
        let idx = (ix * self.width + iy) as usize;
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
                    points: (0..(self.width - 1))
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
