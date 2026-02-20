use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};

use effnes_basic_cpu::vm::VM as BasicVM;
use effnes_bus::{basic::BasicMemory, peripheral::Peripheral};
use effnes_ca_cpu::vm::VM as CycleAccurateVM;
use effnes_cpu::consts::Flags;
use effnes_cpu::cpu::Cpu;
use effnes_cpu::debug::{self, DebugCpu, State};

mod common;

fn nestest(mut cpu: impl Cpu + DebugCpu + Peripheral) {
    let mut io = BasicMemory::default_with(0);
    {
        let mut rom = File::open("res/nestest/nestest.nes").unwrap();
        rom.seek(SeekFrom::Start(16)).unwrap();
        rom.read_exact(&mut io.memory[0xC000..0xFFFF]).unwrap();
    }

    cpu.cold_reset();
    cpu.set_flags(Flags::from_bits_retain(0x24));
    cpu.set_pc(0xC000);
    cpu.set_sp(0xFD);
    cpu.set_cc(7);

    let file = File::open("res/nestest/nestest.log").unwrap();
    let reader = BufReader::new(file);
    let mut stream = reader.lines();

    loop {
        let line = match stream.next() {
            Some(result) => result.unwrap(),
            _ => {
                break;
            }
        };

        let exp = State {
            pc: u16::from_str_radix(&line[0..4], 16).unwrap(),
            ix: u8::from_str_radix(&line[55..57], 16).unwrap(),
            iy: u8::from_str_radix(&line[60..62], 16).unwrap(),
            ac: u8::from_str_radix(&line[50..52], 16).unwrap(),
            sp: u8::from_str_radix(&line[71..73], 16).unwrap(),
            ps: Flags::from_bits(u8::from_str_radix(&line[65..67], 16).unwrap()).unwrap(),
            cc: usize::from_str_radix(&line[90..], 10).unwrap(),

            // TODO: Set the correct Addressing Mode
            am: effnes_cpu::addr::AddressingMode::Implied,
        };

        while cpu.state().cc < exp.cc {
            cpu.cycle(&mut io);
        }

        println!("{}", line);
        debug::debug(&cpu, &io);
        println!("");
        assert_state_eq!("NESTEST", cpu, exp);
    }
}

#[test]
fn nestest_cycle_accurate() {
    nestest(CycleAccurateVM::default());
}

#[test]
fn nestest_basic() {
    nestest(BasicVM::default());
}
