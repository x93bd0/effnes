use std::convert::TryFrom;

/// CPU interrupt vectors.
pub enum CPUVector {
    /// Non-Maskable Interrupt
    Nmi = 0xFFFA,
    /// Reset
    Rst = 0xFFFC,
    /// Break
    Brk = 0xFFFE,
}

/// CPU flags.
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

#[repr(u8)]
#[derive(PartialEq)]
pub enum AddrMode {
    Absolute = 0x4,
    AbsoluteX = 0x7,
    AbsoluteY = 0x8,
    Accumulator = 0x0,
    Immediate = 0x1,
    Implied = 0x2,
    Indirect = 0x6,
    IndirectX = 0xb,
    IndirectY = 0xc,
    Relative = 0x3,
    ZeroPage = 0x5,
    ZeroPageX = 0x9,
    ZeroPageY = 0xa,
}

#[repr(u8)]
pub enum OpCode {
    Adc = 0x1,
    An1 = 0x3a,
    An2 = 0x3b,
    And = 0x2,
    Ane = 0x3c,
    Arr = 0x3d,
    Asl = 0x3,
    Asr = 0x39,
    Bcc = 0x9,
    Bcs = 0xa,
    Beq = 0xc,
    Bit = 0x4,
    Bmi = 0x6,
    Bne = 0xb,
    Bpl = 0x5,
    Brk = 0xd,
    Bvc = 0x7,
    Bvs = 0x8,
    Clc = 0x13,
    Cld = 0x18,
    Cli = 0x15,
    Clv = 0x17,
    Cmp = 0xe,
    Cpx = 0xf,
    Cpy = 0x10,
    Dcp = 0x3e,
    Dec = 0x11,
    Dex = 0x25,
    Dey = 0x29,
    Eor = 0x12,
    Inc = 0x1a,
    Inx = 0x26,
    Iny = 0x2a,
    Isc = 0x3f,
    Jam = 0x4d,
    Jmp = 0x1b,
    Jsr = 0x1c,
    Las = 0x40,
    Lax = 0x41,
    Lda = 0x1d,
    Ldx = 0x1e,
    Ldy = 0x1f,
    Lsr = 0x20,
    Lxa = 0x42,
    Nop = 0x21,
    Ora = 0x22,
    Pha = 0x33,
    Php = 0x35,
    Pla = 0x34,
    Plp = 0x36,
    Rla = 0x43,
    Rol = 0x2b,
    Ror = 0x2c,
    Rra = 0x44,
    Rti = 0x2d,
    Rts = 0x2e,
    Sax = 0x45,
    Sbc = 0x2f,
    Sbx = 0x46,
    Sec = 0x14,
    Sed = 0x19,
    Sei = 0x16,
    Sha = 0x47,
    Shx = 0x48,
    Shy = 0x49,
    Slo = 0x4a,
    Sre = 0x4b,
    Sta = 0x30,
    Stx = 0x37,
    Sty = 0x38,
    Tas = 0x4c,
    Tax = 0x23,
    Tay = 0x27,
    Tsx = 0x32,
    Txa = 0x24,
    Txs = 0x31,
    Tya = 0x28,
}

