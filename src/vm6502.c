#include "../inc/vm6502.h"
#include "../inc/ops6502.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>  // TODO: Remove

#define READ_BYTE(ins, addr, out)	ins->read(ins, addr, 1, out)
#define READ_ADDR(ins, addr, out)	ins->read(ins, addr, 2, (uint8_t*)(out))
#define WRITE_BYTE(ins, addr, in)	ins->write(ins, addr, 1, &in)
#define NEXT_BYTE(ins, out)			READ_BYTE(ins, (ins)->PC++, out)
#define NEXT_ADDR(ins, out)			{\
	READ_ADDR(ins, (ins)->PC, out); \
	ins->PC += 2; \
}

#define FLAG_ENB(ins, flag)			(ins)->P |= flag
#define FLAG_DIS(ins, flag)			(ins)->P &= ~(flag)
#define FLAG_GET(ins, flag)			((ins)->P & (flag))
#define FLAG_UPD(ins, flag, stat)	if (stat) { FLAG_ENB(ins, flag); } \
									else { FLAG_DIS(ins, flag); }

#define FLAGS_NZ(ins, data)			{\
	FLAG_UPD(ins, FLAG_NEGATIVE, (data) & 0x80); \
	FLAG_UPD(ins, FLAG_ZERO, !(data)); \
}

#define STACK_PSH8(ins, data)		{WRITE_BYTE(ins, ins->S-- | 0x100, data);printf("PUSH %#2x ", data);}
#define STACK_POP8(ins, out)		{READ_BYTE(ins, ++ins->S | 0x100, out);printf("READ %#2x ", *out);}
#define STACK_PSH16(ins, data)		{ \
	uint8_t temp = data >> 8; \
	STACK_PSH8(ins, temp); \
	temp = data & 0xff; \
	STACK_PSH8(ins, temp); \
}
#define STACK_POP16(ins, out)		{ \
	uint8_t temp; \
	STACK_POP8(ins, &temp); \
	STACK_POP8(ins, (uint8_t*)out); \
	(*out) = (*out << 8) + temp; \
}


VM6502* VM6502_init(VM6502_RamIO ramio, void* slot)
{
	VM6502* ins = malloc(sizeof(VM6502));
	ins->read = ramio.read;
	ins->write = ramio.write;

	ins->slot = slot;
	ins->PC = ins->X = ins->Y = ins->A \
			= ins->S = ins->P = ins->H = 0;

	ins->ex_interrupt = 0;
	ins->cycles = 0;

	return ins;
}

void VM6502_store(VM6502* ins, void* data)
{
	ins->slot = data;
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
	ins->ex_interrupt = 0;
	READ_ADDR(ins, RST_VECTOR, &ins->PC);

	uint8_t null = 0;
	WRITE_BYTE(ins, 0x4015, null);

	if (ins->cycles == 0) {
		ins->P = 0x36;
		ins->S = 0xff;
		ins->A = ins->X = ins->Y = 0;

		WRITE_BYTE(ins, 0x4017, null);

		uint8_t chunk[20];
		memset(chunk, 0, 20);
		ins->write(ins, 0x4000, 20, chunk);

		// TODO: Reset Noise Channel & APU FC
		return;
	}

	ins->cycles = 0;
	// TODO: Set status ORed with 0x04 & reset APU things
	FLAG_ENB(ins, FLAG_INTDIS);
}

void VM6502_NMI(VM6502* ins)
{
	uint16_t t_addr;
	uint8_t t_byte;

	t_byte = (ins->PC >> 8) & 0xff;
	STACK_PSH8(ins, t_byte);
	t_byte = ins->PC & 0xff;
	STACK_PSH8(ins, t_byte);
	t_byte = ins->S & (~FLAG_BREAK);
	STACK_PSH8(ins, t_byte);
	FLAG_ENB(ins, FLAG_INTDIS);
	READ_ADDR(ins, NMI_VECTOR, &t_addr);
	ins->PC = t_addr;
}

