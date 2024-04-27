#define ADC 0x69
#define ADC_ZPG 0x65
#define ADC_ZPX 0x75
#define ADC_ABS 0x6D
#define ADC_ABX 0x7D
#define ADC_ABY 0x79
#define ADC_INX 0x61
#define ADC_INY 0x71
#define AND 0x29
#define AND_ZPG 0x25
#define AND_ZPX 0x35
#define AND_ABS 0x2D
#define AND_ABX 0x3D
#define AND_ABY 0x39
#define AND_INX 0x21
#define AND_INY 0x31
#define ASL_ACC 0x0A
#define ASL_ZPG 0x06
#define ASL_ZPX 0x16
#define ASL_ABS 0x0E
#define ASL_ABX 0x1E
#define BIT_ZPG 0x24
#define BIT_ABS 0x2C
#define BPL_REL 0x10
#define BMI_REL 0x30
#define BVC_REL 0x50
#define BVS_REL 0x70
#define BCC_REL 0x90
#define BCS_REL 0xB0
#define BNE_REL 0xD0
#define BEQ_REL 0xF0
#define BRK 0x00
#define CMP 0xC9
#define CMP_ZPG 0xC5
#define CMP_ZPX 0xD5
#define CMP_ABS 0xCD
#define CMP_ABX 0xDD
#define CMP_ABY 0xD9
#define CMP_INX 0xC1
#define CMP_INY 0xD1
#define CPX 0xE0
#define CPX_ZPG 0xE4
#define CPX_ABS 0xEC
#define CPY 0xC0
#define CPY_ZPG 0xC4
#define CPY_ABS 0xCC
#define DEC_ZPG 0xC6
#define DEC_ZPX 0xD6
#define DEC_ABS 0xCE
#define DEC_ABX 0xDE
#define EOR 0x49
#define EOR_ZPG 0x45
#define EOR_ZPX 0x55
#define EOR_ABS 0x4D
#define EOR_ABX 0x5D
#define EOR_ABY 0x59
#define EOR_INX 0x41
#define EOR_INY 0x51
#define CLC 0x18
#define SEC 0x38
#define CLI 0x58
#define SEI 0x78
#define CLV 0xB8
#define CLD 0xD8
#define SED 0xF8
#define INC_ZPG 0xE6
#define INC_ZPX 0xF6
#define INC_ABS 0xEE
#define INC_ABX 0xFE
#define JMP_ABS 0x4C
#define JMP_IND 0x6C
#define JSR_ABS 0x20
#define LDA 0xA9
#define LDA_ZPG 0xA5
#define LDA_ZPX 0xB5
#define LDA_ABS 0xAD
#define LDA_ABX 0xBD
#define LDA_ABY 0xB9
#define LDA_INX 0xA1
#define LDA_INY 0xB1
#define LDX 0xA2
#define LDX_ZPG 0xA6
#define LDX_ZPY 0xB6
#define LDX_ABS 0xAE
#define LDX_ABY 0xBE
#define LDY 0xA0
#define LDY_ZPG 0xA4
#define LDY_ZPX 0xB4
#define LDY_ABS 0xAC
#define LDY_ABX 0xBC
#define LSR_ACC 0x4A
#define LSR_ZPG 0x46
#define LSR_ZPX 0x56
#define LSR_ABS 0x4E
#define LSR_ABX 0x5E
#define NOP 0xEA
#define ORA 0x09
#define ORA_ZPG 0x05
#define ORA_ZPX 0x15
#define ORA_ABS 0x0D
#define ORA_ABX 0x1D
#define ORA_ABY 0x19
#define ORA_INX 0x01
#define ORA_INY 0x11
#define TAX 0xAA
#define TXA 0x8A
#define DEX 0xCA
#define INX 0xE8
#define TAY 0xA8
#define TYA 0x98
#define DEY 0x88
#define INY 0xC8
#define ROL_ACC 0x2A
#define ROL_ZPG 0x26
#define ROL_ZPX 0x36
#define ROL_ABS 0x2E
#define ROL_ABX 0x3E
#define ROR_ACC 0x6A
#define ROR_ZPG 0x66
#define ROR_ZPX 0x76
#define ROR_ABS 0x6E
#define ROR_ABX 0x7E
#define RTI 0x40
#define RTS 0x60
#define SBC 0xE9
#define SBC_ZPG 0xE5
#define SBC_ZPX 0xF5
#define SBC_ABS 0xED
#define SBC_ABX 0xFD
#define SBC_ABY 0xF9
#define SBC_INX 0xE1
#define SBC_INY 0xF1
#define STA_ZPG 0x85
#define STA_ZPX 0x95
#define STA_ABS 0x8D
#define STA_ABX 0x9D
#define STA_ABY 0x99
#define STA_INX 0x81
#define STA_INY 0x91
#define TXS 0x9A
#define TSX 0xBA
#define PHA 0x48
#define PLA 0x68
#define PHP 0x08
#define PLP 0x28
#define STX_ZPG 0x86
#define STX_ZPY 0x96
#define STX_ABS 0x8E
#define STY_ZPG 0x84
#define STY_ZPX 0x94
#define STY_ABS 0x8C

