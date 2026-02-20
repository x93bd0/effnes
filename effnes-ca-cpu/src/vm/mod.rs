use effnes_bus::{InspectBus, MemoryBus};
use effnes_cpu::{
    addr::{AddressingMode, IndexRegister},
    consts::{CpuVector, Flags},
    inspect::{InspectCpu, State as CpuState},
    opcode::Mnemonic,
};

#[derive(Debug, PartialEq)]
enum AddressResolverState {
    FetchOperand,
    FetchAddress {
        high_byte: bool,
    },
    FetchZeroPageAddress {
        high_byte: bool,
    },
    IndXDummyRead,
    IndZPDummyRead,
    AddIndexRegister {
        index_register: IndexRegister,
        bump_page: bool,
    },
    ZeroPageAddIndexRegister,
}

#[derive(Debug, PartialEq)]
enum State {
    Fetch,
    ResolveAddress(AddressResolverState),
    Process,
    Write { dummy: bool, data: u8 },
    Halt,
}

pub struct VM<T: MemoryBus> {
    /// (Register) Program Counter
    r_pc: u16,

    /// (Register) Stack Pointer
    r_sp: u8,
    /// (Register) ACcumulator
    r_ac: u8,
    /// (Register) Index X
    r_ix: u8,
    /// (Register) Index Y
    r_iy: u8,
    /// (Register) Program Status
    r_ps: Flags,

    /// (Internal) Next STate:
    /// Stores the next state that the state machine will execute.
    i_nst: State,

    /// (Internal) ADdressing Mode
    /// Stores the addressing mode that the current opcode will use.
    /// It is set on the [State::Fetch] state.
    i_adm: AddressingMode,

    /// (Internal) OPeRand
    /// Stores the operand of any instruction of the form (OpCode)(Operand)
    /// (as seen in the following addressing modes [AddressingMode::ZeroPage],
    /// [AddressingMode::ZeroPageI], [AddressingMode::IndirectI]).
    i_opr: u8,

    /// (Internal) EXecute
    /// Stores the opcode being executed.
    /// It is set on the [State::Fetch] state.
    i_ex: u8,

    /// (Internal) Address Bus
    /// Stores an address that is read in the [State::Process] state on every
    /// [AddressingMode] except [AddressingMode::Implied].
    i_ab: u16,

    /// (Internal) TiMing
    /// Stores the executed cycle number of the current instruction.
    /// It is set to T0 on every [State::Fetch] state.
    i_tm: u8,

    /// Input and Output bus
    /// Used for communication between the emulated CPU and its peripherals
    /// (RAM, PPU, APU, etc.).
    pub io: T,
}

macro_rules! update_register {
    ($vm:ident.$reg:ident = $value:expr) => {
        $vm.$reg = $value;
        $vm.set_nz_flags(($vm.$reg).into());
    };
}

impl<T: MemoryBus> VM<T> {
    pub fn new(memory: T) -> Self {
        Self {
            r_pc: 0x8000,
            r_sp: 0,
            r_ac: 0,
            r_ix: 0,
            r_iy: 0,
            r_ps: Flags::empty(),

            i_nst: State::Fetch,
            i_adm: AddressingMode::Implied,
            i_opr: 0,
            i_ex: 0,
            i_ab: 0,
            i_tm: 0,

            io: memory,
        }
    }

    fn next_byte(&mut self) -> u8 {
        let out: u8 = self.io.read_u8(self.r_pc);
        self.r_pc = self.r_pc.wrapping_add(1);
        out
    }

    fn stack_push_byte(&mut self, value: u8) {
        self.io.write_u8((self.r_sp as u16) | 0x100, value);
        self.r_sp = self.r_sp.wrapping_sub(1);
    }

    fn stack_pop_byte(&mut self) -> u8 {
        self.r_sp = self.r_sp.wrapping_add(1);
        self.io.read_u8((self.r_sp as u16) | 0x100)
    }

    fn set_flag(&mut self, flag: Flags, value: bool) {
        if value {
            self.r_ps |= flag;
        } else {
            self.r_ps = self.r_ps.difference(flag);
        }
    }

    fn set_nz_flags(&mut self, value: u8) {
        self.set_flag(Flags::Negative, value & 0x80 > 0);
        self.set_flag(Flags::Zero, value == 0);
    }

