use super::MOON_GRAVITY;

/// Generic PID controller.
#[derive(Debug)]
struct PidController {
    kp: f32,
    ki: f32,
    kd: f32,
    prev_error: f32,
    integral: f32,
}

impl PidController {
    /// Creates a new PID controller.
    fn new(kp: f32, ki: f32, kd: f32) -> Self {
        Self {
            kp,
            ki,
            kd,
            prev_error: 0.0,
            integral: 0.0,
        }
    }

    /// Calculates error and returns a control value.
    fn update(&mut self, setpoint: f32, measured: f32, dt: f32) -> f32 {
        let error = setpoint - measured;
        self.integral += error * dt;
        let derivative = (error - self.prev_error) / dt;
        self.prev_error = error;
        self.kp * error + self.ki * self.integral + self.kd * derivative
    }
}

/// Vertical velocity controller.
///
/// This is intended to be similar to the Apollo lander's rate-of-descent (RoD) controller, but the
/// PID values are totally fabricated.
#[derive(Debug)]
pub struct VerticalVelocityController {
    pid: PidController,
    target: f32,
    thrust: f32,
}
impl VerticalVelocityController {
    /// Creates a new vertical velocity controller.
    pub fn new(target: f32, thrust: f32) -> Self {
        Self {
            pid: PidController::new(0.8, 0.05, 0.3),
            target,
            thrust,
        }
    }

    /// Returns the target vertical velocity.
    pub fn target(&self) -> f32 {
        self.target
    }

    /// Updates the target vertical velocity.
    pub fn adjust_target(&mut self, delta: f32) {
        self.target += delta;
    }

    /// Calculates error, and returns a thrust factor [0.0, 1.0].
    ///
    /// The implementation takes into consideration the current mass and tilt (from vertical) of
    /// the lander, to determine the appropriate amount of throttle to use to achieve the desired
    /// vertical velocity.
    pub fn compute_throttle(&mut self, current: f32, mass: f32, tilt: f32, dt: f32) -> f32 {
        let desired_accel = self.pid.update(self.target, current, dt);
        let total_vertical_accel = MOON_GRAVITY + desired_accel;
        let required_vertical_force = mass * total_vertical_accel;
        let throttle = required_vertical_force / (self.thrust * tilt.cos());
        throttle.clamp(0.0, 1.0)
    }
}
