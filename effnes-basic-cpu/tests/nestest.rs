use std::default::Default;
/// Basic implementation of the NESTEST suite for automatically testing the VM.
///
/// # Requirements
/// For this test to work, there needs to be a directory named `res`, with the
/// following content:
/// - `res/nestest/nestest.nes`: "NESTEST" rom.
/// - `res/nestest/nestest.log`: "NESTEST" execution log.
///
use std::fs::File;
use std::io::{self, BufRead, BufReader, Lines, Read, Seek, SeekFrom};

mod common;
use effnes_basic_cpu::vm::VM;
use effnes_bus::{basic::BasicMemory, InspectBus, MemoryBus};

/// Validate a VM run by comparing it to the output from the NESTEST suite.
fn validate(vm: &VM<BasicMemory>, line: io::Result<String>) {
    println!("{}", vm);
    let data = line.unwrap();
    common::validate_cpu(
        vm,
        common::CPUStatus {
            pc: u16::from_str_radix(&data[0..4], 16).unwrap(),
            x: u8::from_str_radix(&data[55..57], 16).unwrap(),
            y: u8::from_str_radix(&data[60..62], 16).unwrap(),
            a: u8::from_str_radix(&data[50..52], 16).unwrap(),
            s: u8::from_str_radix(&data[71..73], 16).unwrap(),
            p: u8::from_str_radix(&data[65..67], 16).unwrap(),
            cycles: usize::from_str_radix(&data[90..], 10).unwrap(),
        },
        "NESTEST".to_string(),
    );

    assert_eq!(0, vm.io.peek_u16(2));
}

#[test]
fn nestest() -> io::Result<()> {
    let mut vm = VM::new(BasicMemory::default_with(0));
    let mut rom = File::open("res/nestest/nestest.nes").unwrap();
    rom.seek(SeekFrom::Start(16)).unwrap();
    rom.read_exact(&mut vm.io.memory[0xC000..0xFFFF]).unwrap();

    vm.reset();
    vm.pc = 0xC000;
    vm.p = 0x24;
    vm.s = 0xfd;
    vm.cycles = 7;

    let file = File::open("res/nestest/nestest.log")?;
    let reader = BufReader::new(file);

    let mut stream = reader.lines();
    while vm.cycles < 1_000_000 {
        let result = match stream.next() {
            Some(result) => result,
            _ => {
                break;
            }
        };

        validate(&vm, result);
        vm.run(1);
    }

    Ok(())
}
