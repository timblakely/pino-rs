use crate::comms::fdcan::{ExtendedFdcanFrame, ReceivedMessage};

use super::fdcan::FdcanMessageTranslator;

// Just for testing; do not use in regular communication.
pub struct Debug {
    pub foo: u32,
    pub bar: f32,
    pub baz: u8,
    pub toot: [u8; 3],
}

impl ExtendedFdcanFrame for Debug {
    fn id(&self) -> u32 {
        0xA
    }

    fn unpack(buffer: &[u32; 16]) -> Debug {
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

    fn pack(&self, buffer: &mut [u32; 16]) -> u8 {
        buffer[0] = self.foo;
        buffer[1] = self.bar.to_bits();
        buffer[2] = (self.baz as u32) << 24
            | (self.toot[2] as u32) << 16
            | (self.toot[1] as u32) << 8
            | (self.toot[0] as u32);
        24
    }
}

pub struct Debug2 {
    pub first: u32,
    pub second: i32,
}

impl ExtendedFdcanFrame for Debug2 {
    fn id(&self) -> u32 {
        0xB
    }

    fn unpack(buffer: &[u32; 16]) -> Debug2 {
        Debug2 {
            first: buffer[0] as u32,
            second: buffer[1] as i32,
        }
    }

    fn pack(&self, buffer: &mut [u32; 16]) -> u8 {
        buffer[0] = self.first as u32;
        buffer[1] = self.second as u32;
        8
    }
}

enum Messages {
    Debug(Debug),
    Debug2(Debug2),
}

impl FdcanMessageTranslator for Messages {
    fn id(&self) -> u32 {
        match self {
            Self::Debug(x) => x.id(),
            Self::Debug2(x) => x.id(),
        }
    }

    fn unpack(message: &ReceivedMessage) -> Option<Self> {
        match message.id {
            0xA => Some(Messages::Debug(Debug::unpack(&message.data))),
            0xB => Some(Messages::Debug2(Debug2::unpack(&message.data))),
            _ => None,
        }
    }

    fn pack(&self, buffer: &mut [u32; 16]) -> u8 {
        match self {
            Self::Debug(x) => x.pack(buffer),
            Self::Debug2(x) => x.pack(buffer),
        }
    }
}
