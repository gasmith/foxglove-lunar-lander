use super::MOON_GRAVITY;

/// Rate of descent controller.
#[derive(Debug)]
struct PidController {
    kp: f32,
    ki: f32,
    kd: f32,
    prev_error: f32,
    integral: f32,
}

impl PidController {
    fn new(kp: f32, ki: f32, kd: f32) -> Self {
        Self {
            kp,
            ki,
            kd,
            prev_error: 0.0,
            integral: 0.0,
        }
    }

    fn update(&mut self, setpoint: f32, measured: f32, dt: f32) -> f32 {
        let error = setpoint - measured;
        self.integral += error * dt;
        let derivative = (error - self.prev_error) / dt;
        self.prev_error = error;
        self.kp * error + self.ki * self.integral + self.kd * derivative
    }
}

/// Rate-of-descent controller.
#[derive(Debug)]
pub struct RodController {
    pid: PidController,
    target: f32,
    thrust: f32,
}
impl RodController {
    pub fn new(target: f32, thrust: f32) -> Self {
        Self {
            pid: PidController::new(0.8, 0.05, 0.3),
            target,
            thrust,
        }
    }

    pub fn target(&self) -> f32 {
        self.target
    }

    pub fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    pub fn compute_throttle(&mut self, current: f32, mass: f32, tilt: f32, dt: f32) -> f32 {
        let desired_accel = self.pid.update(self.target, current, dt);
        let total_vertical_accel = MOON_GRAVITY + desired_accel;
        let required_vertical_force = mass * total_vertical_accel;
        let throttle = required_vertical_force / (self.thrust * tilt.cos());
        throttle.clamp(0.0, 1.0)
    }
}