const char* FROMASM[] = {
  "BRK", "ORA", "ILG", "ILG", "ILG", "ORA", "ASL", "ILG", "PHP", "ORA", "ASL", "ILG", "ILG", "ORA", "ASL", "ILG",
  "BPL", "ORA", "ILG", "ILG", "ILG", "ORA", "ASL", "ILG", "CLC", "ORA", "ILG", "ILG", "ILG", "ORA", "ASL", "ILG",
  "JSR", "AND", "ILG", "ILG", "BIT", "AND", "ROL", "ILG", "PLP", "AND", "ROL", "ILG", "BIT", "AND", "ROL", "ILG",
  "BMI", "AND", "ILG", "ILG", "ILG", "AND", "ROL", "ILG", "SEC", "AND", "ILG", "ILG", "ILG", "AND", "ROL", "ILG",
  "RTI", "EOR", "ILG", "ILG", "ILG", "EOR", "LSR", "ILG", "PHA", "EOR", "LSR", "ILG", "JMP", "EOR", "LSR", "ILG",
  "BVC", "EOR", "ILG", "ILG", "ILG", "EOR", "LSR", "ILG", "CLI", "EOR", "ILG", "ILG", "ILG", "EOR", "LSR", "ILG",
  "RTS", "ADC", "ILG", "ILG", "ILG", "ADC", "ROR", "ILG", "PLA", "ADC", "ROR", "ILG", "JMP", "ADC", "ROR", "ILG",
  "BVS", "ADC", "ILG", "ILG", "ILG", "ADC", "ROR", "ILG", "SEI", "ADC", "ILG", "ILG", "ILG", "ADC", "ROR", "ILG",
  "ILG", "STA", "ILG", "ILG", "STY", "STA", "STX", "ILG", "DEY", "ILG", "TXA", "ILG", "STY", "STA", "STX", "ILG",
  "BCC", "STA", "ILG", "ILG", "STY", "STA", "STX", "ILG", "TYA", "STA", "TXS", "ILG", "ILG", "STA", "ILG", "ILG",
  "LDY", "LDA", "LDX", "ILG", "LDY", "LDA", "LDX", "ILG", "TAY", "LDA", "TAX", "ILG", "LDY", "LDA", "LDX", "ILG",
  "BCS", "LDA", "ILG", "ILG", "LDY", "LDA", "LDX", "ILG", "CLV", "LDA", "TSX", "ILG", "LDY", "LDA", "LDX", "ILG",
  "CPY", "CMP", "ILG", "ILG", "CPY", "CMP", "DEC", "ILG", "INY", "CMP", "DEX", "ILG", "CPY", "CMP", "DEC", "ILG",
  "BNE", "CMP", "ILG", "ILG", "ILG", "CMP", "DEC", "ILG", "CLD", "CMP", "ILG", "ILG", "ILG", "CMP", "DEC", "ILG",
  "CPX", "SBC", "ILG", "ILG", "CPX", "SBC", "INC", "ILG", "INX", "SBC", "NOP", "ILG", "CPX", "SBC", "INC", "ILG",
  "BEQ", "SBC", "ILG", "ILG", "ILG", "SBC", "INC", "ILG", "SED", "SBC", "ILG", "ILG", "ILG", "SBC", "INC", "ILG"
};
