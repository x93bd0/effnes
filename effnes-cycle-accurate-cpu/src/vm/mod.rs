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
    next_state: State,
    addr_mode: AddressingMode,

    operand: u8,
    execute: u8,
    address: u16,

    r_pc: u16,
    r_sp: u8,
    r_ac: u8,
    r_ix: u8,
    r_iy: u8,
    r_ps: Flags,
    i_tm: u8,

    io: T,
}

impl<T: MemoryBus> VM<T> {
    pub fn new(memory: T) -> Self {
        Self {
            next_state: State::FetchOpCode,
            addr_mode: AddressingMode::Implied,

            operand: 0,
            address: 0,
            execute: 0,

            r_pc: 0x8000,
            r_sp: 0,
            r_ac: 0,
            r_ix: 0,
            r_iy: 0,
            r_ps: Flags::empty(),
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

    pub fn cycle(&mut self) {
        self.i_tm += 1;
        self.next_state = 'new_state_match: {
            match &self.next_state {
                State::Halt => State::Halt,

                State::FetchOpCode => {
                    self.i_tm = 0;
                    self.r_pc = self.r_pc.wrapping_add(1);
                    self.execute = self.next_byte();
                    self.addr_mode = self.execute.into();

                    match self.addr_mode {
                        AddressingMode::Implied => State::Process,
                        AddressingMode::Immediate => {
                            self.address = self.r_pc;
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
                            self.operand = self.io.read_byte(self.r_pc);
                            self.address = self.operand as u16;
                            match &self.addr_mode {
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
                                self.address = self.next_byte() as u16;
                                State::ResolveAddress(FetchAddress { high_nybble: true })
                            } else {
                                self.address += (self.io.read_byte(self.r_pc) as u16) << 8;
                                match &self.addr_mode {
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
                            self.io.read_byte(self.operand as u16);
                            self.operand = self.operand.wrapping_add(self.r_ix);
                            State::ResolveAddress(FetchZeroPageAddress { high_nybble: false })
                        }

                        IndZPDummyRead => {
                            self.io.read_byte(self.address);
                            State::ResolveAddress(ZeroPageAddIndexRegister)
                        }

                        FetchZeroPageAddress { high_nybble } => {
                            if !high_nybble {
                                self.address = self.io.read_byte(self.operand as u16) as u16;
                                self.operand = self.operand.wrapping_add(1);
                                State::ResolveAddress(FetchZeroPageAddress { high_nybble: true })
                            } else {
                                self.address +=
                                    (self.io.read_byte(self.operand as u16) as u16) << 8;
                                match self.addr_mode {
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
                                self.address = self.address.wrapping_add(0x0100);
                                State::Process
                            } else {
                                let index = if *index_register == IndexRegister::X {
                                    self.r_ix
                                } else {
                                    self.r_iy
                                };
                                let low_nybble = (self.address as u8).wrapping_add(index);
                                if low_nybble < index {
                                    self.address = (self.address & 0xFF00) + low_nybble as u16;
                                    self.io.read_byte(self.address);
                                    State::ResolveAddress(AddIndexRegister {
                                        index_register: *index_register,
                                        bump_page: true,
                                    })
                                } else {
                                    self.address += index as u16;
                                    State::Process
                                }
                            }
                        }

                        ZeroPageAddIndexRegister => {
                            if let AddressingMode::ZeroPageI(ir) = self.addr_mode {
                                self.operand =
                                    self.operand.wrapping_add(if ir == IndexRegister::X {
                                        self.r_ix
                                    } else {
                                        self.r_iy
                                    });
                            } else {
                                unreachable!();
                            }

                            self.address = self.operand as u16;
                            State::Process
                        }
                    }
                }

                State::Process => {
                    // TODO: Don't read address on store operations
                    match self.addr_mode {
                        AddressingMode::Implied => (),
                        _ => self.operand = self.io.read_byte(self.address),
                    };

                    let mnemonic: Mnemonic = self.execute.into();

                    use Mnemonic::*;
                    match mnemonic {
                        Jam => {
                            break 'new_state_match State::Halt;
                        }
                        Lda => {
                            self.r_ac = self.operand;
                        }
                        Ldx => {
                            self.r_ix = self.operand;
                        }
                        Ldy => {
                            self.r_iy = self.operand;
                        }
                        Sta => {
                            self.io.write_byte(self.address, self.r_ac);
                        }
                        Stx => {
                            self.io.write_byte(self.address, self.r_ix);
                        }
                        Sty => {
                            self.io.write_byte(self.address, self.r_iy);
                        }
                        Nop => {}
                        _ => todo!(),
                    };

                    State::FetchOpCode
                }

                _ => unreachable!(),
            }
        };
    }
}

impl<T: MemoryBus + Default> Default for VM<T> {
    fn default() -> Self {
        VM::new(T::default())
    }
}

// #[cfg(test)]
mod tests;
