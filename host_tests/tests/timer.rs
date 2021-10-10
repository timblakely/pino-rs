#[cfg(test)]
mod tests {
    use bldc::timer::iteratively_calculate_timer_config;

    #[test]
    fn evenly_dividable_timer_config() {
        let calculation = iteratively_calculate_timer_config(170_000_000, 80000., 0.)
            .expect("Unable to get timing");
        println!("Got timer config: {:?}", calculation)
    }
}
