use foxglove::schemas::{
    Color, FrameTransform, ModelPrimitive, Pose, Quaternion, SceneEntity, SceneEntityDeletion,
    SceneUpdate, TextPrimitive, Vector3,
};
use foxglove::static_typed_channel;
use glam::{EulerRot, Quat, Vec3};
use serde::Serialize;

use crate::controls::Controls;
use crate::convert::IntoFg;
use crate::message::Metric;

static_typed_channel!(LANDER, "/lander", SceneUpdate);
static_typed_channel!(LANDER_FT, "/lander_ft", FrameTransform);
static_typed_channel!(LANDER_ANGULAR_VELOCITY, "/lander_angular_velocity", Vector3);
static_typed_channel!(LANDER_VELOCITY, "/lander_velocity", Vector3);
static_typed_channel!(LANDER_ORIENTATION, "/lander_orientation", Quaternion);
static_typed_channel!(LANDER_FUEL_MASS, "/lander_fuel_mass", Metric);
static_typed_channel!(LANDER_COURSE, "/lander_course", Vector3);
static_typed_channel!(LANDING_REPORT, "/landing_report", LandingReport);
static_typed_channel!(BANNER, "/banner", SceneUpdate);
static_typed_channel!(BANNER_FT, "/banner_ft", FrameTransform);

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

/// Descent fuel burn rate at full thrust, in kg/s.
const APOLLO_LANDER_FUEL_BURN_RATE_KGPS: f32 = 15.0;

/// RCS thrust power in newton-meters.
///
/// The module had sixteen 440N thrusters arranged in quads, and we're estimating that they're
/// about 2m from the center of mass.
const APOLLO_LANDER_TORQUE_NM: f32 = 3700.0;

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

#[derive(Debug, Clone, Copy, Serialize, schemars::JsonSchema)]
#[serde(rename = "snake_case")]
pub enum LandingStatus {
    Landed,
    Missed,
    Crashed,
}

#[derive(Debug, Clone, Copy, Serialize, schemars::JsonSchema)]
#[serde(rename = "snake_case")]
pub enum LandingCriterionType {
    VerticalSpeed,
    HorizontalSpeed,
    Tilt,
    AngularSpeed,
    DistanceFromLandingZone,
}

#[derive(Debug, Clone, Copy, Serialize, schemars::JsonSchema)]
pub struct LandingCriterion {
    r#type: LandingCriterionType,
    max: f32,
    actual: f32,
}
impl LandingCriterion {
    fn ok(&self) -> bool {
        self.actual <= self.max
    }

    fn score(&self) -> f32 {
        (self.max - self.actual) / self.max
    }
}

#[derive(Debug, Default, Clone, Serialize, schemars::JsonSchema)]
pub struct LandingReport {
    #[serde(skip_serializing_if = "Option::is_none")]
    report: Option<LandingReportInner>,
}

#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct LandingReportInner {
    status: LandingStatus,
    summary: String,
    score: f32,
    criteria: Vec<LandingCriterion>,
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
    landing_zone_radius: u32,
}