    pub fn cold_reset(&mut self) {
        self.r_ps = Flags::empty();
        self.r_ac = 0;
        self.r_ix = 0;
        self.r_iy = 0;

        self.r_sp = 0x00;
        self.r_pc = CpuVector::Rst as u16;

        self.warm_reset();
    }

    pub fn warm_reset(&mut self) {
        self.r_ps |= Flags::IntDis;
        self.r_sp = self.r_sp.wrapping_sub(0x03);

        self.i_nst = State::Fetch;
        self.i_adm = AddressingMode::Implied;
        self.i_opr = 0;
        self.i_ex = 0;
        self.i_ab = 0;
        self.i_tm = 0;
    }

    pub fn cycle(&mut self) {
        self.i_tm += 1;
        self.i_nst = 'new_state_match: {
            match &self.i_nst {
                State::Halt => State::Halt,

                State::Fetch => {
                    self.i_tm = 0;
                    self.r_pc = self.r_pc.wrapping_add(1);
                    self.i_ex = self.next_byte();
                    self.i_adm = self.i_ex.into();

                    match self.i_adm {
                        AddressingMode::Implied => State::Process,
                        AddressingMode::Immediate => {
                            self.i_ab = self.r_pc;
                            State::Process
                        }
                        AddressingMode::ZeroPage
                        | AddressingMode::ZeroPageI(_)
                        | AddressingMode::IndirectI(_) => {
                            State::ResolveAddress(AddressResolverState::FetchOperand)
                        }
                        _ => State::ResolveAddress(AddressResolverState::FetchAddress {
                            high_byte: false,
                        }),
                    }
                }

                State::ResolveAddress(ads) => {
                    use AddressResolverState::*;

                    match ads {
                        FetchOperand => {
                            self.i_opr = self.io.read_u8(self.r_pc);
                            self.i_ab = self.i_opr as u16;
                            match &self.i_adm {
                                AddressingMode::ZeroPage => State::Process,
                                AddressingMode::ZeroPageI(_) => {
                                    State::ResolveAddress(IndZPDummyRead)
                                }
                                AddressingMode::IndirectI(ir) => {
                                    State::ResolveAddress(if *ir == IndexRegister::X {
                                        IndXDummyRead
                                    } else {
                                        FetchZeroPageAddress { high_byte: false }
                                    })
                                }
                                _ => unreachable!(),
                            }
                        }

                        FetchAddress { high_byte } => {
                            if !high_byte {
                                self.i_ab = self.next_byte() as u16;
                                State::ResolveAddress(FetchAddress { high_byte: true })
                            } else {
                                self.i_ab += (self.io.read_u8(self.r_pc) as u16) << 8;
                                match &self.i_adm {
                                    AddressingMode::AbsoluteI(ir) => {
                                        State::ResolveAddress(AddIndexRegister {
                                            index_register: *ir,
                                            bump_page: false,
                                        })
                                    }
                                    _ => State::Process,
                                }
                            }
                        }

                        IndXDummyRead => {
                            self.io.read_u8(self.i_opr as u16);
                            self.i_opr = self.i_opr.wrapping_add(self.r_ix);
                            State::ResolveAddress(FetchZeroPageAddress { high_byte: false })
                        }

                        IndZPDummyRead => {
                            self.io.read_u8(self.i_ab);
                            State::ResolveAddress(ZeroPageAddIndexRegister)
                        }

                        FetchZeroPageAddress { high_byte } => {
                            if !high_byte {
                                self.i_ab = self.io.read_u8(self.i_opr as u16) as u16;
                                self.i_opr = self.i_opr.wrapping_add(1);
                                State::ResolveAddress(FetchZeroPageAddress { high_byte: true })
                            } else {
                                self.i_ab += (self.io.read_u8(self.i_opr as u16) as u16) << 8;
                                match self.i_adm {
                                    AddressingMode::IndirectI(IndexRegister::Y) => {
                                        State::ResolveAddress(AddIndexRegister {
                                            index_register: IndexRegister::Y,
                                            bump_page: false,
                                        })
                                    }
                                    _ => State::Process,
                                }
                            }
                        }

                        AddIndexRegister {
                            index_register,
                            bump_page,
                        } => {
                            if *bump_page {
                                self.i_ab = self.i_ab.wrapping_add(0x0100);
                                State::Process
                            } else {
                                let index = if *index_register == IndexRegister::X {
                                    self.r_ix
                                } else {
                                    self.r_iy
                                };
                                let low_byte = (self.i_ab as u8).wrapping_add(index);
                                if low_byte < index {
                                    self.i_ab = (self.i_ab & 0xFF00) + low_byte as u16;
                                    self.io.read_u8(self.i_ab);
                                    State::ResolveAddress(AddIndexRegister {
                                        index_register: *index_register,
                                        bump_page: true,
                                    })
                                } else {
                                    self.i_ab += index as u16;
                                    State::Process
                                }
                            }
                        }

                        ZeroPageAddIndexRegister => {
                            if let AddressingMode::ZeroPageI(ir) = self.i_adm {
                                self.i_opr = self.i_opr.wrapping_add(if ir == IndexRegister::X {
                                    self.r_ix
                                } else {
                                    self.r_iy
                                });
                            } else {
                                unreachable!();
                            }

                            self.i_ab = self.i_opr as u16;
                            State::Process
                        }
                    }
                }

                State::Process => {
                    // TODO: Don't read address on store operations
                    match self.i_adm {
                        AddressingMode::Implied => (),
                        _ => self.i_opr = self.io.read_u8(self.i_ab),
                    };

                    let mnemonic: Mnemonic = self.i_ex.into();

                    use Mnemonic::*;
                    match mnemonic {
                        Jam => {
                            break 'new_state_match State::Halt;
                        }

                        // Single byte instructions
                        Clx { flag } => {
                            self.r_ps = self.r_ps.difference(flag);
                        }

                        Sfx { flag } => {
                            self.r_ps = self.r_ps.union(flag);
                        }

                        Inx => {
                            update_register!(self.r_ix = self.r_ix.wrapping_add(1));
                        }
                        Iny => {
                            update_register!(self.r_ix = self.r_ix.wrapping_add(1));
                        }
                        Dex => {
                            update_register!(self.r_ix = self.r_ix.wrapping_sub(1));
                        }
                        Dey => {
                            update_register!(self.r_iy = self.r_iy.wrapping_sub(1));
                        }

                        Tax => {
                            update_register!(self.r_ix = self.r_ac);
                        }
                        Txa => {
                            update_register!(self.r_ac = self.r_ix);
                        }
                        Tay => {
                            update_register!(self.r_iy = self.r_ac);
                        }
                        Tya => {
                            update_register!(self.r_ac = self.r_iy);
                        }
                        Tsx => {
                            update_register!(self.r_ix = self.r_sp);
                        }
                        Txs => {
                            update_register!(self.r_sp = self.r_ix);
                        }

                        Nop => {}

                        // Internal execution on memory data
                        Adc => {
                            let value: u16 = (self.r_ac as u16)
                                + (self.i_opr as u16)
                                + (self.r_ps.contains(Flags::Carry) as u16);
                            self.set_flag(Flags::Carry, value > 0xFF);
                            self.set_flag(
                                Flags::Overflow,
                                (self.r_ac & 0x80) == (self.i_opr & 0x80)
                                    && (self.r_ac & 0x80) != (value as u8 & 0x80),
                            );
                            update_register!(self.r_ac = value as u8);
                        }
                        Sbc => {
                            let value: u16 = ((self.r_ac as u16) + (!self.i_opr as u16))
                                .wrapping_sub(self.r_ps.contains(Flags::Carry) as u16);
                            self.set_flag(Flags::Carry, value > 0xFF);
                            self.set_flag(
                                Flags::Overflow,
                                (self.r_ac & 0x80) == (self.i_opr & 0x80)
                                    && (self.r_ac & 0x80) != (value as u8 & 0x80),
                            );
                            update_register!(self.r_ac = value as u8);
                        }

                        And => {
                            update_register!(self.r_ac = self.r_ac & self.i_opr);
                        }

                        Bit => {
                            self.set_flag(Flags::Zero, self.r_ac & self.i_opr != 0);
                            self.set_flag(
                                Flags::Negative,
                                self.i_opr & Flags::Negative.bits() != 0,
                            );
                            self.set_flag(
                                Flags::Overflow,
                                self.i_opr & Flags::Overflow.bits() != 0,
                            );
                        }

                        Cmp => {
                            let value: u16 = (self.r_ac as u16) + (!self.i_opr as u16);
                            self.set_flag(Flags::Carry, value > 0xFF);
                            self.set_flag(
                                Flags::Overflow,
                                (self.r_ac & 0x80) == (self.i_opr & 0x80)
                                    && (self.r_ac & 0x80) != (value as u8 & 0x80),
                            );
                        }
                        Cpx => {
                            let value: u16 = (self.r_ix as u16) + (!self.i_opr as u16);
                            self.set_flag(Flags::Carry, value > 0xFF);
                            self.set_flag(
                                Flags::Overflow,
                                (self.r_ix & 0x80) == (self.i_opr & 0x80)
                                    && (self.r_ix & 0x80) != (value as u8 & 0x80),
                            );
                        }
                        Cpy => {
                            let value: u16 = (self.r_iy as u16) + (!self.i_opr as u16);
                            self.set_flag(Flags::Carry, value > 0xFF);
                            self.set_flag(
                                Flags::Overflow,
                                (self.r_iy & 0x80) == (self.i_opr & 0x80)
                                    && (self.r_iy & 0x80) != (value as u8 & 0x80),
                            );
                        }

                        Eor => {
                            update_register!(self.r_ac = self.r_ac ^ self.i_opr);
                        }

                        Lda => {
                            update_register!(self.r_ac = self.i_opr);
                        }
                        Ldx => {
                            update_register!(self.r_ix = self.i_opr);
                        }
                        Ldy => {
                            update_register!(self.r_iy = self.i_opr);
                        }

                        Ora => {
                            update_register!(self.r_ac = self.r_ac | self.i_opr);
                        }

                        // Store operations
                        Sta => {
                            self.io.write_u8(self.i_ab, self.r_ac);
                        }
                        Stx => {
                            self.io.write_u8(self.i_ab, self.r_ix);
                        }
                        Sty => {
                            self.io.write_u8(self.i_ab, self.r_iy);
                        }

                        // Read-Modify-Write operations
                        Asl => {
                            todo!();
                        }

                        Dec => {
                            todo!();
                        }
                        Inc => {
                            todo!();
                        }

                        Lsr => {
                            todo!();
                        }

                        Rol => {
                            todo!();
                        }
                        Ror => {
                            todo!();
                        }

                        // Miscellaneous operations
                        Bxx { flag, set } => {
                            if self.r_ps.contains(flag) == set {
                                // TODO: correct cycle count
                                self.r_pc = self.r_pc.wrapping_add(self.i_opr as u16);
                            }
                        }

                        Brk => {
                            todo!();
                        }
                        Jmp => {
                            todo!();
                        }
                        Jsr => {
                            todo!();
                        }

                        Pha => {
                            todo!();
                        }
                        Php => {
                            todo!();
                        }

                        Pla => {
                            todo!();
                        }
                        Plp => {
                            todo!();
                        }

                        Rti => {
                            todo!();
                        }
                        Rts => {
                            todo!();
                        }

                        // Illegal operations
                        Anc => {
                            todo!();
                        }

                        Ane => {
                            todo!();
                        }

                        Arr => {
                            todo!();
                        }

                        Asr => {
                            todo!();
                        }

                        Dcp => {
                            todo!();
                        }

                        Isc => {
                            todo!();
                        }

                        Las => {
                            todo!();
                        }

                        Lax => {
                            todo!();
                        }

                        Lxa => {
                            todo!();
                        }

                        Rra => {
                            todo!();
                        }

                        Rla => {
                            todo!();
                        }

                        Sax => {
                            todo!();
                        }

                        Sbx => {
                            todo!();
                        }

                        Sha => {
                            todo!();
                        }

                        Shx => {
                            todo!();
                        }

                        Shy => {
                            todo!();
                        }

                        Slo => {
                            todo!();
                        }

                        Sre => {
                            todo!();
                        }

                        Tas => {
                            todo!();
                        }
                    };

                    State::Fetch
                }

                State::Write { dummy: true, data } => State::Write {
                    dummy: false,
                    data: *data,
                },

                State::Write { dummy: false, data } => {
                    self.io.write_u8(self.i_ab, *data);
                    State::Fetch
                }
            }
        };
    }
}

impl<T: MemoryBus + InspectBus> InspectCpu for VM<T> {
    const CYCLE_ACCURATE: bool = true;
    fn state(&self) -> CpuState {
        CpuState {
            pc: self.r_pc,
            sp: self.r_sp,
            ac: self.r_ac,
            ix: self.r_ix,
            iy: self.r_iy,
            am: self.i_adm,
            ps: self.r_ps,
        }
    }
}

impl<T: MemoryBus + Default> Default for VM<T> {
    fn default() -> Self {
        VM::new(T::default())
    }
}

#[cfg(test)]
mod tests;
