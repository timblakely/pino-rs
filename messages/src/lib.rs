#![cfg_attr(not(test), no_std)]
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
}
