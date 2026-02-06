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
    const MAGIC: u8 = 0xFA;

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

    fn get_status() -> Status {
        Status {
            t: None,
            pc: None,
            ac: None,
            ix: None,
            iy: None,
            sp: None,
            am: None,
            ab: None,
            op: None,
            st: None,
        }
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

    macro_rules! _assert_vm_property_eq {
        ($vm:ident, $prop:ident, $expr:expr) => {
            assert!(
                $vm.$prop == $expr,
                "assertion `VM[T{}].{} != {}` failed\n  left: {:?}\n right: {:?}",
                $vm.i_tm,
                stringify!($prop),
                stringify!($expr),
                $vm.$prop,
                $expr
            );
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
                        _assert_vm_property_eq!($vm, $v, *p);
                    }
                };
            }
            for_each_vm_field!(one);
        };
    }

    macro_rules! assert_branches {
        ($vm:ident, $st:ident, set<$reg:ident>($expr:expr)) => {
            $st.$reg = Some($expr);
        };

        ($vm:ident, $st:ident, unset<$reg:ident>()) => {
            $st.$reg = None;
        };

        ($vm:ident, $st:ident, +) => {
            $vm.cycle();
        };

        ($vm:ident, $st:ident, =) => {
            assert_status!($vm, $st);
            if let Some(ref mut t) = $st.t {
                *t += 1;
            }
        };
    }

    macro_rules! assert_execution_eq {
        ($vm:ident, $st:ident, {$( ($($item:tt)+) )+}) => {
            $(
                assert_branches!($vm, $st, $($item)+);
            )+
        }
    }

    fn assert_next_instr_is_nop(mut vm: VM<BasicMemory>, mut st: Status) {
        let pc = vm.r_pc.wrapping_add(1);
        assert_execution_eq!(vm, st, {
            (set<t>(0))
            (set<pc>(pc + 1))
            (set<op>(NOP_IMP))
            (set<am>(AddressingMode::Implied))
            (set<st>(State::Process))
            (+) (=)
            (set<st>(State::FetchOpCode))
            (+) (=)
        });
    }

    #[test]
    fn test_imm_addressing() {
        let mut vm = get_vm();
        let mut st = get_status();
        setup_memory!(vm {
            vm.r_pc.wrapping_add(1) => LDA_IMM,
            vm.r_pc.wrapping_add(2) => MAGIC,
            vm.r_pc.wrapping_add(3) => NOP_IMP
        });

        let pc = vm.r_pc.wrapping_add(1);
        assert_execution_eq!(vm, st, {
            (set<t>(0))
            (set<pc>(pc + 1))
            (set<ab>(pc + 1))
            (set<op>(LDA_IMM))
            (set<am>(AddressingMode::Immediate))
            (set<st>(State::Process))
            (+) (=)
            (set<t>(1))
            (set<ac>(MAGIC))
            (set<st>(State::FetchOpCode))
            (+) (=)
        });

        assert_next_instr_is_nop(vm, st);
    }

    #[test]
    fn test_zp_addressing() {
        let mut vm = get_vm();
        let mut st = get_status();
        setup_memory!(vm {
            vm.r_pc.wrapping_add(1) => LDA_ZPG,
            vm.r_pc.wrapping_add(2) => MAGIC,
            vm.r_pc.wrapping_add(3) => NOP_IMP,
            MAGIC as u16 => MAGIC ^ 0xFF
        });

        let pc = vm.r_pc.wrapping_add(1);
        assert_execution_eq!(vm, st, {
            (set<t>(0))
            (set<pc>(pc + 1))
            (set<op>(LDA_ZPG))
            (set<am>(AddressingMode::ZeroPage))
            (set<st>(State::ResolveAddress(AddressResolverState::FetchOperand)))
            (+) (=)
            (set<st>(State::Process))
            (set<ab>(MAGIC as u16))
            (+) (=)
            (set<st>(State::FetchOpCode))
            (set<ac>(MAGIC ^ 0xFF))
            (+) (=)
        });

        assert_next_instr_is_nop(vm, st);
    }

    #[test]
    fn test_zpx_addressing() {
        let mut vm = get_vm();
        let mut st = get_status();
        let pc = vm.r_pc.wrapping_add(1);
        setup_memory!(vm {
            pc => LDA_ZPX,
            pc.wrapping_add(1) => MAGIC ^ 0xFF,
            pc.wrapping_add(2) => NOP_IMP,
            (MAGIC ^ 0xFF).wrapping_add(MAGIC) as u16 => MAGIC ^ 0xFF
        } [r_ix => MAGIC]);

        assert_execution_eq!(vm, st, {
            (set<t>(0))
            (set<pc>(pc + 1))
            (set<op>(LDA_ZPX))
            (set<am>(AddressingMode::ZeroPageI(IndexRegister::X)))
            (set<st>(State::ResolveAddress(AddressResolverState::FetchOperand)))
            (+) (=)
            (set<st>(State::ResolveAddress(AddressResolverState::IndZPDummyRead)))
            (set<ab>((MAGIC ^ 0xFF) as u16))
            (+) (=)
            (set<st>(State::ResolveAddress(AddressResolverState::ZeroPageAddIndexRegister)))
            (+) (=)
            (set<st>(State::Process))
            (set<ab>((MAGIC ^ 0xFF).wrapping_add(MAGIC) as u16))
            (+) (=)
            (set<st>(State::FetchOpCode))
            (set<ac>(MAGIC ^ 0xFF))
            (+) (=)
        });

        assert_next_instr_is_nop(vm, st);
    }

    #[test]
    fn test_zpy_addressing() {
        let mut vm = get_vm();
        let mut st = get_status();
        let pc = vm.r_pc.wrapping_add(1);
        setup_memory!(vm {
            pc => LDX_ZPY,
            pc.wrapping_add(1) => MAGIC ^ 0xFF,
            pc.wrapping_add(2) => NOP_IMP,
            (MAGIC ^ 0xFF).wrapping_add(MAGIC) as u16 => MAGIC ^ 0xFF
        } [r_iy => MAGIC]);

        assert_execution_eq!(vm, st, {
            (set<t>(0))
            (set<pc>(pc + 1))
            (set<op>(LDX_ZPY))
            (set<am>(AddressingMode::ZeroPageI(IndexRegister::Y)))
            (set<st>(State::ResolveAddress(AddressResolverState::FetchOperand)))
            (+) (=)
            (set<st>(State::ResolveAddress(AddressResolverState::IndZPDummyRead)))
            (set<ab>((MAGIC ^ 0xFF) as u16))
            (+) (=)
            (set<st>(State::ResolveAddress(AddressResolverState::ZeroPageAddIndexRegister)))
            (+) (=)
            (set<st>(State::Process))
            (set<ab>((MAGIC ^ 0xFF).wrapping_add(MAGIC) as u16))
            (+) (=)
            (set<st>(State::FetchOpCode))
            (set<ix>(MAGIC ^ 0xFF))
            (+) (=)
        });

        assert_next_instr_is_nop(vm, st);
    }

    #[test]
    fn test_abs_addressing() {
        let mut vm = get_vm();
        let mut st = get_status();
        let pc = vm.r_pc.wrapping_add(1);

        let addr = (((MAGIC ^ 0xFF) as u16) << 8) + (MAGIC as u16);
        setup_memory!(vm {
            pc => LDA_ABS,
            pc.wrapping_add(1) => (addr & 0x00FF) as u8,
            pc.wrapping_add(2) => ((addr & 0xFF00) >> 8) as u8,
            pc.wrapping_add(3) => NOP_IMP,
            addr => MAGIC ^ 0xF0
        });

        assert_execution_eq!(vm, st, {
            (set<t>(0))
            (set<pc>(pc + 1))
            (set<op>(LDA_ABS))
            (set<am>(AddressingMode::Absolute))
            (set<st>(State::ResolveAddress(AddressResolverState::FetchAddress { high_nybble: false })))
            (+) (=)
            (set<pc>(pc + 2))
            (set<st>(State::ResolveAddress(AddressResolverState::FetchAddress { high_nybble: true })))
            (set<ab>(addr & 0x00FF))
            (+) (=)
            (set<st>(State::Process))
            (set<ab>(addr))
            (+) (=)
            (set<st>(State::FetchOpCode))
            (set<ac>(MAGIC ^ 0xF0))
            (+) (=)
        });

        assert_next_instr_is_nop(vm, st);
    }
}
