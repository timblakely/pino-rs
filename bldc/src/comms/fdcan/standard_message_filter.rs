use super::StandardMessageFilter;

pub type R = super::R<u32, StandardMessageFilter>;
pub type W = super::W<u32, StandardMessageFilter>;

impl stm32g4::ResetValue for StandardMessageFilter {
    type Type = u32;
    #[inline(always)]
    fn reset_value() -> Self::Type {
        0
    }
}

// pub type EFEC_R = stm32g4::R<u8, u8>;
// pub struct EFEC_W<'a> {
//     w: &'a mut W,
// }
// impl<'a> EFEC_W<'a> {
//     #[inline(always)]
//     pub unsafe fn bits(self, value: u8) -> &'a mut W {
//         self.w.0.bits = (&(self.w.0).bits & !(0b111 << 29)) | ((value as u32) & 0b111);
//         self.w
//     }
// }
