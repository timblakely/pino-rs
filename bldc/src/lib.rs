#![cfg_attr(not(test), no_std)]

pub mod util;

pub mod comms;
pub mod control_loops;
pub mod cordic;
pub mod current_sensing;
pub mod driver;
pub mod encoder;
pub mod foc;
pub mod ic;
pub mod led;
pub mod pi_controller;
pub mod pwm;
pub mod timer;
