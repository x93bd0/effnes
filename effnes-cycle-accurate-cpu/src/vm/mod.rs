use crate::{
    addr::{AddressingMode, IndexRegister},
    consts::Flags,
    opcode::Mnemonic,
};
use effnes_bus::MemoryBus;

#[derive(Debug, PartialEq)]
enum AddressResolverState {
    FetchOperand,
    FetchAddress {
        high_nybble: bool,
    },
    FetchZeroPageAddress {
        high_nybble: bool,
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
    FetchOpCode,
    ResolveAddress(AddressResolverState),
    Process,
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
    /// It is set on the [State::FetchOpcode] state.
    i_adm: AddressingMode,

    /// (Internal) OPeRand
    /// Stores the operand of any instruction of the form (OpCode)(Operand)
    /// (as seen in the following addressing modes [AddressingMode::ZeroPage],
    /// [AddressingMode::ZeroPageI], [AddressingMode::IndirectI]).
    i_opr: u8,

    /// (Internal) EXecute
    /// Stores the opcode being executed.
    /// It is set on the [State::FetchOpcode] state.
    i_ex: u8,

    /// (Internal) Address Bus
    /// Stores an address that is read in the [State::Process] state on every
    /// [AddressingMode] except [AddressingMode::Implied].
    i_ab: u16,

    /// (Internal) TiMing
    /// Stores the executed cycle number of the current instruction.
    /// It is set to T0 on every [State::FetchOpcode] state.
    i_tm: u8,

    /// Input and Output bus
    /// Used for communication between the emulated CPU and its peripherals
    /// (RAM, PPU, APU, etc.).
    pub io: T,
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

            i_nst: State::FetchOpCode,
            i_adm: AddressingMode::Implied,
            i_opr: 0,
            i_ex: 0,
            i_ab: 0,
            i_tm: 0,

            io: memory,
        }
    }

    fn next_byte(&mut self) -> u8 {
        let out: u8 = self.io.read_byte(self.r_pc);
        self.r_pc = self.r_pc.wrapping_add(1);
        out
    }

    fn stack_push_byte(&mut self, value: u8) {
        self.io.write_byte((self.r_sp as u16) | 0x100, value);
        self.r_sp = self.r_sp.wrapping_sub(1);
    }

    fn stack_pop_byte(&mut self) -> u8 {
        self.r_sp = self.r_sp.wrapping_add(1);
        self.io.read_byte((self.r_sp as u16) | 0x100)
    }

    pub fn cold_reset(&mut self) {
        self.r_ps = Flags::Reserved | Flags::Break | Flags::IntDis | Flags::Zero;
        self.r_sp = 0xFF;
        self.r_ac = 0;
        self.r_ix = 0;
        self.r_iy = 0;

        self.i_nst = State::FetchOpCode;
        self.i_adm = AddressingMode::Implied;
        self.i_opr = 0;
        self.i_ex = 0;
        self.i_ab = 0;
        self.i_tm = 0;
    }

    pub fn warm_reset(&mut self) {}

    pub fn cycle(&mut self) {
        self.i_tm += 1;
        self.i_nst = 'new_state_match: {
            match &self.i_nst {
                State::Halt => State::Halt,

                State::FetchOpCode => {
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
                            high_nybble: false,
                        }),
                    }
                }

                State::ResolveAddress(ads) => {
                    use AddressResolverState::*;

                    match ads {
                        FetchOperand => {
                            self.i_opr = self.io.read_byte(self.r_pc);
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
                                        FetchZeroPageAddress { high_nybble: false }
                                    })
                                }
                                _ => unreachable!(),
                            }
                        }

                        FetchAddress { high_nybble } => {
                            if !high_nybble {
                                self.i_ab = self.next_byte() as u16;
                                State::ResolveAddress(FetchAddress { high_nybble: true })
                            } else {
                                self.i_ab += (self.io.read_byte(self.r_pc) as u16) << 8;
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
                            self.io.read_byte(self.i_opr as u16);
                            self.i_opr = self.i_opr.wrapping_add(self.r_ix);
                            State::ResolveAddress(FetchZeroPageAddress { high_nybble: false })
                        }

                        IndZPDummyRead => {
                            self.io.read_byte(self.i_ab);
                            State::ResolveAddress(ZeroPageAddIndexRegister)
                        }

                        FetchZeroPageAddress { high_nybble } => {
                            if !high_nybble {
                                self.i_ab = self.io.read_byte(self.i_opr as u16) as u16;
                                self.i_opr = self.i_opr.wrapping_add(1);
                                State::ResolveAddress(FetchZeroPageAddress { high_nybble: true })
                            } else {
                                self.i_ab += (self.io.read_byte(self.i_opr as u16) as u16) << 8;
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
                                let low_nybble = (self.i_ab as u8).wrapping_add(index);
                                if low_nybble < index {
                                    self.i_ab = (self.i_ab & 0xFF00) + low_nybble as u16;
                                    self.io.read_byte(self.i_ab);
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
                        _ => self.i_opr = self.io.read_byte(self.i_ab),
                    };

                    let mnemonic: Mnemonic = self.i_ex.into();

                    use Mnemonic::*;
                    match mnemonic {
                        Jam => {
                            break 'new_state_match State::Halt;
                        }
                        Lda => {
                            self.r_ac = self.i_opr;
                        }
                        Ldx => {
                            self.r_ix = self.i_opr;
                        }
                        Ldy => {
                            self.r_iy = self.i_opr;
                        }
                        Sta => {
                            self.io.write_byte(self.i_ab, self.r_ac);
                        }
                        Stx => {
                            self.io.write_byte(self.i_ab, self.r_ix);
                        }
                        Sty => {
                            self.io.write_byte(self.i_ab, self.r_iy);
                        }
                        Nop => {}
                        _ => todo!(),
                    };

                    State::FetchOpCode
                }
            }
        };
    }
}

impl<T: MemoryBus + Default> Default for VM<T> {
    fn default() -> Self {
        VM::new(T::default())
    }
}

#[cfg(test)]
mod tests;
