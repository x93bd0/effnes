use crate::consts;


/// Memory Bus
pub trait Memory {
    fn new() -> Self;
    fn read_byte(&self, addr: u16) -> u8;
    fn read_addr(&self, addr: u16) -> u16;
    fn write_byte(&mut self, addr: u16, data: u8);
    fn write_addr(&mut self, addr: u16, data: u16);
}


/// 6502 Virtual Machine
pub struct VM<T: Memory> {
    /// The program counter.
    pub pc: u16,

    /// The X register.
    pub x: u8,
    /// The Y register.
    pub y: u8,
    /// The Accumulator register.
    pub a: u8,
    /// The stack pointer register.
    pub s: u8,
    /// The program status register.
    pub p: u8,

    /// An internal flag used for validating an execution (checking that the code didn't halt).
    pub h: u8,
    /// An internal flag used for halting the CPU while it's running (by the memory bus).
    pub ex_interrupt: u8,
    /// The CPU cycle count.
    pub cycles: usize,
    /// An Input/Output capable Memory Bus.
    pub io: T,
}

impl<T: Memory> Default for VM<T> {
    fn default() -> VM<T> {
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
}

impl<T: Memory> VM<T> {
    /// Instanciates a new Virtual Machine.
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

    /// Resets the CPU by putting it into a known state.
    ///
    /// ## Behaviour
    ///
    /// There are two reset variants:
    /// 1. Cold-reset
    /// 2. Warm-reset
    ///
    /// In a cold-reset, the CPU its supposed to set every register with a
    /// known value. This behaviour is the one implemented in this method.
    ///
    /// However, in a warm reset, the CPU only sets the `consts::Flag::IntDis`,
    /// the program counter, and the stack pointer to a known value. This was
    /// also taken into account while implementing it.
    ///
    /// The internal registers are also reseted by this method, independently
    /// of which type of reset it is.
    ///
    pub fn reset(self: &mut VM<T>) -> bool {
        self.h = 0;
        self.cycles = 0;
        self.ex_interrupt = 0;

        self.pc = self.io.read_addr(consts::CPUVector::Brk as u16);

        if self.cycles == 0 {
            self.p = 0x36;
            self.s = 0xFF;
            self.a = 0;
            self.x = 0;
            self.y = 0;

            return true;
        }

        self.cycles = 0;
        self.enable_flag(consts::Flag::IntDis);
        false
    }

