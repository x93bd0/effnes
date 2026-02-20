use crate::consts::{self, AddrMode};
use effnes_bus::{InspectBus, MemoryBus};
use std::fmt;

/// 6502 Virtual Machine
pub struct VM<T: MemoryBus> {
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
    // A magic constant involved in highly unstable opcodes.
    pub magic: u8,
}

impl<T: MemoryBus + Default> Default for VM<T> {
    /// Instanciates a new Virtual Machine.
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
            io: T::default(),
            magic: 0xFE,
        }
    }
}

impl<T: MemoryBus> VM<T> {
    fn enable_flag(&mut self, flag: consts::Flag) {
        self.p |= flag as u8;
    }

    fn disable_flag(&mut self, flag: consts::Flag) {
        self.p &= !(flag as u8);
    }

    fn set_flag(&mut self, flag: consts::Flag, value: bool) {
        if value {
            self.enable_flag(flag);
        } else {
            self.disable_flag(flag);
        }
    }

    fn get_flag(&self, flag: consts::Flag) -> u8 {
        self.p & (flag as u8)
    }

    fn set_nz_flags(&mut self, value: u8) {
        self.set_flag(consts::Flag::Negative, value & 0x80 > 0);
        self.set_flag(consts::Flag::Zero, value == 0);
    }

    fn next_byte(&mut self) -> u8 {
        let out: u8 = self.io.read_u8(self.pc);
        self.pc = self.pc.wrapping_add(1);
        out
    }

    fn next_addr(&mut self) -> u16 {
        let out: u16 = self.io.read_u16(self.pc);
        self.pc = self.pc.wrapping_add(2);
        out
    }

    /// Pushes a byte into 0x100 .. 0x1FF, depending on the current value of
    /// the stack pointer. It pushes first, then substracts one from the stack
    /// pointer.
    ///
    /// It supports stack pointer overflow.
    /// (0x100 - 1 == 0x1FF, in the current implementation)
    pub fn stack_push_byte(&mut self, value: u8) {
        self.io.write_u8((self.s as u16) | 0x100, value);
        self.s = self.s.wrapping_sub(1);
    }

    /// Pushes an address into 0x100 .. 0x1FF, depending on the current value of
    /// the stack pointer. It pushes first, then substracts two from the stack
    /// pointer.
    ///
    /// It supports stack pointer overflow.
    /// (0x100 - 1 == 0x1FF, in the current implementation)
    pub fn stack_push_addr(&mut self, value: u16) {
        self.stack_push_byte((value >> 8) as u8);
        self.stack_push_byte(value as u8);
    }

    /// Pops a byte from 0x100 .. 0x1FF, depending on the current value of
    /// the stack pointer. It adds one to the stack pointer, then returns the
    /// value of the current stack pointer position.
    ///
    /// It supports stack pointer underflow.
    /// (0x1FF + 1 == 0x100, in the current implementation)
    pub fn stack_pop_byte(&mut self) -> u8 {
        self.s = self.s.wrapping_add(1);
        self.io.read_u8((self.s as u16) | 0x100)
    }

    /// Pops an address from 0x100 .. 0x1FF, depending on the current value of
    /// the stack pointer. It adds two to the stack pointer, then returns the
    /// value of the current stack pointer position, shifted by eight to the
    /// left, plus the value of the previous stack pointer position
    /// (original_stack_pointer + 1).
    ///
    /// It supports stack pointer underflow.
    /// (0x1FF + 1 == 0x100, in the current implementation)
    pub fn stack_pop_addr(&mut self) -> u16 {
        (self.stack_pop_byte() as u16) + ((self.stack_pop_byte() as u16) << 8)
    }

    pub fn irq(&mut self) {}

