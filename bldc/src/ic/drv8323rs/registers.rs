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

pub mod gdhs {
    use crate::util::bitfield;
    use crate::{readable_accessor, readwrite_field, writable_accessor};
    pub type ReadProxy = bitfield::ReadProxy<u16, GateDriveHighSide>;
    pub type WriteProxy = bitfield::WriteProxy<u16, GateDriveHighSide>;

    readwrite_field!(LOCK, u8, 0b111, 8);
    readwrite_field!(IDRIVEP_HS, u8, 0b1111, 4);
    readwrite_field!(IDRIVEN_HS, u8, 0b1111, 0);

    impl ReadProxy {
        readable_accessor!(lock, LOCK, u8, 0b111, 8);
        readable_accessor!(idrive_p, IDRIVEP_HS, u8, 0b1111, 4);
        readable_accessor!(idrive_n, IDRIVEN_HS, u8, 0b1111, 0);
    }

    impl WriteProxy {
        writable_accessor!(lock, LOCK);
        writable_accessor!(idrive_p, IDRIVEP_HS);
        writable_accessor!(idrive_n, IDRIVEN_HS);
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
