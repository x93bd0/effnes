use bitflags::bitflags;

pub enum CpuVector {
    Nmi = 0xFFFA,
    Rst = 0xFFFC,
    Brk = 0xFFFE,
}

bitflags! {
    #[derive(Copy, Clone)]
    pub struct Flags: u8 {
        const Carry    = 0b0000_0001;
        const Zero     = 0b0000_0010;
        const IntDis   = 0b0000_0100;
        const Decimal  = 0b0000_1000;
        const Break    = 0b0001_0000;
        const Reserved = 0b0010_0000;
        const Overflow = 0b0100_0000;
        const Negative = 0b1000_0000;
    }
}

impl Into<u8> for Flags {
    fn into(self) -> u8 {
        self.bits()
    }
}
