#include "mappers/mapper.h"
#include "inc/vm6502.h"
#include "testasm.h"

#include <stdlib.h>
#include <fcntl.h>
#include <stdio.h>


#define u8 uint8_t
#define u16 uint16_t
#define u32 uint32_t

#define min(x, y) ((x) > (y) ? (y) : (x))


typedef struct Context
{
	u8* memory;
	MapperHeader header; 
} Context;


u16 io_read(VM6502* vm, u16 addr, u16 size, u8* out)
{
	for (u16 x = 0; x < size; x++)
		out[x] = ((Context*)vm->slot)->memory[addr + x];
	return size;
}

u16 io_write(VM6502* vm, u16 addr, u16 size, u8* input)
{
	// printf("requested write for %#4x ... %#4x\n", addr, addr + (size - 1));
	// for (uint x = 0; x < size; x++)
	// 	printf("%2x ", input[x]);
	// printf("\n");

	for (u16 x = 0; x < size; x++)
	{
		if (addr + x >= 0x8000 && addr <= 0xBFFF)
			((Context*)vm->slot)->memory[addr + x] = input[x],
			((Context*)vm->slot)->memory[((addr + x) - 0x8000) + 0xC000] = input[x];
		else if (addr + x >= 0xC000)
			((Context*)vm->slot)->memory[addr + x] = input[x],
			((Context*)vm->slot)->memory[((addr + x) - 0xC000) + 0x8000] = input[x];
		else
			((Context*)vm->slot)->memory[addr + x] = input[x];
	}
	return size;
}


signed main()
{
	VM6502* vm = VM6502_init((VM6502_RamIO){
		.read=(VM6502_RamIORead)io_read, .write=(VM6502_RamIOWrite)io_write
	}, NULL);

	Context* ctx = malloc(sizeof(Context));
	ctx->memory = malloc(sizeof(u8) * (USHRT_MAX + 3));
	ctx->memory[0] = 0;

	for (u16 x = 0; x < USHRT_MAX; x++)
		ctx->memory[x + 1] = 0;

	FILE* fd = fopen("rom.nes", "r");
	if (!fd)
	{
		printf("[ERROR] Can't open 'rom.nes'!\n");
		exit(1);
	}

	char header[17];
	fread(header, 16, 1, fd);

	ctx->header = MI_fetch(header);
	u8 rom_size = MI_prgrom(ctx->header);

	if (!rom_size)
	{
		printf("[ERROR] 'rom.nes' has no Program ROM!\n");
		exit(1);
	}

	u8* code = (u8*)malloc(sizeof(u8) * (16384 * rom_size + 1));
	fseek(fd, MI_fprgoff(ctx->header), SEEK_SET);
	fread(code, 16384 * rom_size, 1, fd);
	fclose(fd);

	VM6502_store(vm, ctx);
	vm->write(vm, 0x8000, 16384 * rom_size, code);
	VM6502_reset(vm);

	// u16 rst_vec, nmi_vec, brk_vec;
	// vm->read(vm, RST_VECTOR, 2, (u8*)&rst_vec);
	// vm->read(vm, NMI_VECTOR, 2, (u8*)&nmi_vec);
	// vm->read(vm, BRK_VECTOR, 2, (u8*)&brk_vec);

	vm->PC = 0xC000;
	vm->S = 0xfd;
	vm->P = 0x24;
	// vm->read(vm, BRK_VECTOR, 2, (u8*)&vm->pc);

	uint x;
	scanf("%d", &x);
	uint cc = 7;
	while (x--)
	{
		// C000  4C F5 C5  JMP $C5F5                       A:00 X:00 Y:00 P:24 SP:FD PPU:  0, 21 CYC:7
		u8 data[3];
		vm->read(vm, vm->PC, min(3, USHRT_MAX - vm->PC + 1), data);
		printf("%4x  %2x %2x %2x  %s                             A:%2x X:%2x Y:%2x P:%2x SP:%2x             CYC: %d ",
			vm->PC, data[0], data[1], data[2], FROMASM[data[0]], vm->A, vm->X, vm->Y, vm->P, vm->S, cc);
		vm->debug_addr = 0;
		VM6502_run(vm, 1);
		printf("%d %#4x\n", vm->ex_interrupt, vm->debug_addr);
		vm->ex_interrupt = 0;
		cc += vm->cycles;
	}
}
