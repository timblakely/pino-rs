use crate::writable_variant_from;

pub trait Addr {
    fn addr() -> u8;
}

pub mod fs1 {
    use crate::util::bitfield;
    use crate::{readable_accessor, readwrite_field, writable_accessor};
    pub type ReadProxy = bitfield::ReadProxy<u16, FaultStatus1>;
    pub type WriteProxy = bitfield::WriteProxy<u16, FaultStatus1>;

    readwrite_field!(FAULT, u8, 0b1, 10);
    readwrite_field!(VDS_OCP, u8, 0b1, 9);
    readwrite_field!(GDF, u8, 0b1, 8);
    readwrite_field!(UVLO, u8, 0b1, 7);
    readwrite_field!(OTSD, u8, 0b1, 6);
    readwrite_field!(VDS_HA, u8, 0b1, 5);
    readwrite_field!(VDS_LA, u8, 0b1, 4);
    readwrite_field!(VDS_HB, u8, 0b1, 3);
    readwrite_field!(VDS_LB, u8, 0b1, 2);
    readwrite_field!(VDS_HC, u8, 0b1, 1);
    readwrite_field!(VDS_LC, u8, 0b1, 0);

    impl ReadProxy {
        readable_accessor!(any_fault, FAULT, u8, 0b1, 10);
        readable_accessor!(overcurrent, VDS_OCP, u8, 0b1, 9);
        readable_accessor!(gate_drive, GDF, u8, 0b1, 8);
        readable_accessor!(under_voltage, UVLO, u8, 0b1, 7);
        readable_accessor!(over_temp, OTSD, u8, 0b1, 6);
        readable_accessor!(overcurrent_a_high, VDS_HA, u8, 0b1, 5);
        readable_accessor!(overcurrent_a_low, VDS_LA, u8, 0b1, 4);
        readable_accessor!(overcurrent_b_high, VDS_HB, u8, 0b1, 3);
        readable_accessor!(overcurrent_b_low, VDS_LB, u8, 0b1, 2);
        readable_accessor!(overcurrent_c_high, VDS_HC, u8, 0b1, 1);
        readable_accessor!(overcurrent_c_low, VDS_LC, u8, 0b1, 0);
    }

    impl WriteProxy {
        writable_accessor!(any_fault, FAULT);
        writable_accessor!(overcurrent, VDS_OCP);
        writable_accessor!(gate_drive, GDF);
        writable_accessor!(under_voltage, UVLO);
        writable_accessor!(over_temp, OTSD);
        writable_accessor!(overcurrent_a_high, VDS_HA);
        writable_accessor!(overcurrent_a_low, VDS_LA);
        writable_accessor!(overcurrent_b_high, VDS_HB);
        writable_accessor!(overcurrent_b_low, VDS_LB);
        writable_accessor!(overcurrent_c_high, VDS_HC);
        writable_accessor!(overcurrent_c_low, VDS_LC);
    }

    pub type FaultStatus1 = bitfield::Bitfield<u16, _FaultStatus1>;
    impl bitfield::Readable for FaultStatus1 {}
    impl bitfield::Writeable for FaultStatus1 {}
    #[allow(missing_docs)]
    #[doc(hidden)]
    pub struct _FaultStatus1;
    impl super::Addr for FaultStatus1 {
        fn addr() -> u8 {
            0x0
        }
    }
}

pub mod fs2 {
    use crate::util::bitfield;
    use crate::{readable_accessor, readwrite_field, writable_accessor};
    pub type ReadProxy = bitfield::ReadProxy<u16, FaultStatus2>;
    pub type WriteProxy = bitfield::WriteProxy<u16, FaultStatus2>;

    readwrite_field!(SA_OC, u8, 0b1, 10);
    readwrite_field!(SB_OC, u8, 0b1, 9);
    readwrite_field!(SC_OC, u8, 0b1, 8);
    readwrite_field!(OTW, u8, 0b1, 7);
    readwrite_field!(CPUV, u8, 0b1, 6);
    readwrite_field!(VGS_HA, u8, 0b1, 5);
    readwrite_field!(VGS_LA, u8, 0b1, 4);
    readwrite_field!(VGS_HB, u8, 0b1, 3);
    readwrite_field!(VGS_LB, u8, 0b1, 2);
    readwrite_field!(VGS_HC, u8, 0b1, 1);
    readwrite_field!(VGS_LC, u8, 0b1, 0);

