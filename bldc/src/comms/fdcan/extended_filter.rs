mod f0 {
    use super::ExtendedFilterMode;
    use crate::util::bitfield;
    use crate::{readable_accessor, readwrite_field, writable_accessor};
    pub type ReadProxy = bitfield::ReadProxy<u32, F0>;
    pub type WriteProxy = bitfield::WriteProxy<u32, F0>;

    readwrite_field!(EFEC, u8, 0b111, 29, ExtendedFilterMode);
    readwrite_field!(bitsafe ID1, u32, 0x1FFF_FFFF, 0);

    impl ReadProxy {
        readable_accessor!(mode, EFEC, u8, 0b111, 29);
        readable_accessor!(id1, ID1, u32, 0x1FFF_FFFF, 0);
    }

    impl WriteProxy {
        writable_accessor!(mode, EFEC);
        writable_accessor!(id1, ID1);
    }

    pub type F0 = bitfield::Bitfield<u32, _F0>;
    impl bitfield::Readable for F0 {}
    impl bitfield::Writeable for F0 {}
    #[allow(missing_docs)]
    #[doc(hidden)]
    pub struct _F0;
}

mod f1 {
    use super::ExtendedFilterType;
    use crate::util::bitfield;
    use crate::{readable_accessor, readwrite_field, writable_accessor};
    pub type ReadProxy = bitfield::ReadProxy<u32, F1>;
    pub type WriteProxy = bitfield::WriteProxy<u32, F1>;

    readwrite_field!(EFT, u8, 0b11, 30, ExtendedFilterType);
    readwrite_field!(bitsafe ID2, u32, 0x1FFF_FFFF, 0);

    impl ReadProxy {
        readable_accessor!(filter_type, EFT, u8, 0b11, 30);
        readable_accessor!(id2, ID2, u32, 0x1FFF_FFFF, 0);
    }

    impl WriteProxy {
        writable_accessor!(filter_type, EFT);
        writable_accessor!(id2, ID2);
    }

    pub type F1 = bitfield::Bitfield<u32, _F1>;
    impl bitfield::Readable for F1 {}
    impl bitfield::Writeable for F1 {}
    #[allow(missing_docs)]
    #[doc(hidden)]
    pub struct _F1;
}

pub enum ExtendedFilterMode {
    Disable = 0b000,
    StoreRxFIFO0 = 0b001,
    StoreRxFIFO1 = 0b010,
    Reject = 0b011,
    SetPriority = 0b100,
    SetPriorityStoreRxFIFO0 = 0b101,
    SetPriorityStoreRxFIFO1 = 0b110,
}

pub enum ExtendedFilterType {
    Range = 0b00,
    Dual = 0b01,
    Classic = 0b10,
    RangeNoXIDAM = 0b11,
}

#[repr(C)]
pub struct ExtendedFilter {
    pub f0: f0::F0,
    pub f1: f1::F1,
}
