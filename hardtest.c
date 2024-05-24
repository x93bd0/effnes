#include "mappers/mapper.h"
#include "inc/vm6502.h"
//#include "testasm.h"
#include <stdlib.h>
#include <fcntl.h>
#include <stdio.h>

#define u8  uint8_t
#define u16 uint16_t
#define u32 uint32_t
u8 GLOBAL_DEBUG = 0, WRITE_EVENT = 0;
#define debug(x) if (GLOBAL_DEBUG) {x;}


void __MOS6502_TRACE(VM6502* VM)
{
	u8 out[3];
	VM->read(VM, VM->pc, 3, out);
	//fprintf(stderr, "%4x | %2x %2x %2x | %s | %2x %2x %2x %2x |", VM->pc, out[0], out[1], out[2], FROMASM[out[0]], VM->Acc, VM->iX, VM->iY, VM->Sp);
	uint8_t status = VM->status;
	for (uint x = 0; x < 8; x++, status <<= 1)
	{
		if (x == 2 || x == 3) continue;
		fprintf(stderr, "%d", (status & 0x80) > 0);
	}
	fprintf(stderr, "|\n");
}


void __MOS6502_DEBUG(VM6502* VM)
{
	fprintf(stderr, "*---------------------------------------*\n");
	fprintf(stderr, "- ID |   X |   Y |   A | Ptr | NV-BDIZC |\n");
	fprintf(stderr, "- NO | %3d | %3d | %3d | %3d | ", VM->iX, VM->iY, VM->Acc, VM->Sp);
	uint8_t status = VM->status;
	for (uint x = 0; x < 8; x++, status <<= 1)
		fprintf(stderr, "%d", (status & 0x80) > 0);
	fprintf(stderr, " |\n");
	fprintf(stderr, "- HX |  %2x |  %2x |  %2x |  %2x |", VM->iX, VM->iY, VM->Acc, VM->Sp);
	fprintf(stderr, " %#8x |\n", VM->status);

	u8 v1; VM->read(VM, 0x100 | ((VM->Sp + 1) & 0xff), 1, &v1);
	u8 v2; VM->read(VM, 0x100 | ((VM->Sp + 2) & 0xff), 1, &v2);
	u8 v3; VM->read(VM, 0x100 | ((VM->Sp + 3) & 0xff), 1, &v3);
	fprintf(stderr, "- LIS = %#4x%2x%2x\n", v1, v2, v3);

	uint8_t b1; VM->read(VM, VM->pc, 1, &b1);
	uint8_t a[2]; VM->read(VM, VM->pc + 1, 2, a);
	// fprintf(stderr, "- PTR = %#6x (%2x (%s) %2x%2x)\n", VM->pc, b1, FROMASM[b1], a[0], a[1]);
	fprintf(stderr, "*---------------------------------------*\n");
}


u16 io_read(VM6502* vm, u16 addr, u16 size, u8* out)
{
	for (u16 x = 0; x < size; x++)
		out[x] = ((u8*)vm->slot)[addr + x];
	return size;
}

u16 io_write(VM6502* vm, u16 addr, u16 size, u8* input)
{
	debug(printf("[DEBUG] Write %#6x <-> %#6x\n", addr, addr + size - 1));
	if (!WRITE_EVENT)
		WRITE_EVENT = 1;
	uint i = 0;
	debug(printf("        "));
	for (u16 x = 0; x < size; x++)
	{
		((u8*)vm->slot)[addr + x] = input[x];
		debug({
			i++;
			printf("%2x ", input[x]); 
			if (i == 8)
			{
				printf("\n        ");
				i = 0;
			}
		});
	} if (i > 0) printf("\n");
	return size;
}


u32 min_u32(u32 a, u32 b)
{
	if (a > b)
	 return b;
	return a;
}


int main()
{
	VM6502* machine = VM6502_init(
		(LFRMethod)io_read, (WTRMethod)io_write);
	VM6502_store(machine, malloc(sizeof(u8)*64*1024));

	for (uint x = 0; x < 64*1024; x++)
		((u8*)machine->slot)[x] = 0;

	FILE* fd = fopen("rom.nes", "r");
	if (!fd)
	{
		printf("[ERROR] Can't open 'rom.nes'\n");
		goto free0;
	}

	char header[17];
	fread(header, 16, 1, fd);
	MapperInf ROM = MI_fetch(header);
	u8 rs = MI_prgrom(ROM);

	if (!rs)
	{
		printf("[ERROR] 'rom.nes' has no Program ROM!\n");
		goto free1;
	}

	u8* code = (u8*)malloc(sizeof(u8) * (16384 * rs + 1));
	fseek(fd, MI_fprgoff(ROM), SEEK_SET);
	fread(code, 16384 * rs, 1, fd);
	fclose(fd);

	debug({
		printf("[DEBUG] Dumping PRGROM to prg.rom\n");
		FILE* out = fopen("prg.rom", "w");
		if (!out)
			printf("[DEBUG] Can't open file 'prg.rom'\n");
		else
		{
			fwrite(code, 16384 * rs, 1, out);
			fclose(out);

			printf("[DEBUG] Succesfully dumped\n");
		}
	});

	machine->write(machine, 0x8000, 16384 * rs, code);
	VM6502_reset(machine);

	printf("[DEBUG] Setup Complete!\n");
	printf("Run for `N cycles` = ");

	u32 cyc;
	scanf("%u", &cyc);

	u16 nmiaddr;
	machine->read(machine, 0xFFFA, 2, (u8*)&nmiaddr);
	machine->pc = nmiaddr;
	printf("START ADDRESS = %#6x\n", machine->pc);

	// GLOBAL_DEBUG = 1;
	int cyct = 0;
	while (cyc > 0)
	{
		int c = VM6502_run_eff(machine, 1);
		cyct += c, cyc -= min_u32(cyc, c);
		// __MOS6502_TRACE(machine);
		// debug(__MOS6502_DEBUG(machine));
		// debug(printf("Total Cycles: %d\n", cyct));
	}

	while (1)
	{
			cyct += VM6502_run_eff(machine, 1);
		// __MOS6502_TRACE(machine);
		// debug(__MOS6502_DEBUG(machine));
		// debug(printf("Total Cycles: %d\n", cyct));

		char a;
		scanf("%c\n", &a);
		if (a == 'k') break;
	}

	free(code);
free1:
	free(ROM);
free0:
	free(machine->slot);
	free(machine);
	printf("[DEBUG] Code Execution Finalized Correctly!\n");
}
