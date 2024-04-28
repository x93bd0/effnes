#include "../inc/vm6502.h"
#include <stdlib.h>
#include <string.h>

#define READ_BYTE(ins, addr, out) ins->read(ins, addr, 1, out)
#define READ_ADDR(ins, addr, out) ins->read(ins, addr, 2, (uint8_t*)out)

#define NEXT_BYTE(ins, out) READ_BYTE(ins, ins->pc++, out)
#define NEXT_ADDR(ins, out) {READ_ADDR(ins, ins->pc, out); ins->pc += 2;}

#define SET_FLAG(ins, flag) \
  ins->status ^= (flag & ins->status) ^ flag;

#define UNSET_FLAG(ins, flag) \
  ins->status ^= flag & ins->status;

#define UPD_FLAG(ins, flag, val) \
  if ((val) > 0) {SET_FLAG(ins, flag);} else UNSET_FLAG(ins, flag);

#define FETCH_FLAG(ins, flag) \
  ((ins->status & flag) > 0)

typedef struct VM6502 {
  void* slot;
  uint8_t halted;

  uint16_t pc;
  uint8_t iX, iY,
          Acc, Sp,
          status;
  uintmx_t cc;

  LFRMethod read;
  WTRMethod write;
  uint8_t ExInterrupt;
} VM6502;

VM6502* VM6502_init(LFRMethod read, WTRMethod write)
{
  VM6502* ins = malloc(sizeof(VM6502));
  ins->read = read, ins->write = write, ins->slot = NULL;
  ins->cc = ins->halted = 0;
  return ins;
}

void VM6502_store(VM6502* ins, void* slotdata)
{
  ins->slot = slotdata;
}

void* VM6502_slot(VM6502* ins)
{
  return ins->slot;
}

/*
  - VM6502_reset
  Resets the Intel 6502 processor to its initial status,
  RAM remains unchanged

  Source: https://www.nesdev.org/wiki/CPU_power_up_state
*/

void VM6502_reset(VM6502* ins)
{
  ins->ExInterrupt = 0;
  uint16_t addr;
  READ_ADDR(ins, 0xfffc, &addr);
  ins->pc = addr;

  uint8_t in = 0;
  if (ins->cc == 0) {
    ins->status = 0x34;
    ins->Acc = ins->iX =
      ins->iY = ins->cc = 0;
    ins->Sp = 0xFD;

    ins->write(ins, 0x4015, 1, &in);
    ins->write(ins, 0x4017, 1, &in);

    uint8_t chunks[20];
    memset(chunks, 0, 20);
    ins->write(ins, 0x4000, 20, chunks);

    // TODO: Reset Noise Channel & APU FC
    return;
  }

  ins->cc = 0;
  ins->write(ins, 0x4015, 1, &in);
  // TODO: Set status ORed with 0x04 & reset APU things
  SET_FLAG(ins, FLAG_INTDIS);
}

#define SET_NZ_FLAGS(ins, val) \
  UPD_FLAG(ins, FLAG_NEGATIVE, (val & 0x80) > 0); \
  UPD_FLAG(ins, FLAG_ZERO, !val)

#define ST_PUSH(ins, val) { \
  ins->write(ins, ins->Sp | 0x100, 1, &val); \
  ins->Sp = ins->Sp == 0 ? 255 : ins->Sp - 1; \
}
#define ST_POP(ins, out) { \
  ins->Sp = (ins->Sp + 1) & 0xff; \
  READ_BYTE(ins, ins->Sp | 0x100, out); \
}

#ifdef MOS6502_DEBUG
void __MOS6502_DEBUG(VM6502*);
#endif

