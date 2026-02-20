use effnes_basic_cpu::vm;
use effnes_bus::MemoryBus;

/// Basic memory implementation for testing purposes.
pub struct CPUStatus {
    pub pc: u16,
    pub x: u8,
    pub y: u8,
    pub a: u8,
    pub s: u8,
    pub p: u8,
    pub cycles: usize,
}

pub fn validate_cpu<T: MemoryBus>(vm: &vm::VM<T>, status: CPUStatus, error_string: String) {
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
        vm.x, status.x,
        "Test Suite Error <{}>\nInvalid `X register`",
        error_string
    );
    assert_eq!(
        vm.y, status.y,
        "Test Suite Error <{}>\nInvalid `Y register`",
        error_string
    );
    assert_eq!(
        vm.p, status.p,
        "Test Suite Error <{}>\nInvalid `Program Status`",
        error_string
    );
    assert_eq!(
        vm.s, status.s,
        "Test Suite Error <{}>\nInvalid `Stack Pointer`",
        error_string
    );
    assert_eq!(
        vm.cycles, status.cycles,
        "Test Suite Error <{}>\nInvalid `Cycles`",
        error_string
    );
}
