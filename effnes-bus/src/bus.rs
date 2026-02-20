/// Memory Bus
pub trait MemoryBus {
    /// Reads a byte from `addr`.
    ///
    /// Implementations may perform side effects (e.g., MMIO behavior).
    fn read_u8(&mut self, addr: u16) -> u8;

    /// Reads an address from `addr`.
    ///
    /// Implementations may perform side effects (e.g., MMIO behavior).
    fn read_u16(&mut self, addr: u16) -> u16;

    /// Writes `data` into `addr`.
    fn write_u8(&mut self, addr: u16, data: u8);
}

/// Inspect Bus
pub trait InspectBus {
    /// Reads a byte from `addr`.
    ///
    /// This must not mutate the bus or peripheral state.
    fn peek_u8(&self, addr: u16) -> u8;

    /// Reads an address from `{addr + 1}{addr}`.
    ///
    /// This must not mutate the bus or peripheral state.
    fn peek_u16(&self, addr: u16) -> u16;
}
