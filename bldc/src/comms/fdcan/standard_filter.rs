use crate::util::bitfield;
use crate::util::bitfield::{Bitfield, Readable, Writeable};
pub type ReadProxy = bitfield::ReadProxy<u32, StandardFilter>;
pub type WriteProxy = bitfield::WriteProxy<u32, StandardFilter>;

pub type SFT_R = bitfield::ReadProxy<u8, u8>;
pub struct SFT_W<'a> {
    w: &'a mut WriteProxy,
}
pub enum FilterType {
    Range = 0b00,
    Dual = 0b01,
    Classic = 0b10,
    Disabled = 0b11,
}
impl From<FilterType> for u8 {
    #[inline(always)]
    fn from(variant: FilterType) -> Self {
        variant as _
    }
}
impl<'a> SFT_W<'a> {
    #[inline(always)]
    pub unsafe fn bits(self, value: u8) -> &'a mut WriteProxy {
        self.w.bits = (self.w.bits & !(0b11 << 30)) | (((value as u32) & 0b11) << 30);
        self.w
    }
    #[inline(always)]
    pub fn variant(self, variant: FilterType) -> &'a mut WriteProxy {
        unsafe { self.bits(variant.into()) }
    }
}

pub type SFEC_R = bitfield::ReadProxy<u8, u8>;
pub struct SFEC_W<'a> {
    w: &'a mut WriteProxy,
}
pub enum Action {
    Disable = 0b000,
    StoreRxFIFO0 = 0b001,
    StoreRxFIFO1 = 0b010,
    Reject = 0b011,
    SetPriority = 0b100,
    SetPriorityStoreRxFIFO0 = 0b101,
    SetPriorityStoreRxFIFO1 = 0b110,
}
impl From<Action> for u8 {
    #[inline(always)]
    fn from(variant: Action) -> Self {
        variant as _
    }
}
impl<'a> SFEC_W<'a> {
    #[inline(always)]
    pub unsafe fn bits(self, value: u8) -> &'a mut WriteProxy {
        self.w.bits = (self.w.bits & !(0b111 << 27)) | (((value as u32) & 0b111) << 27);
        self.w
    }
    #[inline(always)]
    pub fn variant(self, variant: Action) -> &'a mut WriteProxy {
        unsafe { self.bits(variant.into()) }
    }
}

pub type SFID1_R = bitfield::ReadProxy<u16, u16>;
pub struct SFID1_W<'a> {
    w: &'a mut WriteProxy,
}
impl<'a> SFID1_W<'a> {
    #[inline(always)]
    pub unsafe fn bits(self, value: u8) -> &'a mut WriteProxy {
        self.w.bits = (self.w.bits & !(0x7FF << 16)) | (((value as u32) & 0x7FF) << 16);
        self.w
    }
}

pub type SFID2_R = bitfield::ReadProxy<u16, u16>;
pub struct SFID2_W<'a> {
    w: &'a mut WriteProxy,
}
impl<'a> SFID2_W<'a> {
    #[inline(always)]
    pub unsafe fn bits(self, value: u8) -> &'a mut WriteProxy {
        self.w.bits = (self.w.bits & !(0x7FF << 0)) | (((value as u32) & 0x7FF) << 0);
        self.w
    }
}

#[allow(missing_docs)]
#[doc(hidden)]
pub struct _StandardFilter;

impl ReadProxy {
    #[inline(always)]
    pub fn sft(&self) -> SFT_R {
        SFT_R::new(((self.bits >> 30) & 0b11) as u8)
    }
    #[inline(always)]
    pub fn sfec(&self) -> SFEC_R {
        SFEC_R::new(((self.bits >> 27) & 0b111) as u8)
    }
    #[inline(always)]
    pub fn sfid1(&self) -> SFID1_R {
        SFID1_R::new(((self.bits >> 16) & 0x7FF) as u16)
    }
    #[inline(always)]
    pub fn sfid2(&self) -> SFID2_R {
        SFID2_R::new(((self.bits >> 0) & 0x7FF) as u16)
    }
}

impl WriteProxy {
    #[inline(always)]
    pub fn sft(&mut self) -> SFT_W {
        SFT_W { w: self }
    }
    #[inline(always)]
    pub fn sfec(&mut self) -> SFEC_W {
        SFEC_W { w: self }
    }
    #[inline(always)]
    pub fn sfid1(&mut self) -> SFID1_W {
        SFID1_W { w: self }
    }
    #[inline(always)]
    pub fn sfid2(&mut self) -> SFID2_W {
        SFID2_W { w: self }
    }
}

pub type StandardFilter = Bitfield<u32, _StandardFilter>;
impl Readable for StandardFilter {}
impl Writeable for StandardFilter {}
