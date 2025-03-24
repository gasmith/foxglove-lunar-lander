use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use clap::Parser;
use foxglove::schemas::{
    ArrowPrimitive, FrameTransform, Point3, Pose, Quaternion, SceneEntity, SceneUpdate,
    TriangleListPrimitive, Vector3,
};
use foxglove::{WebSocketServer, static_typed_channel};
use glam::{EulerRot, Quat, Vec3};
use noise::{NoiseFn, Perlin};
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;

static_typed_channel!(LANDSCAPE, "/landscape", SceneUpdate);
static_typed_channel!(LANDER, "/lander", SceneUpdate);
static_typed_channel!(FT, "/ft", FrameTransform);

#[derive(Debug, Parser)]
#[command(version, about)]
struct Config {
    #[arg(short, long)]
    seed: Option<u64>,

    #[arg(short, long, default_value_t = 3)]
    num_landing_zones: u16,
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("fatal: {e}");
    }
}

fn seed_rng(config: &Config) -> ChaCha8Rng {
    let seed = config.seed.unwrap_or_else(rand::random);
    println!("seed: {seed}");
    ChaCha8Rng::seed_from_u64(seed)
}

async fn run() -> anyhow::Result<()> {
    let config = Config::try_parse().context("failed to parse flags")?;
    let mut rng = seed_rng(&config);
    let server = WebSocketServer::new().name("fg-lander").start().await?;
    let mut height_map = HeightMap::new(rng.random());
    for _ in 0..config.num_landing_zones {
        height_map.add_random_landing_zone(&mut rng);
    }
    let landscape = Landscape::new(height_map);
    let lander = Lander::apollo();
    let controls = Arc::new(Controls::default());
    tokio::task::spawn(log_forever(landscape, lander, controls));
    tokio::signal::ctrl_c().await.ok();
    server.stop().await;
    Ok(())
}

async fn log_forever(landscape: Landscape, mut lander: Lander, controls: Arc<Controls>) {
    let dt = 1.0 / 30.0;
    loop {
        landscape.log();
        lander.step(dt, &controls);
        lander.log();
        tokio::time::sleep(Duration::from_secs_f32(dt)).await;
    }
}

const LANDING_ZONE_RADIUS: u32 = 5;
const LANDING_ZONE_BLEND_RADIUS: u32 = 7;

struct HeightMap {
    width: u32,
    depth: u32,
    z: Vec<f64>,
}
impl HeightMap {
    fn new(seed: u32) -> Self {
        let width = 100;
        let depth = 100;
        let noise_scale = 0.1;
        let z_scale = 2.0;
        let perlin = Perlin::new(seed);
        Self {
            width,
            depth,
            z: (0..width)
                .flat_map(|x| {
                    (0..depth).map(move |y| {
                        z_scale * perlin.get([x as f64 * noise_scale, y as f64 * noise_scale])
                    })
                })
                .collect(),
        }
    }

    fn add_random_landing_zone<R: Rng>(&mut self, rng: &mut R) {
        let center_x = rng.random_range(
            LANDING_ZONE_BLEND_RADIUS..self.width.saturating_sub(LANDING_ZONE_BLEND_RADIUS),
        );
        let center_y = rng.random_range(
            LANDING_ZONE_BLEND_RADIUS..self.depth.saturating_sub(LANDING_ZONE_BLEND_RADIUS),
        );
        self.add_landing_zone(center_x, center_y);
    }

    fn add_landing_zone(&mut self, center_x: u32, center_y: u32) {
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

    fn scene_entity(&self) -> SceneEntity {
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

struct Landscape {
    height_map: HeightMap,
    frame_transform: FrameTransform,
    scene_update: SceneUpdate,
}
impl Landscape {
    fn new(height_map: HeightMap) -> Self {
        let frame_transform = FrameTransform {
            parent_frame_id: "world".into(),
            child_frame_id: "landscape".into(),
            translation: Some(Vector3 {
                x: height_map.width as f64 / -2.0,
                y: height_map.depth as f64 / -2.0,
                z: 0.0,
            }),
            ..Default::default()
        };
        let scene_update = SceneUpdate {
            entities: vec![height_map.scene_entity()],
            ..Default::default()
        };
        Self {
            height_map,
            frame_transform,
            scene_update,
        }
    }

    fn log(&self) {
        FT.log(&self.frame_transform);
        LANDSCAPE.log(&self.scene_update);
    }
}

#[derive(Default)]
struct Controls {
    thrust: f32,
    rotation: Vec3,
}

/// Moon gravitational vector in meters/sec/sec.
const MOON_GRAVITY: Vec3 = Vec3 {
    x: 0.0,
    y: 0.0,
    z: -1.62,
};

struct Lander {
    position: Vec3,
    velocity: Vec3,
    rotation: Quat,
    angular_velocity: Vec3,
    mass: f32,
    thrust_power: f32,
    torque_power: f32,
}

trait IntoFg<T> {
    fn into_fg(self) -> T;
}
impl IntoFg<Vector3> for Vec3 {
    fn into_fg(self) -> Vector3 {
        Vector3 {
            x: self.x.into(),
            y: self.y.into(),
            z: self.z.into(),
        }
    }
}
impl IntoFg<Quaternion> for Quat {
    fn into_fg(self) -> Quaternion {
        Quaternion {
            x: self.x.into(),
            y: self.y.into(),
            z: self.z.into(),
            w: self.w.into(),
        }
    }
}

// Apollo lunar lander.
const LANDER_INITIAL_MASS_KG: f32 = 15_200.0;
const LANDER_THRUST_POWER_N: f32 = 44_500.0;
const LANDER_TORQUE_POWER_N: f32 = 3_000.0;

impl Lander {
    fn apollo() -> Self {
        Self {
            position: Vec3::Z * 100.0,
            velocity: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            angular_velocity: Vec3::ZERO,
            mass: LANDER_INITIAL_MASS_KG,
            thrust_power: LANDER_THRUST_POWER_N,
            torque_power: LANDER_TORQUE_POWER_N,
        }
    }

    fn step(&mut self, dt: f32, controls: &Controls) {
        // Gravity.
        self.velocity += MOON_GRAVITY * dt;

        // Apply thrust.
        let thrust_dir = self.rotation * Vec3::Z;
        let thrust_force = thrust_dir * controls.thrust * self.thrust_power;
        self.velocity += (thrust_force / self.mass) * dt;

        // Apply torque.
        let torque = controls.rotation * self.torque_power;
        self.angular_velocity += torque * dt;

        // Update position & orientation.
        self.position += self.velocity * dt;
        self.rotation *= Quat::from_euler(
            EulerRot::XYZ,
            self.angular_velocity.x * dt,
            self.angular_velocity.y * dt,
            self.angular_velocity.z * dt,
        );
    }

    fn frame_transform(&self) -> FrameTransform {
        FrameTransform {
            parent_frame_id: "world".into(),
            child_frame_id: "lander".into(),
            translation: Some(self.position.into_fg()),
            ..Default::default()
        }
    }

    fn scene_entity(&self) -> SceneEntity {
        SceneEntity {
            frame_id: "lander".into(),
            arrows: vec![ArrowPrimitive {
                pose: Some(Pose {
                    orientation: Some(self.rotation.into_fg()),
                    ..Default::default()
                }),
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    fn log(&self) {
        FT.log(&self.frame_transform());
        LANDER.log(&SceneUpdate {
            entities: vec![self.scene_entity()],
            ..Default::default()
        });
    }
}
