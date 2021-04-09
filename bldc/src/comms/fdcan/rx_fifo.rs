mod r0 {
    use crate::util::bitfield;
    use crate::{readable_accessor, readable_field};
    pub type ReadProxy = bitfield::ReadProxy<u32, R0>;

    readable_field!(ESI, u8);
    readable_field!(XTD, u8);
    readable_field!(RTR, u8);
    readable_field!(SID, u32);
    readable_field!(EID, u32);

    impl ReadProxy {
        readable_accessor!(error_state, ESI, u8, 0b1, 31);
        readable_accessor!(extended, XTD, u8, 0b1, 30);
        readable_accessor!(remote_transmission, RTR, u8, 0b1, 29);
        readable_accessor!(standard_id, SID, u32, 0x7FF, 18);
        readable_accessor!(extended_id, EID, u32, 0x1FFF_FFFF, 0);
    }

    pub type R0 = bitfield::Bitfield<u32, _R0>;
    impl bitfield::Readable for R0 {}
    #[allow(missing_docs)]
    #[doc(hidden)]
    pub struct _R0;
}

mod r1 {
    use crate::util::bitfield;
    use crate::{readable_accessor, readable_field};
    pub type ReadProxy = bitfield::ReadProxy<u32, R1>;

    readable_field!(ANMF, u8);
    readable_field!(FIDX, u8);
    readable_field!(FDF, u8);
    readable_field!(BRS, u8);
    readable_field!(DLC, u8);
    readable_field!(RXTS, u16);

    impl ReadProxy {
        readable_accessor!(accepted_non_matching, ANMF, u8, 0b1, 31);
        readable_accessor!(filter_idx, FIDX, u8, 0b1111111, 24);
        readable_accessor!(fd, FDF, u8, 0b1, 21);
        readable_accessor!(bit_rate_switch, BRS, u8, 0b1, 20);
        readable_accessor!(data_length, DLC, u8, 0b111, 16);
        readable_accessor!(timestamp, RXTS, u16, 0xFFFF, 0);
    }

    pub type R1 = bitfield::Bitfield<u32, _R1>;
    impl bitfield::Readable for R1 {}
    #[allow(missing_docs)]
    #[doc(hidden)]
    pub struct _R1;
}

#[repr(C)]
pub struct RxFifo {
    r0: r0::R0,
    r1: r1::R1,
    data: [u32; 16],
}

impl RxFifo {
    pub fn data(&self) -> &[u32; 16] {
        &self.data
    }
    pub fn id(&self) -> u32 {
        let r0 = self.r0.read();
        match r0.extended().bits() {
            0 => r0.standard_id().bits() as u32,
            _ => r0.extended_id().bits(),
        }
    }
}
