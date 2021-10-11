#[cfg(test)]
mod tests {
    use bldc::timer::{iteratively_calculate_timer_config, TimerCalculation, TimerConfig};

    #[test]
    fn evenly_dividable_timer_config() {
        let calculation = iteratively_calculate_timer_config(170_000_000, 80000., 0.)
            .expect("Unable to get timing");
        assert!(matches!(
            calculation,
            TimerCalculation::Exact(TimerConfig {
                prescalar: 1,
                arr: 2125,
            })
        ));
    }

    #[test]
    fn not_evenly_dividable_timer_config() {
        let calculation = iteratively_calculate_timer_config(170_000_000, 79000., 5.)
            .expect("Unable to get timing");
        assert!(matches!(
            calculation,
            TimerCalculation::Approximate(TimerConfig {
                prescalar: 1,
                arr: 2152,
            })
        ));
    }

    #[test]
    fn not_evenly_dividable_timer_config_too_tight_tolerance() {
        assert!(matches!(
            iteratively_calculate_timer_config(170_000_000, 79000., 0.001),
            None
        ));
    }

    #[test]
    fn exact_with_prescalar_change() {
        let calculation =
            iteratively_calculate_timer_config(170_000_000, 80., 0.).expect("Unable to get timing");
        assert!(matches!(
            calculation,
            TimerCalculation::Exact(TimerConfig {
                prescalar: 34,
                arr: 62500,
            })
        ));
    }

    #[test]
    fn appproximate_with_prescalar_change() {
        let calculation = iteratively_calculate_timer_config(170_000_000, 1.7789, 0.000001)
            .expect("Unable to get timing");
        assert!(matches!(
            calculation,
            TimerCalculation::Approximate(TimerConfig {
                prescalar: 1470,
                arr: 65010,
            })
        ));
    }
}
