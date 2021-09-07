use super::fdcan::FdcanMessage;

// Just for testing; do not use in regular communication.
pub struct ForcePwm {
    pub foo: u32,
    pub pwm_duty: f32,
    pub baz: u8,
    pub toot: [u8; 3],
}

pub struct IdleCurrentSense {
    pub duration: f32,
}

pub struct SetCurrents {
    pub q: f32,
    pub d: f32,
}

pub struct EStop {}

pub enum Messages {
    IdleCurrentSense(IdleCurrentSense),
    ForcePwm(ForcePwm),
    SetCurrents(SetCurrents),
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

impl ExtendedFdcanFrame for ForcePwm {
    fn unpack(message: &FdcanMessage) -> Self {
        let buffer = message.data;
        ForcePwm {
            foo: buffer[0],
            pwm_duty: f32::from_bits(buffer[1]),
            baz: (buffer[2] & 0xFF) as u8,
            toot: [
                ((buffer[2] & (0xFF << 8)) >> 8) as u8,
                ((buffer[2] & (0xFF << 16)) >> 16) as u8,
                (buffer[2] >> 24) as u8,
            ],
        }
    }

    fn pack(&self) -> FdcanMessage {
        FdcanMessage::new(
            0xA,
            [
                self.foo as u32,
                self.pwm_duty.to_bits(),
                (self.baz as u32) << 24
                    | (self.toot[2] as u32) << 16
                    | (self.toot[1] as u32) << 8
                    | (self.toot[0] as u32),
            ],
        )
    }
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

impl ExtendedFdcanFrame for SetCurrents {
    fn unpack(message: &FdcanMessage) -> Self {
        let buffer = message.data;
        SetCurrents {
            q: buffer[0] as f32,
            d: buffer[1] as f32,
        }
    }

    fn pack(&self) -> FdcanMessage {
        FdcanMessage::new(0xB, [self.q as u32, self.d as u32])
    }
}

impl Messages {
    pub fn unpack_fdcan(message: &FdcanMessage) -> Option<Self> {
        match message.id {
            0x0 => Some(Self::EStop(EStop::unpack(message))),
            0xA => Some(Self::ForcePwm(ForcePwm::unpack(message))),
            0xB => Some(Self::SetCurrents(SetCurrents::unpack(message))),
            0xC => Some(Self::IdleCurrentSense(IdleCurrentSense::unpack(message))),
            _ => None,
        }
    }
}
