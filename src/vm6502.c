#include "../inc/vm6502.h"
#include <stdlib.h>
#include <string.h>

// TODO: st_push816, st_pop816
// TODO: Possible bug in CP* (No substraction was done)

#define u16 uint16_t
#define u8  uint8_t

/*
void read_addr(VM6502* vm, u16 addr, u16* out)
{
	vm->read(vm, addr, 2, (u8*)out);
}

void read_byte(VM6502* vm, u16 addr, u8* out)
{
	vm->read(vm, addr, 1, out);
}
*/

#define	read_byte(ins, addr, out)	ins->read(ins, addr, 1, out)
#define	read_addr(ins, addr, out)	ins->read(ins, addr, 2, (uint8_t*)(out))
#define	next_byte(ins, out)				read_byte(ins, (ins)->pc++, out)
#define	next_addr(ins, out)				{read_addr(ins, (ins)->pc, out); ins->pc += 2;}

#define	set_flag(ins, flag)				ins->status ^= ((flag) & (ins)->status) ^ (flag)
#define	unset_flag(ins, flag)			ins->status ^= (flag) & (ins)->status
#define	upd_flag(ins, flag, val)	ins->status ^= ((flag) & (ins)->status) ^ ((val) ? (flag) : 0)
#define	fetch_flag(ins, flag)			(((ins)->status & (flag)) > 0)

#define	nz_flags(ins, no)					upd_flag(ins, FLAG_NEGATIVE, (no) & 0x80); \
																	upd_flag(ins, FLAG_ZERO, no == 0);
#define	st_push8(ins, no)					ins->write(ins, ins->Sp | 0x100, 1, &no); \
																	ins->Sp = !ins->Sp ? 0xff : ins->Sp - 1;
#define	st_pop8(ins, out)					ins->Sp = (ins->Sp + 1) & 0xff; \
																	read_byte(ins, ins->Sp | 0x100, out);

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

inline void* VM6502_slot(VM6502* ins)
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
	read_addr(ins, 0xfffc, &addr);
	ins->pc = addr;

	uint8_t in = 0;
	if (ins->cc == 0) {
		ins->status = 0x36;
		ins->Acc = ins->iX =
			ins->iY = ins->cc = 0;
		ins->Sp = 0xff;

		ins->write(ins, 0x4015, 1, &in);
		ins->write(ins, 0x4017, 1, &in);

		uint8_t chunk[20];
		memset(chunk, 0, 20);
		ins->write(ins, 0x4000, 20, chunk);

		// TODO: Reset Noise Channel & APU FC
		return;
	}

	ins->cc = 0;
	ins->write(ins, 0x4015, 1, &in);
	// TODO: Set status ORed with 0x04 & reset APU things
	set_flag(ins, FLAG_INTDIS);
}

#ifdef MOS6502_DEBUG
void __MOS6502_DEBUG(VM6502*);
#endif

