use foxglove::schemas::{
    FrameTransform, ModelPrimitive, Pose, Quaternion, SceneEntity, SceneUpdate, Vector3,
};
use foxglove::static_typed_channel;
use glam::{EulerRot, Quat, Vec2, Vec3};
use serde::Serialize;

use crate::controls::Controls;
use crate::convert::IntoFg;
use crate::landing::{LandingCriterion, LandingReport};

mod controllers;
use controllers::VerticalVelocityController;

#[derive(Serialize, schemars::JsonSchema)]
struct LanderMetrics {
    altitude: f64,
    fuel_mass: f64,
    vertical_velocity_target: f64,
}

static_typed_channel!(LANDER, "/lander", SceneUpdate);
static_typed_channel!(LANDER_ANGULAR_VELOCITY, "/lander_angular_velocity", Vector3);
static_typed_channel!(LANDER_COURSE, "/lander_course", Vector3);
static_typed_channel!(LANDER_METRICS, "/lander_metrics", LanderMetrics);
static_typed_channel!(LANDER_ORIENTATION, "/lander_orientation", Quaternion);
static_typed_channel!(LANDER_VELOCITY, "/lander_velocity", Vector3);

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
const APOLLO_LANDER_DCS_THRUST_N: f32 = 45_000.0;

/// Descent fuel burn rate at full thrust, in kg/s.
const APOLLO_LANDER_FUEL_BURN_RATE_KGPS: f32 = 15.0;

/// RCS thrust for strafing.
///
/// The module had sixteen 440N thrusters arranged in quads. For any direction, there are two
/// thrusters to use.
const APOLLO_LANDER_RCS_THRUST_N: f32 = 880.0;

/// RCS torque in newton-meters.
///
/// The module had sixteen 440N thrusters arranged in quads, and we're estimating that they're
/// about 2m from the center of mass.
const APOLLO_LANDER_RCS_TORQUE_NM: f32 = 3700.0;

/// Estimated inertial profile for the lunar lander.
///
/// The lander is roughly round in the horizontal plane with a diameter of 4.2m, and it's 7m
/// tall with both ascent & descent stages.
///
/// If we model it as a cylinder, the moments of inertia are:
const APOLLO_LANDER_INERTIA: Vec3 = Vec3 {
    x: (3.0 * 2.1 * 2.1 + 7.0 * 7.0) / 12.0,
    y: (3.0 * 2.1 * 2.1 + 7.0 * 7.0) / 12.0,
    z: (2.1 * 2.1) / 2.0,
};

/// Moon gravitational constant in meters/s^2.
const MOON_GRAVITY: f32 = -1.62;

pub struct Lander {
    position: Vec3,
    velocity: Vec3,
    rotation: Quat,
    angular_velocity: Vec3,
    dry_mass: f32,
    payload_mass: f32,
    fuel_mass: f32,
    dcs_thrust: f32,
    rcs_thrust: f32,
    rcs_torque: f32,
    landing_zone_radius: u32,
    vertical_velocity_controller: VerticalVelocityController,
}

impl Lander {
    pub fn new(
        position: Vec3,
        vertical_velocity: f32,
        vertical_velocity_target: f32,
        landing_zone_radius: u32,
    ) -> Self {
        Self {
            position,
            velocity: Vec3::Z * vertical_velocity,
            rotation: Quat::IDENTITY,
            angular_velocity: Vec3::ZERO,
            dry_mass: APOLLO_LANDER_DRY_MASS_KG,
            payload_mass: APOLLO_LANDER_PAYLOAD_MASS_KG,
            fuel_mass: APOLLO_LANDER_INITIAL_FUEL_MASS_KG,
            dcs_thrust: APOLLO_LANDER_DCS_THRUST_N,
            rcs_thrust: APOLLO_LANDER_RCS_THRUST_N,
            rcs_torque: APOLLO_LANDER_RCS_TORQUE_NM,
            landing_zone_radius,
            vertical_velocity_controller: VerticalVelocityController::new(
                vertical_velocity_target,
                APOLLO_LANDER_DCS_THRUST_N,
            ),
        }
    }

    pub fn stop(&mut self) {
        self.velocity = Vec3::ZERO;
        self.angular_velocity = Vec3::ZERO;
    }

