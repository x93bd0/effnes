#ifndef __VM6502_H
#define __VM6502_H

#include <limits.h>
#include <stdint.h>

#ifndef uintmx_t
	#define uintmx_t uint32_t
	#define UINTVM_MAX (((uintmx_t)1 << 31) - 1)
#endif

typedef uint16_t (*VM6502_RamIORead)(void* VM, uint16_t ADDR, uint16_t SIZE, uint8_t* OUT);
typedef uint16_t (*VM6502_RamIOWrite)(void* VM, uint16_t ADDR, uint16_t SIZE, uint8_t* INPUT);

typedef struct {
	VM6502_RamIORead  read;
	VM6502_RamIOWrite write;
} VM6502_RamIO;

typedef struct VM6502 {
	void* slot;

	uint16_t PC; // Program Counter
	uint8_t X, // X register
			Y, // Y register
			A, // Accumulator
			S, // Stack Pointer
			P, // Program Status
			H; // Halted

	uint8_t ex_interrupt;
	uintmx_t cycles;

	VM6502_RamIORead  read;
	VM6502_RamIOWrite write;
	uint16_t debug_addr;
} VM6502;


VM6502*	VM6502_init		(VM6502_RamIO, void*);
void 	VM6502_store	(VM6502*, void*);
void*	VM6502_slot		(VM6502*);
void	VM6502_reset	(VM6502*);

void		VM6502_NMI		(VM6502*);
uintmx_t	VM6502_run		(VM6502*, uintmx_t);

#endif
