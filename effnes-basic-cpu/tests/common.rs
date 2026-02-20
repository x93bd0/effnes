use effnes_cpu::inspect::{InspectCpu, State};

pub fn validate_cpu(vm: &impl InspectCpu, expected: State, error_string: String) {
    let s = vm.state();
    assert_eq!(
        s.pc, expected.pc,
        "Test Suite Error <{}>\nInvalid `Program Counter`",
        error_string
    );
    assert_eq!(
        s.ac, expected.ac,
        "Test Suite Error <{}>\nInvalid `Accumulator`",
        error_string
    );
    assert_eq!(
        s.ix, expected.ix,
        "Test Suite Error <{}>\nInvalid `X register`",
        error_string
    );
    assert_eq!(
        s.iy, expected.iy,
        "Test Suite Error <{}>\nInvalid `Y register`",
        error_string
    );
    assert_eq!(
        s.pc, expected.pc,
        "Test Suite Error <{}>\nInvalid `Program Status`",
        error_string
    );
    assert_eq!(
        s.sp, expected.sp,
        "Test Suite Error <{}>\nInvalid `Stack Pointer`",
        error_string
    );
    assert_eq!(
        s.cc, expected.cc,
        "Test Suite Error <{}>\nInvalid `Cycles`",
        error_string
    );
}
