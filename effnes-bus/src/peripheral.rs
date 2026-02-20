use crate::MemoryBus;

pub trait Peripheral {
    fn cold_reset(&mut self);
    fn warm_reset(&mut self);

    fn recv(&mut self, addr: u16, value: u8);
    fn cycle(&mut self, io: &mut impl MemoryBus);
}
