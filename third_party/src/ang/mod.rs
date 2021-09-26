// Ensure that we're using the f32 version, since `ang` defaults to f64

use ang::Angle as BaseAngle;

pub type Angle = BaseAngle<f32>;
