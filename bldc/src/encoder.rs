use crate::ic::ma702::{AngleState, Ma702, StreamingPolling};
use core::f32::consts::PI;
use num_traits::float::FloatCore;
use third_party::ang::Angle;

const TWO_PI: f32 = PI * 2.;

#[derive(Clone, Copy)]
pub struct EncoderState {
    electrical_angle: Angle,
    electrical_velocity: Angle,
}

impl EncoderState {
    pub fn new(electrical_angle: Angle, electrical_velocity: Angle) -> EncoderState {
        EncoderState {
            electrical_angle,
            electrical_velocity,
        }
    }
}

struct PllObserverCounts {
    kp: f32,
    ki: f32,
    // Note: These are represented as floats, but are actually fractional representations of the
    // encoder readings.
    angle: f32,
    velocity: f32,
}

impl PllObserverCounts {
    pub fn with_bandwidth(bandwidth: f32) -> Self {
        // This is based on a critically-damped pos/vel observer where the poles are on top of each
        // other. The `A` matrix is effectively $\begin{vmatrix}-k_p & 1 \\ -k_i & 0 \end{vmatrix}$,
        // which after running through SymPy for the eigenvalues (`Matrix([[-x, 1], [-y,
        // 0]]).eigenvals()`) gives us $kp = -2 * bandwidth$ and $ki = \frac{k_p^2}{4}$
        // For more info, see [this thread on ODrive's
        // forum](https://discourse.odriverobotics.com/t/rotor-encoder-pll-and-velocity/224/4)
        let kp = 2.0 * bandwidth;
        let ki = 0.25 * (kp * kp);
        // TODO(blakely): Don't panic here; return a `Result`.
        if kp < 1. {
            panic!(
                "Observer bandwidth needs to be >= 0.5 due to discretization limitations. At {} \
                 rads/s kp={}",
                bandwidth, kp
            );
        }
        PllObserverCounts::with_gains(kp, ki)
    }

    // Allow instantiating with gains directly
    pub fn with_gains(kp: f32, ki: f32) -> Self {
        // TODO(blakely): Don't panic here; return a `Result`.
        if kp < 1. {
            panic!("Due to discretization kp must be >= 1.0. Got {}", kp);
        }
        PllObserverCounts {
            kp,
            ki,
            angle: 0.,
            velocity: 0.,
        }
    }

    pub fn update(&mut self, dt: f32, observed_encoder_reading: u16) -> (f32, f32) {
        // Predict the current position.
        self.angle += dt * self.velocity;
        // Discrete phase detector. We need to discretize the continuous (float) prediction above,
        // so we need to figure out if the prediction is ahead or behind of where the actual
        // observed angle is. If our predicted angle is a bit behind and discretization hasn't
        // stepped up to the new value yet, but the encoder _has_, the following will be 1.0f.
        let error = (observed_encoder_reading as i32 - (self.angle.floor()) as i32) as f32;
        // Update the predicted angle based on the damping effect of kp, and update the velocity
        // measurement (stiffness?).
        self.angle += dt * self.kp * error;
        self.velocity += dt * self.ki * error;

        (self.angle, self.velocity)
    }
}

struct PllObserverRadians {
    kp: f32,
    ki: f32,
    min_d_theta: Angle,
    angle: Angle,
    velocity: Angle,
}

impl PllObserverRadians {
    pub fn with_bandwidth(bandwidth: f32, min_d_theta: Angle) -> Self {
        // This is based on a critically-damped pos/vel observer where the poles are on top of each
        // other. The `A` matrix is effectively $\begin{vmatrix}-k_p & 1 \\ -k_i & 0 \end{vmatrix}$,
        // which after running through SymPy for the eigenvalues (`Matrix([[-x, 1], [-y,
        // 0]]).eigenvals()`) gives us $kp = -2 * bandwidth$ and $ki = \frac{k_p^2}{4}$
        // For more info, see [this thread on ODrive's
        // forum](https://discourse.odriverobotics.com/t/rotor-encoder-pll-and-velocity/224/4)
        let kp = 2.0 * bandwidth;
        let ki = 0.25 * (kp * kp);
        // TODO(blakely): Don't panic here; return a `Result`.
        if kp < 1. {
            panic!(
                "Observer bandwidth needs to be >= 0.5 due to discretization limitations. At {} \
             rads/s kp={}",
                bandwidth, kp
            );
        }
        PllObserverRadians::with_gains(kp, ki, min_d_theta)
    }