// TODO: Better io_write
// TODO: Optimize UPD_FLAG
// TODO: Check if page boundary crossed
uintmx_t VM6502_run_eff(VM6502* ins, uintmx_t cycles)
{
  ins->cc = 0;
  while (ins->cc < cycles)
  {
    uint8_t raw_op;
    NEXT_BYTE(ins, &raw_op);

    if (!JUMPTABLE[raw_op])
    {
      ins->halted = 1;
      break;
    }

    uint8_t op = JUMPTABLE[raw_op] >> 9,
      am = (JUMPTABLE[raw_op] >> 5) & 0b1111,
      tim = (JUMPTABLE[raw_op] >> 2) & 0b111,
      ett = (JUMPTABLE[raw_op] >> 1) & 0x1;

    uint16_t faddr;
    uint8_t b1, b2;

    switch (am)
    {
      case MODE_IMM:
        faddr = ins->pc++;
        break;
      case MODE_REL:
        NEXT_BYTE(ins, &b1);
        faddr = ins->pc + (int8_t)b1;
        break;
      case MODE_ABS:
        NEXT_ADDR(ins, &faddr);
        break;
      case MODE_IND:
        NEXT_ADDR(ins, &faddr);
        if (faddr && 0xff == 0xff)
        {
          uint8_t b2;
          READ_BYTE(ins, faddr & 0xff, &b1);
          READ_BYTE(ins, faddr, &b2);
          faddr = (b1 << 8) | b2;
        } else
          READ_ADDR(ins, faddr, &faddr);
        break;
      case MODE_ZPG:
        NEXT_BYTE(ins, &b1);
        faddr = b1;
        break;
      case MODE_ABX:
        NEXT_ADDR(ins, &faddr);
        ett += ((faddr + (int8_t)ins->iX) & 0xff00) != (ins->pc & 0xff00);
        faddr += (int8_t)ins->iX;
        break;
      case MODE_ABY:
        NEXT_ADDR(ins, &faddr);
        ett += ((faddr + (int8_t)ins->iY) & 0xff00) != (ins->pc & 0xff00);
        faddr += (int8_t)ins->iY;
        break;
      case MODE_ZPX:
        NEXT_BYTE(ins, &b1);
        faddr = (b1 + (int8_t)ins->iX) % 256;
        break;
      case MODE_ZPY:
        NEXT_BYTE(ins, &b1);
        faddr = (b1 + (int8_t)ins->iY) % 256;
        break;
      case MODE_IIX:
      {
        uint8_t b3;
        NEXT_BYTE(ins, &b1);
        READ_BYTE(ins, (b1 + (int8_t)ins->iX) % 256, &b2);
        READ_BYTE(ins, (b1 + (int8_t)ins->iX + 1) % 256, &b3);
        faddr = b2 + (b3 << 8);
        break;
      }
      case MODE_IIY:
      {
        uint8_t b3;
        NEXT_BYTE(ins, &b1);
        READ_BYTE(ins, b1, &b2);
        READ_BYTE(ins, (b1 + 1) % 256, &b3);
        ett += ((b2 + (b3 << 8) + ins->iY) & 0xff00) != (ins->pc & 0xff00);
        faddr = b2 + (b3 << 8) + ins->iY;   // TODO: Probably wrong
        break;
      }
      default:
        break;
    }

    switch (op)
    {
      case OP_ADC:
        READ_BYTE(ins, faddr, &b1);
        faddr = ins->Acc + b1 + FETCH_FLAG(ins, FLAG_CARRY);
        UPD_FLAG(ins, FLAG_CARRY, faddr > 0xff);
        UPD_FLAG(ins, FLAG_OVERFLOW, (~(ins->Acc ^ b1) & (ins->Acc ^ faddr) & 0x80));
        ins->Acc = faddr & 0xff;
        SET_NZ_FLAGS(ins, ins->Acc);
        break;
      case OP_AND:
        READ_BYTE(ins, faddr, &b1);
        ins->Acc &= b1;
        SET_NZ_FLAGS(ins, ins->Acc);
        break;
      case OP_ASL:
        if (am == MODE_ACC)
        {
          b1 = (ins->Acc << 1) & 0xff;
          b2 = ins->Acc & 0x80;
          ins->Acc = b1;
        } else
        {
          READ_BYTE(ins, faddr, &b1);
          b2 = b1 & 0x80;
          b1 = (b1 << 1) & 0xff;
          ins->write(ins, faddr, 1, &b1);
        }

        SET_NZ_FLAGS(ins, b1);
        UPD_FLAG(ins, FLAG_CARRY, b2);
        break;
      case OP_BCC:
        if (!FETCH_FLAG(ins, FLAG_CARRY))
        { ett = 1; ins->pc = faddr; }
        break;
      case OP_BCS:
        if (FETCH_FLAG(ins, FLAG_CARRY))
        { ett = 1; ins->pc = faddr; }
        break;
      case OP_BEQ:
        if (FETCH_FLAG(ins, FLAG_ZERO))
        { ett = 1; ins->pc = faddr; }
        break;
      case OP_BIT:
        if (am == MODE_ACC)
        {
          SET_NZ_FLAGS(ins, ins->Acc);
          UPD_FLAG(ins, FLAG_OVERFLOW, ins->Acc & 0x40);
        } else
        {
          READ_BYTE(ins, faddr, &b1);
          uint8_t res = ins->Acc & b1;
          SET_NZ_FLAGS(ins, res);
          UPD_FLAG(ins, FLAG_OVERFLOW, res & 0x40);
        }
        break;
      case OP_BMI:
        if (FETCH_FLAG(ins, FLAG_NEGATIVE))
        { ett = 1; ins->pc = faddr; }
        break;
      case OP_BNE:
        if (!FETCH_FLAG(ins, FLAG_ZERO))
        { ett = 1; ins->pc = faddr; }
        break;
      case OP_BPL: // Approved
        if (!FETCH_FLAG(ins, FLAG_NEGATIVE))
        { ett = 1; ins->pc = faddr; }
        break;
      case OP_BRK:
        b1 = (ins->pc >> 8) & 0xff;
        ST_PUSH(ins, b1);
        b1 = ins->pc & 0xff;
        ST_PUSH(ins, b1);
        b1 = ins->status | FLAG_BREAK;
        ST_PUSH(ins, b1);
        SET_FLAG(ins, FLAG_INTDIS);
        READ_ADDR(ins, 0xfffe, &faddr);
        ins->pc = faddr;
        break;
      case OP_BVC:
        if (!FETCH_FLAG(ins, FLAG_OVERFLOW))
        { ett = 1; ins->pc = faddr; }
        break;
      case OP_BVS:
        if (FETCH_FLAG(ins, FLAG_OVERFLOW))
        { ett = 1; ins->pc = faddr; }
        break;
      case OP_CLC:
        UNSET_FLAG(ins, FLAG_CARRY);
        break;
      case OP_CLD:
        UNSET_FLAG(ins, FLAG_DECIMAL);
        break;
      case OP_CLI:
        UNSET_FLAG(ins, FLAG_INTDIS);
        break;
      case OP_CLV:
        UNSET_FLAG(ins, FLAG_OVERFLOW);
        break;
      case OP_CMP:
        READ_BYTE(ins, faddr, &b1);
        UPD_FLAG(ins, FLAG_CARRY, ins->Acc >= b1);
        SET_NZ_FLAGS(ins, ins->Acc);  // TODO: Possible bug
        break;
      case OP_CPX:
        READ_BYTE(ins, faddr, &b1);
        UPD_FLAG(ins, FLAG_CARRY, ins->iX >= b1);
        SET_NZ_FLAGS(ins, ins->iX);  // TODO: Possible bug
        break;
      case OP_CPY:
        READ_BYTE(ins, faddr, &b1);
        UPD_FLAG(ins, FLAG_CARRY, ins->iY >= b1);
        SET_NZ_FLAGS(ins, ins->iY);  // TODO: Possible bug
        break;
      case OP_DEC:
        READ_BYTE(ins, faddr, &b1);
        --b1;
        // TODO: Try to do a 'unsafe_write'
        ins->write(ins, faddr, 1, &b1);
        SET_NZ_FLAGS(ins, b1);
        break;
      case OP_DEX:
        ins->iX = ins->iX - 1;
        SET_NZ_FLAGS(ins, ins->iX);
        break;
      case OP_DEY:
        ins->iY = ins->iY - 1;
        SET_NZ_FLAGS(ins, ins->iY);
        break;
      case OP_EOR:
        READ_BYTE(ins, faddr, &b1);
        ins->Acc ^= b1;
        SET_NZ_FLAGS(ins, ins->Acc);
        break;
      case OP_INC:
        READ_BYTE(ins, faddr, &b1);
        ++b1;
        ins->write(ins, faddr, 1, &b1);
        // TODO: OVERFLOW???
        break;
      case OP_INX:
        ins->iX = ins->iX + 1;
        break;
      case OP_INY:
        ins->iY = ins->iY + 1;
        break;
      case OP_JMP:  // Partially implemented, see http://www.6502.org/tutorials/6502opcodes.html#JMP for more info
        ins->pc = faddr;
        break;
      case OP_JSR:
        ins->pc--;
        b1 = (ins->pc >> 8) & 0xff;
        ST_PUSH(ins, b1);
        b1 = ins->pc & 0xff;
        ST_PUSH(ins, b1);
        ins->pc = faddr;
        break;
      case OP_LDA:
        READ_BYTE(ins, faddr, &ins->Acc);
        SET_NZ_FLAGS(ins, ins->Acc);
        break;
      case OP_LDX:
        READ_BYTE(ins, faddr, &ins->iX);
        SET_NZ_FLAGS(ins, ins->iX);
        break;
      case OP_LDY:
        READ_BYTE(ins, faddr, &ins->iY);
        SET_NZ_FLAGS(ins, ins->iY);
        break;
      case OP_LSR:
        if (am == MODE_ACC)
        {
          b1 = ins->Acc >> 1;
          b2 = ins->Acc & 0x1;
          ins->Acc = b1;
        } else
        {
          READ_BYTE(ins, faddr, &b1);
          b2 = b1 & 0x1;
          b1 >>= 1;
          ins->write(ins, faddr, 1, &b1);
        }

        UPD_FLAG(ins, FLAG_CARRY, b2);
        SET_NZ_FLAGS(ins, b1);
        break;
      case OP_NOP:
        break;
      case OP_ORA:
        READ_BYTE(ins, faddr, &b1);
        ins->Acc |= b1;
        SET_NZ_FLAGS(ins, ins->Acc);
        break;
      case OP_PHA:
        ins->write(ins, 0x100 + ins->Sp++, 1, &ins->Acc);
        break;
      case OP_PHP:
        ins->write(ins, 0x100 + ins->Sp++, 1, &ins->status);
        break;
      case OP_PLA:
        READ_BYTE(ins, 0x100 + --ins->Sp, &ins->Acc);
        break;
      case OP_PLP:
        READ_BYTE(ins, 0x100 + --ins->Sp, &ins->status);
        break;
      case OP_ROL:
        if (am == MODE_ACC)
        {
          b2 = ins->Acc & 0x80;
          b1 = ((ins->Acc << 1) & 0xff) + FETCH_FLAG(ins, FLAG_CARRY);
          ins->Acc = b1;
        } else
        {
          READ_BYTE(ins, faddr, &b1);
          b2 = b1 & 0x80;
          b1 = ((b1 << 1) & 0xff) + FETCH_FLAG(ins, FLAG_CARRY);
          ins->write(ins, faddr, 1, &b1);
        }

        SET_NZ_FLAGS(ins, b1);
        UPD_FLAG(ins, FLAG_CARRY, b2);
        break;
      case OP_ROR:
        if (am == MODE_ACC)
        {
          b2 = ins->Acc & 0x1;
          b1 = (ins->Acc >> 1) + (FETCH_FLAG(ins, FLAG_CARRY) << 7);
          ins->Acc = b1;
        } else
        {
          READ_BYTE(ins, faddr, &b1);
          b2 = b1 & 0x1;
          b1 = (b1 >> 1) + (FETCH_FLAG(ins, FLAG_CARRY) << 7);
          ins->write(ins, faddr, 1, &b1);
        }

        SET_NZ_FLAGS(ins, b1);
        UPD_FLAG(ins, FLAG_CARRY, b2);
        break;
      case OP_RTI:
        ST_POP(ins, &ins->status);
        ST_POP(ins, &b1); ST_POP(ins, &b2);
        ins->pc = (b2 << 8) + b1;
        break;
      case OP_RTS:
        ST_POP(ins, &b1); ST_POP(ins, &b2);
        ins->pc = (b2 << 8) + b1 + 1;
        break;
      case OP_SBC:
        READ_BYTE(ins, faddr, &b1);
        b1 = ~b1;
        faddr = ins->Acc + b1 + FETCH_FLAG(ins, FLAG_CARRY);
        UPD_FLAG(ins, FLAG_CARRY, faddr > 0xff);
        UPD_FLAG(ins, FLAG_OVERFLOW, (~(ins->Acc ^ b1) & (ins->Acc ^ faddr) & 0x80));
        ins->Acc = faddr & 0xff;
        SET_NZ_FLAGS(ins, ins->Acc);
        break;
      case OP_SEC:
        SET_FLAG(ins, FLAG_CARRY);
        break;
      case OP_SED:
        SET_FLAG(ins, FLAG_DECIMAL);
        break;
      case OP_SEI:
        SET_FLAG(ins, FLAG_INTDIS);
        break;
      case OP_STA:
        ins->write(ins, faddr, 1, &ins->Acc);
        break;
      case OP_STX:
        ins->write(ins, faddr, 1, &ins->iX);
        break;
      case OP_STY:
        ins->write(ins, faddr, 1, &ins->iY);
        break;
      case OP_TAX:
        ins->iX = ins->Acc;
        SET_NZ_FLAGS(ins, ins->Acc);
        break;
      case OP_TAY:
        ins->iY = ins->Acc;
        SET_NZ_FLAGS(ins, ins->Acc);
        break;
      case OP_TSX:
        ins->iX = ins->Sp;
        SET_NZ_FLAGS(ins, ins->Sp);
        break;
      case OP_TXA:
        ins->Acc = ins->iX;
        SET_NZ_FLAGS(ins, ins->iX);
        break;
      case OP_TXS:
        ins->Sp = ins->iX;
        SET_NZ_FLAGS(ins, ins->iX);
        break;
      case OP_TYA:
        ins->Acc = ins->iY;
        SET_NZ_FLAGS(ins, ins->iY);
        break;
      default:
        // TODO: Throw error
        break;
    }

    ins->cc += tim + (ett == 2);
    if (ins->halted) break;
  }

  return ins->cc;
}
