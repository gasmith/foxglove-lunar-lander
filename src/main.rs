use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use bytes::Bytes;
use clap::Parser;
use foxglove::schemas::{
    Color, FrameTransform, ModelPrimitive, Point3, Pose, Quaternion, SceneEntity, SceneUpdate,
    SpherePrimitive, TriangleListPrimitive, Vector3,
};
use foxglove::websocket::{Capability, Client, ClientChannel, ServerListener};
use foxglove::{WebSocketServer, static_typed_channel};
use glam::{EulerRot, Quat, Vec3};
use noise::{NoiseFn, Perlin};
use parking_lot::RwLock;
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use serde::Deserialize;

static_typed_channel!(LANDSCAPE, "/landscape", SceneUpdate);
static_typed_channel!(LANDSCAPE_FT, "/landscape_ft", FrameTransform);
static_typed_channel!(LANDING_ZONE, "/landing_zone", SceneUpdate);
static_typed_channel!(LANDING_ZONE_FT, "/landing_zone_ft", FrameTransform);
static_typed_channel!(LANDER, "/lander", SceneUpdate);
static_typed_channel!(LANDER_FT, "/lander_ft", FrameTransform);
static_typed_channel!(LANDER_ANGULAR_VELOCITY, "/lander_angular_velocity", Vector3);
static_typed_channel!(
    LANDER_HORIZONTAL_VELOCITY,
    "/lander_horizontal_velocity",
    Vector3
);
static_typed_channel!(
    LANDER_VERTICAL_VELOCITY,
    "/lander_vertical_velocity",
    Vector3
);
static_typed_channel!(LANDER_ORIENTATION, "/lander_orientation", Quaternion);

static APOLLO_LUNAR_MODULE_URDF: &[u8] = include_bytes!("../assets/urdf/apollo-lunar-module.urdf");
static APOLLO_LUNAR_MODULE_STL: &[u8] = include_bytes!("../assets/meshes/apollo-lunar-module.stl");

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

    let mut height_map = HeightMap::new(rng.random());
    height_map.set_random_landing_zone(&mut rng);
    let landscape = Landscape::new(height_map);

    let controls = SharedControls::default();
    let server = WebSocketServer::new()
        .name("fg-lander")
        .capabilities([Capability::ClientPublish])
        .supported_encodings(["json"])
        .fetch_asset_handler_blocking_fn(fetch_asset)
        .listener(Listener::new(controls.clone()).into_listener())
        .start()
        .await?;

    tokio::task::spawn(game_loop(landscape, controls));
    tokio::signal::ctrl_c().await.ok();
    server.stop().await;
    Ok(())
}

fn fetch_asset(_client: Client, url: String) -> anyhow::Result<Bytes> {
    println!("fetch asset: {url}");
    match url.as_str() {
        "package://meshes/apollo-lunar-module.stl" => {
            Ok(Bytes::from_static(APOLLO_LUNAR_MODULE_STL))
        }
        "package://urdf/apollo-lunar-module.urdf" => {
            Ok(Bytes::from_static(APOLLO_LUNAR_MODULE_URDF))
        }
        _ => Err(anyhow::anyhow!("not found")),
    }
}

async fn game_loop(landscape: Landscape, controls: SharedControls) {
    let dt = 1.0 / 30.0;

    let mut lander = Lander::apollo(landscape.lander_start_position());
    loop {
        if controls.is_reset_pending() || lander.position.z < 0.0 {
            lander = Lander::apollo(landscape.lander_start_position());
            controls.do_reset();
        }
        landscape.log();
        lander.step(dt, &controls);
        lander.log();
        tokio::time::sleep(Duration::from_secs_f32(dt)).await;
    }
}

struct Listener {
    controls: SharedControls,
}
impl Listener {
    fn new(controls: SharedControls) -> Self {
        Self { controls }
    }

