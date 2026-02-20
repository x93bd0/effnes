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
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom};

mod common;
use effnes_bus::{basic::BasicMemory, peripheral::Peripheral, InspectBus};
use effnes_cpu::consts::Flags;
use effnes_cpu::debug::{self, DebugCpu, State};

use effnes_basic_cpu::vm::VM;

/// Validate a VM run by comparing it to the output from the NESTEST suite.
fn validate(io: &impl InspectBus, vm: &impl DebugCpu, line: io::Result<String>) {
    // println!("{}", vm);
    debug::debug(vm, io);
    println!();

    let data = line.unwrap();
    println!("{}", data);
    common::validate_cpu(
        vm,
        State {
            pc: u16::from_str_radix(&data[0..4], 16).unwrap(),
            ix: u8::from_str_radix(&data[55..57], 16).unwrap(),
            iy: u8::from_str_radix(&data[60..62], 16).unwrap(),
            ac: u8::from_str_radix(&data[50..52], 16).unwrap(),
            sp: u8::from_str_radix(&data[71..73], 16).unwrap(),
            ps: Flags::from_bits(u8::from_str_radix(&data[65..67], 16).unwrap()).unwrap(),
            cc: usize::from_str_radix(&data[90..], 10).unwrap(),

            // TODO: Set the correct Addressing Mode
            am: effnes_cpu::addr::AddressingMode::Implied,
        },
        "NESTEST".to_string(),
    );

    assert_eq!(0, io.peek_u16(2));
}

#[test]
fn nestest() -> io::Result<()> {
    let mut io = BasicMemory::default_with(0);
    let mut vm = VM::default();

    {
        let mut rom = File::open("res/nestest/nestest.nes").unwrap();
        rom.seek(SeekFrom::Start(16)).unwrap();
        rom.read_exact(&mut io.memory[0xC000..0xFFFF]).unwrap();
    }

    vm.cold_reset();
    vm.set_flags(Flags::from_bits_retain(0x24));
    vm.set_pc(0xC000);
    vm.set_sp(0xFD);
    vm.set_cc(7);

    let file = File::open("res/nestest/nestest.log")?;
    let reader = BufReader::new(file);

    let mut stream = reader.lines();
    while vm.state().cc < 1_000_000 {
        let result = match stream.next() {
            Some(result) => result,
            _ => {
                break;
            }
        };

        validate(&io, &vm, result);
        vm.cycle(&mut io);
    }

    Ok(())
}
