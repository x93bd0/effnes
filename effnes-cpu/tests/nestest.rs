use std::fs::File;
use std::io::{self, BufRead, BufReader, Lines, Read, Seek, SeekFrom};
use std::path::Path;

use effnes_bus::Memory;
use effnes_cpu::vm::VM;

struct BasicMemory {
    data: Box<[u8]>,
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

/// Fetched from: https://doc.rust-lang.org/rust-by-example/std_misc/file/read_lines.html
fn read_lines<P>(filename: P) -> io::Result<Lines<BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(BufReader::new(file).lines())
}

fn validate(vm: &VM<BasicMemory>, line: io::Result<String>) {
    let data: String = line.unwrap();

    let mut ptr: &str = &data[0..4];
    assert_eq!(
        u16::from_str_radix(&ptr, 16).unwrap(),
        vm.pc,
        "Testing `Program Counter` ([correct] {} == [code's] {:x})",
        ptr,
        vm.pc
    );
    ptr = &data[50..52];
    assert_eq!(
        u8::from_str_radix(&ptr, 16).unwrap(),
        vm.a,
        "Testing `Accumulator`"
    );
    ptr = &data[55..57];
    assert_eq!(
        u8::from_str_radix(&ptr, 16).unwrap(),
        vm.x,
        "Testing `X Register`"
    );
    ptr = &data[60..62];
    assert_eq!(
        u8::from_str_radix(&ptr, 16).unwrap(),
        vm.y,
        "Testing `Y Register`"
    );
    ptr = &data[65..67];
    assert_eq!(
        u8::from_str_radix(&ptr, 16).unwrap(),
        vm.p,
        "Testing `Program Status`"
    );
    ptr = &data[71..73];
    assert_eq!(
        u8::from_str_radix(&ptr, 16).unwrap(),
        vm.s,
        "Testing `Stack Pointer`"
    );
    ptr = &data[90..];
    assert_eq!(
        usize::from_str_radix(&ptr, 10).unwrap(),
        vm.cycles,
        "Testing `Cycles`"
    );
}

#[test]
fn main() {
    let break_addr: u16 = 0x2000;

    let mut vm: VM<BasicMemory> = Default::default();
    let mut rom = File::open("res/nestest.nes").unwrap();
    rom.seek(SeekFrom::Start(16)).unwrap();
    rom.read_exact(&mut vm.io.data[0xC000..0xFFFF]).unwrap();

    vm.reset();
    vm.pc = 0xC000;
    vm.p = 0x24;
    vm.s = 0xff;
    vm.cycles = 7;
    vm.stack_push_addr(break_addr);

    let mut stream: Lines<BufReader<File>> = read_lines("res/nestest.log").unwrap();
    while vm.cycles < 1_000_000 {
        let result = match stream.next() {
            Some(result) => result,
            None => {
                break;
            }
        };

        validate(&vm, result);
        vm.run(1);
    }
}