    fn into_listener(self) -> Arc<dyn ServerListener> {
        Arc::new(self)
    }
}
impl ServerListener for Listener {
    fn on_message_data(&self, _client: Client, channel: &ClientChannel, payload: &[u8]) {
        if channel.schema_name != "sensor_msgs/Joy" {
            return;
        }
        let msg: GamepadMsg = match serde_json::from_slice(payload) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("failed to deserialize /joy message: {e}");
                return;
            }
        };
        let thrust = msg.thrust();
        let rotation = msg.rotation();
        if msg.reset() {
            self.controls.request_reset();
        }
        self.controls.update(thrust, rotation);
    }
}

#[derive(Debug, Deserialize)]
struct GamepadMsg {
    axes: Vec<f32>,
    buttons: Vec<f32>,
}

#[allow(dead_code)]
impl GamepadMsg {
    const JOY_LX: usize = 0;
    const JOY_LY: usize = 1;
    const JOY_RX: usize = 2;
    const JOY_RY: usize = 3;
    const BUTTON_X: usize = 0;
    const BUTTON_O: usize = 1;
    const BUTTON_S: usize = 2;
    const BUTTON_T: usize = 3;
    const BUTTON_L1: usize = 4;
    const BUTTON_R1: usize = 5;
    const BUTTON_L2: usize = 6;
    const BUTTON_R2: usize = 7;
    const BUTTON_UP: usize = 12;
    const BUTTON_DOWN: usize = 13;
    const BUTTON_LEFT: usize = 14;
    const BUTTON_RIGHT: usize = 15;
    const BUTTON_PS: usize = 16;
    const JOY_DEAD_ZONE_TOL: f32 = 0.10;

    fn reset(&self) -> bool {
        self.buttons[Self::BUTTON_PS] > 0.0
    }

    fn read_axes(&self, idx: usize) -> f32 {
        let raw = self.axes[idx];
        if raw.abs() < Self::JOY_DEAD_ZONE_TOL {
            0.0
        } else {
            raw.clamp(-1.0, 1.0)
        }
    }

    fn thrust(&self) -> f32 {
        self.read_axes(Self::JOY_LY).max(0.0)
    }

    fn pitch(&self) -> f32 {
        self.read_axes(Self::JOY_RY)
    }

    fn yaw(&self) -> f32 {
        self.read_axes(Self::JOY_RX)
    }

    fn roll(&self) -> f32 {
        -self.buttons[Self::BUTTON_L2] + self.buttons[Self::BUTTON_R2]
    }

    fn rotation(&self) -> Vec3 {
        Vec3 {
            x: self.pitch(),
            y: self.yaw(),
            z: self.roll(),
        }
    }
}

const LANDING_ZONE_RADIUS: u32 = 5;
const LANDING_ZONE_BLEND_RADIUS: u32 = 7;

struct HeightMap {
    width: u32,
    depth: u32,
    landing_zone: Vec3,
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

    fn random_range<R: Rng>(&self, rng: &mut R, margin: u32, max: u32) -> u32 {
        rng.random_range(margin..max.saturating_sub(margin))
    }

    fn center(&self) -> Vec3 {
        Vec3 {
            x: self.width as f32 / 2.0,
            y: self.depth as f32 / 2.0,
            z: 0.0,
        }
    }

    fn landing_zone(&self) -> Vec3 {
        self.landing_zone
    }

