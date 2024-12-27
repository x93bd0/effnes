use crate::consts;

pub trait Memory<T> {
    fn new() -> T;
    fn read_byte(&self, addr: u16) -> u8;
    fn read_addr(&self, addr: u16) -> u16;
    fn write_byte(&mut self, addr: u16, data: u8);
    fn write_addr(&mut self, addr: u16, data: u16);
}

pub struct VM<T: Memory<T>> {
    pub pc: u16,
    pub x: u8,
    pub y: u8,
    pub a: u8,
    pub s: u8,
    pub p: u8,
    pub h: u8,

    pub ex_interrupt: u8,
    pub cycles: usize,
    pub io: T,
}

impl<T: Memory<T>> VM<T> {
    pub fn new() -> VM<T> {
        VM {
            pc: 0x8000,
            x: 0,
            y: 0,
            a: 0,
            s: 0,
            p: 0,
            h: 0,
            ex_interrupt: 0,
            cycles: 0,
            io: T::new(),
        }
    }

    fn enable_flag(self: &mut VM<T>, flag: consts::Flag) {
        self.p |= flag as u8;
    }

    fn disable_flag(self: &mut VM<T>, flag: consts::Flag) {
        self.p &= !(flag as u8);
    }

    fn set_flag(self: &mut VM<T>, flag: consts::Flag, value: bool) {
        if value {
            self.enable_flag(flag);
        } else {
            self.disable_flag(flag);
        }
    }

    fn get_flag(self: &mut VM<T>, flag: consts::Flag) -> u8 {
        self.p & (flag as u8)
    }

    fn set_nz_flags(self: &mut VM<T>, value: u8) {
        self.set_flag(consts::Flag::Negative, value & 0x80 > 0);
        self.set_flag(consts::Flag::Zero, value == 0);
    }

    fn next_byte(self: &mut VM<T>) -> u8 {
        let out: u8 = self.io.read_byte(self.pc);
        self.pc = self.pc.wrapping_add(1);
        out
    }

    fn next_addr(self: &mut VM<T>) -> u16 {
        let out: u16 = self.io.read_addr(self.pc);
        self.pc = self.pc.wrapping_add(2);
        out
    }

    fn stack_push_byte(self: &mut VM<T>, value: u8) {
        self.io.write_byte((self.s as u16) | 0x100, value);
        self.s = self.s.wrapping_sub(1);
    }

    fn stack_push_addr(self: &mut VM<T>, value: u16) {
        self.stack_push_byte((value >> 8) as u8);
        self.stack_push_byte(value as u8);
    }

    fn stack_pop_byte(self: &mut VM<T>) -> u8 {
        self.s = self.s.wrapping_add(1);
        self.io.read_byte((self.s as u16) | 0x100)
    }

    fn stack_pop_addr(self: &mut VM<T>) -> u16 {
        (self.stack_pop_byte() as u16) + ((self.stack_pop_byte() as u16) << 8)
    }

    pub fn reset(self: &mut VM<T>) {
        self.ex_interrupt = 0;
        self.pc = self.io.read_addr(consts::CPUVector::Brk as u16);

        self.io.write_byte(0x4015, 0);
        if self.cycles == 0 {
            self.p = 0x36;
            self.s = 0xFF;
            self.a = 0;
            self.x = 0;
            self.y = 0;

            self.io.write_byte(0x4017, 0);
            // TODO: memset 0 0x4000 ,,, 0x4000 + 19
            // TODO: Reset Noise Channel & APU FC
        }

        self.cycles = 0;
        self.enable_flag(consts::Flag::IntDis);
    }

