use crate::current_sensing::CurrentMeasurement;

use super::fdcan::FdcanMessage;

// Current sense for a given duration.
pub struct IdleCurrentSense {
    pub duration: f32,
}

// Emergency Stop message.
pub struct EStop {}

pub enum Messages {
    IdleCurrentSense(IdleCurrentSense),
    EStop(EStop),
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

impl ExtendedFdcanFrame for EStop {
    fn unpack(_: &FdcanMessage) -> Self {
        panic!("Unack not supported");
    }

    fn pack(&self) -> FdcanMessage {
        FdcanMessage::new(0x0, [])
    }
}

impl ExtendedFdcanFrame for CurrentMeasurement {
    fn unpack(_: &FdcanMessage) -> Self {
        panic!("Unpack not supported")
    }

    fn pack(&self) -> FdcanMessage {
        FdcanMessage::new(
            0xD,
            [
                self.phase_a.to_bits(),
                self.phase_b.to_bits(),
                self.phase_c.to_bits(),
            ],
        )
    }
}

impl Messages {
    pub fn unpack_fdcan(message: &FdcanMessage) -> Option<Self> {
        match message.id {
            0x0 => Some(Self::EStop(EStop::unpack(message))),
            0xC => Some(Self::IdleCurrentSense(IdleCurrentSense::unpack(message))),
            _ => None,
        }
    }
}