    fn set_random_landing_zone<R: Rng>(&mut self, rng: &mut R) {
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

struct LandingZone {
    frame_transform: FrameTransform,
    scene_update: SceneUpdate,
}
impl LandingZone {
    fn new(landing_zone: Vec3) -> Self {
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

    fn log(&self) {
        LANDING_ZONE_FT.log(&self.frame_transform);
        LANDING_ZONE.log(&self.scene_update);
    }
}

struct Landscape {
    height_map: HeightMap,
    frame_transform: FrameTransform,
    scene_update: SceneUpdate,
    landing_zone: LandingZone,
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
        let landing_zone = LandingZone::new(height_map.landing_zone());
        Self {
            height_map,
            frame_transform,
            landing_zone,
            scene_update,
        }
    }

    fn lander_start_position(&self) -> Vec3 {
        self.height_map.center() - self.height_map.landing_zone() + (Vec3::Z * 100.0)
    }

    fn log(&self) {
        LANDSCAPE_FT.log(&self.frame_transform);
        LANDSCAPE.log(&self.scene_update);
        self.landing_zone.log();
    }
}

#[derive(Default, Clone)]
struct SharedControls(Arc<RwLock<Controls>>);
impl SharedControls {
    fn thrust(&self) -> f32 {
        self.0.read().thrust
    }

    fn rotation(&self) -> Vec3 {
        self.0.read().rotation
    }

    fn is_reset_pending(&self) -> bool {
        self.0.read().reset
    }

    fn request_reset(&self) {
        self.0.write().reset = true;
    }

    fn do_reset(&self) {
        let mut inner = self.0.write();
        inner.reset = false;
        inner.thrust = 0.0;
        inner.rotation = Vec3::ZERO;
    }

    fn update(&self, thrust: f32, rotation: Vec3) {
        let mut inner = self.0.write();
        inner.thrust = thrust;
        inner.rotation = rotation;
    }
}

#[derive(Default, Debug)]
struct Controls {
    reset: bool,
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
const APOLLO_LANDER_INITIAL_MASS_KG: f32 = 15_200.0;
const APOLLO_LANDER_THRUST_POWER_N: f32 = 44_500.0;
const APOLLO_LANDER_TORQUE_POWER_N: f32 = 3.0;

impl Lander {
    fn apollo(position: Vec3) -> Self {
        Self {
            position,
            velocity: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            angular_velocity: Vec3::ZERO,
            mass: APOLLO_LANDER_INITIAL_MASS_KG,
            thrust_power: APOLLO_LANDER_THRUST_POWER_N,
            torque_power: APOLLO_LANDER_TORQUE_POWER_N,
        }
    }

    fn step(&mut self, dt: f32, controls: &SharedControls) {
        // Gravity.
        self.velocity += MOON_GRAVITY * dt;

        // Apply thrust.
        let thrust_dir = self.rotation * Vec3::Z;
        let thrust_force = thrust_dir * controls.thrust() * self.thrust_power;
        self.velocity += (thrust_force / self.mass) * dt;

        // Apply torque.
        let torque = controls.rotation() * self.torque_power;
        self.angular_velocity += torque * dt;

        // Rotational damping
        self.angular_velocity *= 0.98;

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
            parent_frame_id: "landing_zone".into(),
            child_frame_id: "lander".into(),
            translation: Some(self.position.into_fg()),
            rotation: Some(self.rotation.into_fg()),
            ..Default::default()
        }
    }

    fn scene_entity(&self) -> SceneEntity {
        SceneEntity {
            frame_id: "lander".into(),
            models: vec![ModelPrimitive {
                url: "package://meshes/apollo-lunar-module.stl".into(),
                pose: Some(Pose::default()),
                scale: Some(Vector3 {
                    x: 1.0,
                    y: 1.0,
                    z: 1.0,
                }),
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    fn log(&self) {
        LANDER_FT.log(&self.frame_transform());
        LANDER_ANGULAR_VELOCITY.log(&self.angular_velocity.into_fg());
        LANDER_HORIZONTAL_VELOCITY.log(&Vector3 {
            x: self.velocity.x.into(),
            y: self.velocity.y.into(),
            z: 0.0,
        });
        LANDER_VERTICAL_VELOCITY.log(&Vector3 {
            x: 0.0,
            y: 0.0,
            z: self.velocity.z.into(),
        });
        LANDER_ORIENTATION.log(&self.rotation.into_fg());
        LANDER.log(&SceneUpdate {
            entities: vec![self.scene_entity()],
            ..Default::default()
        });
    }
}
