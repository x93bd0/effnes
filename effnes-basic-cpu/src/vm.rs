use crate::consts;
use effnes_bus::{peripheral::Peripheral, MemoryBus};
use effnes_cpu::{
    addr::{AddressingMode, IndexRegister},
    consts::{CpuVector, Flags},
    inspect::{InspectCpu, State as CpuState},
    opcode::{Mnemonic, OpCode},
};

/// 6502 Virtual Machine
pub struct VM {
    /// The program counter.
    r_pc: u16,

    /// The X register.
    r_ix: u8,
    /// The Y register.
    r_iy: u8,
    /// The Accumulator register.
    r_ac: u8,
    /// The stack pointer register.
    r_sp: u8,
    /// The program status register.
    r_ps: Flags,

    /// An internal flag used for validating an execution (checking that the code didn't halt).
    i_hl: u8,
    /// The CPU cycle count.
    i_cc: usize,
    // A magic constant involved in highly unstable opcodes.
    magic: u8,
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
        let out: u8 = io.read_u8(self.r_pc);
        self.r_pc = self.r_pc.wrapping_add(1);
        out
    }

    fn next_addr(&mut self, io: &mut impl MemoryBus) -> u16 {
        let out: u16 = io.read_u16(self.r_pc);
        self.r_pc = self.r_pc.wrapping_add(2);
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
            r_pc: 0x8000,
            r_ix: 0,
            r_iy: 0,
            r_ac: 0,
            r_sp: 0,
            r_ps: Flags::empty(),
            i_hl: 0,
            i_cc: 0,
            magic: 0xFE,
        }
    }
}

impl Peripheral for VM {
    fn recv(&mut self, _: u16, _: u8) {}