impl Lander {
    pub fn new(position: Vec3, landing_zone_radius: u32) -> Self {
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
            landing_zone_radius,
        }
    }

    pub fn stop(&mut self) {
        self.velocity = Vec3::ZERO;
        self.angular_velocity = Vec3::ZERO;
    }

    pub fn step(&mut self, dt: f32, controls: &Controls) {
        // Gravity.
        self.velocity += MOON_GRAVITY * Vec3::Z * dt;

        let total_mass = self.dry_mass + self.payload_mass + self.fuel_mass;
        if self.fuel_mass > 0.0 {
            // Apply thrust.
            let thrust = controls.thrust();
            let thrust_dir = self.rotation * Vec3::Z;
            let thrust_force = thrust_dir * thrust * self.thrust_power;
            self.velocity += (thrust_force / total_mass) * dt;

            // Consume fuel.
            let fuel_consumed = thrust * APOLLO_LANDER_FUEL_BURN_RATE_KGPS * dt;
            self.fuel_mass = (self.fuel_mass - fuel_consumed).max(0.0);
        }

        // Apply torque.
        let torque = controls.rotation() * self.torque_power;
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

    fn landing_report(&self) -> Option<LandingReportInner> {
        if !self.has_landed() {
            return None;
        }

        let criteria = self.landing_criteria();
        let mut score = 0.0;
        let mut first_problem = None;
        for crit in &criteria {
            if !crit.ok() && first_problem.is_none() {
                first_problem = Some(crit.r#type);
            }
            score += crit.score() * 2.0;
        }
        let status = match first_problem {
            None => LandingStatus::Landed,
            Some(LandingCriterionType::DistanceFromLandingZone) => LandingStatus::Missed,
            Some(_) => LandingStatus::Crashed,
        };
        let summary = match first_problem {
            Some(LandingCriterionType::VerticalSpeed) => {
                "The lander redefined the term 'lunar impactor'. NASA's department of craters thanks you."
            }
            Some(LandingCriterionType::HorizontalSpeed) => {
                "You landed... sideways. The ground wasn't ready for that level of enthusiasm."
            }
            Some(LandingCriterionType::Tilt) => {
                "You came in like a majestic leaning tower of 'nope'."
            }
            Some(LandingCriterionType::AngularSpeed) => {
                "Still spinning on landing; were you trying for a celebratory twirl?"
            }
            Some(LandingCriterionType::DistanceFromLandingZone) => {
                "You stuck the landing - on the wrong part of the moon."
            }
            None => "The eagle has landed.",
        };
        Some(LandingReportInner {
            status,
            summary: summary.to_string(),
            score,
            criteria,
        })
    }

    fn landing_criteria(&self) -> Vec<LandingCriterion> {
        macro_rules! criterion {
            ($variant:tt, $max:expr, $eval:expr) => {
                LandingCriterion {
                    r#type: LandingCriterionType::$variant,
                    max: $max,
                    actual: $eval,
                }
            };
        }
        vec![
            criterion!(VerticalSpeed, 3.0, self.vertical_speed()),
            criterion!(HorizontalSpeed, 1.0, self.horizontal_speed()),
            criterion!(Tilt, 0.25, self.tilt()),
            criterion!(AngularSpeed, 0.25, self.angular_speed()),
            criterion!(
                DistanceFromLandingZone,
                self.landing_zone_radius as f32,
                self.distance_from_landing_zone()
            ),
        ]
    }

    /// Vertical speed in meters/s.
    fn vertical_speed(&self) -> f32 {
        self.velocity.z.abs()
    }

    /// Horizontal speed in meters/s.
    fn horizontal_speed(&self) -> f32 {
        Vec3 {
            x: self.velocity.x,
            y: self.velocity.y,
            z: 0.0,
        }
        .length()
    }

    /// Tilt from upright in radians.
    fn tilt(&self) -> f32 {
        let up = self.rotation * Vec3::Z;
        up.angle_between(Vec3::Z)
    }

    /// Angular speed in radians/s.
    fn angular_speed(&self) -> f32 {
        self.angular_velocity.length()
    }

    /// Distance from landing zone in meters.
    fn distance_from_landing_zone(&self) -> f32 {
        self.position.length()
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
        LANDER_COURSE.log(&(-self.position).into_fg());
        LANDER.log(&SceneUpdate {
            entities: vec![self.scene_entity()],
            ..Default::default()
        });
    }

    pub fn clear_landing_report(&self) {
        BANNER_FT.log_static(&FrameTransform {
            parent_frame_id: "lander".into(),
            child_frame_id: "banner".into(),
            ..Default::default()
        });
        BANNER.log_static(&SceneUpdate {
            deletions: vec![SceneEntityDeletion {
                id: "banner".into(),
                ..Default::default()
            }],
            ..Default::default()
        });
        LANDING_REPORT.log_static(&LandingReport::default());
    }

    pub fn log_landing_report(&self) {
        let Some(report) = self.landing_report() else {
            return;
        };
        let ((r, g, b), text) = match report.status {
            LandingStatus::Landed => ((0.0, 1.0, 0.0), "LANDED"),
            LandingStatus::Missed => ((1.0, 1.0, 0.0), "MISSED"),
            LandingStatus::Crashed => ((1.0, 0.0, 0.0), "YOU DIED"),
        };
        BANNER_FT.log_static(&FrameTransform {
            parent_frame_id: "lander".into(),
            child_frame_id: "banner".into(),
            translation: Some(Vector3 {
                z: 5.0,
                ..Default::default()
            }),
            ..Default::default()
        });
        BANNER.log_static(&SceneUpdate {
            entities: vec![SceneEntity {
                frame_id: "banner".into(),
                id: "banner".into(),
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
        });
        LANDING_REPORT.log_static(&LandingReport {
            report: Some(report),
        });
    }
}
