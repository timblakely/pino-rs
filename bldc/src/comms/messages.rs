use crate::comms::fdcan::ExtendedFdcanFrame;

// Just for testing; do not use in regular communication.
pub struct DebugMessage {
    pub foo: u32,
    pub bar: f32,
    pub baz: u8,
    pub toot: [u8; 3],
}

impl ExtendedFdcanFrame for DebugMessage {
    fn id(&self) -> u32 {
        0xA
    }

    fn unpack(buffer: &[u32; 16]) -> DebugMessage {
        DebugMessage {
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
        3
    }
}
