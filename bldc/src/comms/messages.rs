// Emergency Stop message.
pub struct EStop {}

#[derive(Clone, Copy)]
pub enum Message {
    EStop = 0x0,
    PhaseCurrents = 0xD,
    CalibrateADC = 0xF,
    CurrentDistribution = 0x10,
    Resistance = 0x12,
    EncoderResults = 0x13,
    Inductances = 0x14,
    CalibrateEZero = 0x15,
    EZero = 0x16,
    TorqueControl = 0x17,
    PosVelControl = 0x18,
    PosVelCommand = 0x19,

    BeginStateStream = 0x1A,
    SensorState = 0x1B,
    EndStateStream = 0x1C,

    Unknown,
}

impl From<u32> for Message {
    fn from(id: u32) -> Self {
        match id {
            0x15 => Message::CalibrateEZero,
            0x17 => Message::TorqueControl,
            0x18 => Message::PosVelControl,
            0x19 => Message::PosVelCommand,
            0x1A => Message::BeginStateStream,
            0x1B => Message::EndStateStream,
            _ => Message::Unknown,
        }
    }
}
