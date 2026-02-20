pub trait Cpu {
    fn cold_reset(&mut self);
    fn warm_reset(&mut self);
    fn cycle(&mut self) -> u8;
}
