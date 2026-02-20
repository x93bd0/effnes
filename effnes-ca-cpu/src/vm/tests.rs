use super::*;
use AddressResolverState::*;
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
    nst: Option<State>,
}

fn get_vm() -> VM<BasicMemory> {
    let mut vm = VM::new(BasicMemory::default_with(JAM));
    vm.r_pc = 0xF000;
    vm
}

macro_rules! setup_memory {
        (
            $vm:ident {
                $( $i:expr => $j:expr ),*
            }
            $([ $( $reg:ident => $val: expr ),* ])?
        ) => {
            $(
                $vm.io.write_byte($i, $j);
            )*

            $(
                $(
                    $vm.$reg = $val;
                )*
            )?
        };
    }

macro_rules! for_each_vm_field {
        ($m:ident) => {
            $m!(nst => i_nst);
            $m!(op => i_ex);
            $m!(am => i_adm);
            $m!(ab => i_ab);
            $m!(t  => i_tm);
            $m!(pc => r_pc);
            $m!(ac => r_ac);
            $m!(ix => r_ix);
            $m!(iy => r_iy);
            $m!(sp => r_sp);
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
            nst => (State::Process)
        }) (=)

        (cycle {
            nst => State::Fetch
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
                nst => State::Process
            }) (=)

            (cycle {
                ac => data,
                nst => State::Fetch
            }) (=)
        });

        assert_next_instr_is_nop(&mut vm, &mut st);
        setup_memory!(vm {
                vm.r_pc.wrapping_add(1) => JAM,
                vm.r_pc.wrapping_add(2) => JAM,
                vm.r_pc.wrapping_add(3) => JAM
            } [r_pc => 0xF000]);
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
                nst => State::ResolveAddress(FetchOperand)
            }) (=)

            (cycle {
                nst => State::Process,
                ab => data as u16,
            }) (=)

            (cycle {
                nst => State::Fetch,
                ac => data ^ 0xFF
            }) (=)
        });

        assert_next_instr_is_nop(&mut vm, &mut st);
        setup_memory!(vm {
                vm.r_pc.wrapping_add(1) => JAM,
                vm.r_pc.wrapping_add(2) => JAM,
                vm.r_pc.wrapping_add(3) => JAM,
                data as u16 => JAM
            } [r_pc => 0xF000]);
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

            assert_execution_eq!(vm, st, {
                (cycle {
                    t  => 0,
                    pc => pc.wrapping_add(1),
                    op => LDA_ZPX,
                    am => AddressingMode::ZeroPageI(IndexRegister::X),
                    nst => State::ResolveAddress(FetchOperand),
                }) (=)

                (cycle {
                    nst => State::ResolveAddress(IndZPDummyRead),
                    ab => zpaddr.into(),
                }) (=)

                (cycle {
                    nst => State::ResolveAddress(ZeroPageAddIndexRegister),
                }) (=)

                (cycle {
                    nst => State::Process,
                    ab => zpaddr
                        .wrapping_add(index) as u16,
                }) (=)

                (cycle {
                    nst => State::Fetch,
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

            assert_execution_eq!(vm, st, {
                (cycle {
                    t => 0,
                    pc => pc + 1,
                    op => LDX_ZPY,
                    am => AddressingMode::ZeroPageI(IndexRegister::Y),
                    nst => State::ResolveAddress(FetchOperand)
                }) (=)

                (cycle {
                    nst => State::ResolveAddress(IndZPDummyRead),
                    ab => zpaddr.into()
                }) (=)

                (cycle {
                    nst => State::ResolveAddress(ZeroPageAddIndexRegister)
                }) (=)

                (cycle {
                    nst => State::Process,
                    ab => zpaddr
                        .wrapping_add(index) as u16
                }) (=)

                (cycle {
                    nst => State::Fetch,
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
    for low_byte in 0..512 {
        let mut st = Status::default();

        let pc = vm.r_pc.wrapping_add(1);
        let opcode = LDA_ABS;
        let addr = (0xFF00_u16).wrapping_add(low_byte);

        setup_memory!(vm {
            pc => opcode,
            pc.wrapping_add(1) => (addr & 0x00FF) as u8,
            pc.wrapping_add(2) => ((addr & 0xFF00) >> 8) as u8,
            pc.wrapping_add(3) => NOP_IMP,
            addr => low_byte as u8
        });

        assert_execution_eq!(vm, st, {
            (cycle {
                t => 0,
                pc => pc + 1,
                op => opcode,
                am => AddressingMode::Absolute,
                nst => State::ResolveAddress(FetchAddress { high_byte: false })
            }) (=)

            (cycle {
                pc => pc + 2,
                nst => State::ResolveAddress(FetchAddress { high_byte: true }),
                ab => addr & 0x00FF
            }) (=)

            (cycle {
                nst => State::Process,
                ab => addr
            }) (=)

            (cycle {
                nst => State::Fetch,
                ac => low_byte as u8
            }) (=)
        });

        assert_next_instr_is_nop(&mut vm, &mut st);
        setup_memory!(vm {
                pc => JAM,
                pc.wrapping_add(1) => JAM,
                pc.wrapping_add(2) => JAM,
                pc.wrapping_add(3) => JAM,
                addr => JAM
            } [r_pc => 0xF000]);
    }
}

fn test_abi_addressing(opcode: u8, index_register: IndexRegister) {
    let mut vm = get_vm();
    for low_byte in 0..512 {
        for index in 0..=255_u8 {
            let mut st = Status::default();

            let pc = vm.r_pc.wrapping_add(1);
            let addr = (0xFF00_u16).wrapping_add(low_byte);

            setup_memory!(vm {
                pc => opcode,
                pc.wrapping_add(1) => (addr & 0x00FF) as u8,
                pc.wrapping_add(2) => ((addr & 0xFF00) >> 8) as u8,
                pc.wrapping_add(3) => NOP_IMP,
                addr.wrapping_add(index.into()) => low_byte as u8
            });

            if index_register == IndexRegister::X {
                setup_memory!(vm {} [r_ix => index]);
            } else {
                setup_memory!(vm {} [r_iy => index]);
            }

            assert_execution_eq!(vm, st, {
                (cycle {
                    t => 0,
                    pc => pc.wrapping_add(1),
                    op => opcode,
                    am => AddressingMode::AbsoluteI(index_register),
                    nst => State::ResolveAddress(FetchAddress { high_byte: false })
                }) (=)

                (cycle {
                    pc => pc.wrapping_add(2),
                    nst => State::ResolveAddress(FetchAddress { high_byte: true }),
                    ab => addr & 0x00FF
                }) (=)

                (cycle {
                    nst => State::ResolveAddress(AddIndexRegister {
                        index_register: index_register, bump_page: false
                    }),
                    ab => addr
                }) (=)
            });

            if ((addr & 0x00FF) as u8).wrapping_add(index) < index {
                assert_execution_eq!(vm, st, {
                    (cycle {
                        nst => State::ResolveAddress(AddIndexRegister {
                            index_register: index_register, bump_page: true
                        }),
                        ab => (addr & 0xFF00) + (addr as u8).wrapping_add(index) as u16
                    }) (=)
                });
            }

            assert_execution_eq!(vm, st, {
                (cycle {
                    nst => State::Process,
                    ab => addr.wrapping_add(index.into())
                }) (=)

                (cycle {
                    nst => State::Fetch,
                    ac => low_byte as u8
                }) (=)
            });

            assert_next_instr_is_nop(&mut vm, &mut st);
            setup_memory!(vm {
                    pc => JAM,
                    pc.wrapping_add(1) => JAM,
                    pc.wrapping_add(2) => JAM,
                    pc.wrapping_add(3) => JAM,
                    addr => JAM
                } [r_ix => 0, r_iy => 0, r_pc => 0xF000]);
        }
    }
}

#[test]
fn test_abx_addressing() {
    test_abi_addressing(LDA_ABX, IndexRegister::X);
}

#[test]
fn test_aby_addressing() {
    test_abi_addressing(LDA_ABY, IndexRegister::Y);
}
