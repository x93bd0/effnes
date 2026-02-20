use crate::{
    addr::{AddressingMode, IndexRegister},
    consts::Flags,
    opcode::{Mnemonic, OpCode},
};
use effnes_bus::InspectBus;
use std::fmt::Display;

pub struct State {
    pub pc: u16,
    pub sp: u8,
    pub ac: u8,
    pub ix: u8,
    pub iy: u8,
    pub am: AddressingMode,
    pub ps: Flags,

    pub cc: usize,
}

pub trait InspectCpu {
    fn is_cycle_accurate(&self) -> bool;
    fn state(&self) -> State;
}

pub fn debug_cpu(io: &dyn InspectBus, vm: &dyn InspectCpu) {
    let s = vm.state();
    let opc: OpCode = io.peek_u8(s.pc);
    let am: AddressingMode = opc.into();

    print!("VM[{:2x}", opc);

    {
        use AddressingMode::*;
        use IndexRegister::{X, Y};
        let (out, len) = match am {
            Immediate => (format!("#${:02x}", io.peek_u8(s.pc)), 1),
            Relative => (
                format!(
                    "${:04x}",
                    s.pc.wrapping_add(2)
                        .wrapping_add_signed((io.peek_u8(s.pc) as i8) as i16)
                ),
                1,
            ),
            ZeroPage => (format!("${:02x}", io.peek_u8(s.pc)), 1),
            Absolute => (format!("${:04x}", io.peek_u16(s.pc)), 2),
            Indirect => (format!("(${:04x})", io.peek_u16(s.pc)), 2),
            ZeroPageI(X) => (format!("${:02x},X", io.peek_u8(s.pc)), 1),
            ZeroPageI(Y) => (format!("${:02x},Y", io.peek_u8(s.pc)), 1),
            AbsoluteI(X) => (format!("${:04x},X", io.peek_u16(s.pc)), 2),
            AbsoluteI(Y) => (format!("${:04x},Y", io.peek_u16(s.pc)), 2),
            IndirectI(X) => (format!("(${:02x},X)", io.peek_u8(s.pc)), 1),
            IndirectI(Y) => (format!("(${:02x}),Y", io.peek_u8(s.pc)), 1),
            Implied => ("".into(), 0),
        };

        for ind in 0..len {
            print!(" {:2x}", io.peek_u8(s.pc.wrapping_add(ind)));
        }

        for _ in len..2 {
            print!("   ");
        }

        print!(
            " | {} {}",
            <Mnemonic as From<OpCode>>::from(opc.into()),
            out
        );
        for _ in (4 + out.len())..14 {
            print!(" ");
        }
    }

    print!(
        "| A:{:02x} X:{:02x} Y:{:02x} S:{:02x} P:{:02x}] | PC:{:04x} CYC:{}",
        s.ac,
        s.ix,
        s.iy,
        s.sp,
        s.ps.bits(),
        s.pc,
        s.cc
    );
}
