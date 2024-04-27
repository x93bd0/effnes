#ifndef VM6502_H
#define VM6502_H

#include "ops6502.h"
#include <limits.h>

#ifndef uintmx_t
  #define uintmx_t uint32_t
  #define UINTVM_MAX (2<<29)-1
#endif

typedef struct VM6502 VM6502;
typedef struct VM6502RAM_IO VM6502RAM_IO;
typedef uint16_t (*LFRMethod)(void* VM, VM6502RAM_IO* IO, uint16_t ADDR, uint16_t SIZE, uint8_t* OUT);
typedef uint16_t (*WTRMethod)(void* VM, VM6502RAM_IO* IO, uint16_t ADDR, uint16_t SIZE, uint8_t* INPUT);

struct VM6502RAM_IO {
  LFRMethod read;
  WTRMethod write;
};

VM6502* VM6502_init(LFRMethod, WTRMethod);
void VM6502_store(VM6502*, void*);
void* VM6502_slot(VM6502*);
void VM6502_reset(VM6502*);
VM6502RAM_IO* VM6502_ramio(VM6502*);
uintmx_t VM6502_run(VM6502*, uintmx_t);

#define FLAG_CARRY    0x0
#define FLAG_ZERO     0x1
#define FLAG_INTDIS   0x2
#define FLAG_DECIMAL  0x3
#define FLAG_BREAK    0x4
#define FLAG_OVERFLOW 0x6
#define FLAG_NEGATIVE 0x7

#define RAMIO_read(vm, pos, size, out) \
  VM6502_ramio(vm)->read(vm, VM6502_ramio(vm), pos, size, (uint8_t*)(out));

#define RAMIO_write(vm, pos, size, in) \
  VM6502_ramio(vm)->write(vm, VM6502_ramio(vm), pos, size, (uint8_t*)(in));

#endif
