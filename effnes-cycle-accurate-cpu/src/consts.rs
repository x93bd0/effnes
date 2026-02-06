pub enum CpuVector {
    Nmi = 0xFFFA,
    Rst = 0xFFFC,
    Brk = 0xFFFE
}

pub enum Flag {
    Carry = 0b1,
    Zero = 0b10,
    IntDis = 0b100,
    Decimal = 0b1000,
    Break = 0b10000,
    Reserved = 0b100000,
    Overflow = 0b1000000,
    Negative = 0b10000000,
}
