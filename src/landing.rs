use std::{collections::HashMap, sync::LazyLock};

use foxglove::static_typed_channel;
use rand::seq::IndexedRandom;
use serde::Serialize;

static_typed_channel!(LANDING_REPORT, "/landing_report", LandingReportMsg);

static REMARKS: LazyLock<HashMap<LandingCriterionType, Vec<&'static str>>> = LazyLock::new(|| {
    [
        (
            LandingCriterionType::VerticalSpeed,
            vec![
                "You've redefined the term 'lunar impactor'.",
                "NASA's crater department thanks you for the new research subject.",
            ],
        ),
        (
            LandingCriterionType::HorizontalSpeed,
            vec!["You landed... sideways. The ground wasn't ready for that level of enthusiasm."],
        ),
        (
            LandingCriterionType::Tilt,
            vec!["You came in like a majestic leaning tower of 'nope'."],
        ),
        (
            LandingCriterionType::AngularSpeed,
            vec!["You were still spinning on landing. Were you trying for a celebratory twirl?"],
        ),
        (
            LandingCriterionType::DistanceFromTarget,
            vec!["You stuck the landing - on the wrong part of the moon."],
        ),
    ]
    .into_iter()
    .collect()
});

#[derive(Debug, Clone, Copy, Serialize, schemars::JsonSchema)]
#[serde(rename = "snake_case")]
pub enum LandingStatus {
    Landed,
    Missed,
    Crashed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, schemars::JsonSchema)]
#[serde(rename = "snake_case")]
pub enum LandingCriterionType {
    VerticalSpeed,
    HorizontalSpeed,
    Tilt,
    AngularSpeed,
    DistanceFromTarget,
}

impl LandingCriterionType {
    fn choose_remark(self) -> &'static str {
        REMARKS
            .get(&self)
            .and_then(|rs| rs.choose(&mut rand::rng()))
            .unwrap()
    }
}

#[derive(Debug, Clone, Copy, Serialize, schemars::JsonSchema)]
pub struct LandingCriterion {
    r#type: LandingCriterionType,
    max: f32,
    actual: f32,
}
impl LandingCriterion {
    pub fn vertical_speed(max: f32, actual: f32) -> Self {
        Self {
            r#type: LandingCriterionType::VerticalSpeed,
            max,
            actual,
        }
    }

    pub fn horizontal_speed(max: f32, actual: f32) -> Self {
        Self {
            r#type: LandingCriterionType::HorizontalSpeed,
            max,
            actual,
        }
    }

    pub fn tilt(max: f32, actual: f32) -> Self {
        Self {
            r#type: LandingCriterionType::Tilt,
            max,
            actual,
        }
    }

    pub fn angular_speed(max: f32, actual: f32) -> Self {
        Self {
            r#type: LandingCriterionType::AngularSpeed,
            max,
            actual,
        }
    }

    pub fn distance_from_target(max: f32, actual: f32) -> Self {
        Self {
            r#type: LandingCriterionType::DistanceFromTarget,
            max,
            actual,
        }
    }

    fn ok(&self) -> bool {
        self.actual <= self.max
    }

    fn score(&self) -> f32 {
        (self.max - self.actual) / self.max
    }
}

#[derive(Debug, Clone, Serialize, schemars::JsonSchema)]
pub struct LandingReport {
    status: LandingStatus,
    remark: &'static str,
    score: f32,
    criteria: Vec<LandingCriterion>,
}

#[derive(Debug, Default, Clone, Serialize, schemars::JsonSchema)]
pub struct LandingReportMsg<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    report: Option<&'a LandingReport>,
}

impl LandingReport {
    pub fn new(criteria: Vec<LandingCriterion>) -> Self {
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
            Some(LandingCriterionType::DistanceFromTarget) => LandingStatus::Missed,
            Some(_) => LandingStatus::Crashed,
        };
        let remark = match first_problem {
            Some(p) => p.choose_remark(),
            None => "The eagle has landed.",
        };
        Self {
            status,
            remark,
            score,
            criteria,
        }
    }

    /// Returns the landing status.
    pub fn status(&self) -> LandingStatus {
        self.status
    }

    /// Clears the previous landing report.
    pub fn clear() {
        LANDING_REPORT.log(&LandingReportMsg::default());
    }

    /// Logs the landing report.
    pub fn log(&self) {
        LANDING_REPORT.log(&LandingReportMsg { report: Some(self) })
    }
}
