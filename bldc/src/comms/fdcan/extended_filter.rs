mod f0 {
    use crate::util::bitfield;
    use crate::{readable_accessor, readwrite_field, writable_accessor};
    pub type ReadProxy = bitfield::ReadProxy<u32, F0>;
    pub type WriteProxy = bitfield::WriteProxy<u32, F0>;

    readwrite_field!(EFEC, u8, 0b111, 29);
    readwrite_field!(ID1, u32, 0x1FFF_FFFF, 0);

    impl ReadProxy {
        readable_accessor!(config, EFEC, u8, 0b111, 29);
        readable_accessor!(id1, ID1, u32, 0x1FFF_FFFF, 0);
    }

    impl WriteProxy {
        writable_accessor!(config, EFEC);
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
    use crate::util::bitfield;
    use crate::{readable_accessor, readwrite_field, writable_accessor};
    pub type ReadProxy = bitfield::ReadProxy<u32, F1>;
    pub type WriteProxy = bitfield::WriteProxy<u32, F1>;

    readwrite_field!(EFT, u8, 0b11, 30);
    readwrite_field!(ID2, u32, 0x1FFF_FFFF, 0);

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

#[repr(C)]
pub struct ExtendedFilter {
    f0: f0::F0,
    f1: f1::F1,
}
