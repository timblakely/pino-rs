use super::{ExtendedFdcanFrame, FdcanMessage};

mod t0 {
    pub type ReadProxy = bitfield::ReadProxy<u32, T0>;
    pub type WriteProxy = bitfield::WriteProxy<u32, T0>;
    use crate::util::bitfield;
    use crate::{readable_accessor, readwrite_field, writable_accessor};

    readwrite_field!(ESI, u8, 0b1, 31);
    readwrite_field!(XTD, u8, 0b1, 30);
    readwrite_field!(RTR, u8, 0b1, 29);
    readwrite_field!(bitsafe SID, u32, 0x7FF, 18);
    readwrite_field!(bitsafe EID, u32, 0x1FFF_FFFF, 0);

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

    readwrite_field!(bitsafe MM, u8, 0xFF, 24);
    readwrite_field!(EFC, u8, 0b1, 23);
    readwrite_field!(FDF, u8, 0b1, 21);
    readwrite_field!(BRS, u8, 0b1, 20);
    readwrite_field!(bitsafe DLC, u8, 0b1111, 16);

    impl ReadProxy {
        readable_accessor!(message_marker, MM, u8, 0xFF, 24);
        readable_accessor!(store_fifo, EFC, u8, 0b1, 23);
        readable_accessor!(fdcan_frame, FDF, u8, 0b1, 21);
        readable_accessor!(bit_rate_switch, BRS, u8, 0b1, 20);
        readable_accessor!(data_length, DLC, u8, 0b1111, 16);
    }

    impl WriteProxy {
        writable_accessor!(message_marker, MM);
        writable_accessor!(store_fifo, EFC);
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

impl TxFifo {
    pub fn data_mut(&mut self) -> &mut [u32; 16] {
        &mut self.data
    }

    pub fn assign(&mut self, frame: &FdcanMessage) {
        self.t0.update(|_, w| {
            w.error_state()
                .clear_bit()
                .extended()
                .set_bit()
                .remote_transmission()
                .clear_bit()
                .extended_id()
                .set(frame.id)
        });
        self.data.copy_from_slice(&frame.data);
        let frame_size_bytes = match frame.size {
            x if x <= 8 => x,
            x if x <= 12 => 9,
            x if x <= 16 => 10,
            x if x <= 20 => 11,
            x if x <= 24 => 12,
            x if x <= 32 => 13,
            x if x <= 48 => 14,
            _ => 15, // 64
        };
        self.t1.update(|_, w| {
            w.message_marker()
                // TODO(blakely): Un-hard-code this if we need message marking.
                .set(123)
                .store_fifo()
                .set_bit()
                .fdcan_frame()
                .set_bit()
                .bit_rate_switch()
                .set_bit()
                .data_length()
                .set(frame_size_bytes)
        });
    }
}