    fn cold_reset(&mut self) {
        self.r_ps = Flags::empty();
        self.r_ac = 0;
        self.r_ix = 0;
        self.r_iy = 0;

        self.r_sp = 0x00;
        self.r_pc = CpuVector::Rst as u16;

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
        let opcode: OpCode = self.next_byte(io);

        let am: AddressingMode = opcode.into();

        let internal_repr: u16 = consts::TRANSLATION_TABLE[opcode as usize];
        let base_timing: u8 = ((internal_repr >> 1) & 0b111) as u8;
        let mut timing: u8 = (internal_repr & 0b1) as u8;

        let mut t_byte1: u8 = 0;
        let t_byte2: u8;
        let mut t_addr: u16;

        use AddressingMode::*;
        use IndexRegister::*;
        match am {
            Immediate => {
                t_addr = self.r_pc;
                self.r_pc += 1;
            }

            Relative => {
                t_byte1 = self.next_byte(io);
                t_addr = self.r_pc.wrapping_add_signed((t_byte1 as i8) as i16);
                t_byte1 = if (t_addr & 0xff00) != (self.r_pc & 0xff00) {
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

            AbsoluteI(X) => {
                t_addr = self.next_addr(io);
                if ((t_addr.wrapping_add(self.r_ix as u16)) & 0xff00) != (t_addr & 0xff00) {
                    timing += 1;
                }

                t_addr += self.r_ix as u16;
            }

            AbsoluteI(Y) => {
                t_addr = self.next_addr(io);
                if ((t_addr.wrapping_add(self.r_iy as u16)) & 0xff00) != (t_addr & 0xff00) {
                    timing += 1;
                }

                t_addr = t_addr.wrapping_add(self.r_iy as u16);
            }

            ZeroPageI(X) => {
                t_byte1 = self.next_byte(io);
                t_addr = t_byte1.wrapping_add_signed(self.r_ix as i8) as u16;
            }

            ZeroPageI(Y) => {
                t_byte1 = self.next_byte(io);
                t_addr = t_byte1.wrapping_add_signed(self.r_iy as i8) as u16;
            }

            IndirectI(X) => {
                t_byte1 = self.next_byte(io);
                t_addr = ((io.read_u8(t_byte1.wrapping_add_signed((self.r_ix as i8) + 1) as u16)
                    as u16)
                    << 8)
                    + (io.read_u8(t_byte1.wrapping_add_signed(self.r_ix as i8) as u16) as u16);
            }

            IndirectI(Y) => {
                t_byte1 = self.next_byte(io);
                t_addr = io.read_u8(t_byte1 as u16) as u16;
                t_addr += (io.read_u8(t_byte1.wrapping_add(1) as u16) as u16) << 8;
                if ((t_addr.wrapping_add(self.r_iy as u16)) & 0xff00) != (t_addr & 0xff00) {
                    timing += 1;
                }

                t_addr = t_addr.wrapping_add(self.r_iy as u16);
            }

            _ => {
                t_addr = 0;
            }
        };

        use Mnemonic::*;
        let mne: Mnemonic = opcode.into();
        match mne {
            // Memory / Registers
            Lda => {
                self.r_ac = io.read_u8(t_addr);
                self.set_nz_flags(self.r_ac);
            }

            Ldx => {
                self.r_ix = io.read_u8(t_addr);
                self.set_nz_flags(self.r_ix);
            }

            Ldy => {
                self.r_iy = io.read_u8(t_addr);
                self.set_nz_flags(self.r_iy);
            }

            Sta => io.write_u8(t_addr, self.r_ac),

            Stx => io.write_u8(t_addr, self.r_ix),

            Sty => io.write_u8(t_addr, self.r_iy),

            Tax => {
                self.r_ix = self.r_ac;
                self.set_nz_flags(self.r_ac);
            }

            Tay => {
                self.r_iy = self.r_ac;
                self.set_nz_flags(self.r_ac);
            }

            Tsx => {
                self.r_ix = self.r_sp;
                self.set_nz_flags(self.r_sp);
            }

            Txa => {
                self.r_ac = self.r_ix;
                self.set_nz_flags(self.r_ix);
            }

            Txs => {
                self.r_sp = self.r_ix;
            }

            Tya => {
                self.r_ac = self.r_iy;
                self.set_nz_flags(self.r_iy);
            }

            // Stack
            Pha => {
                self.stack_push_byte(io, self.r_ac);
            }

            Php => {
                self.stack_push_byte(io, (self.r_ps | Flags::Break).bits());
            }

            Pla => {
                self.r_ac = self.stack_pop_byte(io);
                self.set_nz_flags(self.r_ac);
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
                self.r_ix = self.r_ix.wrapping_sub(1);
                self.set_nz_flags(self.r_ix);
            }

            Dey => {
                self.r_iy = self.r_iy.wrapping_sub(1);
                self.set_nz_flags(self.r_iy);
            }

            Inc => {
                t_byte1 = io.read_u8(t_addr).wrapping_add(1);
                io.write_u8(t_addr, t_byte1);
                self.set_nz_flags(t_byte1);
            }

            Inx => {
                self.r_ix = self.r_ix.wrapping_add(1);
                self.set_nz_flags(self.r_ix);
            }

            Iny => {
                self.r_iy = self.r_iy.wrapping_add(1);
                self.set_nz_flags(self.r_iy);
            }

            // Arithmetic
            Sbc => {
                // Pseudo-Composite
                t_byte1 = !io.read_u8(t_addr);
                t_addr = (self.r_ac as u16)
                    + (t_byte1 as u16)
                    + (self.r_ps.contains(Flags::Carry) as u16);
                self.set_flag(Flags::Carry, t_addr > 0xFF);
                self.set_flag(
                    Flags::Overflow,
                    (!(self.r_ac ^ t_byte1) & (self.r_ac ^ (t_addr as u8))) & 0x80 != 0,
                );
                self.r_ac = t_addr as u8;
                self.set_nz_flags(self.r_ac);
            }

            Adc => {
                t_byte1 = io.read_u8(t_addr);
                t_addr = (self.r_ac as u16)
                    + (t_byte1 as u16)
                    + (self.r_ps.contains(Flags::Carry) as u16);
                self.set_flag(Flags::Carry, t_addr > 0xFF);
                self.set_flag(
                    Flags::Overflow,
                    (!(self.r_ac ^ t_byte1) & (self.r_ac ^ (t_addr as u8))) & 0x80 != 0,
                );
                self.r_ac = t_addr as u8;
                self.set_nz_flags(self.r_ac);
            }

            And => {
                self.r_ac &= io.read_u8(t_addr);
                self.set_nz_flags(self.r_ac);
            }

            Eor => {
                self.r_ac ^= io.read_u8(t_addr);
                self.set_nz_flags(self.r_ac);
            }

            Ora => {
                self.r_ac |= io.read_u8(t_addr);
                self.set_nz_flags(self.r_ac);
            }

            // Shift / Rotate
            Asl => {
                t_byte1 = if am == Implied {
                    self.r_ac
                } else {
                    io.read_u8(t_addr)
                };

                self.set_flag(Flags::Carry, t_byte1 & 0x80 != 0);
                t_byte1 <<= 1;

                if am == Implied {
                    self.r_ac = t_byte1;
                } else {
                    io.write_u8(t_addr, t_byte1);
                }

                self.set_nz_flags(t_byte1);
            }

            Lsr => {
                t_byte1 = if am == Implied {
                    self.r_ac
                } else {
                    io.read_u8(t_addr)
                };

                self.set_flag(Flags::Carry, t_byte1 & 0x1 != 0);
                t_byte1 >>= 1;

                if am == Implied {
                    self.r_ac = t_byte1;
                } else {
                    io.write_u8(t_addr, t_byte1);
                }

                self.set_nz_flags(t_byte1);
            }

            Rol => {
                t_byte1 = if am == Implied {
                    self.r_ac
                } else {
                    io.read_u8(t_addr)
                };

                t_byte2 = self.r_ps.contains(Flags::Carry) as u8;
                self.set_flag(Flags::Carry, t_byte1 & 0x80 != 0);
                t_byte1 <<= 1;
                t_byte1 = t_byte1.wrapping_add(t_byte2);

                if am == Implied {
                    self.r_ac = t_byte1;
                } else {
                    io.write_u8(t_addr, t_byte1);
                }

                self.set_nz_flags(t_byte1);
            }

            Ror => {
                t_byte1 = if am == Implied {
                    self.r_ac
                } else {
                    io.read_u8(t_addr)
                };

                t_byte2 = self.r_ps.contains(Flags::Carry) as u8;
                self.set_flag(Flags::Carry, t_byte1 & 0x1 != 0);
                t_byte1 >>= 1;
                t_byte1 = t_byte1.wrapping_add(if t_byte2 == 1 { 0x80 } else { 0 });

                if am == Implied {
                    self.r_ac = t_byte1;
                } else {
                    io.write_u8(t_addr, t_byte1);
                }

                self.set_nz_flags(t_byte1);
            }

            // Flags
            Clx { flag } => {
                self.r_ps = self.r_ps.difference(flag);
            }

            Sfx { flag } => {
                self.r_ps = self.r_ps.union(flag);
            }

            // Comparisons
            Cmp => {
                t_byte1 = io.read_u8(t_addr);
                self.set_flag(Flags::Carry, self.r_ac >= t_byte1);
                self.set_nz_flags(self.r_ac.wrapping_sub(t_byte1));
            }

            Cpx => {
                t_byte1 = io.read_u8(t_addr);
                self.set_flag(Flags::Carry, self.r_ix >= t_byte1);
                self.set_nz_flags(self.r_ix.wrapping_sub(t_byte1));
            }

            Cpy => {
                t_byte1 = io.read_u8(t_addr);
                self.set_flag(Flags::Carry, self.r_iy >= t_byte1);
                self.set_nz_flags(self.r_iy.wrapping_sub(t_byte1));
            }

            // Conditional
            Bxx { flag, set } => {
                if self.r_ps.contains(flag) == set {
                    timing += 1 + t_byte1;
                    self.r_pc = t_addr;
                }
            }

            // Jumps / Subroutines
            Jmp => {
                self.r_pc = t_addr;
            }

            Jsr => {
                self.r_pc = self.r_pc.wrapping_sub(1);
                self.stack_push_addr(io, self.r_pc);
                self.r_pc = t_addr;
            }

            Rts => {
                self.r_pc = self.stack_pop_addr(io).wrapping_add(1);
            }

            // Interrupts
            Brk => {
                self.stack_push_addr(io, self.r_pc.wrapping_add(1));
                self.stack_push_byte(io, (self.r_ps | Flags::Break).bits());
                self.r_ps = self.r_ps.union(Flags::IntDis);
                self.r_pc = io.read_u16(CpuVector::Brk as u16);
            }

            Rti => {
                self.r_ps = Flags::from_bits_retain(
                    (self.stack_pop_byte(io) & !(<Flags as Into<u8>>::into(Flags::Break)))
                        | (<Flags as Into<u8>>::into(Flags::Reserved)),
                );
                self.r_pc = self.stack_pop_addr(io);
            }

            Bit => {
                t_byte1 = io.read_u8(t_addr);
                t_byte2 = self.r_ac & t_byte1;
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
                self.r_ac &= io.read_u8(t_addr);
                self.set_flag(Flags::Carry, self.r_ac & 1 != 0);
                self.r_ac >>= 1;
                self.set_nz_flags(self.r_ac);
            }

            Anc => {
                self.r_ac &= io.read_u8(t_addr);
                self.set_flag(Flags::Carry, self.r_ac & 0x80 != 0);
                self.set_nz_flags(self.r_ac);
            }

            Ane => {
                self.r_ac &= self.magic & self.r_ix & io.read_u8(t_addr);
                self.set_nz_flags(self.r_ac);
            }

            Arr => {
                self.r_ac = ((self.r_ac & io.read_u8(t_addr)) >> 1)
                    | ((self.r_ps.contains(Flags::Carry) as u8) << 7);
                self.set_nz_flags(self.r_ac);
                self.set_flag(Flags::Carry, self.r_ac & 0x40 != 0);
                self.set_flag(
                    Flags::Overflow,
                    (self.r_ac & 0x40) ^ ((self.r_ac & 0x20) << 1) != 0,
                );
            }

            // DEC + CMP
            Dcp => {
                // DEC code
                t_byte1 = io.read_u8(t_addr).wrapping_sub(1);
                io.write_u8(t_addr, t_byte1);

                // CMP code
                self.set_flag(Flags::Carry, self.r_ac >= t_byte1);
                self.set_nz_flags(self.r_ac.wrapping_sub(t_byte1));
            }

            // INC + SBC
            Isc => {
                // INC code
                t_byte1 = io.read_u8(t_addr).wrapping_add(1);
                io.write_u8(t_addr, t_byte1);

                // SBC code
                t_byte1 = !t_byte1;
                t_addr = (self.r_ac as u16)
                    + (t_byte1 as u16)
                    + (self.r_ps.contains(Flags::Carry) as u16);
                self.set_flag(Flags::Carry, t_addr > 0xFF);
                self.set_flag(
                    Flags::Overflow,
                    (!(self.r_ac ^ t_byte1) & (self.r_ac ^ (t_addr as u8))) & 0x80 != 0,
                );
                self.r_ac = t_addr as u8;
                self.set_nz_flags(self.r_ac);
            }

            Las => {
                self.r_sp = io.read_u8(t_addr) & self.r_sp;
                self.r_ac = self.r_sp;
                self.r_ix = self.r_sp;
                self.set_nz_flags(self.r_sp);
            }

            // LDA + LDX
            Lax => {
                self.r_ac = io.read_u8(t_addr);
                self.r_ix = self.r_ac;
                self.set_nz_flags(self.r_ac);
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
                self.r_ac &= t_byte1;
                self.set_nz_flags(self.r_ac);
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
                t_addr = (self.r_ac as u16)
                    + (t_byte1 as u16)
                    + (self.r_ps.contains(Flags::Carry) as u16);
                self.set_flag(Flags::Carry, t_addr > 0xFF);
                self.set_flag(
                    Flags::Overflow,
                    (!(self.r_ac ^ t_byte1) & (self.r_ac ^ (t_addr as u8))) & 0x80 != 0,
                );
                self.r_ac = t_addr as u8;
                self.set_nz_flags(self.r_ac);
            }

            // S(Accumulator & X register)
            Sax => {
                io.write_u8(t_addr, self.r_ac & self.r_ix);
            }

            Sbx => {
                self.r_ix &= self.r_ac;
                t_addr = (self.r_ix as u16).wrapping_sub(io.read_u8(t_addr) as u16);
                self.set_flag(Flags::Carry, t_addr < 0x100);
                self.r_ix = t_addr as u8;
                self.set_nz_flags(self.r_ix);
            }

            Sha => {
                t_byte1 = match am {
                    // TODO: Only read one bit
                    AbsoluteI(Y) => {
                        ((io.read_u16(self.r_pc.wrapping_sub(2)) >> 8) as u8).wrapping_add(1)
                    }
                    ZeroPageI(Y) => {
                        t_byte1 = io.read_u8(self.r_pc.wrapping_sub(1)).wrapping_add(1);
                        io.read_u8(t_byte1 as u16)
                    }
                    _ => {
                        self.i_hl = 1;
                        return;
                    }
                };

                io.write_u8(t_addr, self.r_ac & self.r_ix & t_byte1);
            }

            Shx | Shy => {
                // TODO: Only read one byte
                t_byte2 = io.read_u8(self.r_pc.wrapping_sub(1));
                t_byte1 =
                    t_byte2.wrapping_add(1) & (if mne == Shx { self.r_ix } else { self.r_iy });

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
                self.r_ac |= io.read_u8(t_addr);
                self.set_nz_flags(self.r_ac);
            }

            // LSR + EOR
            Sre => {
                // LSR code
                t_byte1 = io.read_u8(t_addr);
                self.set_flag(Flags::Carry, t_byte1 & 0x1 != 0);
                t_byte1 >>= 1;
                io.write_u8(t_addr, t_byte1);

                // EOR code
                self.r_ac ^= t_byte1;
                self.set_nz_flags(self.r_ac);
            }

            Tas => {
                // TODO: Only read one byte
                self.r_sp = self.r_ac & self.r_ix;
                // TODO: Remove extra var
                let ex = ((io.read_u16(self.r_pc.wrapping_sub(2)) >> 8) as u8).wrapping_add(1);
                io.write_u8(t_addr, self.r_sp & ex);
            }

            Jam => {
                self.i_hl = 1;
            }

            _ => todo!(),
        }

        let cycle_expr: u8 = 1 + base_timing + if timing > 0 { timing - 1 } else { 0 };
        self.i_cc += cycle_expr as usize;
    }
}

impl InspectCpu for VM {
    fn is_cycle_accurate(&self) -> bool {
        false
    }

    fn state(&self) -> CpuState {
        CpuState {
            pc: self.r_pc,
            sp: self.r_sp,
            ac: self.r_ac,
            ix: self.r_ix,
            iy: self.r_iy,
            am: effnes_cpu::addr::AddressingMode::Implied,
            ps: self.r_ps,
            cc: self.i_cc,
        }
    }
}
