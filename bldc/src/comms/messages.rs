use crate::{commutation::phase_current::PhaseCurrentCommand, current_sensing::CurrentMeasurement};

use super::fdcan::FdcanMessage;

// TODO(blakely): Move these into their respective control loop files. No need to be in messages.rs

// Current sense for a given duration.
pub struct IdleCurrentSense {
    pub duration: f32,
}

// Build a 16-bin distribution of current, one phase at a time.
pub struct IdleCurrentDistribution {
    pub duration: f32,
    pub center_current: f32,
    pub current_range: f32,
    pub phase: u8,
}

// Response to IdleCurrentDistribution
pub struct CurrentDistribution<'a> {
    pub bins: &'a [u32; 16],
}

// Measure the inductance of the windings
pub struct MeasureInductance {
    pub duration: f32,
    pub frequency: u32,
    pub pwm_duty: f32,
    pub sample_pwm_percent: f32,
}
// Return value for inductances
pub struct Inductances<'a> {
    pub inductances: &'a [f32; 3],
}

// Measure the resistance of the windings.
pub struct MeasureResistance {
    pub duration: f32,
    pub target_voltage: f32,
    pub phase: crate::commutation::measure_resistance::Phase,
}

// Calibrate ADC values.
pub struct CalibrateADC {
    pub duration: f32,
}

// Emergency Stop message.
pub struct EStop {}

pub enum Messages {
    IdleCurrentSense(IdleCurrentSense),
    IdleCurrentDistribution(IdleCurrentDistribution),
    CalibrateADC(CalibrateADC),
    MeasureInductance(MeasureInductance),
    MeasureResistance(MeasureResistance),
    EStop(EStop),
    PhaseCurrentCommand(PhaseCurrentCommand),
}

// TODO(blakely): split into received/sent, since some of the messages only make sense for incoming
// or outgoing messages.
pub trait ExtendedFdcanFrame {
    // Unpack the message from a buffer.
    fn unpack(message: &FdcanMessage) -> Self;

    // Pack the message into a buffer of up to 64 bytes, returning the number of bytes that were
    // packed.
    fn pack(&self) -> FdcanMessage;
}

impl ExtendedFdcanFrame for IdleCurrentSense {
    fn unpack(message: &FdcanMessage) -> Self {
        let buffer = message.data;
        IdleCurrentSense {
            duration: f32::from_bits(buffer[0]),
        }
    }

    fn pack(&self) -> FdcanMessage {
        panic!("Pack not supported");
    }
}

impl ExtendedFdcanFrame for IdleCurrentDistribution {
    fn unpack(message: &FdcanMessage) -> Self {
        let buffer = message.data;
        IdleCurrentDistribution {
            duration: f32::from_bits(buffer[0]),
            center_current: f32::from_bits(buffer[1]),
            current_range: f32::from_bits(buffer[2]),
            phase: buffer[3] as u8 & 0xFF,
        }
    }

    fn pack(&self) -> FdcanMessage {
        panic!("Pack not supported");
    }
}

impl<'a> ExtendedFdcanFrame for CurrentDistribution<'a> {
    fn unpack(_: &FdcanMessage) -> Self {
        panic!("Unack not supported");
    }

    fn pack(&self) -> FdcanMessage {
        FdcanMessage::new(0xF, self.bins)
    }
}

impl ExtendedFdcanFrame for MeasureInductance {
    fn unpack(message: &FdcanMessage) -> Self {
        let buffer = message.data;
        MeasureInductance {
            duration: f32::from_bits(buffer[0]),
            frequency: buffer[1],
            pwm_duty: f32::from_bits(buffer[2]),
            sample_pwm_percent: f32::from_bits(buffer[3]),
        }
    }

    fn pack(&self) -> FdcanMessage {
        panic!("Pack not supported");
    }
}

impl<'a> ExtendedFdcanFrame for Inductances<'a> {
    fn unpack(_: &FdcanMessage) -> Self {
        panic!("Unack not supported");
    }

    fn pack(&self) -> FdcanMessage {
        FdcanMessage::new(
            0x11,
            &[
                self.inductances[0].to_bits(),
                self.inductances[1].to_bits(),
                self.inductances[2].to_bits(),
            ],
        )
    }
}

impl ExtendedFdcanFrame for MeasureResistance {
    fn unpack(message: &FdcanMessage) -> Self {
        let buffer = message.data;
        MeasureResistance {
            duration: f32::from_bits(buffer[0]),
            target_voltage: f32::from_bits(buffer[1]),

            phase: match buffer[2] & 0xFFu32 {
                0 => crate::commutation::measure_resistance::Phase::A,
                1 => crate::commutation::measure_resistance::Phase::B,
                _ => crate::commutation::measure_resistance::Phase::C,
            },
        }
    }

    fn pack(&self) -> FdcanMessage {
        panic!("Pack not supported");
    }
}

impl ExtendedFdcanFrame for EStop {
    fn unpack(_: &FdcanMessage) -> Self {
        EStop {}
    }

    fn pack(&self) -> FdcanMessage {
        panic!("Pack not supported");
    }
}

impl ExtendedFdcanFrame for CurrentMeasurement {
    fn unpack(_: &FdcanMessage) -> Self {
        panic!("Unpack not supported")
    }

    fn pack(&self) -> FdcanMessage {
        FdcanMessage::new(
            0xD,
            &[
                self.phase_a.to_bits(),
                self.phase_b.to_bits(),
                self.phase_c.to_bits(),
            ],
        )
    }
}

impl ExtendedFdcanFrame for CalibrateADC {
    fn unpack(message: &FdcanMessage) -> Self {
        let buffer = message.data;
        CalibrateADC {
            duration: f32::from_bits(buffer[0]),
        }
    }

    fn pack(&self) -> FdcanMessage {
        panic!("Pack not supported");
    }
}

impl Messages {
    pub fn unpack_fdcan(message: &FdcanMessage) -> Option<Self> {
        match message.id {
            0x0 => Some(Self::EStop(EStop::unpack(message))),
            0xC => Some(Self::IdleCurrentSense(IdleCurrentSense::unpack(message))),
            0xE => Some(Self::IdleCurrentDistribution(
                IdleCurrentDistribution::unpack(message),
            )),
            0xF => Some(Self::CalibrateADC(CalibrateADC::unpack(message))),
            0x10 => Some(Self::MeasureInductance(MeasureInductance::unpack(message))),
            0x11 => Some(Self::MeasureResistance(MeasureResistance::unpack(message))),
            0x12 => Some(Self::PhaseCurrentCommand(PhaseCurrentCommand::unpack(
                message,
            ))),
            _ => None,
        }
    }
}
