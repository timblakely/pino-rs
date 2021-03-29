use crate::util::bitfield;
use crate::util::bitfield::{Bitfield, Readable, Writeable};
pub type ReadProxy = bitfield::ReadProxy<u32, StandardFilter>;
pub type WriteProxy = bitfield::WriteProxy<u32, StandardFilter>;

pub type StandardFilter = Bitfield<u32, _StandardFilter>;
impl Readable for StandardFilter {}
impl Writeable for StandardFilter {}

pub type SFT_R = bitfield::ReadProxy<u8, u8>;
pub struct SFT_W<'a> {
    w: &'a mut WriteProxy,
}
pub enum StandardFilterType {
    Range = 0b00,
    Dual = 0b01,
    Classic = 0b10,
    Disabled = 0b11,
}
impl From<StandardFilterType> for u8 {
    #[inline(always)]
    fn from(variant: StandardFilterType) -> Self {
        variant as _
    }
}
impl<'a> SFT_W<'a> {
    #[inline(always)]
    pub unsafe fn bits(self, value: u8) -> &'a mut WriteProxy {
        self.w.bits = (self.w.bits & !(0b11 << 30)) | (((value as u32) & 0b11) << 20);
        self.w
    }
    #[inline(always)]
    pub fn variant(self, variant: StandardFilterType) -> &'a mut WriteProxy {
        unsafe { self.bits(variant.into()) }
    }
    #[inline(always)]
    pub fn classic(self) -> &'a mut WriteProxy {
        self.variant(StandardFilterType::Classic)
    }
}

pub type SFEC_R = bitfield::ReadProxy<u8, u8>;
pub type SFID1_R = bitfield::ReadProxy<u16, u16>;
pub type SFID2_R = bitfield::ReadProxy<u16, u16>;

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
        SFID1_R::new(((self.bits >> 16) & 0b11111111111) as u16)
    }
    #[inline(always)]
    pub fn sfid2(&self) -> SFID2_R {
        SFID2_R::new(((self.bits >> 0) & 0b11111111111) as u16)
    }
}

impl WriteProxy {
    #[inline(always)]
    pub fn sft(&mut self) -> SFT_W {
        SFT_W { w: self }
    }

    #[inline(always)]
    pub fn classic(&mut self) -> &mut WriteProxy {
        SFT_W { w: self }.variant(StandardFilterType::Classic)
    }
}
