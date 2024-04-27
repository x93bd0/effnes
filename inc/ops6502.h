#ifndef OPS6502_H
#define OPS6502_H
#include <stdint.h>

/*
  Official Opcodes
  Source: http://www.6502.org/tutorials/6502opcodes.html
*/

#define OP_ADC  0x01
#define OP_AND  0x02
#define OP_ASL  0x03
#define OP_BCC  0x04
#define OP_BCS  0x05
#define OP_BEQ  0x06
#define OP_BIT  0x07
#define OP_BMI  0x08
#define OP_BNE  0x09
#define OP_BPL  0x0A
#define OP_BRK  0x0B
#define OP_BVC  0x0C
#define OP_BVS  0x0D
#define OP_CLC  0x0E
#define OP_CLD  0x0F
#define OP_CLI  0x10
#define OP_CLV  0x11
#define OP_CMP  0x12
#define OP_CPX  0x13
#define OP_CPY  0x14
#define OP_DEC  0x15
#define OP_DEX  0x16
#define OP_DEY  0x17
#define OP_EOR  0x18
#define OP_INC  0x19
#define OP_INX  0x1A
#define OP_INY  0x1B
#define OP_JMP  0x1C
#define OP_JSR  0x1D
#define OP_LDA  0x1E
#define OP_LDX  0x1F
#define OP_LDY  0x20
#define OP_LSR  0x21
#define OP_NOP  0x22
#define OP_ORA  0x23
#define OP_PHA  0x24
#define OP_PHP  0x25
#define OP_PLA  0x26
#define OP_PLP  0x27
#define OP_ROL  0x28
#define OP_ROR  0x29
#define OP_RTI  0x2A
#define OP_RTS  0x2B
#define OP_SBC  0x2C
#define OP_SEC  0x2D
#define OP_SED  0x2E
#define OP_SEI  0x2F
#define OP_STA  0x30
#define OP_STX  0x31
#define OP_STY  0x32
#define OP_TAX  0x33
#define OP_TAY  0x34
#define OP_TSX  0x35
#define OP_TXA  0x36
#define OP_TXS  0x37
#define OP_TYA  0x38

/*
  Op Codes Jump Table
  000000000000000
  ^^^^^^    ^^^ ^
  OpCode^^^^Tim^U
        AdMd   E

  AdMd: Addressing Mode
  Tim:  Execution Time
  E:    Extra Time if Page Boundary Crossed
  U:    Unused
*/

