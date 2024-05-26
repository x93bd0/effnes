#ifndef VM6502_H
#define VM6502_H

#include "ops6502.h"
#include <limits.h>

#ifndef uintmx_t
	#define uintmx_t uint32_t
	#define UINTVM_MAX (2<<29)-1
#endif

typedef uint16_t (*LFRMethod)(void* VM, uint16_t ADDR, uint16_t SIZE, uint8_t* OUT);
typedef uint16_t (*WTRMethod)(void* VM, uint16_t ADDR, uint16_t SIZE, uint8_t* INPUT);

typedef struct VM6502 {
	void* slot;

	uint16_t pc;
	uint8_t iX, iY,
		Acc, Sp,
		status, halted;
	uint8_t ExInterrupt;
	uintmx_t cc;

	LFRMethod read;
	WTRMethod write;
} VM6502;

VM6502* VM6502_init(LFRMethod, WTRMethod);
void VM6502_store(VM6502*, void*);
void* VM6502_slot(VM6502*);
void VM6502_reset(VM6502*);

void VM6502_NMI(VM6502*);
uintmx_t VM6502_run(VM6502*, uintmx_t);
uintmx_t VM6502_run_eff(VM6502*, uintmx_t);

#define FLAG_CARRY    0b1
#define FLAG_ZERO     0b10
#define FLAG_INTDIS   0b100
#define FLAG_DECIMAL  0b1000
#define FLAG_BREAK    0b10000
#define FLAG_OVERFLOW 0b1000000
#define FLAG_NEGATIVE 0b10000000

#define NMI_VECTOR  0xfffa
#define RST_VECTOR  0xfffc
#define BRK_VECTOR  0xfffe

#define RAMIO_read(vm, pos, size, out) \
	VM6502_ramio(vm)->read(vm, VM6502_ramio(vm), pos, size, (uint8_t*)(out));

#define RAMIO_write(vm, pos, size, in) \
	VM6502_ramio(vm)->write(vm, VM6502_ramio(vm), pos, size, (uint8_t*)(in));

#endif
