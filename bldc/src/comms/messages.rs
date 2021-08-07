use super::fdcan::{ExtendedFdcanFrame, FdcanMessage};

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

    fn pack(&self, buffer: &mut [u32; 16]) -> FdcanMessage {
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

    fn pack(&self, buffer: &mut [u32; 16]) -> FdcanMessage {
        FdcanMessage::new(0xB, [self.first, self.second as u32])
    }
}