    impl ReadProxy {
        readable_accessor!(sense_a_overcurrent, SA_OC, u8, 0b1, 10);
        readable_accessor!(sense_b_overcurrent, SB_OC, u8, 0b1, 9);
        readable_accessor!(sense_c_overcurrent, SC_OC, u8, 0b1, 8);
        readable_accessor!(overtemp, OTW, u8, 0b1, 7);
        readable_accessor!(charge_pump_uvlo, CPUV, u8, 0b1, 6);
        readable_accessor!(gate_drive_fault_high_a, VGS_HA, u8, 0b1, 5);
        readable_accessor!(gate_drive_fault_low_a, VGS_LA, u8, 0b1, 4);
        readable_accessor!(gate_drive_fault_high_b, VGS_HB, u8, 0b1, 3);
        readable_accessor!(gate_drive_fault_low_b, VGS_LB, u8, 0b1, 2);
        readable_accessor!(gate_drive_fault_high_c, VGS_HC, u8, 0b1, 1);
        readable_accessor!(gate_drive_fault_low_c, VGS_LC, u8, 0b1, 0);
    }

    impl WriteProxy {
        writable_accessor!(sense_a_overcurrent, SA_OC);
        writable_accessor!(sense_b_overcurrent, SB_OC);
        writable_accessor!(sense_c_overcurrent, SC_OC);
        writable_accessor!(overtemp, OTW);
        writable_accessor!(charge_pump_uvlo, CPUV);
        writable_accessor!(gate_drive_fault_high_a, VGS_HA);
        writable_accessor!(gate_drive_fault_low_a, VGS_LA);
        writable_accessor!(gate_drive_fault_high_b, VGS_HB);
        writable_accessor!(gate_drive_fault_low_b, VGS_LB);
        writable_accessor!(gate_drive_fault_high_c, VGS_HC);
        writable_accessor!(gate_drive_fault_low_c, VGS_LC);
    }

    pub type FaultStatus2 = bitfield::Bitfield<u16, _FaultStatus2>;
    impl bitfield::Readable for FaultStatus2 {}
    impl bitfield::Writeable for FaultStatus2 {}
    #[allow(missing_docs)]
    #[doc(hidden)]
    pub struct _FaultStatus2;
    impl super::Addr for FaultStatus2 {
        fn addr() -> u8 {
            0x1
        }
    }
}

pub mod cr {
    use super::PwmMode;
    use crate::util::bitfield;
    use crate::{readable_accessor, readwrite_field, writable_accessor};
    pub type ReadProxy = bitfield::ReadProxy<u16, ControlRegister>;
    pub type WriteProxy = bitfield::WriteProxy<u16, ControlRegister>;

    readwrite_field!(DIS_CPUV, u8, 0b1, 9);
    readwrite_field!(DIS_GDF, u8, 0b1, 8);
    readwrite_field!(OTW_REP, u8, 0b1, 7);
    readwrite_field!(PWM_MODE, u8, 0b11, 5, PwmMode);
    readwrite_field!(PWM1_COM, u8, 0b1, 4);
    readwrite_field!(PWM1_DIR, u8, 0b1, 3);
    readwrite_field!(COAST, u8, 0b1, 2);
    readwrite_field!(BRAKE, u8, 0b1, 1);
    readwrite_field!(CLR_FLT, u8, 0b1, 0);

    impl ReadProxy {
        readable_accessor!(disable_uvlo, DIS_CPUV, u8, 0b1, 9);
        readable_accessor!(disable_gate_drive_fault, DIS_GDF, u8, 0b1, 8);
        readable_accessor!(overtemp_reporting, OTW_REP, u8, 0b1, 7);
        readable_accessor!(pwm_mode, PWM_MODE, u8, 0b11, 5);
        readable_accessor!(pwm_synchronous_rectification, PWM1_COM, u8, 0b1, 4);
        readable_accessor!(owm_or_with_inhc, PWM1_DIR, u8, 0b1, 3);
        readable_accessor!(all_fets_to_hiz, COAST, u8, 0b1, 2);
        readable_accessor!(short_low_side_fets, BRAKE, u8, 0b1, 1);
        readable_accessor!(clear_latched_faults, CLR_FLT, u8, 0b1, 0);
    }