    pub fn run(self: &mut VM<T>, cycles: usize) {
        self.cycles = 0;
        while self.cycles < cycles && self.ex_interrupt == 0 {
            let opcode: u8 = self.next_byte();

            let internal_repr: u16 = consts::TRANSLATION_TABLE[opcode as usize];
            let internal_opcode: u8 = (internal_repr >> 9) as u8;
            let address_mode: u8 = ((internal_repr >> 5) & 0b1111) as u8;
            let mut timing: u8 = ((internal_repr >> 2) & 0b111) as u8;

            print!(" {} ", timing);

            let mut t_byte1: u8 = 0;
            let t_byte2: u8;
            let mut t_addr: u16;

            match address_mode {
                consts::addr_mode::IMMEDIATE => {
                    t_addr = self.pc;
                    self.pc += 1;
                }

                consts::addr_mode::RELATIVE => {
                    t_byte1 = self.next_byte();
                    t_addr = self.pc.wrapping_add_signed((t_byte1 as i8) as i16);
                    t_byte1 = if (t_addr & 0xff00) != (self.pc & 0xff00) {1} else {0};
                }

                consts::addr_mode::ABSOLUTE => {
                    t_addr = self.next_addr();
                }

                consts::addr_mode::INDIRECT => {
                    t_addr = self.next_addr();
                    if t_addr & 0xFF == 0xFF {
                        t_addr = (self.io.read_byte(t_addr) as u16)
                            | ((self.io.read_byte(t_addr & 0xff00) as u16) << 8);
                    } else {
                        t_addr = self.io.read_addr(t_addr);
                    }
                }

                consts::addr_mode::ZEROPAGE => {
                    t_addr = self.next_byte() as u16;
                }

                consts::addr_mode::ABSOLUTEX => {
                    t_addr = self.next_addr();
                    if ((t_addr.wrapping_add(self.x as u16)) & 0xff00) != (t_addr & 0xff00) {
                        timing += 1;
                    }

                    t_addr += self.x as u16;
                }

                consts::addr_mode::ABSOLUTEY => {
                    t_addr = self.next_addr();
                    if ((t_addr.wrapping_add(self.y as u16)) & 0xff00) != (t_addr & 0xff00) {
                        timing += 1;
                    }

                    t_addr = t_addr.wrapping_add(self.y as u16);
                }

                consts::addr_mode::ZEROPAGEX => {
                    t_byte1 = self.next_byte();
                    t_addr = t_byte1.wrapping_add_signed(self.x as i8) as u16;
                }

                consts::addr_mode::ZEROPAGEY => {
                    t_byte1 = self.next_byte();
                    t_addr = t_byte1.wrapping_add_signed(self.y as i8) as u16;
                }

                consts::addr_mode::INDIRECTX => {
                    t_byte1 = self.next_byte();
                    t_addr = ((self
                        .io
                        .read_byte(t_byte1.wrapping_add_signed((self.x as i8) + 1) as u16)
                        as u16)
                        << 8)
                        + (self
                            .io
                            .read_byte(t_byte1.wrapping_add_signed(self.x as i8) as u16)
                            as u16);
                }

                consts::addr_mode::INDIRECTY => {
                    t_byte1 = self.next_byte();
                    t_addr = self.io.read_byte(t_byte1 as u16) as u16;
                    t_addr += (self.io.read_byte(t_byte1.wrapping_add(1) as u16) as u16) << 8;
                    if ((t_addr.wrapping_add(self.y as u16)) & 0xff00) != (t_addr & 0xff00) {
                        timing += 1;
                    }

                    t_addr = t_addr.wrapping_add(self.y as u16);
                }

                _ => {
                    t_addr = 0;
                }
            };

            // print!(" (taddr -> {:2x}) ", self.io.read_byte(t_addr));
            print!(" {} ", timing);

            match internal_opcode {
                consts::opcode::LDA => {
                    self.a = self.io.read_byte(t_addr);
                    self.set_nz_flags(self.a);
                }

                consts::opcode::LDX => {
                    self.x = self.io.read_byte(t_addr);
                    self.set_nz_flags(self.x);
                }

                consts::opcode::LDY => {
                    self.y = self.io.read_byte(t_addr);
                    self.set_nz_flags(self.y);
                }

                consts::opcode::LAX => {
                    // Composite
                    self.a = self.io.read_byte(t_addr);
                    self.x = self.a;
                    self.set_nz_flags(self.a);
                }

                consts::opcode::SAX => {
                    self.io.write_byte(t_addr, self.a & self.x);
                }

                consts::opcode::STA => self.io.write_byte(t_addr, self.a),

                consts::opcode::STX => self.io.write_byte(t_addr, self.x),

                consts::opcode::STY => self.io.write_byte(t_addr, self.y),

                consts::opcode::TAX => {
                    self.x = self.a;
                    self.set_nz_flags(self.a);
                }

                consts::opcode::TAY => {
                    self.y = self.a;
                    self.set_nz_flags(self.a);
                }

                consts::opcode::TSX => {
                    self.x = self.s;
                    self.set_nz_flags(self.s);
                }

                consts::opcode::TXA => {
                    self.a = self.x;
                    self.set_nz_flags(self.x);
                }

                consts::opcode::TXS => {
                    self.s = self.x;
                }

                consts::opcode::TYA => {
                    self.a = self.y;
                    self.set_nz_flags(self.y);
                }

                consts::opcode::PHA => {
                    self.stack_push_byte(self.a);
                }

                consts::opcode::PHP => {
                    self.stack_push_byte(self.p | (consts::Flag::Break as u8));
                }

                consts::opcode::PLA => {
                    self.a = self.stack_pop_byte();
                    self.set_nz_flags(self.a);
                }

                consts::opcode::PLP => {
                    self.p = (self.stack_pop_byte() & !(consts::Flag::Break as u8))
                        | (consts::Flag::Reserved as u8);
                }

                consts::opcode::DEC => {
                    t_byte1 = self.io.read_byte(t_addr).wrapping_sub(1);
                    self.io.write_byte(t_addr, t_byte1);
                    self.set_nz_flags(t_byte1);
                }

                consts::opcode::DEX => {
                    self.x = self.x.wrapping_sub(1);
                    self.set_nz_flags(self.x);
                }

                consts::opcode::DEY => {
                    self.y = self.y.wrapping_sub(1);
                    self.set_nz_flags(self.y);
                }

                consts::opcode::INC => {
                    t_byte1 = self.io.read_byte(t_addr).wrapping_add(1);
                    self.io.write_byte(t_addr, t_byte1);
                    self.set_nz_flags(t_byte1);
                }

                consts::opcode::INX => {
                    self.x = self.x.wrapping_add(1);
                    self.set_nz_flags(self.x);
                }

                consts::opcode::INY => {
                    self.y = self.y.wrapping_add(1);
                    self.set_nz_flags(self.y);
                }

                consts::opcode::ISC => {
                    // Composite
                    t_byte1 = self.io.read_byte(t_addr).wrapping_add(1);
                    self.io.write_byte(t_addr, t_byte1);
                    t_byte1 = !t_byte1;
                    t_addr = (self.a as u16)
                        + (t_byte1 as u16)
                        + (self.get_flag(consts::Flag::Carry) as u16);
                    self.set_flag(consts::Flag::Carry, t_addr > 0xFF);
                    self.set_flag(
                        consts::Flag::Overflow,
                        (!(self.a ^ t_byte1) & (self.a ^ (t_addr as u8))) & 0x80 != 0,
                    );
                    self.a = t_addr as u8;
                    self.set_nz_flags(self.a);
                }

                consts::opcode::SBC => {
                    // Pseudo-Composite
                    t_byte1 = !self.io.read_byte(t_addr);
                    t_addr = (self.a as u16)
                        + (t_byte1 as u16)
                        + (self.get_flag(consts::Flag::Carry) as u16);
                    self.set_flag(consts::Flag::Carry, t_addr > 0xFF);
                    self.set_flag(
                        consts::Flag::Overflow,
                        (!(self.a ^ t_byte1) & (self.a ^ (t_addr as u8))) & 0x80 != 0,
                    );
                    self.a = t_addr as u8;
                    self.set_nz_flags(self.a);
                }

                consts::opcode::ADC => {
                    t_byte1 = self.io.read_byte(t_addr);
                    t_addr = (self.a as u16)
                        + (t_byte1 as u16)
                        + (self.get_flag(consts::Flag::Carry) as u16);
                    self.set_flag(consts::Flag::Carry, t_addr > 0xFF);
                    self.set_flag(
                        consts::Flag::Overflow,
                        (!(self.a ^ t_byte1) & (self.a ^ (t_addr as u8))) & 0x80 != 0,
                    );
                    self.a = t_addr as u8;
                    self.set_nz_flags(self.a);
                }

                consts::opcode::RRA => {
                    // Composite
                    t_byte1 = self.io.read_byte(t_addr);
                    t_byte2 = self.get_flag(consts::Flag::Carry);
                    self.set_flag(consts::Flag::Carry, t_byte1 & 0x1 != 0);
                    t_byte1 >>= 1;
                    t_byte1 = t_byte1.wrapping_add(if t_byte2 != 0 { 0x80 } else { 0 });
                    self.io.write_byte(t_addr, t_byte1);

                    // ADC Code
                    t_addr = (self.a as u16)
                        + (t_byte1 as u16)
                        + (self.get_flag(consts::Flag::Carry) as u16);
                    self.set_flag(consts::Flag::Carry, t_addr > 0xFF);
                    self.set_flag(
                        consts::Flag::Overflow,
                        (!(self.a ^ t_byte1) & (self.a ^ (t_addr as u8))) & 0x80 != 0,
                    );
                    self.a = t_addr as u8;
                    self.set_nz_flags(self.a);
                }

                consts::opcode::RLA => {
                    // Composite
                    t_byte1 = self.io.read_byte(t_addr);
                    t_byte2 = self.get_flag(consts::Flag::Carry);
                    self.set_flag(consts::Flag::Carry, t_byte1 & 0x80 != 0);
                    t_byte1 <<= 1;
                    t_byte1 = t_byte1.wrapping_add(t_byte2);
                    self.io.write_byte(t_addr, t_byte1);

                    // AND code
                    self.a &= t_byte1;
                    self.set_nz_flags(self.a);
                }

                consts::opcode::AND => {
                    self.a &= self.io.read_byte(t_addr);
                    self.set_nz_flags(self.a);
                }

                consts::opcode::SRE => {
                    // Composite
                    t_byte1 = self.io.read_byte(t_addr);
                    self.set_flag(consts::Flag::Carry, t_byte1 & 0x1 != 0);
                    t_byte1 >>= 1;
                    self.io.write_byte(t_addr, t_byte1);

                    // EOR code
                    self.a ^= t_byte1;
                    self.set_nz_flags(self.a);
                }

                consts::opcode::EOR => {
                    self.a ^= self.io.read_byte(t_addr);
                    self.set_nz_flags(self.a);
                }

                consts::opcode::SLO => {
                    t_byte1 = self.io.read_byte(t_addr);
                    self.set_flag(consts::Flag::Carry, t_byte1 & 0x80 != 0);
                    t_byte1 <<= 1;
                    self.io.write_byte(t_addr, t_byte1);

                    // ORA code
                    self.a |= self.io.read_byte(t_addr);
                    self.set_nz_flags(self.a);
                }

                consts::opcode::ORA => {
                    self.a |= self.io.read_byte(t_addr);
                    self.set_nz_flags(self.a);
                }

                consts::opcode::ASL => {
                    t_byte1 = if address_mode == consts::addr_mode::ACCUMULATOR {
                        self.a
                    } else {
                        self.io.read_byte(t_addr)
                    };

                    self.set_flag(consts::Flag::Carry, t_byte1 & 0x80 != 0);
                    t_byte1 <<= 1;

                    if address_mode == consts::addr_mode::ACCUMULATOR {
                        self.a = t_byte1;
                    } else {
                        self.io.write_byte(t_addr, t_byte1);
                    }

                    self.set_nz_flags(t_byte1);
                }

                consts::opcode::LSR => {
                    t_byte1 = if address_mode == consts::addr_mode::ACCUMULATOR {
                        self.a
                    } else {
                        self.io.read_byte(t_addr)
                    };

                    self.set_flag(consts::Flag::Carry, t_byte1 & 0x1 != 0);
                    t_byte1 >>= 1;

                    if address_mode == consts::addr_mode::ACCUMULATOR {
                        self.a = t_byte1;
                    } else {
                        self.io.write_byte(t_addr, t_byte1);
                    }

                    self.set_nz_flags(t_byte1);
                }

                consts::opcode::ROL => {
                    t_byte1 = if address_mode == consts::addr_mode::ACCUMULATOR {
                        self.a
                    } else {
                        self.io.read_byte(t_addr)
                    };

                    t_byte2 = self.get_flag(consts::Flag::Carry);
                    self.set_flag(consts::Flag::Carry, t_byte1 & 0x80 != 0);
                    t_byte1 <<= 1;
                    t_byte1 = t_byte1.wrapping_add(t_byte2);

                    if address_mode == consts::addr_mode::ACCUMULATOR {
                        self.a = t_byte1;
                    } else {
                        self.io.write_byte(t_addr, t_byte1);
                    }

                    self.set_nz_flags(t_byte1);
                }

                consts::opcode::ROR => {
                    t_byte1 = if address_mode == consts::addr_mode::ACCUMULATOR {
                        self.a
                    } else {
                        self.io.read_byte(t_addr)
                    };

                    t_byte2 = self.get_flag(consts::Flag::Carry);
                    self.set_flag(consts::Flag::Carry, t_byte1 & 0x1 != 0);
                    t_byte1 >>= 1;
                    t_byte1 = t_byte1.wrapping_add(if t_byte2 == 1 { 0x80 } else { 0 });

                    if address_mode == consts::addr_mode::ACCUMULATOR {
                        self.a = t_byte1;
                    } else {
                        self.io.write_byte(t_addr, t_byte1);
                    }

                    self.set_nz_flags(t_byte1);
                }

                consts::opcode::CLC => {
                    self.disable_flag(consts::Flag::Carry);
                }

                consts::opcode::CLD => {
                    self.disable_flag(consts::Flag::Decimal);                    
                }

                consts::opcode::CLI => {
                    self.disable_flag(consts::Flag::IntDis);
                }

                consts::opcode::CLV => {
                    self.disable_flag(consts::Flag::Overflow);
                }

                consts::opcode::SEC => {
                    self.enable_flag(consts::Flag::Carry);
                }

                consts::opcode::SED => {
                    self.enable_flag(consts::Flag::Decimal);
                }

                consts::opcode::SEI => {
                    self.enable_flag(consts::Flag::IntDis);
                }

                consts::opcode::DCP => {    // Composite
                    t_byte1 = self.io.read_byte(t_addr).wrapping_sub(1);
                    self.io.write_byte(t_addr, t_byte1);

                    // CMP code
                    self.set_flag(consts::Flag::Carry, self.a >= t_byte1);
                    self.set_nz_flags(self.a.wrapping_sub(t_byte1));
                }

                consts::opcode::CMP => {
                    t_byte1 = self.io.read_byte(t_addr);
                    self.set_flag(consts::Flag::Carry, self.a >= t_byte1);
                    self.set_nz_flags(self.a.wrapping_sub(t_byte1));
                }

                consts::opcode::CPX => {
                    t_byte1 = self.io.read_byte(t_addr);
                    self.set_flag(consts::Flag::Carry, self.x >= t_byte1);
                    self.set_nz_flags(self.x.wrapping_sub(t_byte1));
                }

                consts::opcode::CPY => {
                    t_byte1 = self.io.read_byte(t_addr);
                    self.set_flag(consts::Flag::Carry, self.y >= t_byte1);
                    self.set_nz_flags(self.y.wrapping_sub(t_byte1));
                }

                consts::opcode::BCC => {
                    if self.get_flag(consts::Flag::Carry) == 0 {
                        timing += 1 + t_byte1;
                        self.pc = t_addr;
                    }
                }

                consts::opcode::BCS => {
                    if self.get_flag(consts::Flag::Carry) != 0 {
                        timing += 1 + t_byte1;
                        self.pc = t_addr;
                    }
                }

                consts::opcode::BEQ => {
                    if self.get_flag(consts::Flag::Zero) != 0 {
                        timing += 1 + t_byte1;
                        self.pc = t_addr;
                    }
                }

                consts::opcode::BMI => {
                    if self.get_flag(consts::Flag::Negative) != 0 {
                        timing += 1 + t_byte1;
                        self.pc = t_addr;
                    }
                }

                consts::opcode::BNE => {
                    if self.get_flag(consts::Flag::Zero) == 0 {
                        timing += 1 + t_byte1;
                        self.pc = t_addr;
                    }
                }

                consts::opcode::BPL => {
                    if self.get_flag(consts::Flag::Negative) == 0 {
                        timing += 1 + t_byte1;
                        self.pc = t_addr;
                    }
                }

                consts::opcode::BVC => {
                    if self.get_flag(consts::Flag::Overflow) == 0 {
                        timing += 1 + t_byte1;
                        self.pc = t_addr;
                    }
                }

                consts::opcode::BVS => {
                    if self.get_flag(consts::Flag::Overflow) != 0 {
                        timing += 1 + t_byte1;
                        self.pc = t_addr;
                    }
                }

                consts::opcode::JMP => {
                    self.pc = t_addr;
                }

                consts::opcode::JSR => {
                    self.pc = self.pc.wrapping_sub(1);
                    self.stack_push_addr(self.pc);
                    self.pc = t_addr;
                }

                consts::opcode::RTS => {
                    self.pc = self.stack_pop_addr().wrapping_add(1);
                }

                consts::opcode::BRK => {
                    self.stack_push_addr(self.pc);
                    self.stack_push_byte(self.p | (consts::Flag::Break as u8));
                    self.enable_flag(consts::Flag::IntDis);
                    self.pc = self.io.read_addr(consts::CPUVector::Brk as u16);
                }

                consts::opcode::RTI => {
                    self.p = (self.stack_pop_byte() & !(consts::Flag::Break as u8)) | (consts::Flag::Reserved as u8);
                    self.pc = self.stack_pop_addr();
                }

                consts::opcode::BIT => {
                    t_byte1 = self.io.read_byte(t_addr);
                    t_byte2 = self.a & t_byte1;
                    self.set_nz_flags(t_byte2);
                    self.set_flag(consts::Flag::Overflow, t_byte2 & 0x40 != 0);
                    self.p = (self.p & 0b0011_1111) | (t_byte1 & 0b1100_0000);
                }

                consts::opcode::NOP => {}

                _ => {
                    self.h = 1;
                }
            }

            self.cycles += 1 + (timing as usize);
        }
    }
}
