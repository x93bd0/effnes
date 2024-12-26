#ifndef OPS6502_H
#define OPS6502_H
#include <stdint.h>

/*
  Official Opcodes
  Source: http://www.6502.org/tutorials/6502opcodes.html
*/

{opcodes_defs}

/*
  Op Codes Jump Table
  000000000000000
  ^^^^^^    ^^^ ^
  OpCode^^^^Tim^U
        AdMd   E

  AdMd: Addressing Mode
  Tim:  Execution Time - 1
  E:    Extra Time if Page Boundary Crossed
  U:    Unused
*/

static uint16_t JUMPTABLE[0x100] = {{
{jumptable}
}};

/*
  Addressing Modes
  Source: http://www.emulator101.com/6502-addressing-modes.html
*/

// Non-Indexed, non memory
#define ADDRMODE_ACCUM	0x0
#define ADDRMODE_IMMED	0x1
#define ADDRMODE_IMPLD	0x2

// Non-Indexed memory ops
#define ADDRMODE_RELAT	0x3
#define ADDRMODE_ABSOL	0x4
#define ADDRMODE_ZRPAG	0x5
#define ADDRMODE_INDIR	0x6

// Indexed memory ops
#define ADDRMODE_ABSOX	0x7
#define ADDRMODE_ABSOY	0x8
#define ADDRMODE_ZRPAX	0x9
#define ADDRMODE_ZRPAY	0xA
#define ADDRMODE_INDIX	0xB
#define ADDRMODE_INDIY	0xC

#define FLAG_CARRY    0b1
#define FLAG_ZERO     0b10
#define FLAG_INTDIS   0b100
#define FLAG_DECIMAL  0b1000
#define FLAG_BREAK    0b10000
#define FLAG_RESERVED 0b100000
#define FLAG_OVERFLOW 0b1000000
#define FLAG_NEGATIVE 0b10000000

#define NMI_VECTOR  0xFFFA
#define RST_VECTOR  0xFFFC
#define BRK_VECTOR  0xFFFE

#endif
