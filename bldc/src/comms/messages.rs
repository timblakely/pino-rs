use crate::foc::DQCurrents;

use super::fdcan::{FdcanMessage, IncomingFdcanFrame, OutgoingFdcanFrame};

// Emergency Stop message.
pub struct EStop {}

pub struct TorqueControlCmd {
    pub duration: f32,
    pub currents: DQCurrents,
}

#[derive(Clone, Copy)]
pub struct PosVelCommand {
    pub position: f32,
    pub velocity: f32,
    pub stiffness_gain: f32,
    pub damping_gain: f32,
    pub torque_constant: f32,
}

pub struct EZeroMsg {
    pub e_angle: f32,
    pub e_raw: f32,
    pub angle: f32,
    pub angle_raw: u32,
}

pub struct CalibrateEZeroCmd {
    pub duration: f32,
    pub currents: DQCurrents,
}

pub struct StartStreamCmd {
    pub frequency: f32,
}

pub enum Message {
    // EStop = 0x0,
    // PhaseCurrents = 0xD,
    // CalibrateADC = 0xF,
    // CurrentDistribution = 0x10,
    // Resistance = 0x12,
    // EncoderResults = 0x13,
    // Inductances = 0x14,
    CalibrateEZero(CalibrateEZeroCmd),
    // EZero = 0x16,
    TorqueControl(TorqueControlCmd),
    PosVelControl,
    PosVelCommand(PosVelCommand),

    BeginStateStream(StartStreamCmd),
    SensorState,
    EndStateStream,
    Unknown,
}

impl Message {
    pub fn parse(message: FdcanMessage) -> Self {
        match message.id {
            0x15 => Message::CalibrateEZero(CalibrateEZeroCmd::unpack(message)),
            0x17 => Message::TorqueControl(TorqueControlCmd::unpack(message)),
            0x18 => Message::PosVelControl,
            0x19 => Message::PosVelCommand(PosVelCommand::unpack(message)),
            0x1A => Message::BeginStateStream(StartStreamCmd::unpack(message)),
            0x1C => Message::EndStateStream,
            _ => Message::Unknown,
        }
    }
}

// TODO(blakely): move these somewhere else
impl IncomingFdcanFrame for TorqueControlCmd {
    fn unpack(message: FdcanMessage) -> Self {
        let buffer = message.data;
        TorqueControlCmd {
            duration: f32::from_bits(buffer[0]),
            currents: DQCurrents {
                q: f32::from_bits(buffer[1]),
                d: f32::from_bits(buffer[2]),
            },
        }
    }
}

impl IncomingFdcanFrame for PosVelCommand {
    fn unpack(message: FdcanMessage) -> Self {
        let buffer = message.data;
        PosVelCommand {
            position: f32::from_bits(buffer[0]),
            velocity: f32::from_bits(buffer[1]),
            stiffness_gain: f32::from_bits(buffer[2]),
            damping_gain: f32::from_bits(buffer[3]),
            torque_constant: f32::from_bits(buffer[4]),
        }
    }
}

impl<'a> OutgoingFdcanFrame for EZeroMsg {
    fn pack(&self) -> FdcanMessage {
        FdcanMessage::new(
            0x15,
            &[
                self.angle.to_bits(),
                self.angle_raw,
                self.e_angle.to_bits(),
                self.e_raw.to_bits(),
            ],
        )
    }
}

impl IncomingFdcanFrame for CalibrateEZeroCmd {
    fn unpack(message: FdcanMessage) -> Self {
        let buffer = message.data;
        CalibrateEZeroCmd {
            duration: f32::from_bits(buffer[0]),
            currents: DQCurrents {
                q: f32::from_bits(buffer[1]),
                d: f32::from_bits(buffer[2]),
            },
        }
    }
}

impl IncomingFdcanFrame for StartStreamCmd {
    fn unpack(msg: FdcanMessage) -> Self {
        let buffer = msg.data;
        StartStreamCmd {
            frequency: f32::from_bits(buffer[0]),
        }
    }
}

pub trait FdcanID {
    const ID: MessageID;
}

pub enum MessageID {
    EnterTorqueControl = 0x17,
    EnterPosVelControl,
}

impl From<MessageID> for u32 {
    fn from(id: MessageID) -> Self {
        id as u32
    }
}
