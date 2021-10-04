// Emergency Stop message.
pub struct EStop {}

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
}

// impl Messages {
//     pub fn unpack_fdcan(message: &FdcanMessage) -> Option<Self> {
//         match message.id {
//             0x0 => Some(Self::EStop(EStop::unpack(message))),
//             0xC => Some(Self::IdleCurrentSense(IdleCurrentSense::unpack(message))),
//             0xE => Some(Self::IdleCurrentDistribution(
//                 IdleCurrentDistribution::unpack(message),
//             )),
//             0xF => Some(Self::CalibrateADC(CalibrateADC::unpack(message))),
//             0x10 => Some(Self::MeasureInductance(MeasureInductance::unpack(message))),
//             0x11 => Some(Self::MeasureResistance(MeasureResistance::unpack(message))),
//             0x12 => Some(Self::PhaseCurrentCommand(PhaseCurrentCommand::unpack(
//                 message,
//             ))),
//             0x13 => Some(Self::ReadEncoder(ReadEncoderMsg::unpack(message))),
//             0x14 => Some(Self::CalibrateEZero(CalibrateEZeroMsg::unpack(message))),
//             _ => None,
//         }
//     }
// }
