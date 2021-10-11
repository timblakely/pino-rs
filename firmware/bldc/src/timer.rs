use num_traits::float::FloatCore;

#[derive(Debug)]
pub struct TimerConfig {
    pub prescalar: u16,
    pub arr: u16,
}

impl TimerConfig {
    pub fn frequency(&self, core_clock_hz: u32) -> f32 {
        core_clock_hz as f32 / (self.prescalar as f32 * self.arr as f32)
    }
}

#[derive(Debug)]
pub struct TimerInfo {
    pub frequency: f32,
    pub dt: f32,
    pub drift: f32,
}

#[derive(Debug)]
pub enum TimerCalculation {
    Approximate(TimerConfig),
    Exact(TimerConfig),
}

impl TimerCalculation {
    pub fn config(&self) -> &TimerConfig {
        match self {
            Self::Approximate(x) => x,
            Self::Exact(x) => x,
        }
    }
}

pub fn timer_info(core_clock_hz: u32, config: &TimerConfig) -> TimerInfo {
    let timer_frequency = config.frequency(core_clock_hz);
    let drift = (1. / (core_clock_hz as f32 / (config.prescalar * config.arr) as f32)
        - 1. / (timer_frequency as f32))
        * 1e9;
    TimerInfo {
        frequency: timer_frequency,
        dt: 1. / timer_frequency,
        drift,
    }
}

pub fn iteratively_calculate_timer_config(
    core_clock_hz: u32,
    desired_frequency: f32,
    tolerance: f32,
) -> Option<TimerCalculation> {
    let mut prescalar: u16 = 1;
    let mut diff = f32::MAX;
    let mut closest_arr = u16::MAX;
    let mut closest_prescalar: u16 = 1;
    loop {
        let arr = ((core_clock_hz as f32) / (desired_frequency * (prescalar as f32))).ceil() as u32;
        let config = TimerConfig {
            arr: arr as u16,
            prescalar,
        };
        let current_diff = (config.frequency(core_clock_hz) - desired_frequency).abs();
        if arr < (1 << 16) {
            if current_diff < diff {
                closest_prescalar = prescalar;
                closest_arr = arr as u16;
                diff = current_diff;
            }
            if current_diff == 0. {
                return Some(TimerCalculation::Exact(TimerConfig {
                    prescalar: closest_prescalar,
                    arr: closest_arr,
                }));
            }
            if (current_diff as f32) < tolerance {
                // Approximate match
                return Some(TimerCalculation::Approximate(TimerConfig {
                    prescalar: closest_prescalar,
                    arr: closest_arr,
                }));
            }
        }
        prescalar += 1;
        if prescalar == u16::MAX {
            return None;
        }
    }
}
