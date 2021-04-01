mod e0 {
    use crate::util::bitfield;
    use crate::{readable_accessor, readwrite_field, writable_accessor};
    pub type ReadProxy = bitfield::ReadProxy<u32, E0>;
    pub type WriteProxy = bitfield::WriteProxy<u32, E0>;

    readwrite_field!(ESI, u8, 0b1, 31);
    readwrite_field!(XTD, u8, 0b1, 30);
    readwrite_field!(RTR, u8, 0b1, 29);
    readwrite_field!(SID, u32, 0x7FF, 18);
    readwrite_field!(EID, u32, 0x1FFF_FFFF, 0);

    impl ReadProxy {
        readable_accessor!(error_state, ESI, u8, 0b1, 31);
        readable_accessor!(extended, XTD, u8, 0b1, 30);
        readable_accessor!(remote_transmission, RTR, u8, 0b1, 29);
        readable_accessor!(standard_id, SID, u32, 0x7FF, 18);
        readable_accessor!(extended_id, EID, u32, 0x1FFF_FFFF, 0);
    }

    impl WriteProxy {
        writable_accessor!(error_state, ESI);
        writable_accessor!(extended, XTD);
        writable_accessor!(remote_transmission, RTR);
        writable_accessor!(standard_id, SID);
        writable_accessor!(extended_id, EID);
    }

    pub type E0 = bitfield::Bitfield<u32, _E0>;
    impl bitfield::Readable for E0 {}
    impl bitfield::Writeable for E0 {}
    #[allow(missing_docs)]
    #[doc(hidden)]
    pub struct _E0;
}

mod e1 {
    use crate::util::bitfield;
    use crate::{readable_accessor, readwrite_field, writable_accessor};
    pub type ReadProxy = bitfield::ReadProxy<u32, E1>;
    pub type WriteProxy = bitfield::WriteProxy<u32, E1>;

    readwrite_field!(MM, u8, 0xFF, 24);
    readwrite_field!(ET, u8, 0b11, 22);
    readwrite_field!(EDL, u8, 0b1, 21);
    readwrite_field!(BRS, u8, 0b1, 20);
    readwrite_field!(DLC, u8, 0b1111, 16);
    readwrite_field!(TXTS, u8, 0xFFFF, 0);

    impl ReadProxy {
        readable_accessor!(message_marker, MM, u8, 0xFF, 24);
        readable_accessor!(event_type, ET, u8, 0b11, 22);
        readable_accessor!(fdcan_frame, EDL, u8, 0b1, 21);
        readable_accessor!(bit_rate_switch, BRS, u8, 0b1, 20);
        readable_accessor!(data_length, DLC, u8, 0b1111, 16);
        readable_accessor!(timestamp, TXTS, u8, 0xFFFF, 0);
    }

    impl WriteProxy {
        writable_accessor!(message_marker, MM);
        writable_accessor!(event_type, ET);
        writable_accessor!(fdcan_frame, EDL);
        writable_accessor!(bit_rate_switch, BRS);
        writable_accessor!(data_length, DLC);
        writable_accessor!(timestamp, TXTS);
    }

    pub type E1 = bitfield::Bitfield<u32, _E1>;
    impl bitfield::Readable for E1 {}
    impl bitfield::Writeable for E1 {}
    #[allow(missing_docs)]
    #[doc(hidden)]
    pub struct _E1;
}

#[repr(C)]
pub struct TxEvent {
    e0: e0::E0,
    e1: e1::E1,
}
