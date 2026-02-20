use crate::consts::{self, AddrMode, OpCode};
use effnes_bus::{peripheral::Peripheral, MemoryBus};
use effnes_cpu::{
    consts::{CpuVector, Flags},
    inspect::{InspectCpu, State as CpuState},
};

/// 6502 Virtual Machine
pub struct VM {
    /// The program counter.
    pub pc: u16,

    /// The X register.
    pub x: u8,
    /// The Y register.
    pub y: u8,
    /// The Accumulator register.
    pub a: u8,
    /// The stack pointer register.
    pub r_sp: u8,
    /// The program status register.
    pub r_ps: Flags,

    /// An internal flag used for validating an execution (checking that the code didn't halt).
    pub h: u8,
    /// An internal flag used for halting the CPU while it's running (by the memory bus).
    pub ex_interrupt: u8,
    /// The CPU cycle count.
    pub cycles: usize,
    // A magic constant involved in highly unstable opcodes.
    pub magic: u8,
}

impl VM {
    fn set_flag(&mut self, flag: Flags, value: bool) {
        self.r_ps.set(flag, value)
    }

    fn set_nz_flags(&mut self, value: u8) {
        self.set_flag(Flags::Negative, value & 0x80 > 0);
        self.set_flag(Flags::Zero, value == 0);
    }

    fn next_byte(&mut self, io: &mut impl MemoryBus) -> u8 {
        let out: u8 = io.read_u8(self.pc);
        self.pc = self.pc.wrapping_add(1);
        out
    }

    fn next_addr(&mut self, io: &mut impl MemoryBus) -> u16 {
        let out: u16 = io.read_u16(self.pc);
        self.pc = self.pc.wrapping_add(2);
        out
    }

    /// Pushes a byte into 0x100 .. 0x1FF, depending on the current value of
    /// the stack pointer. It pushes first, then substracts one from the stack
    /// pointer.
    ///
    /// It supports stack pointer overflow.
    /// (0x100 - 1 == 0x1FF, in the current implementation)
    pub fn stack_push_byte(&mut self, io: &mut impl MemoryBus, value: u8) {
        io.write_u8((self.r_sp as u16) | 0x100, value);
        self.r_sp = self.r_sp.wrapping_sub(1);
    }

    /// Pushes an address into 0x100 .. 0x1FF, depending on the current value of
    /// the stack pointer. It pushes first, then substracts two from the stack
    /// pointer.
    ///
    /// It supports stack pointer overflow.
    /// (0x100 - 1 == 0x1FF, in the current implementation)
    pub fn stack_push_addr(&mut self, io: &mut impl MemoryBus, value: u16) {
        self.stack_push_byte(io, (value >> 8) as u8);
        self.stack_push_byte(io, value as u8);
    }

    /// Pops a byte from 0x100 .. 0x1FF, depending on the current value of
    /// the stack pointer. It adds one to the stack pointer, then returns the
    /// value of the current stack pointer position.
    ///
    /// It supports stack pointer underflow.
    /// (0x1FF + 1 == 0x100, in the current implementation)
    pub fn stack_pop_byte(&mut self, io: &mut impl MemoryBus) -> u8 {
        self.r_sp = self.r_sp.wrapping_add(1);
        io.read_u8((self.r_sp as u16) | 0x100)
    }

    /// Pops an address from 0x100 .. 0x1FF, depending on the current value of
    /// the stack pointer. It adds two to the stack pointer, then returns the
    /// value of the current stack pointer position, shifted by eight to the
    /// left, plus the value of the previous stack pointer position
    /// (original_stack_pointer + 1).
    ///
    /// It supports stack pointer underflow.
    /// (0x1FF + 1 == 0x100, in the current implementation)
    pub fn stack_pop_addr(&mut self, io: &mut impl MemoryBus) -> u16 {
        (self.stack_pop_byte(io) as u16) + ((self.stack_pop_byte(io) as u16) << 8)
    }

    pub fn irq(&mut self) {}

    pub fn nmi(&mut self) {}
}

