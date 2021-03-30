use crate::{readable_accessor, readwrite_field, writable_accessor};

use crate::util::bitfield;

pub type ReadProxy = bitfield::ReadProxy<u32, StandardFilter>;
pub type WriteProxy = bitfield::WriteProxy<u32, StandardFilter>;

pub enum FilterType {
    Range = 0b00,
    Dual = 0b01,
    Classic = 0b10,
    Disabled = 0b11,
}
readwrite_field!(SFT, u8, 0b11, 30, FilterType);

pub enum Action {
    Disable = 0b000,
    StoreRxFIFO0 = 0b001,
    StoreRxFIFO1 = 0b010,
    Reject = 0b011,
    SetPriority = 0b100,
    SetPriorityStoreRxFIFO0 = 0b101,
    SetPriorityStoreRxFIFO1 = 0b110,
}
readwrite_field!(SFEC, u8, 0b111, 27, Action);
readwrite_field!(SFID1, u16, 0x7FF, 16);
readwrite_field!(SFID2, u16, 0x7FF, 0);

impl ReadProxy {
    readable_accessor!(sft, SFT, u8, 0b11, 30);
    readable_accessor!(sfec, SFEC, u8, 0b111, 27);
    readable_accessor!(sfid1, SFID1, u16, 0x7FFF, 16);
    readable_accessor!(sfid2, SFID2, u16, 0x7FFF, 0);
}

impl WriteProxy {
    writable_accessor!(sft, SFT);
    writable_accessor!(sfec, SFEC);
    writable_accessor!(sfid1, SFID1);
    writable_accessor!(sfid2, SFID2);
}

pub type StandardFilter = bitfield::Bitfield<u32, _StandardFilter>;
impl bitfield::Readable for StandardFilter {}
impl bitfield::Writeable for StandardFilter {}
#[allow(missing_docs)]
#[doc(hidden)]
pub struct _StandardFilter;