// TODO: Better io_write
// TODO: Optimize upd_flag
// TODO: Check if page boundary crossed
uintmx_t VM6502_run_eff(VM6502* ins, uintmx_t cycles)
{
	ins->cc = 0;
	while (ins->cc < cycles && !ins->ExInterrupt)
	{
		uint8_t raw_op;
		next_byte(ins, &raw_op);

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
				next_byte(ins, &b1);
				faddr = ins->pc + (int8_t)b1;
				break;
			case MODE_ABS:
				next_addr(ins, &faddr);
				break;
			case MODE_IND:
				next_addr(ins, &faddr);
				if (faddr && 0xff == 0xff)
				{
					uint8_t b2, msb = faddr >> 8;
					read_byte(ins, faddr, &b1);
					read_byte(ins, (msb << 8) + ((faddr + 1) & 0xff), &b2);
					faddr = (b2 << 8) | b1;
				} else
					read_addr(ins, faddr, &faddr);
				break;
			case MODE_ZPG:
				next_byte(ins, &b1);
				faddr = b1;
				break;
			case MODE_ABX:
				next_addr(ins, &faddr);
				ett += ((faddr + (int8_t)ins->iX) & 0xff00) != (ins->pc & 0xff00);
				faddr += (int8_t)ins->iX;
				break;
			case MODE_ABY:
				next_addr(ins, &faddr);
				ett += ((faddr + (int8_t)ins->iY) & 0xff00) != (ins->pc & 0xff00);
				faddr += (int8_t)ins->iY;
				break;
			case MODE_ZPX:
				next_byte(ins, &b1);
				faddr = (b1 + (int8_t)ins->iX) % 256;
				break;
			case MODE_ZPY:
				next_byte(ins, &b1);
				faddr = (b1 + (int8_t)ins->iY) % 256;
				break;
			case MODE_IIX:
			{
				uint8_t b3;
				next_byte(ins, &b1);
				read_byte(ins, (b1 + (int8_t)ins->iX) % 256, &b2);
				read_byte(ins, (b1 + (int8_t)ins->iX + 1) % 256, &b3);
				faddr = b2 + (b3 << 8);
				break;
			}
			case MODE_IIY:
			{
				uint8_t b3;
				next_byte(ins, &b1);
				read_byte(ins, b1, &b2);
				read_byte(ins, (b1 + 1) % 256, &b3);
				ett += ((b2 + (b3 << 8) + ins->iY) & 0xff00) != (ins->pc & 0xff00);
				faddr = b2 + (b3 << 8) + ins->iY;	 // TODO: Probably wrong
				break;
			}
			default:
				break;
		}

		switch (op)
		{
			case OP_ADC:
				read_byte(ins, faddr, &b1);
				faddr = ins->Acc + b1 + fetch_flag(ins, FLAG_CARRY);
				upd_flag(ins, FLAG_CARRY, faddr > 0xff);
				upd_flag(ins, FLAG_OVERFLOW, (~(ins->Acc ^ b1) & (ins->Acc ^ faddr) & 0x80));
				ins->Acc = faddr & 0xff;
				nz_flags(ins, ins->Acc);
				break;
			case OP_AND:
				read_byte(ins, faddr, &b1);
				ins->Acc &= b1;
				nz_flags(ins, ins->Acc);
				break;
			case OP_ASL:
				if (am == MODE_ACC)
				{
					b1 = (ins->Acc << 1) & 0xff;
					b2 = ins->Acc & 0x80;
					ins->Acc = b1;
				} else
				{
					read_byte(ins, faddr, &b1);
					b2 = b1 & 0x80;
					b1 = (b1 << 1) & 0xff;
					ins->write(ins, faddr, 1, &b1);
				}

				nz_flags(ins, b1);
				upd_flag(ins, FLAG_CARRY, b2);
				break;
			case OP_BCC:
				if (!fetch_flag(ins, FLAG_CARRY))
				{ ett = 1; ins->pc = faddr; }
				break;
			case OP_BCS:
				if (fetch_flag(ins, FLAG_CARRY))
				{ ett = 1; ins->pc = faddr; }
				break;
			case OP_BEQ:
				if (fetch_flag(ins, FLAG_ZERO))
				{ ett = 1; ins->pc = faddr; }
				break;
			case OP_BIT:
				if (am == MODE_ACC)
				{
					nz_flags(ins, ins->Acc);
					upd_flag(ins, FLAG_OVERFLOW, ins->Acc & 0x40);
				} else
				{
					read_byte(ins, faddr, &b1);
					uint8_t res = ins->Acc & b1;
					nz_flags(ins, res);
					upd_flag(ins, FLAG_OVERFLOW, res & 0x40);
				}
				break;
			case OP_BMI:
				if (fetch_flag(ins, FLAG_NEGATIVE))
				{ ett = 1; ins->pc = faddr; }
				break;
			case OP_BNE:
				if (!fetch_flag(ins, FLAG_ZERO))
				{ ett = 1; ins->pc = faddr; }
				break;
			case OP_BPL: // Approved
				if (!fetch_flag(ins, FLAG_NEGATIVE))
				{ ett = 1; ins->pc = faddr; }
				break;
			case OP_BRK:
				b1 = (ins->pc >> 8) & 0xff;
				st_push8(ins, b1);
				b1 = ins->pc & 0xff;
				st_push8(ins, b1);
				b1 = ins->status | FLAG_BREAK;
				st_push8(ins, b1);
				set_flag(ins, FLAG_INTDIS);
				read_addr(ins, 0xfffe, &faddr);
				ins->pc = faddr;
				break;
			case OP_BVC:
				if (!fetch_flag(ins, FLAG_OVERFLOW))
				{ ett = 1; ins->pc = faddr; }
				break;
			case OP_BVS:
				if (fetch_flag(ins, FLAG_OVERFLOW))
				{ ett = 1; ins->pc = faddr; }
				break;
			case OP_CLC:
				unset_flag(ins, FLAG_CARRY);
				break;
			case OP_CLD:
				unset_flag(ins, FLAG_DECIMAL);
				break;
			case OP_CLI:
				unset_flag(ins, FLAG_INTDIS);
				break;
			case OP_CLV:
				unset_flag(ins, FLAG_OVERFLOW);
				break;
			case OP_CMP:
				read_byte(ins, faddr, &b1);
				upd_flag(ins, FLAG_CARRY, ins->Acc >= b1);
				nz_flags(ins, ins->Acc - b1);	// TODO: Possible bug
				break;
			case OP_CPX:
				read_byte(ins, faddr, &b1);
				upd_flag(ins, FLAG_CARRY, ins->iX >= b1);
				nz_flags(ins, ins->iX - b1);	// TODO: Possible bug
				break;
			case OP_CPY:
				read_byte(ins, faddr, &b1);
				upd_flag(ins, FLAG_CARRY, ins->iY >= b1);
				nz_flags(ins, ins->iY - b1);	// TODO: Possible bug
				break;
			case OP_DEC:
				read_byte(ins, faddr, &b1);
				--b1;
				// TODO: Try to do a 'unsafe_write'
				ins->write(ins, faddr, 1, &b1);
				nz_flags(ins, b1);
				break;
			case OP_DEX:
				ins->iX = ins->iX - 1;
				nz_flags(ins, ins->iX);
				break;
			case OP_DEY:
				ins->iY = ins->iY - 1;
				nz_flags(ins, ins->iY);
				break;
			case OP_EOR:
				read_byte(ins, faddr, &b1);
				ins->Acc ^= b1;
				nz_flags(ins, ins->Acc);
				break;
			case OP_INC:
				read_byte(ins, faddr, &b1);
				++b1;
				nz_flags(ins, b1);
				ins->write(ins, faddr, 1, &b1);
				// TODO: OVERFLOW???
				break;
			case OP_INX:
				ins->iX = ins->iX + 1;
				nz_flags(ins, ins->iX);
				break;
			case OP_INY:
				ins->iY = ins->iY + 1;
				nz_flags(ins, ins->iY);
				break;
			case OP_JMP:	// Partially implemented, see http://www.6502.org/tutorials/6502opcodes.html#JMP for more info
				ins->pc = faddr;
				break;
			case OP_JSR:
				ins->pc--;
				b1 = (ins->pc >> 8) & 0xff;
				st_push8(ins, b1);
				b1 = ins->pc & 0xff;
				st_push8(ins, b1);
				ins->pc = faddr;
				break;
			case OP_LDA:
				read_byte(ins, faddr, &ins->Acc);
				nz_flags(ins, ins->Acc);
				break;
			case OP_LDX:
				read_byte(ins, faddr, &ins->iX);
				nz_flags(ins, ins->iX);
				break;
			case OP_LDY:
				read_byte(ins, faddr, &ins->iY);
				nz_flags(ins, ins->iY);
				break;
			case OP_LSR:
				if (am == MODE_ACC)
				{
					b1 = ins->Acc >> 1;
					b2 = ins->Acc & 0x1;
					ins->Acc = b1;
				} else
				{
					read_byte(ins, faddr, &b1);
					b2 = b1 & 0x1;
					b1 >>= 1;
					ins->write(ins, faddr, 1, &b1);
				}

				upd_flag(ins, FLAG_CARRY, b2);
				nz_flags(ins, b1);
				break;
			case OP_NOP:
				break;
			case OP_ORA:
				read_byte(ins, faddr, &b1);
				ins->Acc |= b1;
				nz_flags(ins, ins->Acc);
				break;
			case OP_PHA:
				ins->write(ins, 0x100 + ins->Sp--, 1, &ins->Acc);
				break;
			case OP_PHP:
				ins->write(ins, 0x100 + ins->Sp++, 1, &ins->status);
				break;
			case OP_PLA:
				read_byte(ins, 0x100 + ++ins->Sp, &ins->Acc);
				nz_flags(ins, ins->Acc);
				break;
			case OP_PLP:
				read_byte(ins, 0x100 + --ins->Sp, &ins->status);
				break;
			case OP_ROL:
				if (am == MODE_ACC)
				{
					b2 = ins->Acc & 0x80;
					b1 = ((ins->Acc << 1) & 0xff) + fetch_flag(ins, FLAG_CARRY);
					ins->Acc = b1;
				} else
				{
					read_byte(ins, faddr, &b1);
					b2 = b1 & 0x80;
					b1 = ((b1 << 1) & 0xff) + fetch_flag(ins, FLAG_CARRY);
					ins->write(ins, faddr, 1, &b1);
				}

				nz_flags(ins, b1);
				upd_flag(ins, FLAG_CARRY, b2);
				break;
			case OP_ROR:
				if (am == MODE_ACC)
				{
					b2 = ins->Acc & 0x1;
					b1 = (ins->Acc >> 1) + (fetch_flag(ins, FLAG_CARRY) << 7);
					ins->Acc = b1;
				} else
				{
					read_byte(ins, faddr, &b1);
					b2 = b1 & 0x1;
					b1 = (b1 >> 1) + (fetch_flag(ins, FLAG_CARRY) << 7);
					ins->write(ins, faddr, 1, &b1);
				}

				nz_flags(ins, b1);
				upd_flag(ins, FLAG_CARRY, b2);
				break;
			case OP_RTI:
				st_pop8(ins, &ins->status);
				st_pop8(ins, &b1); st_pop8(ins, &b2);
				ins->pc = (b2 << 8) + b1;
				break;
			case OP_RTS:
				st_pop8(ins, &b1); st_pop8(ins, &b2);
				ins->pc = (b2 << 8) + b1 + 1;
				break;
			case OP_SBC:
				read_byte(ins, faddr, &b1);
				b1 = ~b1;
				faddr = ins->Acc + b1 + fetch_flag(ins, FLAG_CARRY);
				upd_flag(ins, FLAG_CARRY, faddr > 0xff);
				upd_flag(ins, FLAG_OVERFLOW, (~(ins->Acc ^ b1) & (ins->Acc ^ faddr) & 0x80));
				ins->Acc = faddr & 0xff;
				nz_flags(ins, ins->Acc);
				break;
			case OP_SEC:
				set_flag(ins, FLAG_CARRY);
				break;
			case OP_SED:
				set_flag(ins, FLAG_DECIMAL);
				break;
			case OP_SEI:
				set_flag(ins, FLAG_INTDIS);
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
				nz_flags(ins, ins->Acc);
				break;
			case OP_TAY:
				ins->iY = ins->Acc;
				nz_flags(ins, ins->Acc);
				break;
			case OP_TSX:
				ins->iX = ins->Sp;
				nz_flags(ins, ins->Sp);
				break;
			case OP_TXA:
				ins->Acc = ins->iX;
				nz_flags(ins, ins->iX);
				break;
			case OP_TXS:
				ins->Sp = ins->iX;
				nz_flags(ins, ins->iX);
				break;
			case OP_TYA:
				ins->Acc = ins->iY;
				nz_flags(ins, ins->iY);
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
