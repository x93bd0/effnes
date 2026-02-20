use crate::{InspectBus, MemoryBus};

pub struct BasicMemory {
    pub memory: [u8; 65536],
}

impl BasicMemory {
    pub fn default_with(value: u8) -> Self {
        let memory = [value; 65536];
        // TODO: Set up vectors
        Self { memory }
    }
}

impl MemoryBus for BasicMemory {
    fn read_u8(&mut self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }

    fn write_u8(&mut self, addr: u16, data: u8) {
        self.memory[addr as usize] = data
    }

    fn read_u16(&mut self, addr: u16) -> u16 {
        (self.read_u8(addr) as u16) + ((self.read_u8(addr.wrapping_add(1)) as u16) << 8)
    }
}

impl InspectBus for BasicMemory {
    fn peek_u8(&self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }

    fn peek_u16(&self, addr: u16) -> u16 {
        (self.peek_u8(addr) as u16) + ((self.peek_u8(addr + 1) as u16) << 8)
    }
}