    pub fn nmi(&mut self) {}

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
    /// However, in a warm reset, the CPU only sets the [consts::Flag::IntDis],
    /// the program counter, and the stack pointer to a known value. This was
    /// also taken into account while implementing it.
    ///
    /// The internal registers are also reseted by this method, independently
    /// of which type of reset it is.
    ///
    pub fn reset(&mut self) -> bool {
        self.h = 0;
        self.cycles = 0;
        self.ex_interrupt = 0;

        self.pc = self.io.read_u16(consts::CPUVector::Rst as u16);

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
    /// ```ignore
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
    pub fn run(&mut self, cycles: usize) {
        let mut t_cycles: usize = 0;
        while t_cycles < cycles && self.h == 0 {
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
                    t_byte1 = if (t_addr & 0xff00) != (self.pc & 0xff00) {
                        1
                    } else {
                        0
                    };
                }

                consts::AddrMode::Absolute => {
                    t_addr = self.next_addr();
                }

                consts::AddrMode::Indirect => {
                    t_addr = self.next_addr();
                    if t_addr & 0xFF == 0xFF {
                        t_addr = (self.io.read_u8(t_addr) as u16)
                            | ((self.io.read_u8(t_addr & 0xff00) as u16) << 8);
                    } else {
                        t_addr = self.io.read_u16(t_addr);
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
                        .read_u8(t_byte1.wrapping_add_signed((self.x as i8) + 1) as u16)
                        as u16)
                        << 8)
                        + (self
                            .io
                            .read_u8(t_byte1.wrapping_add_signed(self.x as i8) as u16)
                            as u16);
                }

                consts::AddrMode::IndirectY => {
                    t_byte1 = self.next_byte();
                    t_addr = self.io.read_u8(t_byte1 as u16) as u16;
                    t_addr += (self.io.read_u8(t_byte1.wrapping_add(1) as u16) as u16) << 8;
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
                // MemoryBus / Registers
                consts::OpCode::Lda => {
                    self.a = self.io.read_u8(t_addr);
                    self.set_nz_flags(self.a);
                }

                consts::OpCode::Ldx => {
                    self.x = self.io.read_u8(t_addr);
                    self.set_nz_flags(self.x);
                }

                consts::OpCode::Ldy => {
                    self.y = self.io.read_u8(t_addr);
                    self.set_nz_flags(self.y);
                }

                consts::OpCode::Sta => self.io.write_u8(t_addr, self.a),

                consts::OpCode::Stx => self.io.write_u8(t_addr, self.x),

                consts::OpCode::Sty => self.io.write_u8(t_addr, self.y),

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

                // Stack
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

                // Decrements / Increments
                consts::OpCode::Dec => {
                    t_byte1 = self.io.read_u8(t_addr).wrapping_sub(1);
                    self.io.write_u8(t_addr, t_byte1);
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
                    t_byte1 = self.io.read_u8(t_addr).wrapping_add(1);
                    self.io.write_u8(t_addr, t_byte1);
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

                // Arithmetic
                consts::OpCode::Sbc => {
                    // Pseudo-Composite
                    t_byte1 = !self.io.read_u8(t_addr);
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
                    t_byte1 = self.io.read_u8(t_addr);
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

                consts::OpCode::And => {
                    self.a &= self.io.read_u8(t_addr);
                    self.set_nz_flags(self.a);
                }

                consts::OpCode::Eor => {
                    self.a ^= self.io.read_u8(t_addr);
                    self.set_nz_flags(self.a);
                }

                consts::OpCode::Ora => {
                    self.a |= self.io.read_u8(t_addr);
                    self.set_nz_flags(self.a);
                }

                // Shift / Rotate
                consts::OpCode::Asl => {
                    t_byte1 = if address_mode == consts::AddrMode::Accumulator {
                        self.a
                    } else {
                        self.io.read_u8(t_addr)
                    };

                    self.set_flag(consts::Flag::Carry, t_byte1 & 0x80 != 0);
                    t_byte1 <<= 1;

                    if address_mode == consts::AddrMode::Accumulator {
                        self.a = t_byte1;
                    } else {
                        self.io.write_u8(t_addr, t_byte1);
                    }

                    self.set_nz_flags(t_byte1);
                }

                consts::OpCode::Lsr => {
                    t_byte1 = if address_mode == consts::AddrMode::Accumulator {
                        self.a
                    } else {
                        self.io.read_u8(t_addr)
                    };

                    self.set_flag(consts::Flag::Carry, t_byte1 & 0x1 != 0);
                    t_byte1 >>= 1;

                    if address_mode == consts::AddrMode::Accumulator {
                        self.a = t_byte1;
                    } else {
                        self.io.write_u8(t_addr, t_byte1);
                    }

                    self.set_nz_flags(t_byte1);
                }

                consts::OpCode::Rol => {
                    t_byte1 = if address_mode == consts::AddrMode::Accumulator {
                        self.a
                    } else {
                        self.io.read_u8(t_addr)
                    };

                    t_byte2 = self.get_flag(consts::Flag::Carry);
                    self.set_flag(consts::Flag::Carry, t_byte1 & 0x80 != 0);
                    t_byte1 <<= 1;
                    t_byte1 = t_byte1.wrapping_add(t_byte2);

                    if address_mode == consts::AddrMode::Accumulator {
                        self.a = t_byte1;
                    } else {
                        self.io.write_u8(t_addr, t_byte1);
                    }

                    self.set_nz_flags(t_byte1);
                }

                consts::OpCode::Ror => {
                    t_byte1 = if address_mode == consts::AddrMode::Accumulator {
                        self.a
                    } else {
                        self.io.read_u8(t_addr)
                    };

                    t_byte2 = self.get_flag(consts::Flag::Carry);
                    self.set_flag(consts::Flag::Carry, t_byte1 & 0x1 != 0);
                    t_byte1 >>= 1;
                    t_byte1 = t_byte1.wrapping_add(if t_byte2 == 1 { 0x80 } else { 0 });

                    if address_mode == consts::AddrMode::Accumulator {
                        self.a = t_byte1;
                    } else {
                        self.io.write_u8(t_addr, t_byte1);
                    }

                    self.set_nz_flags(t_byte1);
                }

                // Flags
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

                // Comparisons
                consts::OpCode::Cmp => {
                    t_byte1 = self.io.read_u8(t_addr);
                    self.set_flag(consts::Flag::Carry, self.a >= t_byte1);
                    self.set_nz_flags(self.a.wrapping_sub(t_byte1));
                }

                consts::OpCode::Cpx => {
                    t_byte1 = self.io.read_u8(t_addr);
                    self.set_flag(consts::Flag::Carry, self.x >= t_byte1);
                    self.set_nz_flags(self.x.wrapping_sub(t_byte1));
                }

                consts::OpCode::Cpy => {
                    t_byte1 = self.io.read_u8(t_addr);
                    self.set_flag(consts::Flag::Carry, self.y >= t_byte1);
                    self.set_nz_flags(self.y.wrapping_sub(t_byte1));
                }

                consts::OpCode::Bcc => {
                    if self.get_flag(consts::Flag::Carry) == 0 {
                        timing += 1 + t_byte1;
                        self.pc = t_addr;
                    }
                }

                // Conditional
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

                // Jumps / Subroutines
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

                // Interrupts
                consts::OpCode::Brk => {
                    self.stack_push_addr(self.pc.wrapping_add(1));
                    self.stack_push_byte(self.p | (consts::Flag::Break as u8));
                    self.enable_flag(consts::Flag::IntDis);
                    self.pc = self.io.read_u16(consts::CPUVector::Brk as u16);
                }

                consts::OpCode::Rti => {
                    self.p = (self.stack_pop_byte() & !(consts::Flag::Break as u8))
                        | (consts::Flag::Reserved as u8);
                    self.pc = self.stack_pop_addr();
                }

                consts::OpCode::Bit => {
                    t_byte1 = self.io.read_u8(t_addr);
                    t_byte2 = self.a & t_byte1;
                    self.set_nz_flags(t_byte2);
                    self.set_flag(consts::Flag::Overflow, t_byte2 & 0x40 != 0);
                    self.p = (self.p & 0b0011_1111) | (t_byte1 & 0b1100_0000);
                }

                consts::OpCode::Nop => {}

                // Illegal Opcodes
                consts::OpCode::Asr => {
                    // AND code
                    self.a &= self.io.read_u8(t_addr);
                    self.set_flag(consts::Flag::Carry, self.a & 1 != 0);
                    self.a >>= 1;
                    self.set_nz_flags(self.a);
                }

                consts::OpCode::An1 | consts::OpCode::An2 => {
                    self.a &= self.io.read_u8(t_addr);
                    self.set_flag(consts::Flag::Carry, self.a & 0x80 != 0);
                    self.set_nz_flags(self.a);
                }

                consts::OpCode::Ane => {
                    self.a &= self.magic & self.x & self.io.read_u8(t_addr);
                    self.set_nz_flags(self.a);
                }

                consts::OpCode::Arr => {
                    self.a = ((self.a & self.io.read_u8(t_addr)) >> 1)
                        | ((self.get_flag(consts::Flag::Carry) as u8) << 7);
                    self.set_nz_flags(self.a);
                    self.set_flag(consts::Flag::Carry, self.a & 0x40 != 0);
                    self.set_flag(
                        consts::Flag::Overflow,
                        (self.a & 0x40) ^ ((self.a & 0x20) << 1) != 0,
                    );
                }

                // DEC + CMP
                consts::OpCode::Dcp => {
                    // DEC code
                    t_byte1 = self.io.read_u8(t_addr).wrapping_sub(1);
                    self.io.write_u8(t_addr, t_byte1);

                    // CMP code
                    self.set_flag(consts::Flag::Carry, self.a >= t_byte1);
                    self.set_nz_flags(self.a.wrapping_sub(t_byte1));
                }

                // INC + SBC
                consts::OpCode::Isc => {
                    // INC code
                    t_byte1 = self.io.read_u8(t_addr).wrapping_add(1);
                    self.io.write_u8(t_addr, t_byte1);

                    // SBC code
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

                consts::OpCode::Las => {
                    self.s = self.io.read_u8(t_addr) & self.s;
                    self.a = self.s;
                    self.x = self.s;
                    self.set_nz_flags(self.s);
                }

                // LDA + LDX
                consts::OpCode::Lax => {
                    self.a = self.io.read_u8(t_addr);
                    self.x = self.a;
                    self.set_nz_flags(self.a);
                }

                // ROL + AND
                consts::OpCode::Rla => {
                    // ROL code
                    t_byte1 = self.io.read_u8(t_addr);
                    t_byte2 = self.get_flag(consts::Flag::Carry);
                    self.set_flag(consts::Flag::Carry, t_byte1 & 0x80 != 0);
                    t_byte1 <<= 1;
                    t_byte1 = t_byte1.wrapping_add(t_byte2);
                    self.io.write_u8(t_addr, t_byte1);

                    // AND code
                    self.a &= t_byte1;
                    self.set_nz_flags(self.a);
                }

                // ROR + ADC
                consts::OpCode::Rra => {
                    // ROR code
                    t_byte1 = self.io.read_u8(t_addr);
                    t_byte2 = self.get_flag(consts::Flag::Carry);
                    self.set_flag(consts::Flag::Carry, t_byte1 & 0x1 != 0);
                    t_byte1 >>= 1;
                    t_byte1 = t_byte1.wrapping_add(if t_byte2 != 0 { 0x80 } else { 0 });
                    self.io.write_u8(t_addr, t_byte1);

                    // ADC code
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

                // S(Accumulator & X register)
                consts::OpCode::Sax => {
                    self.io.write_u8(t_addr, self.a & self.x);
                }

                consts::OpCode::Sbx => {
                    self.x &= self.a;
                    t_addr = (self.x as u16).wrapping_sub(self.io.read_u8(t_addr) as u16);
                    self.set_flag(consts::Flag::Carry, t_addr < 0x100);
                    self.x = t_addr as u8;
                    self.set_nz_flags(self.x);
                }

                consts::OpCode::Sha => {
                    t_byte1 = match address_mode {
                        // TODO: Only read one bit
                        consts::AddrMode::AbsoluteY => {
                            ((self.io.read_u16(self.pc.wrapping_sub(2)) >> 8) as u8).wrapping_add(1)
                        }
                        consts::AddrMode::ZeroPageY => {
                            t_byte1 = self.io.read_u8(self.pc.wrapping_sub(1));
                            self.io.read_u8(t_byte1 as u16).wrapping_add(1)
                        }
                        _ => {
                            self.h = 1;
                            break;
                        }
                    };

                    self.io.write_u8(t_addr, self.a & self.x & t_byte1);
                }

                consts::OpCode::Shx | consts::OpCode::Shy => {
                    // TODO: Only read one byte
                    t_byte2 = self.io.read_u8(self.pc.wrapping_sub(1));
                    t_byte1 = t_byte2.wrapping_add(1)
                        & (if raw_internal_opcode == (consts::OpCode::Shx as u8) {
                            self.x
                        } else {
                            self.y
                        });

                    if t_byte2 != ((t_addr >> 8) as u8) {
                        t_addr = (t_addr & 0xff) | ((t_byte1 as u16) << 8).wrapping_add(1);
                    }

                    self.io.write_u8(t_addr, t_byte1);
                }

                // ASL + ORA
                consts::OpCode::Slo => {
                    // ASL code
                    t_byte1 = self.io.read_u8(t_addr);
                    self.set_flag(consts::Flag::Carry, t_byte1 & 0x80 != 0);
                    t_byte1 <<= 1;
                    self.io.write_u8(t_addr, t_byte1);

                    // ORA code
                    self.a |= self.io.read_u8(t_addr);
                    self.set_nz_flags(self.a);
                }

                // LSR + EOR
                consts::OpCode::Sre => {
                    // LSR code
                    t_byte1 = self.io.read_u8(t_addr);
                    self.set_flag(consts::Flag::Carry, t_byte1 & 0x1 != 0);
                    t_byte1 >>= 1;
                    self.io.write_u8(t_addr, t_byte1);

                    // EOR code
                    self.a ^= t_byte1;
                    self.set_nz_flags(self.a);
                }

                consts::OpCode::Tas => {
                    // TODO: Only read one byte
                    self.s = self.a & self.x;
                    t_byte1 =
                        ((self.io.read_u16(self.pc.wrapping_sub(2)) >> 8) as u8).wrapping_add(1);
                    self.io.write_u8(t_addr, self.s & t_byte1);
                }

                consts::OpCode::Jam => {
                    self.h = 1;
                }
            }

            let cycle_expr: usize =
                1 + (base_timing as usize) + (if timing > 0 { (timing as usize) - 1 } else { 0 });
            self.cycles += cycle_expr;
            t_cycles += cycle_expr;
        }
    }
}

impl<T: InspectBus + MemoryBus> fmt::Display for VM<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        // AB CD EF `MNE args`

        let opcode = self.io.peek_u8(self.pc);
        let internal_repr: u16 = consts::TRANSLATION_TABLE[opcode as usize];
        let raw_internal_opcode: u8 = (internal_repr >> 8) as u8;
        let raw_address_mode: u8 = ((internal_repr >> 4) & 0b1111) as u8;

        write!(f, "VM[{:2x}", opcode)?;

        let address_mode = match consts::AddrMode::try_from(raw_address_mode) {
            Ok(result) => result,
            Err(_) => {
                write!(f, ", INVALID_ADDR_MODE]")?;
                return Ok(());
            }
        };

        let print;
        let length;
        let pc = self.pc.wrapping_add(1);

        (print, length) = match address_mode {
            consts::AddrMode::Immediate => (format!("#${:02x}", self.io.peek_u8(pc)), 1),
            consts::AddrMode::Relative => (
                format!(
                    "${:04x}",
                    self.pc
                        .wrapping_add(2)
                        .wrapping_add_signed((self.io.peek_u8(pc) as i8) as i16)
                ),
                1,
            ),

            consts::AddrMode::Absolute => (format!("${:04x}", self.io.peek_u16(pc)), 2),

            consts::AddrMode::Indirect => (format!("(${:04x})", self.io.peek_u16(pc)), 2),
            consts::AddrMode::ZeroPage => (format!("${:02x}", self.io.peek_u8(pc)), 1),
            consts::AddrMode::ZeroPageX => (format!("${:02x},X", self.io.peek_u8(pc)), 1),
            consts::AddrMode::ZeroPageY => (format!("${:02x},Y", self.io.peek_u8(pc)), 1),
            consts::AddrMode::AbsoluteX => (format!("${:04x},X", self.io.peek_u16(pc)), 2),
            consts::AddrMode::AbsoluteY => (format!("${:04x},Y", self.io.peek_u16(pc)), 2),
            consts::AddrMode::IndirectX => (format!("(${:02x},X)", self.io.peek_u16(pc)), 1),
            consts::AddrMode::IndirectY => (format!("(${:02x}),Y", self.io.peek_u16(pc)), 1),
            consts::AddrMode::Accumulator => ("A".to_string(), 0),
            consts::AddrMode::Implied => ("".to_string(), 0),
        };

        for x in 0..length {
            write!(f, " {:2x}", self.io.peek_u8(pc.wrapping_add(x)))?;
        }

        for _ in length..2 {
            write!(f, "   ")?;
        }

        write!(
            f,
            " | {} {}",
            consts::MNEMONICS_TABLE[raw_internal_opcode as usize],
            print
        )?;

        for _ in (4 + print.len())..14 {
            write!(f, " ")?;
        }

        write!(
            f,
            "| A:{:02x} X:{:02x} Y:{:02x} S:{:02x} P:{:02x}] | PC:{:04x} CYC:{}",
            self.a, self.x, self.y, self.s, self.p, self.pc, self.cycles
        )?;

        // TODO: Address Inspection
        Ok(())
    }
}
