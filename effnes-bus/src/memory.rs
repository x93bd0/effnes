/// Memory Bus
pub trait Memory {
    /// Should instanciate a new `Memory Bus`.
    fn default() -> Self;

    /// Should read a byte from a certain address and return it.
    fn read_byte(&self, addr: u16) -> u8;

    /// Should read an address from a certain address and return it.
    ///
    /// ## Implementation example
    ///
    /// ```ignore
    /// fn read_addr(self, addr: u16) -> u16 {
    ///     self.read_byte(addr) + (self.read_byte(addr.wrapping_add(1)) << 8)
    /// }
    /// ```
    ///
    fn read_addr(&self, addr: u16) -> u16;

    /// Should write a byte to a certain address.
    fn write_byte(&mut self, addr: u16, data: u8);

    /// Should write a addr to a certain address.
    ///
    /// ## Implementaion example
    ///
    /// ```ignore
    /// fn write_addr(self, addr: u16, data: u16) {
    ///     self.write_byte(addr, data as u8);
    ///     self.write_byte(addr, (data >> 8) as u8);
    /// }
    /// ```
    ///
    fn write_addr(&mut self, addr: u16, data: u16);
}
