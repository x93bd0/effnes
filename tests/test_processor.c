#include "../mappers/mapper.h"
#include "../inc/vm6502.h"
#include "../testasm.h"
#include <stdlib.h>
#include <unistd.h>
#include <string.h>
#include <stdio.h>
#include <time.h>

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
	uint8_t b1; VM->read(VM, VM->pc, 1, &b1);
	uint16_t a; VM->read(VM, VM->pc + 1, 2, (uint8_t*)&a);
	fprintf(stderr, "-- Program Ptr: %#6x (%#4x: %s, (next: %#6x))\n", VM->pc, b1, FROMASM[b1], a);
}

uint16_t NESRAM_WRITE(void* VM, uint16_t START, uint16_t SIZE, uint8_t* INPUT) {
	uint8_t* slot = VM6502_slot((VM6502*)VM);
	printf("[RAM] Write %#6x <-> %#6x\n", START, START + SIZE - 1);

	uint i = 0; printf("%#6x = ", START);
	for (uint ADDR = START; ADDR < START + SIZE; ADDR++)
	{
		slot[ADDR] = INPUT[ADDR - START];
		printf("%2x ", INPUT[ADDR - START]);
		if (i == 7)
		{
			i = 0;
			printf("\n%#6x = ", ADDR + 1);
		} else i++;
	} printf("\n");
	return SIZE;
}

uint16_t NESRAM_READ(void* VM, uint16_t START, uint16_t SIZE, uint8_t* OUT) {
	uint8_t* slot = VM6502_slot((VM6502*)VM);
	for (uint16_t ADDR = START; ADDR < START + SIZE; ADDR++)
		OUT[ADDR - START] = slot[ADDR];
	return SIZE;
}

int main()
{
	VM6502* vm = VM6502_init(NESRAM_READ, NESRAM_WRITE);
	VM6502_store(vm, malloc(sizeof(uint8_t)*64*1024));
	printf("sizeof(VM6502) = %lu\n", sizeof(VM6502));

	char header[17];
	FILE* ROM = fopen("rom.nes", "r");

	fread(header, 16, 1, ROM);
	MapperInf inf = MI_fetch(header);

	fseek(ROM, MI_fprgoff(inf), SEEK_SET);
	int8_t data[16384*2+1];
	fread(data, 16384*2, 1, ROM);

	vm->write(vm, 0x8000, 16384, (uint8_t*)data);
	vm->write(vm, 0xC000, 16384, (uint8_t*)&data[16384]);
	fclose(ROM);

	VM6502_reset(vm);

	uint16_t addr;
	vm->read(vm, 0xfffc, 2, (uint8_t*)&addr);
	vm->pc = addr;

	uintmx_t cycsto = 17897731;

	printf("Starting vm...\r\n");
	clock_t start = clock();
	VM6502_run_eff(vm, cycsto);
	float seconds = (float)(clock() - start) / CLOCKS_PER_SEC;
	__MOS6502_DEBUG(vm);

	float hz = cycsto / seconds;
	printf("ran at %f hertz\n", hz);
	printf("ran at %f mega-hertz\n", hz * 1e-6);

	free(VM6502_slot(vm));
	MI_destroy(inf);
	free(vm);
}

