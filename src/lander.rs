use foxglove::schemas::{
    FrameTransform, ModelPrimitive, Pose, Quaternion, SceneEntity, SceneUpdate, Vector3,
};
use foxglove::static_typed_channel;
use glam::{EulerRot, Quat, Vec3};

use crate::LANDING_ZONE_RADIUS;
use crate::controls::Controls;
use crate::convert::IntoFg;
use crate::message::Metric;

static_typed_channel!(LANDER, "/lander", SceneUpdate);
static_typed_channel!(LANDER_FT, "/lander_ft", FrameTransform);
static_typed_channel!(LANDER_ANGULAR_VELOCITY, "/lander_angular_velocity", Vector3);
static_typed_channel!(LANDER_VELOCITY, "/lander_velocity", Vector3);
static_typed_channel!(LANDER_ORIENTATION, "/lander_orientation", Quaternion);
static_typed_channel!(LANDER_FUEL_MASS, "/lander_fuel_mass", Metric);

/// Base mass for the apollo lander.
const APOLLO_LANDER_DRY_MASS_KG: f32 = 2_150.0;

/// The payload is the ascent stage & ascent fuel.
const APOLLO_LANDER_PAYLOAD_MASS_KG: f32 = 2_150.0 + 2_400.0;

/// Descent fuel mass.
///
/// The initial fuel mass was 8,200 kg, but a good portion of that was spent slowing the descent
/// from 15km. Our simulator picks up at 200m, so let's suppose there's 600kg of fuel left to make
/// for a nail-biting final approach.
const APOLLO_LANDER_INITIAL_FUEL_MASS_KG: f32 = 600.0;

/// Main descent thrust power in newtons.
const APOLLO_LANDER_THRUST_N: f32 = 45_000.0;

/// RCS thrust power in newton-meters.
///
/// This number is a lie. The actual module had sixteen 440N thrusters, arranged in four quads,
/// with multiple thrusters being used to apply torque to adjust attitude. Assuming the RCS
/// thrusters are ~2m from the center of mass, they should be capable of exerting several kN-m of
/// torque, but that makes the simulator utterly unusable. So we cap the torque to a value several
/// orders of magnitude smaller.
const APOLLO_LANDER_TORQUE_NM: f32 = 2.0;

/// Descent fuel burn rate at full thrust, in kg/s.
const APOLLO_LANDER_FUEL_BURN_RATE_KGPS: f32 = 15.0;

/// Moon gravitational vector in meters/sec/sec.
const MOON_GRAVITY: Vec3 = Vec3 {
    x: 0.0,
    y: 0.0,
    z: -1.62,
};

#[derive(Debug, Clone, Copy)]
pub enum LanderStatus {
    Aloft,
    Landed,
    TooFast,
    NotLevel,
    Spinning,
    Missed,
}

pub struct Lander {
    position: Vec3,
    velocity: Vec3,
    rotation: Quat,
    angular_velocity: Vec3,
    dry_mass: f32,
    payload_mass: f32,
    fuel_mass: f32,
    thrust_power: f32,
    torque_power: f32,
}

impl Lander {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            velocity: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            angular_velocity: Vec3::ZERO,
            dry_mass: APOLLO_LANDER_DRY_MASS_KG,
            payload_mass: APOLLO_LANDER_PAYLOAD_MASS_KG,
            fuel_mass: APOLLO_LANDER_INITIAL_FUEL_MASS_KG,
            thrust_power: APOLLO_LANDER_THRUST_N,
            torque_power: APOLLO_LANDER_TORQUE_NM,
        }
    }

    pub fn stop(&mut self) {
        self.velocity = Vec3::ZERO;
        self.angular_velocity = Vec3::ZERO;
    }

    pub fn step(&mut self, dt: f32, controls: &Controls) {
        // Gravity.
        self.velocity += MOON_GRAVITY * dt;

        if self.fuel_mass > 0.0 {
            // Apply thrust.
            let thrust = controls.thrust();
            let thrust_dir = self.rotation * Vec3::Z;
            let thrust_force = thrust_dir * thrust * self.thrust_power;
            let total_mass = self.dry_mass + self.payload_mass + self.fuel_mass;
            self.velocity += (thrust_force / total_mass) * dt;

            // Consume fuel.
            let fuel_consumed = thrust * APOLLO_LANDER_FUEL_BURN_RATE_KGPS * dt;
            self.fuel_mass = (self.fuel_mass - fuel_consumed).max(0.0);
        }

        // Apply torque.
        let torque = controls.rotation() * self.torque_power;
        self.angular_velocity += torque * dt;

        // Dampen rotational velocity to make gameplay a bit easier.
        self.angular_velocity *= 0.99;

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
            LanderStatus::Aloft
        } else if self.vertical_velocity_mag() > 1.0 || self.horizontal_velocity_mag() > 1.0 {
            LanderStatus::TooFast
        } else if self.tilt_radians() > 0.25 {
            LanderStatus::NotLevel
        } else if self.angular_velocity.length() > 0.5 {
            LanderStatus::Spinning
        } else if self.distance_from_center() > LANDING_ZONE_RADIUS as f32 {
            LanderStatus::Missed
        } else {
            LanderStatus::Landed
        }
    }

    fn vertical_velocity_mag(&self) -> f32 {
        self.velocity.z.abs()
    }

    fn horizontal_velocity_mag(&self) -> f32 {
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

    fn tilt_radians(&self) -> f32 {
        let up = self.rotation * Vec3::Z;
        up.angle_between(Vec3::Z)
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
                url: "package://dae/apollo.dae".into(),
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
        LANDER_FUEL_MASS.log(&self.fuel_mass.into_fg());
        LANDER_ORIENTATION.log(&self.rotation.into_fg());
        LANDER_VELOCITY.log(&self.velocity.into_fg());
        LANDER.log(&SceneUpdate {
            entities: vec![self.scene_entity()],
            ..Default::default()
        });
    }
}
