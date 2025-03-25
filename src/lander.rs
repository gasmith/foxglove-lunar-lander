use foxglove::schemas::{
    FrameTransform, ModelPrimitive, Pose, Quaternion, SceneEntity, SceneUpdate, Vector3,
};
use foxglove::static_typed_channel;
use glam::{EulerRot, Quat, Vec3};

use crate::LANDING_ZONE_RADIUS;
use crate::controls::Controls;
use crate::convert::IntoFg;

static_typed_channel!(LANDER, "/lander", SceneUpdate);
static_typed_channel!(LANDER_FT, "/lander_ft", FrameTransform);
static_typed_channel!(LANDER_ANGULAR_VELOCITY, "/lander_angular_velocity", Vector3);
static_typed_channel!(LANDER_VELOCITY, "/lander_velocity", Vector3);
static_typed_channel!(LANDER_ORIENTATION, "/lander_orientation", Quaternion);

/// Moon gravitational vector in meters/sec/sec.
const MOON_GRAVITY: Vec3 = Vec3 {
    x: 0.0,
    y: 0.0,
    z: -1.62,
};

pub struct Lander {
    position: Vec3,
    velocity: Vec3,
    rotation: Quat,
    angular_velocity: Vec3,
    mass: f32,
    thrust_power: f32,
    torque_power: f32,
}

// Apollo lunar lander.
const APOLLO_LANDER_INITIAL_MASS_KG: f32 = 15_200.0;
const APOLLO_LANDER_THRUST_POWER_N: f32 = 44_500.0;
const APOLLO_LANDER_TORQUE_POWER_N: f32 = 3.0;

#[derive(Debug, Clone, Copy)]
pub enum LanderStatus {
    ALOFT,
    LANDED,
    CRASHED,
    MISSED,
}

impl Lander {
    pub fn new(position: Vec3) -> Self {
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

    pub fn step(&mut self, dt: f32, controls: &Controls) {
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

    pub fn status(&self) -> LanderStatus {
        if self.position.z > 0.0 {
            LanderStatus::ALOFT
        } else if self.vertical_velocity().abs() > 1.0 || self.horizontal_velocity().abs() > 1.0 {
            LanderStatus::CRASHED
        } else if self.distance_from_center() > LANDING_ZONE_RADIUS as f32 {
            LanderStatus::MISSED
        } else {
            LanderStatus::LANDED
        }
    }

    fn vertical_velocity(&self) -> f32 {
        self.velocity.z
    }

    fn horizontal_velocity(&self) -> f32 {
        Vec3 {
            x: self.velocity.x,
            y: self.velocity.y,
            z: 0.0,
        }
        .length()
    }

    fn distance_from_center(&self) -> f32 {
        Vec3 {
            x: self.position.x,
            y: self.position.y,
            z: 0.0,
        }
        .length()
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

    pub fn log(&self) {
        LANDER_FT.log(&self.frame_transform());
        LANDER_ANGULAR_VELOCITY.log(&self.angular_velocity.into_fg());
        LANDER_VELOCITY.log(&self.velocity.into_fg());
        LANDER_ORIENTATION.log(&self.rotation.into_fg());
        LANDER.log(&SceneUpdate {
            entities: vec![self.scene_entity()],
            ..Default::default()
        });
    }
}
