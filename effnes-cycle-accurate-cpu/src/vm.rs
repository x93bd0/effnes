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
        self.next_state = match &self.next_state {
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
                            AddressingMode::ZeroPageI(_) => State::ResolveAddress(IndZPDummyRead),
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
                            self.address += (self.io.read_byte(self.operand as u16) as u16) << 8;
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
                            let low_nybble = (self.address as u8).wrapping_add(
                                if *index_register == IndexRegister::X {
                                    self.r_ix
                                } else {
                                    self.r_iy
                                },
                            );
                            if low_nybble < self.r_iy {
                                self.address = (self.address & 0xFF00) + low_nybble as u16;
                                self.io.read_byte(self.address);
                                State::ResolveAddress(AddIndexRegister {
                                    index_register: *index_register,
                                    bump_page: true,
                                })
                            } else {
                                self.address += self.r_iy as u16;
                                State::Process
                            }
                        }
                    }

                    ZeroPageAddIndexRegister => {
                        if let AddressingMode::ZeroPageI(ir) = self.addr_mode {
                            self.operand = self.operand.wrapping_add(if ir == IndexRegister::X {
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
        };
    }
}

impl<T: MemoryBus + Default> Default for VM<T> {
    fn default() -> Self {
        VM::new(T::default())
    }
}

// #[cfg(test)]
mod tests {
    use super::*;
    use effnes_bus::BasicMemory;

    const NOP_IMP: u8 = 0xEA;
    const LDA_IMM: u8 = 0xA9;
    const LDA_ZPG: u8 = 0xA5;
    const LDA_ZPX: u8 = 0xB5;
    const LDA_ABS: u8 = 0xAD;
    const LDA_ABX: u8 = 0xBD;
    const LDA_ABY: u8 = 0xB9;
    const LDA_INX: u8 = 0xA1;
    const LDA_INY: u8 = 0xB1;
    const LDX_ZPY: u8 = 0xB6;
    const JAM: u8 = 0x02;

    #[derive(Default)]
    struct Status {
        t: Option<u8>,
        pc: Option<u16>,
        ac: Option<u8>,
        ix: Option<u8>,
        iy: Option<u8>,
        sp: Option<u8>,
        am: Option<AddressingMode>,
        ab: Option<u16>,
        op: Option<u8>,
        st: Option<State>,
    }

    fn get_vm() -> VM<BasicMemory> {
        let mut vm = VM::new(BasicMemory::default_with(JAM));
        vm.r_pc = 0xF000;
        vm
    }

    macro_rules! setup_memory {
        (
            $vm:ident {
                $( $i:expr => $j:expr ),+
            }
            $([ $( $reg:ident => $val: expr ),* ])?
        ) => {
            $(
                $vm.io.write_byte($i, $j);
            )+

            $(
                $(
                    $vm.$reg = $val;
                )*
            )?
        };
    }

    macro_rules! for_each_vm_field {
        ($m:ident) => {
            $m!(t  => i_tm);
            $m!(pc => r_pc);
            $m!(ac => r_ac);
            $m!(ix => r_ix);
            $m!(iy => r_iy);
            $m!(sp => r_sp);
            $m!(am => addr_mode);
            $m!(ab => address);
            $m!(op => execute);
            $m!(st => next_state);
        };
    }

    macro_rules! assert_status {
        ($vm:ident, $st:ident) => {
            macro_rules! one {
                ($s:ident => $v:ident) => {
                    if let Some(p) = &$st.$s {
                        assert!(
                            $vm.$v == *p,
                            "assertion `VM[T{}].{} != {}` failed\n  left: {:?}\n right: {:?}",
                            $vm.i_tm,
                            stringify!($v),
                            stringify!($st.$s),
                            $vm.$v,
                            *p
                        );
                    }
                };
            }
            for_each_vm_field!(one);
        };
    }

    macro_rules! modify_state {
        ($st:ident {
            $($reg:ident => $data:expr),* $(,)?
        }) => {
            $(
                modify_state!($st, $reg, $data);
            )*
        };

        ($st:ident, $reg:ident, None) => {
            $st.$reg = None;
        };

        ($st:ident, $reg:ident, $data:expr) => {
            $st.$reg = Some($data);
        };
    }

    macro_rules! assert_execution_eq {
        ($vm:ident, $st:ident, {$( ($($item:tt)+) )+}) => {
            $(
                assert_execution_eq!($vm, $st, $($item)+);
            )+
        };

        ($vm:ident, $st:ident, cycle $bl:tt) => {
            modify_state!($st $bl);
            $vm.cycle();
        };

        ($vm:ident, $st:ident, =) => {
            assert_status!($vm, $st);
            if let Some(ref mut t) = $st.t {
                *t += 1;
            }
        };
    }

    fn assert_next_instr_is_nop(vm: &mut VM<BasicMemory>, st: &mut Status) {
        let pc = vm.r_pc.wrapping_add(1);
        assert_execution_eq!(vm, st, {
            (cycle {
                t => 0,
                pc => (pc + 1),
                op => NOP_IMP,
                am => (AddressingMode::Implied),
                st => (State::Process)
            }) (=)

            (cycle {
                st => State::FetchOpCode
            }) (=)
        });
    }

    #[test]
    fn test_imm_addressing() {
        for data in 0..=255 {
            let mut vm = get_vm();
            let mut st = Status::default();
            setup_memory!(vm {
                vm.r_pc.wrapping_add(1) => LDA_IMM,
                vm.r_pc.wrapping_add(2) => data,
                vm.r_pc.wrapping_add(3) => NOP_IMP
            });

            let pc = vm.r_pc.wrapping_add(1);
            assert_execution_eq!(vm, st, {
                (cycle {
                    t => 0,
                    pc => pc + 1,
                    ab => pc + 1,
                    op => LDA_IMM,
                    am => AddressingMode::Immediate,
                    st => State::Process
                }) (=)

                (cycle {
                    ac => data,
                    st => State::FetchOpCode
                }) (=)
            });

            assert_next_instr_is_nop(&mut vm, &mut st);
        }
    }

    #[test]
    fn test_zp_addressing() {
        for data in 0..=255 {
            let mut vm = get_vm();
            let mut st = Status::default();
            setup_memory!(vm {
                vm.r_pc.wrapping_add(1) => LDA_ZPG,
                    vm.r_pc.wrapping_add(2) => data,
                vm.r_pc.wrapping_add(3) => NOP_IMP,
                data as u16 => data ^ 0xFF
            });

            let pc = vm.r_pc.wrapping_add(1);
            assert_execution_eq!(vm, st, {
                (cycle {
                    t => 0,
                    pc => pc + 1,
                    op => LDA_ZPG,
                    am => AddressingMode::ZeroPage,
                    st => State::ResolveAddress(AddressResolverState::FetchOperand)
                }) (=)

                (cycle {
                    st => State::Process,
                    ab => data as u16,
                }) (=)

                (cycle {
                    st => State::FetchOpCode,
                    ac => data ^ 0xFF
                }) (=)
            });

            assert_next_instr_is_nop(&mut vm, &mut st);
        }
    }

    #[test]
    fn test_zpx_addressing() {
        let mut vm = get_vm();
        for zpaddr in 0..255 {
            for index in 0..255 {
                let mut st = Status::default();
                let pc = vm.r_pc.wrapping_add(1);
                let data = zpaddr;

                setup_memory!(vm {
                    pc => LDA_ZPX,
                    pc.wrapping_add(1) => zpaddr,
                    pc.wrapping_add(2) => NOP_IMP,
                    zpaddr.wrapping_add(index) as u16 => data
                } [r_ix => index]);

                use AddressResolverState::*;
                assert_execution_eq!(vm, st, {
                    (cycle {
                        t  => 0,
                        pc => pc.wrapping_add(1),
                        op => LDA_ZPX,
                        am => AddressingMode::ZeroPageI(IndexRegister::X),
                        st => State::ResolveAddress(FetchOperand),
                    }) (=)

                    (cycle {
                        st => State::ResolveAddress(IndZPDummyRead),
                        ab => zpaddr.into(),
                    }) (=)

                    (cycle {
                        st => State::ResolveAddress(ZeroPageAddIndexRegister),
                    }) (=)

                    (cycle {
                        st => State::Process,
                        ab => zpaddr
                            .wrapping_add(index) as u16,
                    }) (=)

                    (cycle {
                        st => State::FetchOpCode,
                        ac => data,
                    }) (=)
                });

                assert_next_instr_is_nop(&mut vm, &mut st);
                setup_memory!(vm {
                    pc => JAM,
                    pc.wrapping_add(1) => JAM,
                    pc.wrapping_add(2) => JAM,
                    zpaddr.wrapping_add(index) as u16 => JAM
                } [r_ix => 0, r_pc => 0xF000]);
            }
        }
    }

    #[test]
    fn test_zpy_addressing() {
        let mut vm = get_vm();
        for zpaddr in 0..255 {
            for index in 0..255 {
                let mut st = Status::default();
                let pc = vm.r_pc.wrapping_add(1);
                let data = zpaddr;

                setup_memory!(vm {
                    pc => LDX_ZPY,
                    pc.wrapping_add(1) => zpaddr,
                    pc.wrapping_add(2) => NOP_IMP,
                    zpaddr.wrapping_add(index) as u16 => data
                } [r_iy => index]);

                use AddressResolverState::*;
                assert_execution_eq!(vm, st, {
                    (cycle {
                        t => 0,
                        pc => pc + 1,
                        op => LDX_ZPY,
                        am => AddressingMode::ZeroPageI(IndexRegister::Y),
                        st => State::ResolveAddress(FetchOperand)
                    }) (=)

                    (cycle {
                        st => State::ResolveAddress(IndZPDummyRead),
                        ab => zpaddr.into()
                    }) (=)

                    (cycle {
                        st => State::ResolveAddress(ZeroPageAddIndexRegister)
                    }) (=)

                    (cycle {
                        st => State::Process,
                        ab => zpaddr
                            .wrapping_add(index) as u16
                    }) (=)

                    (cycle {
                        st => State::FetchOpCode,
                        ix => zpaddr
                    }) (=)
                });

                assert_next_instr_is_nop(&mut vm, &mut st);
                setup_memory!(vm {
                    pc => JAM,
                    pc.wrapping_add(1) => JAM,
                    pc.wrapping_add(2) => JAM,
                    zpaddr.wrapping_add(index) as u16 => JAM
                } [r_iy => 0, r_pc => 0xF000]);
            }
        }
    }

    const MAGIC: u8 = 0xF0;

    #[test]
    fn test_abs_addressing() {
        let mut vm = get_vm();
        for lpaddr in 0..=512 {
            let mut st = Status::default();

            let pc = vm.r_pc.wrapping_add(1);
            let opcode = LDA_ABS;
            let addr = (0xFF00_u16).wrapping_add(lpaddr);

            setup_memory!(vm {
                pc => opcode,
                pc.wrapping_add(1) => (addr & 0x00FF) as u8,
                pc.wrapping_add(2) => ((addr & 0xFF00) >> 8) as u8,
                pc.wrapping_add(3) => NOP_IMP,
                addr => lpaddr as u8
            });

            use AddressResolverState::*;
            assert_execution_eq!(vm, st, {
                (cycle {
                    t => 0,
                    pc => pc + 1,
                    op => opcode,
                    am => AddressingMode::Absolute,
                    st => State::ResolveAddress(FetchAddress { high_nybble: false })
                }) (=)

                (cycle {
                    pc => pc + 2,
                    st => State::ResolveAddress(FetchAddress { high_nybble: true }),
                    ab => addr & 0x00FF
                }) (=)

                (cycle {
                    st => State::Process,
                    ab => addr
                }) (=)

                (cycle {
                    st => State::FetchOpCode,
                    ac => lpaddr as u8
                }) (=)
            });

            assert_next_instr_is_nop(&mut vm, &mut st);
        }
    }
}
