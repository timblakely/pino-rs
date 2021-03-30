mod r0 {
    use crate::util::bitfield;
    use crate::{readable_accessor, readwrite_field, writable_accessor};
    pub type ReadProxy = bitfield::ReadProxy<u32, R0>;
    pub type WriteProxy = bitfield::WriteProxy<u32, R0>;

    readwrite_field!(ESI, u8, 0b1, 31);
    readwrite_field!(XTD, u8, 0b1, 30);
    readwrite_field!(RTR, u8, 0b1, 29);
    readwrite_field!(ID, u32, 0x1FF_FFFF, 0);

    impl ReadProxy {
        readable_accessor!(error_state, ESI, u8, 0b1, 31);
        readable_accessor!(extended_id, XTD, u8, 0b1, 30);
        readable_accessor!(remote_transmission, RTR, u8, 0b1, 29);
        readable_accessor!(id, ID, u32, 0x1FF_FFFF, 0);
    }

    impl WriteProxy {
        writable_accessor!(error_state, ESI);
        writable_accessor!(extended_id, XTD);
        writable_accessor!(remote_transmission, RTR);
        writable_accessor!(id, ID);
    }

    pub type R0 = bitfield::Bitfield<u32, _R0>;
    impl bitfield::Readable for R0 {}
    impl bitfield::Writeable for R0 {}
    #[allow(missing_docs)]
    #[doc(hidden)]
    pub struct _R0;
}

#[repr(C)]
pub struct RxFifo {
    r0: r0::R0,
}