    /// Runs the CPU for `N` cycles, with a maximum error of 7 cycles.
    ///
    /// ## Prelude
    ///
    /// The MOS6502 has 256 available opcodes, as it uses 8-bits for them. But, 
    /// out of those 256, there are only 151 official opcodes, making the other
    /// 101 opcodes illegal. This emulator supports 232 opcodes, leaving only
    /// 24 opcodes unimplemented (temporarily).
    ///
    /// Those 232 opcodes are organized into 64 instructions (with 56 being
    /// official).
    ///
    /// ## Why is an "internal representation" being used?
    ///
    /// As this `Virtual Machine` is targeted to be a cycle accurate one, it 
    /// needs to know how many cycles it takes to run every instruction. It
    /// also needs to know which address mode has every instruction.
    ///
    /// For addressing this problem, the `Virtual Machine` implements an
    /// internal translation jump table for grouping every opcode into an
    /// internal opcode, and also, for being able to identify the address mode,
    /// and timing of each instruction.
    ///
    /// This "translation jump table" is implemented at
    /// [consts::TRANSLATION_TABLE], and its automatically generated by a
    /// script, so it is less prone to human error.
    ///
    /// ## Internal Representation
    ///
    /// ```
    /// 0000000000000000
    /// ^^^^^^^^    ^^^ 
    ///  OpCode ^^^^Tim^
    ///         AdMd   E
    /// ```
    ///
    /// - OpCode: Internal Opcode
    /// - AdMd:   Addressing Mode
    /// - Tim:    Execution Time - 1
    /// - E:      Extra Time if Page Boundary Crossed
    ///
    /// ## Behaviour
    ///
    /// The `Virtual Machine`'s "main code" runs inside a while loop for taking
    /// cycle emulation into account. It runs until a certain "cycle limit" is
    /// reached.
    ///
    /// The "main code" can be divided into 3 parts:
    /// - A decoder
    /// - An address resolver
    /// - An opcode executor
    ///
    /// ### The Decoder
    ///
    /// This is the simplest part of the emulator's orchestra. This part
    /// retrieves information about the MOS6502 opcode, like:
    /// - Internal Opcode: The opcode that is going to be used by the
    ///   `Opcode Executor`.
    /// - Address Mode: The address mode that is going to be used to resolve an
    ///   address for the opcode by the `Address Resolver`.
    /// - Base Timing: The base timing of the opcode (how many cycles it takes
    ///   to run).
    /// - Special Timing: Used for special instructions that don't follow
    ///   MOS6502 standards for extra cycle addition on page boundary cross.
    ///
    /// ### The Address Resolver
    ///
    /// This part uses the `address mode` information from the decoder and
    /// fetches the address that is going to be used by the operation, based on
    /// the requested mode.
    ///
    /// It also does update the `timing` variable if a page boundary was
    /// crossed.
    ///
    /// ### The Opcode Executor
    ///
    /// This is the most complex piece of the emulator's main code. It works by
    /// using a match statement that runs the desired opcode efficiently.
    ///
    pub fn run(self: &mut VM<T>, cycles: usize) {
        while self.cycles < cycles && self.ex_interrupt == 0 {
            let opcode: u8 = self.next_byte();

            let internal_repr: u16 = consts::TRANSLATION_TABLE[opcode as usize];
            let raw_internal_opcode: u8 = (internal_repr >> 8) as u8;
            let raw_address_mode: u8 = ((internal_repr >> 4) & 0b1111) as u8;
            let base_timing: u8 = ((internal_repr >> 1) & 0b111) as u8;
            let mut timing: u8 = (internal_repr & 0b1) as u8;

            let mut t_byte1: u8 = 0;
            let t_byte2: u8;
            let mut t_addr: u16;

            let address_mode_result = consts::AddrMode::try_from(raw_address_mode);
            let address_mode = match address_mode_result {
                Ok(mode) => mode,
                Err(_) => {
                    self.h = 1;
                    return;
                }
            };

            match address_mode {
                consts::AddrMode::Immediate => {
                    t_addr = self.pc;
                    self.pc += 1;
                }

                consts::AddrMode::Relative => {
                    t_byte1 = self.next_byte();
                    t_addr = self.pc.wrapping_add_signed((t_byte1 as i8) as i16);
                    t_byte1 = if (t_addr & 0xff00) != (self.pc & 0xff00) {1} else {0};
                }

                consts::AddrMode::Absolute => {
                    t_addr = self.next_addr();
                }

                consts::AddrMode::Indirect => {
                    t_addr = self.next_addr();
                    if t_addr & 0xFF == 0xFF {
                        t_addr = (self.io.read_byte(t_addr) as u16)
                            | ((self.io.read_byte(t_addr & 0xff00) as u16) << 8);
                    } else {
                        t_addr = self.io.read_addr(t_addr);
                    }
                }

                consts::AddrMode::ZeroPage => {
                    t_addr = self.next_byte() as u16;
                }

                consts::AddrMode::AbsoluteX => {
                    t_addr = self.next_addr();
                    if ((t_addr.wrapping_add(self.x as u16)) & 0xff00) != (t_addr & 0xff00) {
                        timing += 1;
                    }

                    t_addr += self.x as u16;
                }

                consts::AddrMode::AbsoluteY => {
                    t_addr = self.next_addr();
                    if ((t_addr.wrapping_add(self.y as u16)) & 0xff00) != (t_addr & 0xff00) {
                        timing += 1;
                    }

                    t_addr = t_addr.wrapping_add(self.y as u16);
                }

                consts::AddrMode::ZeroPageX => {
                    t_byte1 = self.next_byte();
                    t_addr = t_byte1.wrapping_add_signed(self.x as i8) as u16;
                }

                consts::AddrMode::ZeroPageY => {
                    t_byte1 = self.next_byte();
                    t_addr = t_byte1.wrapping_add_signed(self.y as i8) as u16;
                }

                consts::AddrMode::IndirectX => {
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

                consts::AddrMode::IndirectY => {
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

            let internal_opcode_result = consts::OpCode::try_from(raw_internal_opcode);
            let internal_opcode = match internal_opcode_result {
                Ok(opcode) => opcode,
                Err(_) => {
                    self.h = 1;
                    return;
                }
            };

            match internal_opcode {
                consts::OpCode::Lda => {
                    self.a = self.io.read_byte(t_addr);
                    self.set_nz_flags(self.a);
                }

                consts::OpCode::Ldx => {
                    self.x = self.io.read_byte(t_addr);
                    self.set_nz_flags(self.x);
                }

                consts::OpCode::Ldy => {
                    self.y = self.io.read_byte(t_addr);
                    self.set_nz_flags(self.y);
                }

                consts::OpCode::Lax => {
                    // Composite
                    self.a = self.io.read_byte(t_addr);
                    self.x = self.a;
                    self.set_nz_flags(self.a);
                }

                consts::OpCode::Sax => {
                    self.io.write_byte(t_addr, self.a & self.x);
                }

                consts::OpCode::Sta => self.io.write_byte(t_addr, self.a),

                consts::OpCode::Stx => self.io.write_byte(t_addr, self.x),

                consts::OpCode::Sty => self.io.write_byte(t_addr, self.y),

                consts::OpCode::Tax => {
                    self.x = self.a;
                    self.set_nz_flags(self.a);
                }

                consts::OpCode::Tay => {
                    self.y = self.a;
                    self.set_nz_flags(self.a);
                }

                consts::OpCode::Tsx => {
                    self.x = self.s;
                    self.set_nz_flags(self.s);
                }

                consts::OpCode::Txa => {
                    self.a = self.x;
                    self.set_nz_flags(self.x);
                }

                consts::OpCode::Txs => {
                    self.s = self.x;
                }

                consts::OpCode::Tya => {
                    self.a = self.y;
                    self.set_nz_flags(self.y);
                }

                consts::OpCode::Pha => {
                    self.stack_push_byte(self.a);
                }

                consts::OpCode::Php => {
                    self.stack_push_byte(self.p | (consts::Flag::Break as u8));
                }

                consts::OpCode::Pla => {
                    self.a = self.stack_pop_byte();
                    self.set_nz_flags(self.a);
                }

                consts::OpCode::Plp => {
                    self.p = (self.stack_pop_byte() & !(consts::Flag::Break as u8))
                        | (consts::Flag::Reserved as u8);
                }

                consts::OpCode::Dec => {
                    t_byte1 = self.io.read_byte(t_addr).wrapping_sub(1);
                    self.io.write_byte(t_addr, t_byte1);
                    self.set_nz_flags(t_byte1);
                }

                consts::OpCode::Dex => {
                    self.x = self.x.wrapping_sub(1);
                    self.set_nz_flags(self.x);
                }

                consts::OpCode::Dey => {
                    self.y = self.y.wrapping_sub(1);
                    self.set_nz_flags(self.y);
                }

                consts::OpCode::Inc => {
                    t_byte1 = self.io.read_byte(t_addr).wrapping_add(1);
                    self.io.write_byte(t_addr, t_byte1);
                    self.set_nz_flags(t_byte1);
                }

                consts::OpCode::Inx => {
                    self.x = self.x.wrapping_add(1);
                    self.set_nz_flags(self.x);
                }

                consts::OpCode::Iny => {
                    self.y = self.y.wrapping_add(1);
                    self.set_nz_flags(self.y);
                }

                consts::OpCode::Isc => {
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

                consts::OpCode::Sbc => {
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

                consts::OpCode::Adc => {
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

                consts::OpCode::Rra => {
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

                consts::OpCode::Rla => {
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

                consts::OpCode::And => {
                    self.a &= self.io.read_byte(t_addr);
                    self.set_nz_flags(self.a);
                }

                consts::OpCode::Sre => {
                    // Composite
                    t_byte1 = self.io.read_byte(t_addr);
                    self.set_flag(consts::Flag::Carry, t_byte1 & 0x1 != 0);
                    t_byte1 >>= 1;
                    self.io.write_byte(t_addr, t_byte1);

                    // EOR code
                    self.a ^= t_byte1;
                    self.set_nz_flags(self.a);
                }

                consts::OpCode::Eor => {
                    self.a ^= self.io.read_byte(t_addr);
                    self.set_nz_flags(self.a);
                }

                consts::OpCode::Slo => {
                    t_byte1 = self.io.read_byte(t_addr);
                    self.set_flag(consts::Flag::Carry, t_byte1 & 0x80 != 0);
                    t_byte1 <<= 1;
                    self.io.write_byte(t_addr, t_byte1);

                    // ORA code
                    self.a |= self.io.read_byte(t_addr);
                    self.set_nz_flags(self.a);
                }

                consts::OpCode::Ora => {
                    self.a |= self.io.read_byte(t_addr);
                    self.set_nz_flags(self.a);
                }

                consts::OpCode::Asl => {
                    t_byte1 = if address_mode == consts::AddrMode::Accumulator {
                        self.a
                    } else {
                        self.io.read_byte(t_addr)
                    };

                    self.set_flag(consts::Flag::Carry, t_byte1 & 0x80 != 0);
                    t_byte1 <<= 1;

                    if address_mode == consts::AddrMode::Accumulator {
                        self.a = t_byte1;
                    } else {
                        self.io.write_byte(t_addr, t_byte1);
                    }

                    self.set_nz_flags(t_byte1);
                }

                consts::OpCode::Lsr => {
                    t_byte1 = if address_mode == consts::AddrMode::Accumulator {
                        self.a
                    } else {
                        self.io.read_byte(t_addr)
                    };

                    self.set_flag(consts::Flag::Carry, t_byte1 & 0x1 != 0);
                    t_byte1 >>= 1;

                    if address_mode == consts::AddrMode::Accumulator {
                        self.a = t_byte1;
                    } else {
                        self.io.write_byte(t_addr, t_byte1);
                    }

                    self.set_nz_flags(t_byte1);
                }

                consts::OpCode::Rol => {
                    t_byte1 = if address_mode == consts::AddrMode::Accumulator {
                        self.a
                    } else {
                        self.io.read_byte(t_addr)
                    };

                    t_byte2 = self.get_flag(consts::Flag::Carry);
                    self.set_flag(consts::Flag::Carry, t_byte1 & 0x80 != 0);
                    t_byte1 <<= 1;
                    t_byte1 = t_byte1.wrapping_add(t_byte2);

                    if address_mode == consts::AddrMode::Accumulator {
                        self.a = t_byte1;
                    } else {
                        self.io.write_byte(t_addr, t_byte1);
                    }

                    self.set_nz_flags(t_byte1);
                }

                consts::OpCode::Ror => {
                    t_byte1 = if address_mode == consts::AddrMode::Accumulator {
                        self.a
                    } else {
                        self.io.read_byte(t_addr)
                    };

                    t_byte2 = self.get_flag(consts::Flag::Carry);
                    self.set_flag(consts::Flag::Carry, t_byte1 & 0x1 != 0);
                    t_byte1 >>= 1;
                    t_byte1 = t_byte1.wrapping_add(if t_byte2 == 1 { 0x80 } else { 0 });

                    if address_mode == consts::AddrMode::Accumulator {
                        self.a = t_byte1;
                    } else {
                        self.io.write_byte(t_addr, t_byte1);
                    }

                    self.set_nz_flags(t_byte1);
                }

                consts::OpCode::Clc => {
                    self.disable_flag(consts::Flag::Carry);
                }

                consts::OpCode::Cld => {
                    self.disable_flag(consts::Flag::Decimal);                    
                }

                consts::OpCode::Cli => {
                    self.disable_flag(consts::Flag::IntDis);
                }

                consts::OpCode::Clv => {
                    self.disable_flag(consts::Flag::Overflow);
                }

                consts::OpCode::Sec => {
                    self.enable_flag(consts::Flag::Carry);
                }

                consts::OpCode::Sed => {
                    self.enable_flag(consts::Flag::Decimal);
                }

                consts::OpCode::Sei => {
                    self.enable_flag(consts::Flag::IntDis);
                }

                consts::OpCode::Dcp => {    // Composite
                    t_byte1 = self.io.read_byte(t_addr).wrapping_sub(1);
                    self.io.write_byte(t_addr, t_byte1);

                    // CMP code
                    self.set_flag(consts::Flag::Carry, self.a >= t_byte1);
                    self.set_nz_flags(self.a.wrapping_sub(t_byte1));
                }

                consts::OpCode::Cmp => {
                    t_byte1 = self.io.read_byte(t_addr);
                    self.set_flag(consts::Flag::Carry, self.a >= t_byte1);
                    self.set_nz_flags(self.a.wrapping_sub(t_byte1));
                }

                consts::OpCode::Cpx => {
                    t_byte1 = self.io.read_byte(t_addr);
                    self.set_flag(consts::Flag::Carry, self.x >= t_byte1);
                    self.set_nz_flags(self.x.wrapping_sub(t_byte1));
                }

                consts::OpCode::Cpy => {
                    t_byte1 = self.io.read_byte(t_addr);
                    self.set_flag(consts::Flag::Carry, self.y >= t_byte1);
                    self.set_nz_flags(self.y.wrapping_sub(t_byte1));
                }

                consts::OpCode::Bcc => {
                    if self.get_flag(consts::Flag::Carry) == 0 {
                        timing += 1 + t_byte1;
                        self.pc = t_addr;
                    }
                }

                consts::OpCode::Bcs => {
                    if self.get_flag(consts::Flag::Carry) != 0 {
                        timing += 1 + t_byte1;
                        self.pc = t_addr;
                    }
                }

                consts::OpCode::Beq => {
                    if self.get_flag(consts::Flag::Zero) != 0 {
                        timing += 1 + t_byte1;
                        self.pc = t_addr;
                    }
                }

                consts::OpCode::Bmi => {
                    if self.get_flag(consts::Flag::Negative) != 0 {
                        timing += 1 + t_byte1;
                        self.pc = t_addr;
                    }
                }

                consts::OpCode::Bne => {
                    if self.get_flag(consts::Flag::Zero) == 0 {
                        timing += 1 + t_byte1;
                        self.pc = t_addr;
                    }
                }

                consts::OpCode::Bpl => {
                    if self.get_flag(consts::Flag::Negative) == 0 {
                        timing += 1 + t_byte1;
                        self.pc = t_addr;
                    }
                }

                consts::OpCode::Bvc => {
                    if self.get_flag(consts::Flag::Overflow) == 0 {
                        timing += 1 + t_byte1;
                        self.pc = t_addr;
                    }
                }

                consts::OpCode::Bvs => {
                    if self.get_flag(consts::Flag::Overflow) != 0 {
                        timing += 1 + t_byte1;
                        self.pc = t_addr;
                    }
                }

                consts::OpCode::Jmp => {
                    self.pc = t_addr;
                }

                consts::OpCode::Jsr => {
                    self.pc = self.pc.wrapping_sub(1);
                    self.stack_push_addr(self.pc);
                    self.pc = t_addr;
                }

                consts::OpCode::Rts => {
                    self.pc = self.stack_pop_addr().wrapping_add(1);
                }

                consts::OpCode::Brk => {
                    self.stack_push_addr(self.pc);
                    self.stack_push_byte(self.p | (consts::Flag::Break as u8));
                    self.enable_flag(consts::Flag::IntDis);
                    self.pc = self.io.read_addr(consts::CPUVector::Brk as u16);
                }

                consts::OpCode::Rti => {
                    self.p = (self.stack_pop_byte() & !(consts::Flag::Break as u8)) | (consts::Flag::Reserved as u8);
                    self.pc = self.stack_pop_addr();
                }

                consts::OpCode::Bit => {
                    t_byte1 = self.io.read_byte(t_addr);
                    t_byte2 = self.a & t_byte1;
                    self.set_nz_flags(t_byte2);
                    self.set_flag(consts::Flag::Overflow, t_byte2 & 0x40 != 0);
                    self.p = (self.p & 0b0011_1111) | (t_byte1 & 0b1100_0000);
                }

                consts::OpCode::Nop => {}
            }

            self.cycles += 1 + (base_timing as usize) + (if timing > 0 {(timing as usize) - 1} else {0});
        }
    }
}
