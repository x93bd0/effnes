#include "../mappers/mapper.h"
#include "../inc/vm6502.h"

#include <stdlib.h>
#include <fcntl.h>
#include <stdio.h>
#include <time.h>

#define u8 uint8_t
#define u16 uint16_t
#define u32 uint32_t

u16 io_read(VM6502* vm, u16 addr, u16 size, u8* out)
{
	for (u16 x = 0; x < size; x++)
		out[x] = ((u8*)vm->slot)[addr + x];
	return size;
}

u16 io_write(VM6502* vm, u16 addr, u16 size, u8* input)
{
	for (u16 x = 0; x < size; x++)
		((u8*)vm->slot)[addr + x] = input[x];
	return size;
}

int main()
{
	VM6502* machine = VM6502_init(
		(LFRMethod)io_read, (WTRMethod)io_write);
	VM6502_store(machine, malloc(sizeof(u8) * (64*1024)));

	// De-randomize vRAM
	for (u16 x = 0; x < 64*1024-1; x++) // Used to overflow back to 0
		((u8*)machine->slot)[x] = 0;

	FILE* fd = fopen("rom.nes", "r");
	if (!fd)
	{
		perror("[ERROR] Can't open 'rom.nes'\n");
		goto free0;
	}

	char header[17];
	fread(header, 16, 1, fd);
	MapperInf ROM = MI_fetch(header);
	u8 rs = MI_prgrom(ROM);

	if (!rs)
	{
		perror("[ERROR] 'rom.nes' has no Program ROM!\n");
		goto free1;
	}

	u8* code = (u8*)malloc(sizeof(u8) * (16384 * rs + 1));
	fseek(fd, MI_fprgoff(ROM), SEEK_SET);
	fread(code, 16384 * rs, 1, fd);
	fclose(fd);

	machine->write(machine, 0x8000, 16384 * rs, code);
	VM6502_reset(machine);
	printf("[DEBUG] Setup Complete!\n");

	u16 addr;
	machine->read(
		machine, RST_VECTOR, 2, (u8*)&addr);
	machine->pc = addr;

	VM6502_NMI(machine);

	uintmx_t cyc = 17897731;
	clock_t start = clock();
	VM6502_run_eff(machine, cyc);
	clock_t end = clock();
	float seconds = (float)(end - start) / CLOCKS_PER_SEC;

	float hz = cyc / seconds;
	printf("Ran at %f Hz\n", hz);
	printf("Ran at %f MHz\n", hz * 1e-6);

	free(code);
free1:
	free(ROM);
free0:
	free(machine->slot);
	free(machine);
	printf("[DEBUG] Code Execution Finalized Correctly!\n");
}
