use crate::{addr::AddressingMode, consts::Flags};

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
    const CYCLE_ACCURATE: bool;
    fn state(&self) -> State;
}