    impl WriteProxy {
        writable_accessor!(disable_uvlo, DIS_CPUV);
        writable_accessor!(disable_gate_drive_fault, DIS_GDF);
        writable_accessor!(overtemp_reporting, OTW_REP);
        writable_accessor!(pwm_mode, PWM_MODE);
        writable_accessor!(pwm_synchronous_rectification, PWM1_COM);
        writable_accessor!(owm_or_with_inhc, PWM1_DIR);
        writable_accessor!(all_fets_to_hiz, COAST);
        writable_accessor!(short_low_side_fets, BRAKE);
        writable_accessor!(clear_latched_faults, CLR_FLT);
    }

    pub type ControlRegister = bitfield::Bitfield<u16, _GateDriveHighSide>;
    impl bitfield::Readable for ControlRegister {}
    impl bitfield::Writeable for ControlRegister {}
    #[allow(missing_docs)]
    #[doc(hidden)]
    pub struct _GateDriveHighSide;
    impl super::Addr for ControlRegister {
        fn addr() -> u8 {
            0x02
        }
    }
}

pub mod gdhs {
    use super::DriveCurrent;
    use crate::util::bitfield;
    use crate::{readable_accessor, readwrite_field, writable_accessor};
    pub type ReadProxy = bitfield::ReadProxy<u16, GateDriveHighSide>;
    pub type WriteProxy = bitfield::WriteProxy<u16, GateDriveHighSide>;

    readwrite_field!(LOCK, u8, 0b111, 8);
    readwrite_field!(IDRIVEP_HS, u8, 0b1111, 4, DriveCurrent);
    readwrite_field!(IDRIVEN_HS, u8, 0b1111, 0, DriveCurrent);

    impl ReadProxy {
        readable_accessor!(lock, LOCK, u8, 0b111, 8);
        readable_accessor!(idrive_p_high_side, IDRIVEP_HS, u8, 0b1111, 4);
        readable_accessor!(idrive_n_high_side, IDRIVEN_HS, u8, 0b1111, 0);
    }

    impl WriteProxy {
        writable_accessor!(lock, LOCK);
        writable_accessor!(idrive_p_high_side, IDRIVEP_HS);
        writable_accessor!(idrive_n_high_side, IDRIVEN_HS);
    }

    pub type GateDriveHighSide = bitfield::Bitfield<u16, _GateDriveHighSide>;
    impl bitfield::Readable for GateDriveHighSide {}
    impl bitfield::Writeable for GateDriveHighSide {}
    #[allow(missing_docs)]
    #[doc(hidden)]
    pub struct _GateDriveHighSide;
    impl super::Addr for GateDriveHighSide {
        fn addr() -> u8 {
            0x03
        }
    }
}

pub mod gdls {
    use super::DriveCurrent;
    use crate::util::bitfield;
    use crate::{readable_accessor, readwrite_field, writable_accessor};
    pub type ReadProxy = bitfield::ReadProxy<u16, GateDriveLowSide>;
    pub type WriteProxy = bitfield::WriteProxy<u16, GateDriveLowSide>;

    readwrite_field!(CBC, u8, 0b1, 10);
    readwrite_field!(TDRIVE, u8, 0b11, 8);
    readwrite_field!(IDRIVEP_LS, u8, 0b1111, 4, DriveCurrent);
    readwrite_field!(IDRIVEN_LS, u8, 0b1111, 0, DriveCurrent);

    impl ReadProxy {
        readable_accessor!(cycle_by_cycle, CBC, u8, 0b1, 10);
        readable_accessor!(gate_current_drive_time, TDRIVE, u8, 0b11, 8);
        readable_accessor!(idrive_p_low_side, IDRIVEP_LS, u8, 0b1111, 4);
        readable_accessor!(idrive_n_low_side, IDRIVEN_LS, u8, 0b1111, 0);
    }

    impl WriteProxy {
        writable_accessor!(cycle_by_cycle, CBC);
        writable_accessor!(gate_current_drive_time, TDRIVE);
        writable_accessor!(idrive_p_low_side, IDRIVEP_LS);
        writable_accessor!(idrive_n_low_side, IDRIVEN_LS);
    }

    pub type GateDriveLowSide = bitfield::Bitfield<u16, _GateDriveLowSide>;
    impl bitfield::Readable for GateDriveLowSide {}
    impl bitfield::Writeable for GateDriveLowSide {}
    #[allow(missing_docs)]
    #[doc(hidden)]
    pub struct _GateDriveLowSide;
    impl super::Addr for GateDriveLowSide {
        fn addr() -> u8 {
            0x04
        }
    }
}