    pub fn step(&mut self, dt: f32, controls: &Controls) {
        // Update target vertical velocity.
        self.vertical_velocity_controller
            .adjust_target(controls.get_and_reset_vertical_velocity_delta());

        let total_mass = self.dry_mass + self.payload_mass + self.fuel_mass;
        if self.fuel_mass > 0.0 {
            // Use rate-of-descent PID controller to compute throttle.
            let throttle = self.vertical_velocity_controller.compute_throttle(
                self.velocity.z,
                total_mass,
                self.tilt(),
                dt,
            );

            // Apply throttle.
            let thrust_dir = self.rotation * Vec3::Z;
            let thrust_force = throttle * thrust_dir * self.dcs_thrust;
            self.velocity += (thrust_force / total_mass) * dt;

            // Consume fuel.
            let fuel_consumed = throttle * APOLLO_LANDER_FUEL_BURN_RATE_KGPS * dt;
            self.fuel_mass = (self.fuel_mass - fuel_consumed).max(0.0);
        }

        // Apply strafe.
        let strafe = controls.strafe();
        let strafe_force = self.rotation * Vec3::new(strafe.x, strafe.y, 0.0) * self.rcs_thrust;
        self.velocity += (strafe_force / total_mass) * dt;

        // Apply gravity.
        self.velocity += MOON_GRAVITY * Vec3::Z * dt;

        // Apply torque.
        let torque = controls.rotate() * self.rcs_torque;
        let inertia = total_mass * APOLLO_LANDER_INERTIA;
        self.angular_velocity += (torque / inertia) * dt;

        // Dampen rotational velocity to make gameplay a bit easier.
        self.angular_velocity *= 0.999;

        // Update position & orientation.
        self.position += self.velocity * dt;
        self.rotation *= Quat::from_euler(
            EulerRot::XYZ,
            self.angular_velocity.x * dt,
            self.angular_velocity.y * dt,
            self.angular_velocity.z * dt,
        );
    }

    pub fn has_landed(&self) -> bool {
        self.position.z <= 0.0
    }

    /// Returns a landing report, if the lander has landed.
    pub fn landing_report(&self) -> Option<LandingReport> {
        if self.has_landed() {
            let criteria = self.landing_criteria();
            Some(LandingReport::new(criteria))
        } else {
            None
        }
    }

    /// Returns landing criteria, in order of importance.
    fn landing_criteria(&self) -> Vec<LandingCriterion> {
        vec![
            LandingCriterion::vertical_speed(3.0, self.velocity.z.abs()),
            LandingCriterion::horizontal_speed(
                1.0,
                Vec2::new(self.velocity.x, self.velocity.y).length(),
            ),
            LandingCriterion::tilt(3.0, self.tilt()),
            LandingCriterion::angular_speed(0.25, self.angular_velocity.length()),
            LandingCriterion::distance_from_target(
                self.landing_zone_radius as f32,
                self.position.length(),
            ),
        ]
    }

    /// Tilt from upright in radians.
    fn tilt(&self) -> f32 {
        let up = self.rotation * Vec3::Z;
        up.angle_between(Vec3::Z)
    }

    pub fn frame_transform(&self) -> FrameTransform {
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
            id: "lander".into(),
            frame_id: "lander".into(),
            models: vec![ModelPrimitive {
                url: "package://foxglove-lunar-lander/assets/apollo.dae".into(),
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

    pub fn log_scene(&self) {
        LANDER.log(&SceneUpdate {
            entities: vec![self.scene_entity()],
            deletions: vec![],
        });
    }

    pub fn log(&self) {
        LANDER_METRICS.log(&LanderMetrics {
            altitude: self.position.z.into(),
            fuel_mass: self.fuel_mass.into(),
            vertical_velocity_target: self.vertical_velocity_controller.target().into(),
        });
        LANDER_ANGULAR_VELOCITY.log(&self.angular_velocity.into_fg());
        LANDER_COURSE.log(&(-self.position).into_fg());
        LANDER_ORIENTATION.log(&self.rotation.into_fg());
        LANDER_VELOCITY.log(&self.velocity.into_fg());
    }
}