impl Default for VM {
    fn default() -> Self {
        Self {
            pc: 0x8000,
            x: 0,
            y: 0,
            a: 0,
            r_sp: 0,
            r_ps: Flags::empty(),
            h: 0,
            ex_interrupt: 0,
            cycles: 0,
            magic: 0xFE,
        }
    }
}

impl Peripheral for VM {
    fn recv(&mut self, _: u16, _: u8) {}

    fn cold_reset(&mut self) {
        self.r_ps = Flags::empty();
        self.a = 0;
        self.x = 0;
        self.y = 0;

        self.r_sp = 0x00;
        self.pc = CpuVector::Rst as u16;

        self.warm_reset();
    }

    fn warm_reset(&mut self) {
        self.r_ps |= Flags::IntDis;
        self.r_sp = self.r_sp.wrapping_sub(0x03);
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
    fn cycle(&mut self, io: &mut impl MemoryBus) -> () {
        let opcode: u8 = self.next_byte(io);

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

        use AddrMode::*;
        match address_mode {
            Immediate => {
                t_addr = self.pc;
                self.pc += 1;
            }

            Relative => {
                t_byte1 = self.next_byte(io);
                t_addr = self.pc.wrapping_add_signed((t_byte1 as i8) as i16);
                t_byte1 = if (t_addr & 0xff00) != (self.pc & 0xff00) {
                    1
                } else {
                    0
                };
            }

            Absolute => {
                t_addr = self.next_addr(io);
            }

            Indirect => {
                t_addr = self.next_addr(io);
                if t_addr & 0xFF == 0xFF {
                    t_addr =
                        (io.read_u8(t_addr) as u16) | ((io.read_u8(t_addr & 0xff00) as u16) << 8);
                } else {
                    t_addr = io.read_u16(t_addr);
                }
            }

            ZeroPage => {
                t_addr = self.next_byte(io) as u16;
            }

            AbsoluteX => {
                t_addr = self.next_addr(io);
                if ((t_addr.wrapping_add(self.x as u16)) & 0xff00) != (t_addr & 0xff00) {
                    timing += 1;
                }

                t_addr += self.x as u16;
            }

            AbsoluteY => {
                t_addr = self.next_addr(io);
                if ((t_addr.wrapping_add(self.y as u16)) & 0xff00) != (t_addr & 0xff00) {
                    timing += 1;
                }

                t_addr = t_addr.wrapping_add(self.y as u16);
            }

            ZeroPageX => {
                t_byte1 = self.next_byte(io);
                t_addr = t_byte1.wrapping_add_signed(self.x as i8) as u16;
            }

            ZeroPageY => {
                t_byte1 = self.next_byte(io);
                t_addr = t_byte1.wrapping_add_signed(self.y as i8) as u16;
            }

            IndirectX => {
                t_byte1 = self.next_byte(io);
                t_addr = ((io.read_u8(t_byte1.wrapping_add_signed((self.x as i8) + 1) as u16)
                    as u16)
                    << 8)
                    + (io.read_u8(t_byte1.wrapping_add_signed(self.x as i8) as u16) as u16);
            }

            IndirectY => {
                t_byte1 = self.next_byte(io);
                t_addr = io.read_u8(t_byte1 as u16) as u16;
                t_addr += (io.read_u8(t_byte1.wrapping_add(1) as u16) as u16) << 8;
                if ((t_addr.wrapping_add(self.y as u16)) & 0xff00) != (t_addr & 0xff00) {
                    timing += 1;
                }

                t_addr = t_addr.wrapping_add(self.y as u16);
            }

            _ => {
                t_addr = 0;
            }
        };

        let internal_opcode_result = OpCode::try_from(raw_internal_opcode);
        let internal_opcode = match internal_opcode_result {
            Ok(opcode) => opcode,
            Err(_) => {
                self.h = 1;
                return;
            }
        };

        use OpCode::*;
        match internal_opcode {
            // Memory / Registers
            Lda => {
                self.a = io.read_u8(t_addr);
                self.set_nz_flags(self.a);
            }

            Ldx => {
                self.x = io.read_u8(t_addr);
                self.set_nz_flags(self.x);
            }

            Ldy => {
                self.y = io.read_u8(t_addr);
                self.set_nz_flags(self.y);
            }

            Sta => io.write_u8(t_addr, self.a),

            Stx => io.write_u8(t_addr, self.x),

            Sty => io.write_u8(t_addr, self.y),

            Tax => {
                self.x = self.a;
                self.set_nz_flags(self.a);
            }

            Tay => {
                self.y = self.a;
                self.set_nz_flags(self.a);
            }

            Tsx => {
                self.x = self.r_sp;
                self.set_nz_flags(self.r_sp);
            }

            Txa => {
                self.a = self.x;
                self.set_nz_flags(self.x);
            }

            Txs => {
                self.r_sp = self.x;
            }

            Tya => {
                self.a = self.y;
                self.set_nz_flags(self.y);
            }

            // Stack
            Pha => {
                self.stack_push_byte(io, self.a);
            }

            Php => {
                self.stack_push_byte(io, (self.r_ps | Flags::Break).bits());
            }

            Pla => {
                self.a = self.stack_pop_byte(io);
                self.set_nz_flags(self.a);
            }

            Plp => {
                self.r_ps = Flags::from_bits_retain(
                    (self.stack_pop_byte(io) & !(consts::Flag::Break as u8))
                        | (consts::Flag::Reserved as u8),
                );
            }

            // Decrements / Increments
            Dec => {
                t_byte1 = io.read_u8(t_addr).wrapping_sub(1);
                io.write_u8(t_addr, t_byte1);
                self.set_nz_flags(t_byte1);
            }

            Dex => {
                self.x = self.x.wrapping_sub(1);
                self.set_nz_flags(self.x);
            }

            Dey => {
                self.y = self.y.wrapping_sub(1);
                self.set_nz_flags(self.y);
            }

            Inc => {
                t_byte1 = io.read_u8(t_addr).wrapping_add(1);
                io.write_u8(t_addr, t_byte1);
                self.set_nz_flags(t_byte1);
            }

            Inx => {
                self.x = self.x.wrapping_add(1);
                self.set_nz_flags(self.x);
            }

            Iny => {
                self.y = self.y.wrapping_add(1);
                self.set_nz_flags(self.y);
            }

            // Arithmetic
            Sbc => {
                // Pseudo-Composite
                t_byte1 = !io.read_u8(t_addr);
                t_addr =
                    (self.a as u16) + (t_byte1 as u16) + (self.r_ps.contains(Flags::Carry) as u16);
                self.set_flag(Flags::Carry, t_addr > 0xFF);
                self.set_flag(
                    Flags::Overflow,
                    (!(self.a ^ t_byte1) & (self.a ^ (t_addr as u8))) & 0x80 != 0,
                );
                self.a = t_addr as u8;
                self.set_nz_flags(self.a);
            }

            Adc => {
                t_byte1 = io.read_u8(t_addr);
                t_addr =
                    (self.a as u16) + (t_byte1 as u16) + (self.r_ps.contains(Flags::Carry) as u16);
                self.set_flag(Flags::Carry, t_addr > 0xFF);
                self.set_flag(
                    Flags::Overflow,
                    (!(self.a ^ t_byte1) & (self.a ^ (t_addr as u8))) & 0x80 != 0,
                );
                self.a = t_addr as u8;
                self.set_nz_flags(self.a);
            }

            And => {
                self.a &= io.read_u8(t_addr);
                self.set_nz_flags(self.a);
            }

            Eor => {
                self.a ^= io.read_u8(t_addr);
                self.set_nz_flags(self.a);
            }

            Ora => {
                self.a |= io.read_u8(t_addr);
                self.set_nz_flags(self.a);
            }

            // Shift / Rotate
            Asl => {
                t_byte1 = if address_mode == AddrMode::Accumulator {
                    self.a
                } else {
                    io.read_u8(t_addr)
                };

                self.set_flag(Flags::Carry, t_byte1 & 0x80 != 0);
                t_byte1 <<= 1;

                if address_mode == AddrMode::Accumulator {
                    self.a = t_byte1;
                } else {
                    io.write_u8(t_addr, t_byte1);
                }

                self.set_nz_flags(t_byte1);
            }

            Lsr => {
                t_byte1 = if address_mode == AddrMode::Accumulator {
                    self.a
                } else {
                    io.read_u8(t_addr)
                };

                self.set_flag(Flags::Carry, t_byte1 & 0x1 != 0);
                t_byte1 >>= 1;

                if address_mode == AddrMode::Accumulator {
                    self.a = t_byte1;
                } else {
                    io.write_u8(t_addr, t_byte1);
                }

                self.set_nz_flags(t_byte1);
            }

            Rol => {
                t_byte1 = if address_mode == AddrMode::Accumulator {
                    self.a
                } else {
                    io.read_u8(t_addr)
                };

                t_byte2 = self.r_ps.contains(Flags::Carry) as u8;
                self.set_flag(Flags::Carry, t_byte1 & 0x80 != 0);
                t_byte1 <<= 1;
                t_byte1 = t_byte1.wrapping_add(t_byte2);

                if address_mode == AddrMode::Accumulator {
                    self.a = t_byte1;
                } else {
                    io.write_u8(t_addr, t_byte1);
                }

                self.set_nz_flags(t_byte1);
            }

            Ror => {
                t_byte1 = if address_mode == AddrMode::Accumulator {
                    self.a
                } else {
                    io.read_u8(t_addr)
                };

                t_byte2 = self.r_ps.contains(Flags::Carry) as u8;
                self.set_flag(Flags::Carry, t_byte1 & 0x1 != 0);
                t_byte1 >>= 1;
                t_byte1 = t_byte1.wrapping_add(if t_byte2 == 1 { 0x80 } else { 0 });

                if address_mode == AddrMode::Accumulator {
                    self.a = t_byte1;
                } else {
                    io.write_u8(t_addr, t_byte1);
                }

                self.set_nz_flags(t_byte1);
            }

            // Flags
            Clc => {
                self.r_ps = self.r_ps.difference(Flags::Carry);
            }

            Cld => {
                self.r_ps = self.r_ps.difference(Flags::Decimal);
            }

            Cli => {
                self.r_ps = self.r_ps.difference(Flags::IntDis);
            }

            Clv => {
                self.r_ps = self.r_ps.difference(Flags::Overflow);
            }

            Sec => {
                self.r_ps = self.r_ps.union(Flags::Carry);
            }

            Sed => {
                self.r_ps = self.r_ps.union(Flags::Decimal);
            }

            Sei => {
                self.r_ps = self.r_ps.union(Flags::IntDis);
            }

            // Comparisons
            Cmp => {
                t_byte1 = io.read_u8(t_addr);
                self.set_flag(Flags::Carry, self.a >= t_byte1);
                self.set_nz_flags(self.a.wrapping_sub(t_byte1));
            }

            Cpx => {
                t_byte1 = io.read_u8(t_addr);
                self.set_flag(Flags::Carry, self.x >= t_byte1);
                self.set_nz_flags(self.x.wrapping_sub(t_byte1));
            }

            Cpy => {
                t_byte1 = io.read_u8(t_addr);
                self.set_flag(Flags::Carry, self.y >= t_byte1);
                self.set_nz_flags(self.y.wrapping_sub(t_byte1));
            }

            Bcc => {
                if self.r_ps.contains(Flags::Carry) == false {
                    timing += 1 + t_byte1;
                    self.pc = t_addr;
                }
            }

            // Conditional
            Bcs => {
                if self.r_ps.contains(Flags::Carry) != false {
                    timing += 1 + t_byte1;
                    self.pc = t_addr;
                }
            }

            Beq => {
                if self.r_ps.contains(Flags::Zero) != false {
                    timing += 1 + t_byte1;
                    self.pc = t_addr;
                }
            }

            Bmi => {
                if self.r_ps.contains(Flags::Negative) != false {
                    timing += 1 + t_byte1;
                    self.pc = t_addr;
                }
            }

            Bne => {
                if self.r_ps.contains(Flags::Zero) == false {
                    timing += 1 + t_byte1;
                    self.pc = t_addr;
                }
            }

            Bpl => {
                if self.r_ps.contains(Flags::Negative) == false {
                    timing += 1 + t_byte1;
                    self.pc = t_addr;
                }
            }

            Bvc => {
                if self.r_ps.contains(Flags::Overflow) == false {
                    timing += 1 + t_byte1;
                    self.pc = t_addr;
                }
            }

            Bvs => {
                if self.r_ps.contains(Flags::Overflow) != false {
                    timing += 1 + t_byte1;
                    self.pc = t_addr;
                }
            }

            // Jumps / Subroutines
            Jmp => {
                self.pc = t_addr;
            }

            Jsr => {
                self.pc = self.pc.wrapping_sub(1);
                self.stack_push_addr(io, self.pc);
                self.pc = t_addr;
            }

            Rts => {
                self.pc = self.stack_pop_addr(io).wrapping_add(1);
            }

            // Interrupts
            Brk => {
                self.stack_push_addr(io, self.pc.wrapping_add(1));
                self.stack_push_byte(io, (self.r_ps | Flags::Break).bits());
                self.r_ps = self.r_ps.union(Flags::IntDis);
                self.pc = io.read_u16(CpuVector::Brk as u16);
            }

            Rti => {
                self.r_ps = Flags::from_bits_retain(
                    (self.stack_pop_byte(io) & !(<Flags as Into<u8>>::into(Flags::Break)))
                        | (<Flags as Into<u8>>::into(Flags::Reserved)),
                );
                self.pc = self.stack_pop_addr(io);
            }

            Bit => {
                t_byte1 = io.read_u8(t_addr);
                t_byte2 = self.a & t_byte1;
                self.set_nz_flags(t_byte2);
                self.set_flag(Flags::Overflow, t_byte2 & 0x40 != 0);
                self.r_ps = Flags::from_bits_retain(
                    (self.r_ps.bits() & 0b0011_1111) | (t_byte1 & 0b1100_0000),
                );
            }

            Nop => {}

            // Illegal Opcodes
            Asr => {
                // AND code
                self.a &= io.read_u8(t_addr);
                self.set_flag(Flags::Carry, self.a & 1 != 0);
                self.a >>= 1;
                self.set_nz_flags(self.a);
            }

            An1 | An2 => {
                self.a &= io.read_u8(t_addr);
                self.set_flag(Flags::Carry, self.a & 0x80 != 0);
                self.set_nz_flags(self.a);
            }

            Ane => {
                self.a &= self.magic & self.x & io.read_u8(t_addr);
                self.set_nz_flags(self.a);
            }

            Arr => {
                self.a = ((self.a & io.read_u8(t_addr)) >> 1)
                    | ((self.r_ps.contains(Flags::Carry) as u8) << 7);
                self.set_nz_flags(self.a);
                self.set_flag(Flags::Carry, self.a & 0x40 != 0);
                self.set_flag(
                    Flags::Overflow,
                    (self.a & 0x40) ^ ((self.a & 0x20) << 1) != 0,
                );
            }

            // DEC + CMP
            Dcp => {
                // DEC code
                t_byte1 = io.read_u8(t_addr).wrapping_sub(1);
                io.write_u8(t_addr, t_byte1);

                // CMP code
                self.set_flag(Flags::Carry, self.a >= t_byte1);
                self.set_nz_flags(self.a.wrapping_sub(t_byte1));
            }

            // INC + SBC
            Isc => {
                // INC code
                t_byte1 = io.read_u8(t_addr).wrapping_add(1);
                io.write_u8(t_addr, t_byte1);

                // SBC code
                t_byte1 = !t_byte1;
                t_addr =
                    (self.a as u16) + (t_byte1 as u16) + (self.r_ps.contains(Flags::Carry) as u16);
                self.set_flag(Flags::Carry, t_addr > 0xFF);
                self.set_flag(
                    Flags::Overflow,
                    (!(self.a ^ t_byte1) & (self.a ^ (t_addr as u8))) & 0x80 != 0,
                );
                self.a = t_addr as u8;
                self.set_nz_flags(self.a);
            }

            Las => {
                self.r_sp = io.read_u8(t_addr) & self.r_sp;
                self.a = self.r_sp;
                self.x = self.r_sp;
                self.set_nz_flags(self.r_sp);
            }

            // LDA + LDX
            Lax => {
                self.a = io.read_u8(t_addr);
                self.x = self.a;
                self.set_nz_flags(self.a);
            }

            // ROL + AND
            Rla => {
                // ROL code
                t_byte1 = io.read_u8(t_addr);
                t_byte2 = self.r_ps.contains(Flags::Carry) as u8;
                self.set_flag(Flags::Carry, t_byte1 & 0x80 != 0);
                t_byte1 <<= 1;
                t_byte1 = t_byte1.wrapping_add(t_byte2);
                io.write_u8(t_addr, t_byte1);

                // AND code
                self.a &= t_byte1;
                self.set_nz_flags(self.a);
            }

            // ROR + ADC
            Rra => {
                // ROR code
                t_byte1 = io.read_u8(t_addr);
                t_byte2 = self.r_ps.contains(Flags::Carry) as u8;
                self.set_flag(Flags::Carry, t_byte1 & 0x1 != 0);
                t_byte1 >>= 1;
                t_byte1 = t_byte1.wrapping_add(if t_byte2 != 0 { 0x80 } else { 0 });
                io.write_u8(t_addr, t_byte1);

                // ADC code
                t_addr =
                    (self.a as u16) + (t_byte1 as u16) + (self.r_ps.contains(Flags::Carry) as u16);
                self.set_flag(Flags::Carry, t_addr > 0xFF);
                self.set_flag(
                    Flags::Overflow,
                    (!(self.a ^ t_byte1) & (self.a ^ (t_addr as u8))) & 0x80 != 0,
                );
                self.a = t_addr as u8;
                self.set_nz_flags(self.a);
            }

            // S(Accumulator & X register)
            Sax => {
                io.write_u8(t_addr, self.a & self.x);
            }

            Sbx => {
                self.x &= self.a;
                t_addr = (self.x as u16).wrapping_sub(io.read_u8(t_addr) as u16);
                self.set_flag(Flags::Carry, t_addr < 0x100);
                self.x = t_addr as u8;
                self.set_nz_flags(self.x);
            }

            Sha => {
                t_byte1 = match address_mode {
                    // TODO: Only read one bit
                    AddrMode::AbsoluteY => {
                        ((io.read_u16(self.pc.wrapping_sub(2)) >> 8) as u8).wrapping_add(1)
                    }
                    AddrMode::ZeroPageY => {
                        t_byte1 = io.read_u8(self.pc.wrapping_sub(1)).wrapping_add(1);
                        io.read_u8(t_byte1 as u16)
                    }
                    _ => {
                        self.h = 1;
                        return;
                    }
                };

                io.write_u8(t_addr, self.a & self.x & t_byte1);
            }

            Shx | Shy => {
                // TODO: Only read one byte
                t_byte2 = io.read_u8(self.pc.wrapping_sub(1));
                t_byte1 = t_byte2.wrapping_add(1)
                    & (if raw_internal_opcode == (Shx as u8) {
                        self.x
                    } else {
                        self.y
                    });

                if t_byte2 != ((t_addr >> 8) as u8) {
                    t_addr = (t_addr & 0xff) | ((t_byte1 as u16) << 8).wrapping_add(1);
                }

                io.write_u8(t_addr, t_byte1);
            }

            // ASL + ORA
            Slo => {
                // ASL code
                t_byte1 = io.read_u8(t_addr);
                self.set_flag(Flags::Carry, t_byte1 & 0x80 != 0);
                t_byte1 <<= 1;
                io.write_u8(t_addr, t_byte1);

                // ORA code
                self.a |= io.read_u8(t_addr);
                self.set_nz_flags(self.a);
            }

            // LSR + EOR
            Sre => {
                // LSR code
                t_byte1 = io.read_u8(t_addr);
                self.set_flag(Flags::Carry, t_byte1 & 0x1 != 0);
                t_byte1 >>= 1;
                io.write_u8(t_addr, t_byte1);

                // EOR code
                self.a ^= t_byte1;
                self.set_nz_flags(self.a);
            }

            Tas => {
                // TODO: Only read one byte
                self.r_sp = self.a & self.x;
                // TODO: Remove extra var
                let ex = ((io.read_u16(self.pc.wrapping_sub(2)) >> 8) as u8).wrapping_add(1);
                io.write_u8(t_addr, self.r_sp & ex);
            }

            Jam => {
                self.h = 1;
            }
        }

        let cycle_expr: u8 = 1 + base_timing + if timing > 0 { timing - 1 } else { 0 };
        self.cycles += cycle_expr as usize;
    }
}

impl InspectCpu for VM {
    fn is_cycle_accurate(&self) -> bool {
        false
    }