pub mod ocp {
    use super::{DeadTime, OcpDeglitch, OcpMode, RetryTime, VdsLevel};
    use crate::util::bitfield;
    use crate::{readable_accessor, readwrite_field, writable_accessor};
    pub type ReadProxy = bitfield::ReadProxy<u16, OverCurrentProtection>;
    pub type WriteProxy = bitfield::WriteProxy<u16, OverCurrentProtection>;

    readwrite_field!(TRETRY, u8, 0b1, 10, RetryTime);
    readwrite_field!(DEAD_TIME, u8, 0b11, 8, DeadTime);
    readwrite_field!(OCP_MODE, u8, 0b11, 6, OcpMode);
    readwrite_field!(OCP_DEG, u8, 0b11, 4, OcpDeglitch);
    readwrite_field!(VDS_LVL, u8, 0b1111, 0, VdsLevel);

    impl ReadProxy {
        readable_accessor!(ocp_retry, TRETRY, u8, 0b1, 10);
        readable_accessor!(dead_time, DEAD_TIME, u8, 0b11, 8);
        readable_accessor!(ocp_mode, OCP_MODE, u8, 0b11, 6);
        readable_accessor!(ocp_deglitch, OCP_DEG, u8, 0b11, 4);
        readable_accessor!(vds_level, VDS_LVL, u8, 0b1111, 0);
    }

    impl WriteProxy {
        writable_accessor!(ocp_retry, TRETRY);
        writable_accessor!(dead_time, DEAD_TIME);
        writable_accessor!(ocp_mode, OCP_MODE);
        writable_accessor!(ocp_deglitch, OCP_DEG);
        writable_accessor!(vds_level, VDS_LVL);
    }

    pub type OverCurrentProtection = bitfield::Bitfield<u16, _OverCurrentProtection>;
    impl bitfield::Readable for OverCurrentProtection {}
    impl bitfield::Writeable for OverCurrentProtection {}
    #[allow(missing_docs)]
    #[doc(hidden)]
    pub struct _OverCurrentProtection;
    impl super::Addr for OverCurrentProtection {
        fn addr() -> u8 {
            0x05
        }
    }
}

pub mod csa {
    use super::{
        CsaDivisor, CsaGain, CsaLowSideRef, CsaPositiveInput, OffsetCalibration, SenseOcp,
        SenseOvercurrent,
    };
    use crate::util::bitfield;
    use crate::{readable_accessor, readwrite_field, writable_accessor};
    pub type ReadProxy = bitfield::ReadProxy<u16, CurrentSenseAmplifier>;
    pub type WriteProxy = bitfield::WriteProxy<u16, CurrentSenseAmplifier>;

    readwrite_field!(CSA_FET, u8, 0b1, 10, CsaPositiveInput);
    readwrite_field!(VREF_DIV, u8, 0b1, 9, CsaDivisor);
    readwrite_field!(LS_REF, u8, 0b1, 8, CsaLowSideRef);
    readwrite_field!(CSA_GAIN, u8, 0b11, 6, CsaGain);
    readwrite_field!(DIS_SEN, u8, 0b1, 5, SenseOvercurrent);
    readwrite_field!(CSA_CAL_A, u8, 0b1, 4, OffsetCalibration);
    readwrite_field!(CSA_CAL_B, u8, 0b1, 3, OffsetCalibration);
    readwrite_field!(CSA_CAL_C, u8, 0b1, 2, OffsetCalibration);
    readwrite_field!(SEN_LVL, u8, 0b1, 0, SenseOcp);

    impl ReadProxy {
        readable_accessor!(current_sense_input, CSA_FET, u8, 0b1, 10);
        readable_accessor!(vref_divisor, VREF_DIV, u8, 0b1, 9);
        readable_accessor!(low_side_reference, LS_REF, u8, 0b1, 8);
        readable_accessor!(current_sense_gain, CSA_GAIN, u8, 0b11, 6);
        readable_accessor!(overcurrent_fault, DIS_SEN, u8, 0b1, 5);
        readable_accessor!(offset_calibration_a, CSA_CAL_A, u8, 0b1, 4);
        readable_accessor!(offset_calibration_b, CSA_CAL_B, u8, 0b1, 3);
        readable_accessor!(offset_calibration_c, CSA_CAL_C, u8, 0b1, 2);
        readable_accessor!(sense_level, SEN_LVL, u8, 0b1, 0);
    }