pub const TRANSLATION_TABLE: [u16; 256] = [
    0b110100101100,
    0b10001010111010,
    0b100110100010000,
    0b100101010111110,
    0b10000101010100,
    0b10001001010100,
    0b1101011000,
    0b100101001011000,
    0b11010100100100,
    0b10001000010010,
    0b1100000010,
    0b11101000010010,
    0b10000101000110,
    0b10001001000110,
    0b1101001010,
    0b100101001001010,
    0b10100110011,
    0b10001011001001,
    0b100110100010000,
    0b100101011001110,
    0b10000110010110,
    0b10001010010110,
    0b1110011010,
    0b100101010011010,
    0b1001100100010,
    0b10001010000111,
    0b10000100100010,
    0b100101010001100,
    0b10000101110111,
    0b10001001110111,
    0b1101111100,
    0b100101001111100,
    0b1110001001010,
    0b1010111010,
    0b100110100010000,
    0b100001110111110,
    0b10001010100,
    0b1001010100,
    0b10101101011000,
    0b100001101011000,
    0b11011000100110,
    0b1000010010,
    0b10101100000010,
    0b11101100010010,
    0b10001000110,
    0b1001000110,
    0b10101101001010,
    0b100001101001010,
    0b11000110011,
    0b1011001001,
    0b100110100010000,
    0b100001111001110,
    0b10000110010110,
    0b1010010110,
    0b10101110011010,
    0b100001110011010,
    0b1010000100010,
    0b1010000111,
    0b10000100100010,
    0b100001110001100,
    0b10000101110111,
    0b1001110111,
    0b10101101111100,
    0b100001101111100,
    0b10110100101010,
    0b1001010111010,
    0b100110100010000,
    0b100101110111110,
    0b10000101010100,
    0b1001001010100,
    0b10000001011000,
    0b100101101011000,
    0b11001100100100,
    0b1001000010010,
    0b10000000000010,
    0b11100100010010,
    0b1101101000100,
    0b1001001000110,
    0b10000001001010,
    0b100101101001010,
    0b11100110011,
    0b1001011001001,
    0b100110100010000,
    0b100101111001110,
    0b10000110010110,
    0b1001010010110,
    0b10000010011010,
    0b100101110011010,
    0b1010100100010,
    0b1001010000111,
    0b10000100100010,
    0b100101110001100,
    0b10000101110111,
    0b1001001110111,
    0b10000001111100,
    0b100101101111100,
    0b10111000101010,
    0b110111010,
    0b100110100010000,
    0b100010010111110,
    0b10000101010100,
    0b101010100,
    0b10110001011000,
    0b100010001011000,
    0b11010000100110,
    0b100010010,
    0b10110000000010,
    0b11110100010010,
    0b1101101101000,
    0b101000110,
    0b10110001001010,
    0b100010001001010,
    0b100000110011,
    0b111001001,
    0b100110100010000,
    0b100010011001110,
    0b10000110010110,
    0b110010110,
    0b10110010011010,
    0b100010010011010,
    0b1011000100010,
    0b110000111,
    0b10000100100010,
    0b100010010001100,
    0b10000101110111,
    0b101110111,
    0b10110001111100,
    0b100010001111100,
    0b10000100010010,
    0b11000010111010,
    0b10000100010010,
    0b100010110111010,
    0b11100001010100,
    0b11000001010100,
    0b11011101010100,
    0b100010101010100,
    0b10100100100010,
    0b10000100010010,
    0b10010000100010,
    0b11110000010010,
    0b11100001000110,
    0b11000001000110,
    0b11011101000110,
    0b100010101000110,
    0b100100110011,
    0b11000011001010,
    0b100110100010000,
    0b100011111001010,
    0b11100010010110,
    0b11000010010110,
    0b11011110100110,
    0b100010110100110,
    0b10100000100010,
    0b11000010001000,
    0b11000100100010,
    0b100110010001000,
    0b100100101111000,
    0b11000001111000,
    0b100100010001000,
    0b100011110001000,
    0b1111100010010,
    0b1110110111010,
    0b1111000010010,
    0b100000110111010,
    0b1111101010100,
    0b1110101010100,
    0b1111001010100,
    0b100000101010100,
    0b10011100100010,
    0b1110100010010,
    0b10001100100010,
    0b100001000010010,
    0b1111101000110,
    0b1110101000110,
    0b1111001000110,
    0b100000101000110,
    0b101000110011,
    0b1110111001001,
    0b100110100010000,
    0b100000111001001,
    0b1111110010110,
    0b1110110010110,
    0b1111010100110,
    0b100000110100110,
    0b1011100100010,
    0b1110110000111,
    0b11001000100010,
    0b100000010000111,
    0b1111101110111,
    0b1110101110111,
    0b1111010000111,
    0b100000110000111,
    0b1000000010010,
    0b111010111010,
    0b10000100010010,
    0b11111010111110,
    0b1000001010100,
    0b111001010100,
    0b1000101011000,
    0b11111001011000,
    0b10101000100010,
    0b111000010010,
    0b10010100100010,
    0b100011000010010,
    0b1000001000110,
    0b111001000110,
    0b1000101001010,
    0b11111001001010,
    0b101100110011,
    0b111011001001,
    0b100110100010000,
    0b11111011001110,
    0b10000110010110,
    0b111010010110,
    0b1000110011010,
    0b11111010011010,
    0b1100000100010,
    0b111010000111,
    0b10000100100010,
    0b11111010001100,
    0b10000101110111,
    0b111001110111,
    0b1000101111100,
    0b11111001111100,
    0b111100010010,
    0b10111110111010,
    0b10000100010010,
    0b11111110111110,
    0b111101010100,
    0b10111101010100,
    0b1101001011000,
    0b11111101011000,
    0b10011000100010,
    0b10111100010010,
    0b10000100100010,
    0b10111100010010,
    0b111101000110,
    0b10111101000110,
    0b1101001001010,
    0b11111101001010,
    0b110000110011,
    0b10111111001001,
    0b100110100010000,
    0b11111111001110,
    0b10000110010110,
    0b10111110010110,
    0b1101010011010,
    0b11111110011010,
    0b1100100100010,
    0b10111110000111,
    0b10000100100010,
    0b11111110001100,
    0b10000101110111,
    0b10111101110111,
    0b1101001111100,
    0b11111101111100,
];

