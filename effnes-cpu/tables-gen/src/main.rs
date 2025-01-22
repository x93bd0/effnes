use std::{
    collections::BTreeMap,
    fs::File,
    io::{self, BufRead, BufReader},
};

const ADDRMODES_PATH: &str = "res/addr_modes.raw";
const OPCODES_PATH: &str = "res/opcodes.raw";

fn main() -> io::Result<()> {
    let mut map = BTreeMap::<String, usize>::new();
    let mut aliases = BTreeMap::<String, String>::new();
    let mut opcode_map = BTreeMap::<String, usize>::new();
    let mut mnemonics: Vec<String> = Default::default();
    let mut table: Vec<u16> = vec![0; 256];

    {
        let file = File::open(ADDRMODES_PATH)?;
        let reader = BufReader::new(file);

        let mut discriminant = 0;
        for opt_line in reader.lines() {
            let mut line = opt_line.unwrap();
            let mid = line.find(":").unwrap();
            map.insert((&mut line[..mid]).to_string(), discriminant);
            aliases.insert(
                (&mut line[mid + 1..]).to_string(),
                (&mut line[..mid]).to_string(),
            );
            discriminant += 1;
        }
    }

    {
        let file = File::open(OPCODES_PATH).unwrap();
        let reader = BufReader::new(file);
        let mut discriminant = 0;
        let mut index = 0;

        for opt_line in reader.lines() {
            index += 1;
            let mut line = opt_line.unwrap();
            {
                let mut mnemonic = &mut line[..3];
                if let Some(r) = mnemonic.get_mut(1..) {
                    r.make_ascii_lowercase();
                }
            }

            let mnemonic = &line[..3];
            let mode_abrv = &line[4..7];
            let opcode = &line[8..10];
            let mut time = &line[13..];

            assert_ne!(time.len(), 0);
            let extra = time.chars().last().unwrap() == '+';

            if extra {
                time = &time[..time.len() - 1];
            }

            let utime = time.parse::<u16>().unwrap();
            assert_ne!(utime, 0);

            let uopcode = u16::from_str_radix(opcode, 16).unwrap();
            let iopcode = match opcode_map.get(mnemonic) {
                Some(op) => *op,
                None => {
                    opcode_map.insert(mnemonic.to_string(), discriminant);
                    mnemonics.push(mnemonic.to_string().to_uppercase());
                    discriminant += 1;
                    discriminant - 1
                }
            } as u16;

            assert_eq!(
                table[uopcode as usize], 0,
                "Opcode 0x{uopcode:x} is redefined at line {index}",
            );
            table[uopcode as usize] = (if extra { 1u16 } else { 0u16 })
                + ((utime - 1) << 1)
                + ((map[&aliases[mode_abrv]] as u16) << 4)
                + (iopcode << 8);
        }
    }

    println!("use std::convert::TryFrom;");
    println!("");

    println!("/// CPU interrupt vectors.");
    println!("pub enum CPUVector {{");
    println!("    /// Non-Maskable Interrupt");
    println!("    Nmi = 0xFFFA,");
    println!("    /// Reset");
    println!("    Rst = 0xFFFC,");
    println!("    /// Break");
    println!("    Brk = 0xFFFE,");
    println!("}}");
    println!("");

    println!("/// CPU flags.");
    println!("pub enum Flag {{");
    println!("    Carry = 0b1,");
    println!("    Zero = 0b10,");
    println!("    IntDis = 0b100,");
    println!("    Decimal = 0b1000,");
    println!("    Break = 0b10000,");
    println!("    Reserved = 0b100000,");
    println!("    Overflow = 0b1000000,");
    println!("    Negative = 0b10000000,");
    println!("}}");
    println!("");

    println!("#[repr(u8)]");
    println!("#[derive(PartialEq)]");
    println!("pub enum AddrMode {{");
    for addrmode in &map {
        println!("    {} = 0x{:0x},", addrmode.0, addrmode.1);
    }
    println!("}}");
    println!("");

    println!("#[repr(u8)]");
    println!("pub enum OpCode {{");
    for opcode in &opcode_map {
        println!("    {} = 0x{:0x},", opcode.0, opcode.1);
    }
    println!("}}");
    println!("");

    println!("pub const TRANSLATION_TABLE: [u16; 256] = [");
    for code in table {
        println!("    0b{:0b},", code);
    }
    println!("];");
    println!("");

    println!("impl TryFrom<u8> for AddrMode {{");
    println!("    type Error = ();");
    println!();
    println!("    fn try_from(value: u8) -> Result<Self, Self::Error> {{");
    println!("       match value {{");
    for addrmode in map {
        println!("            {} => Ok(Self::{}),", addrmode.1, addrmode.0);
    }
    println!("            _ => Err(()),");
    println!("       }}");
    println!("    }}");
    println!("}}");
    println!("\n");

    println!("impl TryFrom<u8> for OpCode {{");
    println!("    type Error = ();");
    println!();
    println!("    fn try_from(value: u8) -> Result<Self, Self::Error> {{");
    println!("        match value {{");
    for opcode in opcode_map {
        println!("            {} => Ok(Self::{}),", opcode.1, opcode.0);
    }
    println!("            _ => Err(()),");
    println!("        }}");
    println!("    }}");
    println!("}}");

    println!("");
    println!("pub const MNEMONICS_TABLE: [&str; {}] = [", mnemonics.len());
    for mnemonic in mnemonics {
        println!("    \"{}\",", mnemonic);
    }
    println!("];");

    Ok(())
}
