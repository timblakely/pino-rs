use super::fdcan::FdcanMessage;

// Just for testing; do not use in regular communication.
pub struct Debug {
    pub foo: u32,
    pub bar: f32,
    pub baz: u8,
    pub toot: [u8; 3],
}

pub struct Debug2 {
    pub first: u32,
    pub second: i32,
}

pub enum Messages {
    Debug(Debug),
    Debug2(Debug2),
}

pub trait ExtendedFdcanFrame {
    // Unpack the message from a buffer.
    fn unpack(message: &FdcanMessage) -> Self;

    // Pack the message into a buffer of up to 64 bytes, returning the number of bytes that were
    // packed.
    fn pack(&self) -> FdcanMessage;
}

impl ExtendedFdcanFrame for Debug {
    fn unpack(message: &FdcanMessage) -> Self {
        let buffer = message.data;
        Debug {
            foo: buffer[0],
            bar: f32::from_bits(buffer[0]),
            baz: (buffer[2] >> 24) as u8,
            toot: [
                ((buffer[2] & (0xFF << 16)) >> 16) as u8,
                ((buffer[2] & (0xFF << 8)) >> 8) as u8,
                (buffer[2] & 0xFF) as u8,
            ],
        }
    }

    fn pack(&self) -> FdcanMessage {
        FdcanMessage::new(
            0xA,
            [
                self.foo as u32,
                self.bar.to_bits(),
                (self.baz as u32) << 24
                    | (self.toot[2] as u32) << 16
                    | (self.toot[1] as u32) << 8
                    | (self.toot[0] as u32),
            ],
        )
    }
}

impl ExtendedFdcanFrame for Debug2 {
    fn unpack(message: &FdcanMessage) -> Self {
        let buffer = message.data;
        Debug2 {
            first: buffer[0] as u32,
            second: buffer[1] as i32,
        }
    }

    fn pack(&self) -> FdcanMessage {
        FdcanMessage::new(0xB, [self.first, self.second as u32])
    }
}

impl Messages {
    pub fn unpack_fdcan(message: &FdcanMessage) -> Option<Self> {
        match message.id {
            0xA => Some(Self::Debug(Debug::unpack(message))),
            0xB => Some(Self::Debug2(Debug2::unpack(message))),
            _ => None,
        }
    }
}
