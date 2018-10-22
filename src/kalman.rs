#[derive(Debug, Default)]
pub struct KalmanFilter {
    estimate_position: f64,
    estimate_velocity: f64,
    P_xx: f64,
    P_xv: f64,
    P_vv: f64,
    position_variance: f64,
    velocity_variance: f64,
    R: f64,
    has_velocity: bool,
}

impl KalmanFilter {
    pub fn new(position: f64, velocity: f64) -> KalmanFilter {
        KalmanFilter {
            estimate_position: position,
            estimate_velocity: velocity,
            P_xx: 0.1 * position,
            P_xv: 1.0,
            P_vv: 1.0,
            position_variance: 0.01 * position,
            velocity_variance: 0.01 * position / 1000.0,
            R: 0.01 * position,
            has_velocity: false,
        }
    }

    pub fn reset(&mut self, position: f64, velocity: f64) {
        self.estimate_position = position;
        self.estimate_velocity = velocity;
    }

    pub fn update(&mut self, position: f64, deltat: f64) {
        if deltat <= 0.0 {
            return;
        }

        if !self.has_velocity {
            self.estimate_velocity = (position - self.estimate_position) / deltat;
            self.estimate_position = position;
            self.has_velocity = true;
        } else {
            //Temporal update (predictive)
            self.estimate_position += self.estimate_velocity * deltat;

            // Update covariance
            self.P_xx += deltat * (2.0 * self.P_xv + deltat * self.P_vv);
            self.P_xv += deltat * self.P_vv;

            self.P_xx += deltat * self.position_variance;
            self.P_vv += deltat * self.velocity_variance;

            // Observational update (reactive)
            let vi = 1.0 / (self.P_xx + self.R);

            let kx = self.P_xx * vi;
            let kv = self.P_xv * vi;

            self.estimate_position += (position - self.estimate_position) * kx;
            self.estimate_velocity += (position - self.estimate_position) * kv;

            self.P_xx *= 1.0 - kx;
            self.P_xv *= 1.0 - kx;
            self.P_vv -= kv * self.P_xv;
        }
    }
}
