// Ensure that we're using the f32 version, since `ang` defaults to f64

use ang::Angle as BaseAngle;

pub type Angle = BaseAngle<f32>;

use core::f32::consts::PI;

const TWO_PI: f32 = 2. * PI;

pub trait AbsoluteDist {
    fn abs_dist(&self, other: Angle) -> Angle;
}

impl AbsoluteDist for Angle {
    fn abs_dist(&self, other: Angle) -> Angle {
        match *self - other {
            d_angle if d_angle.in_radians() > PI => Angle::Radians(d_angle.in_radians() - TWO_PI),
            d_angle if d_angle.in_radians() <= -PI => Angle::Radians(d_angle.in_radians() + TWO_PI),
            d_angle => d_angle,
        }
    }
}
