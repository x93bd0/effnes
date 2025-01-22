use effnes_bus::Memory;
use effnes_cpu::vm;

/// Basic memory implementation for testing purposes.
pub struct BasicMemory {
    pub data: Box<[u8]>,
}

pub struct CPUStatus {
    pub pc: u16,
    pub x: u8,
    pub y: u8,
    pub a: u8,
    pub s: u8,
    pub p: u8,
    pub cycles: usize,
}

impl Memory for BasicMemory {
    fn default() -> Self {
        let mem = vec![0; 65536];
        BasicMemory {
            data: mem.into_boxed_slice(),
        }
    }

    fn read_byte(&self, addr: u16) -> u8 {
        self.data[addr as usize]
    }

    fn read_addr(&self, addr: u16) -> u16 {
        (self.data[addr as usize] as u16) + ((self.data[addr.wrapping_add(1) as usize] as u16) << 8)
    }

    fn write_byte(&mut self, addr: u16, data: u8) {
        self.data[addr as usize] = data;
    }

    fn write_addr(&mut self, addr: u16, data: u16) {
        self.data[addr as usize] = data as u8;
        self.data[addr.wrapping_add(1) as usize] = (data >> 8) as u8;
    }
}

pub fn validate_cpu<T: Memory>(vm: &vm::VM<T>, status: CPUStatus, error_string: String) {
    assert_eq!(
        vm.pc, status.pc,
        "Test Suite Error <{}>\nInvalid `Program Counter`",
        error_string
    );
    assert_eq!(
        vm.a, status.a,
        "Test Suite Error <{}>\nInvalid `Accumulator`",
        error_string
    );
    assert_eq!(
        vm.a, status.a,
        "Test Suite Error <{}>\nInvalid `X register`",
        error_string
    );
    assert_eq!(
        vm.a, status.a,
        "Test Suite Error <{}>\nInvalid `Y register`",
        error_string
    );
    assert_eq!(
        vm.a, status.a,
        "Test Suite Error <{}>\nInvalid `Program Status`",
        error_string
    );
    assert_eq!(
        vm.a, status.a,
        "Test Suite Error <{}>\nInvalid `Stack Pointer`",
        error_string
    );
    assert_eq!(
        vm.a, status.a,
        "Test Suite Error <{}>\nInvalid `Cycles`",
        error_string
    );
}
