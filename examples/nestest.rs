use efnes_cpu::memory::Memory;
use efnes_cpu::vm::VM;
use std::fs;
use std::io::{Read, Seek, SeekFrom};

const TESTASM: [&str; 256] = [
    "BRK", "ORA", "ILG", "SLO", "NOP", "ORA", "ASL", "SLO", "PHP", "ORA", "ASL", "ILG", "NOP",
    "ORA", "ASL", "SLO", "BPL", "ORA", "ILG", "SLO", "NOP", "ORA", "ASL", "SLO", "CLC", "ORA",
    "NOP", "SLO", "NOP", "ORA", "ASL", "SLO", "JSR", "AND", "ILG", "RLA", "BIT", "AND", "ROL",
    "RLA", "PLP", "AND", "ROL", "ILG", "BIT", "AND", "ROL", "RLA", "BMI", "AND", "ILG", "RLA",
    "NOP", "AND", "ROL", "RLA", "SEC", "AND", "NOP", "RLA", "NOP", "AND", "ROL", "RLA", "RTI",
    "EOR", "ILG", "SRE", "NOP", "EOR", "LSR", "SRE", "PHA", "EOR", "LSR", "ILG", "JMP", "EOR",
    "LSR", "SRE", "BVC", "EOR", "ILG", "SRE", "NOP", "EOR", "LSR", "SRE", "CLI", "EOR", "NOP",
    "SRE", "NOP", "EOR", "LSR", "SRE", "RTS", "ADC", "ILG", "RRA", "NOP", "ADC", "ROR", "RRA",
    "PLA", "ADC", "ROR", "ILG", "JMP", "ADC", "ROR", "RRA", "BVS", "ADC", "ILG", "RRA", "NOP",
    "ADC", "ROR", "RRA", "SEI", "ADC", "NOP", "RRA", "NOP", "ADC", "ROR", "RRA", "NOP", "STA",
    "NOP", "SAX", "STY", "STA", "STX", "SAX", "DEY", "NOP", "TXA", "ILG", "STY", "STA", "STX",
    "SAX", "BCC", "STA", "ILG", "ILG", "STY", "STA", "STX", "SAX", "TYA", "STA", "TXS", "ILG",
    "ILG", "STA", "ILG", "ILG", "LDY", "LDA", "LDX", "LAX", "LDY", "LDA", "LDX", "LAX", "TAY",
    "LDA", "TAX", "ILG", "LDY", "LDA", "LDX", "LAX", "BCS", "LDA", "ILG", "LAX", "LDY", "LDA",
    "LDX", "LAX", "CLV", "LDA", "TSX", "ILG", "LDY", "LDA", "LDX", "LAX", "CPY", "CMP", "NOP",
    "DCP", "CPY", "CMP", "DEC", "DCP", "INY", "CMP", "DEX", "ILG", "CPY", "CMP", "DEC", "DCP",
    "BNE", "CMP", "ILG", "DCP", "NOP", "CMP", "DEC", "DCP", "CLD", "CMP", "NOP", "DCP", "NOP",
    "CMP", "DEC", "DCP", "CPX", "SBC", "NOP", "ISC", "CPX", "SBC", "INC", "ISC", "INX", "SBC",
    "NOP", "SBC", "CPX", "SBC", "INC", "ISC", "BEQ", "SBC", "ILG", "ISC", "NOP", "SBC", "INC",
    "ISC", "SED", "SBC", "NOP", "ISC", "NOP", "SBC", "INC", "ISC",
];

struct NesMemory {
    data: Box<[u8]>,
}

impl Memory for NesMemory {
    fn default() -> NesMemory {
        NesMemory {
            data: vec![0; 65536].into_boxed_slice(),
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

fn main() {
    let mut vm: VM<NesMemory> = Default::default();

    let mut rom = fs::File::open("rom.nes").unwrap();
    rom.seek(SeekFrom::Start(16)).unwrap();
    rom.read_exact(&mut vm.io.data[0xC000 .. 0xFFFF]).unwrap();
    let mut x: usize = 10000;

    vm.reset();
    vm.pc = 0xC000;
    vm.p = 0x24;
    vm.s = 0xfd;
    vm.cycles = 7;

    const BREAK_ON: u16 = 0x2000;
    vm.stack_push_addr(BREAK_ON.wrapping_sub(1));

    while x > 0 {
        print!(
            "{:4x}  {:2x} {:2x}     {}",
            vm.pc,
            vm.io.read_byte(vm.pc),
            vm.io.read_byte(vm.pc.wrapping_add(1)),
            TESTASM[vm.io.read_byte(vm.pc) as usize],
        );

        print!("                             ");
        print!(
            "A:{:2x} X:{:2x} Y:{:2x} P:{:2x} SP:{:2x}",
            vm.a, vm.x, vm.y, vm.p, vm.s
        );
        print!("             ");
        print!("CYC:{}", vm.cycles);
        println!();

        vm.run(1);
        if vm.h > 0 {
            println!("CPU halted!");
            break;
        }

        if vm.pc == BREAK_ON
        {
            break;
        }

        x -= 1;
    }

    println!("Ran for {} cycles", vm.cycles);
}
