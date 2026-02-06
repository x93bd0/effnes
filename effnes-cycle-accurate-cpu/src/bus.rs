pub trait MemoryBus {
    fn read_byte(&mut self, addr: u16) -> u8;
    fn write_byte(&mut self, addr: u16, data: u8);
}

pub struct BasicMemory {
    memory: [u8; 65536],
}

impl BasicMemory {
    pub fn default_with(value: u8) -> Self {
        let memory = [value; 65536];
        // TODO: Set up vectors
        Self { memory }
    }
}

impl MemoryBus for BasicMemory {
    fn read_byte(&mut self, addr: u16) -> u8 {
        return self.memory[addr as usize];
    }

    fn write_byte(&mut self, addr: u16, data: u8) {
        self.memory[addr as usize] = data;
    }
}