    // Allow instantiating with gains directly
    pub fn with_gains(kp: f32, ki: f32, min_d_theta: Angle) -> Self {
        // TODO(blakely): Don't panic here; return a `Result`.
        if kp < 1. {
            panic!("Due to discretization kp must be >= 1.0. Got {}", kp);
        }
        PllObserverRadians {
            kp,
            ki,
            angle: Angle::Radians(0.),
            velocity: Angle::Radians(0.),
            min_d_theta,
        }
    }

    pub fn update(&mut self, dt: f32, new_reading: Angle) -> (f32, f32) {
        // Predict the current position.
        self.angle += dt * self.velocity;
        // Discrete phase detector. We need to discretize the continuous (float) prediction above,
        // so we need to figure out if the prediction is ahead or behind of where the actual
        // observed angle is. If our predicted angle is a bit behind and discretization hasn't
        // stepped up to the new value yet, but the encoder _has_, the following will be 1.0f.
        let d_theta = (new_reading - self.angle).normalized() - Angle::Radians(PI);

        let error = match d_theta {
            x if x <= -self.min_d_theta => -1.,
            x if x >= self.min_d_theta => 1.,
            _ => 0.,
        };
        // Update the predicted angle based on the damping effect of kp, and update the velocity
        // measurement (stiffness?).
        self.angle += Angle::Radians(dt * self.kp * error);
        self.angle = self.angle.normalized();
        self.velocity += Angle::Radians(dt * self.ki * error);

        (self.angle.in_radians(), self.velocity.in_radians())
    }
}

pub struct Encoder {
    ma702: Ma702<StreamingPolling>,
    pole_pairs: u8,
    angle_state: AngleState,
    state: EncoderState,
    // encoder_observer: PllObserverCounts,
    encoder_observer: PllObserverRadians,
    observer_state: (f32, f32),
}

impl Encoder {
    pub fn new(
        ma702: Ma702<StreamingPolling>,
        pole_pairs: u8,
        velocity_observer_bandwidth: f32,
    ) -> Encoder {
        Encoder {
            ma702,
            pole_pairs,
            angle_state: AngleState::new(),
            state: EncoderState::new(Angle::Radians(0.), Angle::Radians(0.)),
            encoder_observer: PllObserverRadians::with_bandwidth(
                velocity_observer_bandwidth,
                Angle::Radians(TWO_PI / 4096.),
            ),
            observer_state: (0., 0.),
        }
    }

    pub fn update(&mut self, delta_t: f32) {
        let angle_state = self.ma702.update(delta_t);
        self.angle_state = angle_state;
        // if let Some(raw_encoder_reading) = angle_state.raw_angle {
        //     let from_pll = self.encoder_observer.update(delta_t, raw_encoder_reading);
        //     self.observer_state = (from_pll.0 / 4096., from_pll.1 / 4096.);
        // }

        self.observer_state = self.encoder_observer.update(delta_t, angle_state.angle);

        // TODO(blakely): This may be less accurate than using the conversion from raw_angle.
        let electrical_angle = (angle_state.angle * self.pole_pairs as f32).normalized();
        let electrical_velocity = (angle_state.velocity * self.pole_pairs as f32).normalized();

        self.state = EncoderState::new(electrical_angle, electrical_velocity);
    }

    pub fn observer_state(&self) -> (f32, f32) {
        self.observer_state
    }

    pub fn electrical_angle(&self) -> Angle {
        self.state.electrical_angle
    }

    pub fn electrical_velocity(&self) -> Angle {
        self.state.electrical_velocity
    }

    pub fn state(&self) -> &EncoderState {
        &self.state
    }

    pub fn angle_state(&self) -> &AngleState {
        &self.angle_state
    }
}
