#[derive(Debug, Default)]
pub struct KalmanFilter {
    estimate_position: f64,
    estimate_velocity: f64,
    p_xx: f64,
    p_xv: f64,
    p_vv: f64,
    position_variance: f64,
    velocity_variance: f64,
    r: f64,
    has_velocity: bool,
}

impl KalmanFilter {
    pub fn new(position: f64) -> KalmanFilter {
        KalmanFilter {
            estimate_position: position,
            estimate_velocity: 0.0,
            p_xx: 0.1 * position,
            p_xv: 1.0,
            p_vv: 1.0,
            position_variance: 0.1 * position,
            velocity_variance: 0.01 * position / 1000.0,
            r: 0.01 * position,
            has_velocity: false,
        }
    }

    pub fn reset(&mut self, position: f64) {
        self.estimate_position = position;
        self.estimate_velocity = 0.0;
        self.has_velocity = false;
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
            self.p_xx += deltat * (2.0 * self.p_xv + deltat * self.p_vv);
            self.p_xv += deltat * self.p_vv;

            self.p_xx += deltat * self.position_variance;
            self.p_vv += deltat * self.velocity_variance;

            // Observational update (reactive)
            let vi = 1.0 / (self.p_xx + self.r);

            let kx = self.p_xx * vi;
            let kv = self.p_xv * vi;

            self.estimate_position += (position - self.estimate_position) * kx;
            self.estimate_velocity += (position - self.estimate_position) * kv;

            self.p_xx *= 1.0 - kx;
            self.p_xv *= 1.0 - kx;
            self.p_vv -= kv * self.p_xv;
        }
    }

    pub fn predict(&self, position: f64, deltat: f64) -> f64 {
        position + self.estimate_velocity * deltat
    }
}