    fn state(&self) -> CpuState {
        CpuState {
            pc: self.pc,
            sp: self.r_sp,
            ac: self.a,
            ix: self.x,
            iy: self.y,
            am: effnes_cpu::addr::AddressingMode::Implied,
            ps: self.r_ps,
            cc: self.cycles,
        }
    }
}

// impl<T: MemoryBus + InspectBus> fmt::Display for VM<T> {
//     fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
//         // AB CD EF `MNE args`

//         let opcode = io.peek_u8(self.pc);
//         let internal_repr: u16 = consts::TRANSLATION_TABLE[opcode as usize];
//         let raw_internal_opcode: u8 = (internal_repr >> 8) as u8;
//         let raw_address_mode: u8 = ((internal_repr >> 4) & 0b1111) as u8;

//         write!(f, "VM[{:2x}", opcode)?;

//         let address_mode = match consts::AddrMode::try_from(raw_address_mode) {
//             Ok(result) => result,
//             Err(_) => {
//                 write!(f, ", INVALID_ADDR_MODE]")?;
//                 return Ok(());
//             }
//         };

//         let print;
//         let length;
//         let pc = self.pc.wrapping_add(1);

//         (print, length) = match address_mode {
//             consts::AddrMode::Immediate => (format!("#${:02x}", io.peek_u8(pc)), 1),
//             consts::AddrMode::Relative => (
//                 format!(
//                     "${:04x}",
//                     self.pc
//                         .wrapping_add(2)
//                         .wrapping_add_signed((io.peek_u8(pc) as i8) as i16)
//                 ),
//                 1,
//             ),

//             consts::AddrMode::Absolute => (format!("${:04x}", io.peek_u16(pc)), 2),

//             consts::AddrMode::Indirect => (format!("(${:04x})", io.peek_u16(pc)), 2),
//             consts::AddrMode::ZeroPage => (format!("${:02x}", io.peek_u8(pc)), 1),
//             consts::AddrMode::ZeroPageX => (format!("${:02x},X", io.peek_u8(pc)), 1),
//             consts::AddrMode::ZeroPageY => (format!("${:02x},Y", io.peek_u8(pc)), 1),
//             consts::AddrMode::AbsoluteX => (format!("${:04x},X", io.peek_u16(pc)), 2),
//             consts::AddrMode::AbsoluteY => (format!("${:04x},Y", io.peek_u16(pc)), 2),
//             consts::AddrMode::IndirectX => (format!("(${:02x},X)", io.peek_u16(pc)), 1),
//             consts::AddrMode::IndirectY => (format!("(${:02x}),Y", io.peek_u16(pc)), 1),
//             consts::AddrMode::Accumulator => ("A".to_string(), 0),
//             consts::AddrMode::Implied => ("".to_string(), 0),
//         };

//         for x in 0..length {
//             write!(f, " {:2x}", io.peek_u8(pc.wrapping_add(x)))?;
//         }

//         for _ in length..2 {
//             write!(f, "   ")?;
//         }

//         write!(
//             f,
//             " | {} {}",
//             consts::MNEMONICS_TABLE[raw_internal_opcode as usize],
//             print
//         )?;

//         for _ in (4 + print.len())..14 {
//             write!(f, " ")?;
//         }

//         write!(
//             f,
//             "| A:{:02x} X:{:02x} Y:{:02x} S:{:02x} P:{:02x}] | PC:{:04x} CYC:{}",
//             self.a, self.x, self.y, self.s, self.p, self.pc, self.cycles
//         )?;

//         // TODO: Address Inspection
//         Ok(())
//     }
// }
