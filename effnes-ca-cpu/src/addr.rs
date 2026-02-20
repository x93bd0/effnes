use crate::opcode::OpCode;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum IndexRegister {
    X,
    Y,
}

// Implied == Accumulator
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum AddressingMode {
    Implied,
    Immediate,
    ZeroPage,
    ZeroPageI(IndexRegister),
    Absolute,
    AbsoluteI(IndexRegister),
    Indirect,
    IndirectI(IndexRegister),
    Relative,
}

impl From<OpCode> for AddressingMode {
    fn from(opcode: OpCode) -> Self {
        // TODO: Move to corresponding columns (OPT)
        match opcode {
            0x20 => {
                return Self::Absolute;
            }
            0x6C => {
                return Self::Indirect;
            }
            0x96..=0x97 | 0xB6..=0xB7 => {
                return Self::ZeroPageI(IndexRegister::Y);
            }
            0x9E..=0x9F | 0xBE..=0xBF => {
                return Self::AbsoluteI(IndexRegister::Y);
            }
            _ => {}
        };

        let high = opcode >> 4;
        let low = opcode & 0b1111;

        if high & 0b0001 != 0 && low == 0 {
            return Self::Relative;
        }

        match low {
            0x0 | 0x2 => {
                if high <= 0x7 {
                    Self::Implied
                } else {
                    Self::Immediate
                }
            }
            0x1 | 0x3 => Self::IndirectI(if high & 1 != 0 {
                IndexRegister::Y
            } else {
                IndexRegister::X
            }),
            0x4..=0x7 => {
                if high & 1 != 0 {
                    Self::ZeroPageI(IndexRegister::X)
                } else {
                    Self::ZeroPage
                }
            }
            0x8 | 0xA => Self::Implied,
            0x9 | 0xB => {
                if high & 1 != 0 {
                    Self::AbsoluteI(IndexRegister::Y)
                } else {
                    Self::Immediate
                }
            }
            0xC..=0xF => {
                if high & 1 != 0 {
                    Self::AbsoluteI(IndexRegister::X)
                } else {
                    Self::Absolute
                }
            }
            _ => unreachable!(),
        }
    }
}
