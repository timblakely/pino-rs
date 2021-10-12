pub struct PIController {
    k: f32,
    ki: f32,
    ki_integral: f32,
    v_clamp: f32,
}

impl PIController {
    pub fn new(k: f32, ki: f32, v_clamp: f32) -> PIController {
        PIController {
            k,
            ki,
            ki_integral: 0.,
            v_clamp,
        }
    }

    pub fn update(&mut self, measurement: f32, target: f32) -> f32 {
        let error = target - measurement;
        let voltage = self.k * error + self.ki_integral;
        self.ki_integral += self.k * self.ki * error;
        self.ki_integral = self.ki_integral.clamp(-self.v_clamp, self.v_clamp);
        voltage.clamp(-self.v_clamp, self.v_clamp)
    }
}