static uint16_t JUMPTABLE[0x100] = {
     0b1011001011100,  0b100011101111000,                0b0,                0b0,                0b0,  0b100011010101100,      0b11010110100,                0b0,
   0b100101001001100,  0b100011000101000,      0b11000001000,                0b0,                0b0,  0b100011010010000,      0b11010011000,                0b0,
     0b1010001101010,  0b100011110010110,                0b0,                0b0,                0b0,  0b100011100110000,      0b11100111000,                0b0,
     0b1110001001000,  0b100011100010010,                0b0,                0b0,                0b0,  0b100011011110010,      0b11011111100,                0b0,
    0b11101010011000,      0b10101111000,                0b0,                0b0,     0b111010101100,      0b10010101100,  0b101000010110100,                0b0,
   0b100111001010000,      0b10000101000,  0b101000000001000,                0b0,     0b111010010000,      0b10010010000,  0b101000010011000,                0b0,
     0b1000001101010,      0b10110010110,                0b0,                0b0,                0b0,      0b10100110000,  0b101000100111000,                0b0,
   0b101101001001000,      0b10100010010,                0b0,                0b0,                0b0,      0b10011110010,  0b101000011111100,                0b0,
   0b101010001011000,   0b11000101111000,                0b0,                0b0,                0b0,   0b11000010101100,  0b100001010110100,                0b0,
   0b100100001001100,   0b11000000101000,  0b100001000001000,                0b0,   0b11100010001100,   0b11000010010000,  0b100001010011000,                0b0,
     0b1100001101010,   0b11000110010110,                0b0,                0b0,                0b0,   0b11000100110000,  0b100001100111000,                0b0,
    0b10000001001000,   0b11000100010010,                0b0,                0b0,                0b0,   0b11000011110010,  0b100001011111100,                0b0,
   0b101011001011000,       0b1101111000,                0b0,                0b0,                0b0,       0b1010101100,  0b101001010110100,                0b0,
   0b100110001010000,       0b1000101000,  0b101001000001000,                0b0,   0b11100011010100,       0b1010010000,  0b101001010011000,                0b0,
     0b1101001101010,       0b1110010110,                0b0,                0b0,                0b0,       0b1100110000,  0b101001100111000,                0b0,
   0b101111001001000,       0b1100010010,                0b0,                0b0,                0b0,       0b1011110010,  0b101001011111100,                0b0,
                 0b0,  0b110000101111000,                0b0,                0b0,  0b110010010101100,  0b110000010101100,  0b110001010101100,                0b0,
    0b10111001001000,                0b0,  0b110110001001000,                0b0,  0b110010010010000,  0b110000010010000,  0b110001010010000,                0b0,
      0b100001101010,  0b110000110011000,                0b0,                0b0,  0b110010100110000,  0b110000100110000,  0b110001101010000,                0b0,
   0b111000001001000,  0b110000100010100,  0b110111001001000,                0b0,                0b0,  0b110000011110100,                0b0,                0b0,
   0b100000000101000,   0b11110101111000,   0b11111000101000,                0b0,  0b100000010101100,   0b11110010101100,   0b11111010101100,                0b0,
   0b110100001001000,   0b11110000101000,  0b110011001001000,                0b0,  0b100000010010000,   0b11110010010000,   0b11111010010000,                0b0,
      0b101001101010,   0b11110110010110,                0b0,                0b0,  0b100000100110000,   0b11110100110000,   0b11111101010000,                0b0,
    0b10001001001000,   0b11110100010010,  0b110101001001000,                0b0,  0b100000011110010,   0b11110011110010,   0b11111100010010,                0b0,
    0b10100000101000,   0b10010101111000,                0b0,                0b0,   0b10100010101100,   0b10010010101100,   0b10101010110100,                0b0,
    0b11011001001000,   0b10010000101000,   0b10110001001000,                0b0,   0b10100010010000,   0b10010010010000,   0b10101010011000,                0b0,
     0b1001001101010,   0b10010110010110,                0b0,                0b0,                0b0,   0b10010100110000,   0b10101100111000,                0b0,
     0b1111001001000,   0b10010100010010,                0b0,                0b0,                0b0,   0b10010011110010,   0b10101011111100,                0b0,
    0b10011000101000,  0b101100101111000,                0b0,                0b0,   0b10011010101100,  0b101100010101100,   0b11001010110100,                0b0,
    0b11010001001000,  0b101100000101000,  0b100010001001000,                0b0,   0b10011010010000,  0b101100010010000,   0b11001010011000,                0b0,
      0b110001101010,  0b101100110010110,                0b0,                0b0,                0b0,  0b101100100110000,   0b11001100111000,                0b0,
   0b101110001001000,  0b101100100010010,                0b0,                0b0,                0b0,  0b101100011110010,   0b11001011111100,                0b0,
};

/*
  Addressing Modes
  Source: http://www.emulator101.com/6502-addressing-modes.html
*/

// Non-Indexed, non memory
#define MODE_ACC  0x0
#define MODE_IMM  0x1
#define MODE_IMP  0x2

// Non-Indexed memory ops
#define MODE_REL  0x3
#define MODE_ABS  0x4
#define MODE_ZPG  0x5
#define MODE_IND  0x6

// Indexed memory ops
#define MODE_ABX  0x7
#define MODE_ABY  0x8
#define MODE_ZPX  0x9
#define MODE_ZPY  0xA
#define MODE_IIX  0xB
#define MODE_IIY  0xC

#endif