uintmx_t VM6502_run(VM6502* vm, uintmx_t cycles)
{
	vm->cycles = 0;
	while (vm->cycles < cycles && !vm->ex_interrupt)
	{
		uint8_t opcode;
		NEXT_BYTE(vm, &opcode);

		// Illegal opcode detection
		if (!JUMPTABLE[opcode])
		{
			vm->H = 1;
			break;
		}

		uint8_t internal_opcode	= JUMPTABLE[opcode] >> 9,
				address_mode	= (JUMPTABLE[opcode] >> 5) & 0b1111,
				timing			= (JUMPTABLE[opcode] >> 2) & 0b111,
				special_timing	= (JUMPTABLE[opcode] >> 1) & 0b1;

		uint16_t t_addr = 0;
		uint8_t t_byte1, t_byte2;

		switch (address_mode)
		{
			case ADDRMODE_IMMED:
				t_addr = vm->PC++;
				break;
			case ADDRMODE_RELAT:
				NEXT_BYTE(vm, &t_byte1);
				t_addr = vm->PC + (int8_t)t_byte1;
				break;
			case ADDRMODE_ABSOL:
				NEXT_ADDR(vm, &t_addr);
				break;
			case ADDRMODE_INDIR:
				NEXT_ADDR(vm, &t_addr);
				if (t_addr && 0xFF == 0xFF)
				{
					READ_BYTE(vm, t_addr, &t_byte1);
					READ_BYTE(vm, (t_addr & 0xff00) + ((t_addr + 1) & 0xff), &t_byte2);
					t_addr = (t_byte2 << 8) | t_byte1;
				} else
					READ_ADDR(vm, t_addr, &t_addr);
				break;
			case ADDRMODE_ZRPAG:
				NEXT_BYTE(vm, (uint8_t*)&t_addr);
				break;
			case ADDRMODE_ABSOX:
				NEXT_ADDR(vm, &t_addr);
				special_timing += ((t_addr + vm->X) & 0xff00) != (t_addr & 0xff00);
				t_addr += vm->X;
				break;
			case ADDRMODE_ABSOY:
				NEXT_ADDR(vm, &t_addr);
				special_timing += ((t_addr + vm->Y) & 0xff00) != (t_addr & 0xff00);
				t_addr += vm->Y;
				break;
			case ADDRMODE_ZRPAX:
				NEXT_BYTE(vm, &t_byte1);
				t_addr = (t_byte1 + (int8_t)vm->X) % 0x100;
				break;
			case ADDRMODE_ZRPAY:
				NEXT_BYTE(vm, &t_byte1);
				t_addr = (t_byte1 + (int8_t)vm->Y) % 0x100;
				break;
			case ADDRMODE_INDIX:
				NEXT_BYTE(vm, &t_byte1);
				READ_BYTE(vm, (t_byte1 + (int8_t)vm->X) % 0x100, &t_byte2);
				READ_BYTE(vm, (t_byte1 + (int8_t)vm->X + 1) % 0x100, (uint8_t*)&t_addr);
				t_addr = (t_addr << 8) + t_byte2;
				break;
			case ADDRMODE_INDIY:
				NEXT_BYTE(vm, &t_byte1);
				READ_BYTE(vm, t_byte1, (uint8_t*)&t_addr);
				READ_BYTE(vm, (t_byte1 + 1) % 0x100, &t_byte2);
				t_addr += t_byte2 << 8;
				special_timing += ((t_addr + vm->Y) & 0xff00) != (t_addr & 0xff00);
				t_addr += vm->Y;
				break;
			default:
				break;
		}

		// TODO: make internal_opcode ordered
		// 		(this helps the compiler find the opcodes easily)
		// TODO: Collapse similar opcodeds (T**) into one instruction
		t_byte2 = 0;  // Used as a flag in some cases

		switch (internal_opcode)
		{
			// Memory / Registers
			case OP_LDA:
				READ_BYTE(vm, t_addr, &vm->A);
				FLAGS_NZ(vm, vm->A);
				break;
			case OP_LDX:
				printf("addr:%#4x ", t_addr);
				READ_BYTE(vm, t_addr, &vm->X);
				FLAGS_NZ(vm, vm->X);
				break;
			case OP_LDY:
				READ_BYTE(vm, t_addr, &vm->Y);
				FLAGS_NZ(vm, vm->Y);
				break;
			case OP_LAX:  // Composite
				READ_BYTE(vm, t_addr, &vm->A);
				vm->X = vm->A;
				FLAGS_NZ(vm, vm->A);
				break;

			case OP_SAX:  // Composite
				t_byte1 = vm->A & vm->X;
				WRITE_BYTE(vm, t_addr, t_byte1);
				break;
			case OP_STA:
				WRITE_BYTE(vm, t_addr, vm->A);
				break;
			case OP_STX:
				WRITE_BYTE(vm, t_addr, vm->X);
				break;
			case OP_STY:
				WRITE_BYTE(vm, t_addr, vm->Y);
				break;

			case OP_TAX:
				vm->X = vm->A;
				FLAGS_NZ(vm, vm->A);
				break;
			case OP_TAY:
				vm->Y = vm->A;
				FLAGS_NZ(vm, vm->A);
				break;
			case OP_TSX:
				vm->X = vm->S;
				FLAGS_NZ(vm, vm->S);
				break;
			case OP_TXA:
				vm->A = vm->X;
				FLAGS_NZ(vm, vm->X);
				break;
			case OP_TXS:
				vm->S = vm->X;
				break;
			case OP_TYA:
				vm->A = vm->Y;
				FLAGS_NZ(vm, vm->Y);
				break;

			// Stack
			case OP_PHA:
				STACK_PSH8(vm, vm->A);
				break;
			case OP_PHP:
				t_byte1 = vm->P | FLAG_BREAK;
				STACK_PSH8(vm, t_byte1);
				break;

			case OP_PLA:
				STACK_POP8(vm, &vm->A);
				FLAGS_NZ(vm, vm->A);
				break;
			case OP_PLP:
				STACK_POP8(vm, &vm->P);
				vm->P = (vm->P & ~FLAG_BREAK) | FLAG_RESERVED;
				break;

			// Decrements / Increments
			case OP_DEC:
				READ_BYTE(vm, t_addr, &t_byte1);
				--t_byte1;
				WRITE_BYTE(vm, t_addr, t_byte1);
				FLAGS_NZ(vm, t_byte1);
				break;
			case OP_DEX:
				vm->X--;
				FLAGS_NZ(vm, vm->X);
				break;
			case OP_DEY:
				vm->Y--;
				FLAGS_NZ(vm, vm->Y);
				break;

			case OP_INC:
				READ_BYTE(vm, t_addr, &t_byte1);
				++t_byte1;
				WRITE_BYTE(vm, t_addr, t_byte1);
				FLAGS_NZ(vm, t_byte1);
				break;
			case OP_INX:
				vm->X++;
				FLAGS_NZ(vm, vm->X);
				break;
			case OP_INY:
				vm->Y++;
				FLAGS_NZ(vm, vm->Y);
				break;

			// Arithmetic
			case OP_ISC:  // Composite
				READ_BYTE(vm, t_addr, &t_byte1);
				++t_byte1;
				WRITE_BYTE(vm, t_addr, t_byte1);
				t_byte2 = 1;
			case OP_SBC:
				if (!t_byte2)
					READ_BYTE(vm, t_addr, &t_byte1);
				t_byte1 = ~t_byte1;
				t_byte2 = 1;
			case OP_ADC:
				if (!t_byte2)
					READ_BYTE(vm, t_addr, &t_byte1);
				t_addr = vm->A + t_byte1 + FLAG_GET(vm, FLAG_CARRY);
				FLAG_UPD(vm, FLAG_CARRY, t_addr > 0xff);
				FLAG_UPD(vm, FLAG_OVERFLOW, (~(vm->A ^ t_byte1) & (vm->A ^ t_addr)) & 0x80);
				vm->A = t_addr & 0xff;
				FLAGS_NZ(vm, vm->A);
				break;

			// Logical
			case OP_AND:
				READ_BYTE(vm, t_addr, &t_byte1);
				vm->A &= t_byte1;
				FLAGS_NZ(vm, vm->A);
				break;
			case OP_EOR:
				READ_BYTE(vm, t_addr, &t_byte1);
				vm->A ^= t_byte1;
				FLAGS_NZ(vm, vm->A);
				break;
			case OP_ORA:
				READ_BYTE(vm, t_addr, &t_byte1);
				vm->A |= t_byte1;
				FLAGS_NZ(vm, vm->A);
				break;

			// Shift / Rotate
			case OP_ASL:
				if (address_mode == ADDRMODE_ACCUM)
					t_byte1 = vm->A;
				else
					READ_BYTE(vm, t_addr, &t_byte1);

				FLAG_UPD(vm, FLAG_CARRY, t_byte1 & 0x80)
				t_byte1 <<= 1;

				if (address_mode == ADDRMODE_ACCUM)
					vm->A = t_byte1;
				else
					WRITE_BYTE(vm, t_addr, t_byte1);

				FLAGS_NZ(vm, t_byte1);
				break;
			case OP_LSR:
				if (address_mode == ADDRMODE_ACCUM)
					t_byte1 = vm->A;
				else
					READ_BYTE(vm, t_addr, &t_byte1);

				FLAG_UPD(vm, FLAG_CARRY, t_byte1 & 1);
				t_byte1 >>= 1;

				if (address_mode == ADDRMODE_ACCUM)
					vm->A = t_byte1;
				else
					WRITE_BYTE(vm, t_addr, t_byte1);

				FLAGS_NZ(vm, t_byte1);
				break;
			case OP_ROL:
				if (address_mode == ADDRMODE_ACCUM)
					t_byte1 = vm->A;
				else
					READ_BYTE(vm, t_addr, &t_byte1);

				t_byte2 = FLAG_GET(vm, FLAG_CARRY);
				FLAG_UPD(vm, FLAG_CARRY, t_byte1 & 0x80)
				t_byte1 <<= 1, t_byte1 += t_byte2 ? 1 : 0;

				if (address_mode == ADDRMODE_ACCUM)
					vm->A = t_byte1;
				else
					WRITE_BYTE(vm, t_addr, t_byte1);

				FLAGS_NZ(vm, t_byte1);
				break;
			case OP_ROR:
				if (address_mode == ADDRMODE_ACCUM)
					t_byte1 = vm->A;
				else
					READ_BYTE(vm, t_addr, &t_byte1);

				t_byte2 = FLAG_GET(vm, FLAG_CARRY);
				FLAG_UPD(vm, FLAG_CARRY, t_byte1 & 0x1)
				t_byte1 >>= 1, t_byte1 += t_byte2 ? 0x80 : 0;

				if (address_mode == ADDRMODE_ACCUM)
					vm->A = t_byte1;
				else
					WRITE_BYTE(vm, t_addr, t_byte1);

				FLAGS_NZ(vm, t_byte1);
				break;

			// Flags
			case OP_CLC:
				FLAG_DIS(vm, FLAG_CARRY);
				break;
			case OP_CLD:
				FLAG_DIS(vm, FLAG_DECIMAL);
				break;
			case OP_CLI:
				FLAG_DIS(vm, FLAG_INTDIS);
				break;
			case OP_CLV:
				FLAG_DIS(vm, FLAG_OVERFLOW);
				break;

			case OP_SEC:
				FLAG_ENB(vm, FLAG_CARRY);
				break;
			case OP_SED:
				FLAG_ENB(vm, FLAG_DECIMAL);
				break;
			case OP_SEI:
				FLAG_ENB(vm, FLAG_INTDIS);
				break;

			// Comparisons
			case OP_DCP:  // Composite
				READ_BYTE(vm, t_addr, &t_byte1);
				--t_byte1;
				WRITE_BYTE(vm, t_addr, t_byte1);
				t_byte2 = 1;
			case OP_CMP:
				if (!t_byte2)
					READ_BYTE(vm, t_addr, &t_byte1);
				FLAG_UPD(vm, FLAG_CARRY, vm->A >= t_byte1);
				FLAGS_NZ(vm, vm->A - t_byte1);
				break;
			case OP_CPX:
				READ_BYTE(vm, t_addr, &t_byte1);
				FLAG_UPD(vm, FLAG_CARRY, vm->X >= t_byte1);
				FLAGS_NZ(vm, vm->X - t_byte1);
				break;
			case OP_CPY:
				READ_BYTE(vm, t_addr, &t_byte1);
				FLAG_UPD(vm, FLAG_CARRY, vm->Y >= t_byte1);
				FLAGS_NZ(vm, vm->Y - t_byte1);
				break;

			// Conditional
			case OP_BCC:
				if (!FLAG_GET(vm, FLAG_CARRY))
				{
					special_timing++;
					vm->PC = t_addr;
				}
				break;
			case OP_BCS:
				if (FLAG_GET(vm, FLAG_CARRY))
				{
					special_timing++;
					vm->PC = t_addr;
				}
				break;
			case OP_BEQ:
				if (FLAG_GET(vm, FLAG_ZERO))
				{
					special_timing++;
					vm->PC = t_addr;
				}
				break;
			case OP_BMI:
				if (FLAG_GET(vm, FLAG_NEGATIVE))
				{
					special_timing++;
					vm->PC = t_addr;
				}
				break;
			case OP_BNE:
				if (!FLAG_GET(vm, FLAG_ZERO))
				{
					special_timing++;
					vm->PC = t_addr;
				}
				break;
			case OP_BPL:
				if (!FLAG_GET(vm, FLAG_NEGATIVE))
				{
					special_timing++;
					vm->PC = t_addr;
				}
				break;
			case OP_BVC:
				if (!FLAG_GET(vm, FLAG_OVERFLOW))
				{
					special_timing++;
					vm->PC = t_addr;
				}
				break;
			case OP_BVS:
				if (FLAG_GET(vm, FLAG_OVERFLOW))
				{
					special_timing++;
					vm->PC = t_addr;
				}
				break;

			// Jumps / Subroutines
			case OP_JMP:
				vm->PC = t_addr;
				break;
			case OP_JSR:
				vm->PC--;
				STACK_PSH16(vm, vm->PC);
				vm->PC = t_addr;
				break;
			case OP_RTS:
				STACK_POP16(vm, &vm->PC);
				++vm->PC;
				break;

			// Interrupts
			case OP_BRK:
				STACK_PSH16(vm, vm->PC);
				t_byte1 = vm->P | FLAG_BREAK;
				STACK_PSH8(vm, t_byte1);
				FLAG_ENB(vm, FLAG_INTDIS);
				READ_ADDR(vm, BRK_VECTOR, &t_addr);
				vm->PC = t_addr;
				break;
			case OP_RTI:
				STACK_POP8(vm, &vm->P);
				vm->P = (vm->P & ~FLAG_BREAK) | FLAG_RESERVED;
				STACK_POP16(vm, &vm->PC);
				break;

			// Other
			case OP_BIT:
				READ_BYTE(vm, t_addr, &t_byte1);
				t_byte2 = vm->A & t_byte1;
				FLAGS_NZ(vm, t_byte2);
				FLAG_UPD(vm, FLAG_OVERFLOW, t_byte2 & 0x40);
				vm->P &= 0b00111111;
				vm->P |= t_byte1 & 0b11000000;
				break;
			case OP_NOP:
				break;

			default:
				// TODO: throw error
				vm->H = 1;
				break;
		}

		READ_BYTE(vm, t_addr, &vm->ex_interrupt);
		vm->debug_addr = t_addr;
		vm->cycles += 1 + timing + (special_timing >= 2);
		if (vm->H) break;
	}

	return vm->cycles;
}
