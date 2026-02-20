#[macro_export]
macro_rules! assert_state_eq {
    ($suite:literal, $vm:ident, $exp:ident) => {
        assert_state_eq!($suite, $vm, $exp, fields [
            pc, ac, ix, iy, ps, sp, cc
        ]);
    };

    ($suite:literal, $vm:ident, $exp:ident, fields [$($field:ident),*]) => {
        let s = $vm.state();
        $(
            assert_eq!(
                s.$field, $exp.$field,
                "Test Suite Error <{}>\nInvalid `{}`",
                $suite, stringify!($field)
            );
        )*
    };
}