    impl WriteProxy {
        writable_accessor!(current_sense_input, CSA_FET);
        writable_accessor!(vref_divisor, VREF_DIV);
        writable_accessor!(low_side_reference, LS_REF);
        writable_accessor!(current_sense_gain, CSA_GAIN);
        writable_accessor!(overcurrent_fault, DIS_SEN);
        writable_accessor!(offset_calibration_a, CSA_CAL_A);
        writable_accessor!(offset_calibration_b, CSA_CAL_B);
        writable_accessor!(offset_calibration_c, CSA_CAL_C);
        writable_accessor!(sense_level, SEN_LVL);
    }

    pub type CurrentSenseAmplifier = bitfield::Bitfield<u16, _CurrentSenseAmplifier>;
    impl bitfield::Readable for CurrentSenseAmplifier {}
    impl bitfield::Writeable for CurrentSenseAmplifier {}
    #[allow(missing_docs)]
    #[doc(hidden)]
    pub struct _CurrentSenseAmplifier;
    impl super::Addr for CurrentSenseAmplifier {
        fn addr() -> u8 {
            0x06
        }
    }
}

pub enum PwmMode {
    Pwm6x = 0b00,
    Pwm3x = 0b01,
    Pwm1x = 0b10,
    Independent = 0b11,
}

pub enum DriveCurrent {
    Milli10 = 0b0000,
    Milli30 = 0b0001,
    Milli60 = 0b0010,
    Milli80 = 0b0011,
    Milli120 = 0b0100,
    Milli140 = 0b0101,
    Milli170 = 0b0110,
    Milli190 = 0b0111,
    Milli260 = 0b1000,
    Milli330 = 0b1001,
    Milli370 = 0b1010,
    Milli440 = 0b1011,
    Milli570 = 0b1100,
    Milli680 = 0b1101,
    Milli820 = 0b1110,
    Milli1000 = 0b1111,
}

pub enum RetryTime {
    Milli4 = 0,
    Micro50 = 1,
}

pub enum DeadTime {
    Nanos50 = 0b00,
    Nanos100 = 0b01,
    Nanos200 = 0b10,
    Nanos400 = 0b11,
}

pub enum OcpMode {
    LatchedFault = 0b00,
    AutoRetry = 0b01,
    ReportOnly = 0b10,
    Ignore = 0b11,
}

pub enum OcpDeglitch {
    Micros2 = 0b00,
    Micros4 = 0b01,
    Micros6 = 0b10,
    Micros8 = 0b11,
}

pub enum VdsLevel {
    V0_06 = 0b0000,
    V0_13 = 0b0001,
    V0_2 = 0b0010,
    V0_26 = 0b0011,
    V0_31 = 0b0100,
    V0_45 = 0b0101,
    V0_53 = 0b0110,
    V0_6 = 0b0111,
    V0_68 = 0b1000,
    V0_75 = 0b1001,
    V0_94 = 0b1010,
    V1_13 = 0b1011,
    V1_3 = 0b1100,
    V1_5 = 0b1101,
    V1_7 = 0b1110,
    V1_88 = 0b1111,
}

pub enum CsaPositiveInput {
    SPx = 0,
    SHx = 1,
}

pub enum CsaDivisor {
    One = 0,
    Two = 1,
}

pub enum CsaLowSideRef {
    SPx = 0,
    SNx = 1,
}

pub enum CsaGain {
    V5 = 0b00,
    V10 = 0b01,
    V20 = 0b10,
    V40 = 0b11,
}

pub enum SenseOvercurrent {
    Enabled = 0b0,
    Disabled = 0b1,
}

pub enum OffsetCalibration {
    Normal = 0b0,
    Calibration = 0b1,
}

pub enum SenseOcp {
    V0_25 = 0b00,
    V0_5 = 0b01,
    V0_75 = 0b10,
    V1 = 0b11,
}

writable_variant_from!(CsaDivisor, u8);
writable_variant_from!(CsaGain, u8);
writable_variant_from!(CsaLowSideRef, u8);
writable_variant_from!(CsaPositiveInput, u8);
writable_variant_from!(DeadTime, u8);
writable_variant_from!(DriveCurrent, u8);
writable_variant_from!(OcpDeglitch, u8);
writable_variant_from!(OcpMode, u8);
writable_variant_from!(OffsetCalibration, u8);
writable_variant_from!(PwmMode, u8);
writable_variant_from!(RetryTime, u8);
writable_variant_from!(SenseOcp, u8);
writable_variant_from!(SenseOvercurrent, u8);
writable_variant_from!(VdsLevel, u8);
