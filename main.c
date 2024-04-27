#include "mappers/mapper.h"
#include "inc/vm6502.h"
#include "testasm.h"
#include <stdlib.h>
#include <unistd.h>
#include <string.h>
#include <stdio.h>
#include <time.h>

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

void __MOS6502_DEBUG(VM6502* VM)
{
  fprintf(stderr, "-- CPU Debug - MOS6502 --\n");
  fprintf(stderr, "-- Flags: ");

  uint8_t status = VM->status;
  for (uint x = 0; x < 8; x++, status <<= 1)
    fprintf(stderr, "%d", (status & 0x80) > 0);

  fprintf(stderr, "\n          NV-BDIZC\n");
  fprintf(stderr, "--      Cycles: %d (%#4x)\n", VM->cc, VM->cc);
  fprintf(stderr, "--  X register: %d (%#4x)\n", (int8_t)VM->iX, VM->iX);
  fprintf(stderr, "--  Y register: %d (%#4x)\n", (int8_t)VM->iY, VM->iY);
  fprintf(stderr, "--  A register: %d (%#4x)\n", (int8_t)VM->Acc, VM->Acc);
  fprintf(stderr, "--   Stack Ptr: %d (%#4x)\n", VM->Sp, VM->Sp);
  uint8_t b1; VM->read(VM, VM6502_ramio(VM), VM->pc, 1, &b1);
  uint16_t a; VM->read(VM, VM6502_ramio(VM), VM->pc + 1, 2, (uint8_t*)&a);
  fprintf(stderr, "-- Program Ptr: %#6x (%#4x: %s, (next: %#6x))\n", VM->pc, b1, FROMASM[b1], a);
}

uint16_t NESRAM_WRITE(void* VM, VM6502RAM_IO*, uint16_t ADDR, uint16_t SIZE, uint8_t* INPUT) {
  for (uint x = ADDR; x < (ADDR + SIZE); x++)
    ((uint8_t*)VM6502_slot((VM6502*)VM))[x] = INPUT[x - ADDR];
  return SIZE;
}

uint16_t NESRAM_READ(void* VM, VM6502RAM_IO*, uint16_t ADDR, uint16_t SIZE, uint8_t* OUT) {
  for (uint x = ADDR; x < (ADDR + SIZE); x++)
    OUT[x - ADDR] = ((uint8_t*)VM6502_slot((VM6502*)VM))[x];
  return SIZE;
}

int main()
{
  VM6502* vm = VM6502_init(NESRAM_READ, NESRAM_WRITE);
  VM6502_store(vm, malloc(sizeof(uint8_t)*64*1024));

  char header[17];
  FILE* ROM = fopen("rom.nes", "r");
  fread(header, 16, 1, ROM);
  MapperInf inf = MI_fetch(header);

  fseek(ROM, MI_fprgoff(inf), SEEK_SET);
  int8_t data[16384*2+1];
  fread(data, 16384*2, 1, ROM);

  RAMIO_write(vm, 0x8000, 16384, data);
  RAMIO_write(vm, 0xC000, 16384, &data[16384]);
  fclose(ROM);

  FILE* w = fopen("prg.b1.rom", "w");
  fwrite(data, 16384, 1, w);
  fclose(w);

  FILE* w1 = fopen("prg.b2.rom", "w");
  fwrite(&data[16384], 16384, 1, w1);
  fclose(w1);

  char S[1];
  VM6502_reset(vm);
  uintmx_t cycles = 0;

  uint16_t addr;
  RAMIO_read(vm, 0xfffc, 2, &addr);
  vm->pc = addr;

  
  clock_t start = clock();
  VM6502_run(vm, 17897731);
  clock_t end = clock();
  float seconds = (float)(end - start) / CLOCKS_PER_SEC;
  __MOS6502_DEBUG(vm);

  printf("took %f seconds\n", seconds);
}