impl TryFrom<u8> for AddrMode {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
       match value {
            4 => Ok(Self::Absolute),
            7 => Ok(Self::AbsoluteX),
            8 => Ok(Self::AbsoluteY),
            0 => Ok(Self::Accumulator),
            1 => Ok(Self::Immediate),
            2 => Ok(Self::Implied),
            6 => Ok(Self::Indirect),
            11 => Ok(Self::IndirectX),
            12 => Ok(Self::IndirectY),
            3 => Ok(Self::Relative),
            5 => Ok(Self::ZeroPage),
            9 => Ok(Self::ZeroPageX),
            10 => Ok(Self::ZeroPageY),
            _ => Err(()),
       }
    }
}


impl TryFrom<u8> for OpCode {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Adc),
            58 => Ok(Self::An1),
            59 => Ok(Self::An2),
            2 => Ok(Self::And),
            60 => Ok(Self::Ane),
            61 => Ok(Self::Arr),
            3 => Ok(Self::Asl),
            57 => Ok(Self::Asr),
            9 => Ok(Self::Bcc),
            10 => Ok(Self::Bcs),
            12 => Ok(Self::Beq),
            4 => Ok(Self::Bit),
            6 => Ok(Self::Bmi),
            11 => Ok(Self::Bne),
            5 => Ok(Self::Bpl),
            13 => Ok(Self::Brk),
            7 => Ok(Self::Bvc),
            8 => Ok(Self::Bvs),
            19 => Ok(Self::Clc),
            24 => Ok(Self::Cld),
            21 => Ok(Self::Cli),
            23 => Ok(Self::Clv),
            14 => Ok(Self::Cmp),
            15 => Ok(Self::Cpx),
            16 => Ok(Self::Cpy),
            62 => Ok(Self::Dcp),
            17 => Ok(Self::Dec),
            37 => Ok(Self::Dex),
            41 => Ok(Self::Dey),
            18 => Ok(Self::Eor),
            26 => Ok(Self::Inc),
            38 => Ok(Self::Inx),
            42 => Ok(Self::Iny),
            63 => Ok(Self::Isc),
            77 => Ok(Self::Jam),
            27 => Ok(Self::Jmp),
            28 => Ok(Self::Jsr),
            64 => Ok(Self::Las),
            65 => Ok(Self::Lax),
            29 => Ok(Self::Lda),
            30 => Ok(Self::Ldx),
            31 => Ok(Self::Ldy),
            32 => Ok(Self::Lsr),
            66 => Ok(Self::Lxa),
            33 => Ok(Self::Nop),
            34 => Ok(Self::Ora),
            51 => Ok(Self::Pha),
            53 => Ok(Self::Php),
            52 => Ok(Self::Pla),
            54 => Ok(Self::Plp),
            67 => Ok(Self::Rla),
            43 => Ok(Self::Rol),
            44 => Ok(Self::Ror),
            68 => Ok(Self::Rra),
            45 => Ok(Self::Rti),
            46 => Ok(Self::Rts),
            69 => Ok(Self::Sax),
            47 => Ok(Self::Sbc),
            70 => Ok(Self::Sbx),
            20 => Ok(Self::Sec),
            25 => Ok(Self::Sed),
            22 => Ok(Self::Sei),
            71 => Ok(Self::Sha),
            72 => Ok(Self::Shx),
            73 => Ok(Self::Shy),
            74 => Ok(Self::Slo),
            75 => Ok(Self::Sre),
            48 => Ok(Self::Sta),
            55 => Ok(Self::Stx),
            56 => Ok(Self::Sty),
            76 => Ok(Self::Tas),
            35 => Ok(Self::Tax),
            39 => Ok(Self::Tay),
            50 => Ok(Self::Tsx),
            36 => Ok(Self::Txa),
            49 => Ok(Self::Txs),
            40 => Ok(Self::Tya),
            _ => Err(()),
        }
    }
}
