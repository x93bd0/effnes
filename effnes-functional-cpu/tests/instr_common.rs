use std::fs::{read_dir, File};
use std::io::{self, Read, Seek, SeekFrom};
use std::str;

mod common;
use effnes_bus::Memory;
use effnes_cpu::vm::VM;

#[test]
fn instr_tests() -> io::Result<()> {
    let roms = read_dir("res/instr_tests/")?;

    for rom in roms {
        let mut vm: VM<common::BasicMemory> = Default::default();
        let path = {
            let path = rom.unwrap().path();
            let mut rom_file = File::open(path.clone())?;
            rom_file.seek(SeekFrom::Start(16)).unwrap();
            rom_file
                .read_exact(&mut vm.io.data[0x8000..0x10000])
                .unwrap();

            path
        };

        println!(
            "Running test `{}`",
            path.to_str().expect("Couldn't get string out of Path")
        );

        vm.reset();
        while vm.cycles < 1_000_000
            && !(vm.io.read_byte(0x6001) == 0xDE
                && vm.io.read_byte(0x6002) == 0xB0
                && vm.io.read_byte(0x6003) == 0x61)
        {
            vm.run(1);
        }

        assert!(
            (vm.io.read_byte(0x6001) == 0xDE
                && vm.io.read_byte(0x6002) == 0xB0
                && vm.io.read_byte(0x6003) == 0x61),
        );

        let mut return_code = 0;
        while vm.cycles < 30_000_000 && {
            return_code = vm.io.read_byte(0x6000);
            return_code
        } == 0x80
        {
            vm.run(1);
        }

        let mut start = 0x6004;
        let mut end = 0x7000;

        while start < end {
            let mid = start + (end - start + 1) / 2;
            if vm.io.read_byte(mid) == b'\x00' {
                end = mid - 1;
            } else {
                start = mid;
            }
        }

        let string = str::from_utf8(&vm.io.data[0x6004..(end as usize)])
            .expect("Couldn't read test output (problem with `.nes` file)");

        assert_ne!(
            return_code,
            0x80,
            "Test '{}' didn't leave running state.\nExtra info from test:\n{}",
            path.to_str().expect("Couldn't get string out of Path"),
            string
        );

        assert_eq!(
            return_code,
            0x0,
            "Test '{}' failed with return code `{:00x}`\nExtra info from test:\n{}",
            path.to_str().expect("Couldn't get string out of Path"),
            return_code,
            string
        );
    }

    Ok(())
}
