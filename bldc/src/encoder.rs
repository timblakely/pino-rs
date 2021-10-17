use crate::ic::ma702::{Ma702, StreamingPolling};
use core::f32::consts::PI;
use third_party::ang::{AbsoluteDist, Angle};

const TWO_PI: f32 = PI * 2.;

struct PllObserverRadians {
    kp: f32,
    ki: f32,
    min_d_theta: Angle,
    angle: Option<Angle>,
    velocity: Angle,
    d_theta: Angle,
}

struct PllObserverState {
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
            angle: None,
            velocity: Angle::Radians(0.),
            min_d_theta,
            d_theta: Angle::Radians(0.),
        }
    }

    pub fn update(&mut self, dt: f32, new_reading: Angle) -> PllObserverState {
        let previous_angle = match self.angle {
            Some(x) => x,
            None => {
                self.angle = Some(new_reading);
                self.velocity = Angle::Radians(0.);
                return PllObserverState {
                    angle: new_reading,
                    velocity: self.velocity,
                };
            }
        };
        let previous_velocity = self.velocity;

        // Predict the current position.
        let angle = previous_angle + dt * previous_velocity;

        // Calculate change in theta.
        // let d_theta = match new_reading - angle {
        //     d_angle if d_angle.in_radians() > PI => Angle::Radians(d_angle.in_radians() - TWO_PI),
        //     d_angle if d_angle.in_radians() <= -PI => Angle::Radians(d_angle.in_radians() + TWO_PI),
        //     d_angle => d_angle,
        // };
        let d_theta = new_reading.abs_dist(angle);

        // Discrete phase detector. We need to discretize the continuous (float) prediction above,
        // so we need to figure out if the prediction is ahead or behind of where the actual
        // observed angle is. If our predicted angle is a bit behind and discretization hasn't
        // stepped up to the new value yet, but the encoder _has_, the following will be 1.0f.
        let error = match d_theta {
            x if x <= -self.min_d_theta => -1.,
            x if x >= self.min_d_theta => 1.,
            _ => 0.,
        };
        // Update the predicted angle based on the damping effect of kp, and update the velocity
        // measurement (stiffness?).
        let new_angle = (angle + Angle::Radians(dt * self.kp * error)).normalized();
        self.angle = Some(new_angle);
        self.velocity = previous_velocity + Angle::Radians(dt * self.ki * error);
        self.d_theta = d_theta;

        PllObserverState {
            angle: new_angle,
            velocity: self.velocity,
        }
    }
}

#[derive(Clone, Copy)]
pub struct EncoderState {
    pub raw_encoder: u16,
    pub angle: Angle,
    pub velocity: Angle,
    pub angle_multiturn: Angle,
    pub electrical_angle: Angle,
    pub electrical_velocity: Angle,
}

pub struct Encoder {
    ma702: Ma702<StreamingPolling>,
    pole_pairs: u8,
    encoder_observer: PllObserverRadians,
    state: Option<EncoderState>,
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
            encoder_observer: PllObserverRadians::with_bandwidth(
                velocity_observer_bandwidth,
                Angle::Radians(TWO_PI / 4096.),
            ),
            state: None,
        }
    }

    pub fn update(&mut self, delta_t: f32) -> EncoderState {
        let angle_state = self.ma702.update(delta_t);
        let pll_state = self.encoder_observer.update(delta_t, angle_state.angle);
        let electrical_angle = (angle_state.angle * self.pole_pairs as f32).normalized();
        let electrical_velocity = pll_state.velocity * self.pole_pairs as f32;

        let mut new_state = EncoderState {
            raw_encoder: angle_state.raw_angle,
            angle: pll_state.angle,
            velocity: pll_state.velocity,
            angle_multiturn: pll_state.angle,
            electrical_angle,
            electrical_velocity,
        };

        if let Some(previous_state) = self.state {
            let d_theta = pll_state.angle.abs_dist(previous_state.angle);
            new_state.angle_multiturn = previous_state.angle_multiturn + d_theta;
        }
        self.state = Some(new_state);
        new_state
    }

    pub fn state(&self) -> &Option<EncoderState> {
        &self.state
    }
}
