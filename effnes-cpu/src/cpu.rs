use crate::consts::Flags;

pub trait Cpu {
    fn is_cycle_accurate(&self) -> bool;
}
