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
