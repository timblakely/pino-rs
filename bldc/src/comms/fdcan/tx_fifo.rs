mod t0 {
    use crate::util::bitfield;
    use crate::{readable_accessor, readwrite_field, writable_accessor};
    pub type ReadProxy = bitfield::ReadProxy<u32, T0>;
    pub type WriteProxy = bitfield::WriteProxy<u32, T0>;

    readwrite_field!(ESI, u8, 0b1, 31);
    readwrite_field!(XTD, u8, 0b1, 30);
    readwrite_field!(RTR, u8, 0b1, 29);
    readwrite_field!(ID, u32, 0x1FFF_FFFF, 0);

    impl ReadProxy {
        readable_accessor!(error_state, ESI, u8, 0b1, 31);
        readable_accessor!(extended_id, XTD, u8, 0b1, 30);
        readable_accessor!(remote_transmission, RTR, u8, 0b1, 29);
        readable_accessor!(id, ID, u32, 0x1FFF_FFFF, 0);
    }

    impl WriteProxy {
        writable_accessor!(error_state, ESI);
        writable_accessor!(extended_id, XTD);
        writable_accessor!(remote_transmission, RTR);
        writable_accessor!(id, ID);
    }

    pub type T0 = bitfield::Bitfield<u32, _T0>;
    impl bitfield::Readable for T0 {}
    impl bitfield::Writeable for T0 {}
    #[allow(missing_docs)]
    #[doc(hidden)]
    pub struct _T0;
}

mod t1 {
    use crate::util::bitfield;
    use crate::{readable_accessor, readwrite_field, writable_accessor};
    pub type ReadProxy = bitfield::ReadProxy<u32, T1>;
    pub type WriteProxy = bitfield::WriteProxy<u32, T1>;

    readwrite_field!(MM, u8, 0xFF, 24);
    readwrite_field!(EFC, u8, 0b1, 23);
    readwrite_field!(FDF, u8, 0b1, 21);
    readwrite_field!(BRS, u8, 0b1, 20);
    readwrite_field!(DLC, u8, 0b1111, 16);

    impl ReadProxy {
        readable_accessor!(message_marker, MM, u8, 0xFF, 24);
        readable_accessor!(event_type, EFC, u8, 0b1, 23);
        readable_accessor!(fdcan_frame, FDF, u8, 0b1, 21);
        readable_accessor!(bit_rate_switch, BRS, u8, 0b1, 20);
        readable_accessor!(data_length, DLC, u8, 0b1111, 16);
    }

    impl WriteProxy {
        writable_accessor!(message_marker, MM);
        writable_accessor!(event_type, EFC);
        writable_accessor!(fdcan_frame, FDF);
        writable_accessor!(bit_rate_switch, BRS);
        writable_accessor!(data_length, DLC);
    }

    pub type T1 = bitfield::Bitfield<u32, _T1>;
    impl bitfield::Readable for T1 {}
    impl bitfield::Writeable for T1 {}
    #[allow(missing_docs)]
    #[doc(hidden)]
    pub struct _T1;
}

#[repr(C)]
pub struct TxFifo {
    t0: t0::T0,
    t1: t1::T1,
    data: [u32; 16],
}
