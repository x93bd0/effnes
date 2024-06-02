#include "mapper.h"
#include <stdlib.h>
#include <string.h>

MapperInf MI_fetch(char* ROM)
{
	MapperInf inf = (MapperInf)malloc(17);
	strcpy(inf, "INV");
	if (ROM[0] != 0x4E || ROM[1] != 0x45 || ROM[2] != 0x53 || ROM[3] != 0x1A)
		return inf;
	for (uint8_t x = 0; x < 16; x++)
		inf[x] = ROM[x];
	return inf;
}

// iNES flags
uint8_t MI_isNES2(MapperInf INF)
{
	return (INF[7] | (3 << 2)) == INF[7];
}

uint8_t	MI_isINES(MapperInf INF)
{
	return !MI_isNES2(INF);
}

uint16_t MI_mapper(MapperInf INF)
{
	if (MI_isINES(INF))
		return (INF[6] >> 4) + (INF[7] & 0b11110000);
	return 0;
}

uint8_t MI_nmarg(MapperInf INF)
{
	if (MI_isINES(INF))
		return INF[6] & 1;
	return 0;
}

uint8_t MI_prgrom(MapperInf INF)
{
	if (MI_isINES(INF))
		return INF[4];
	return 0;
}

uint8_t MI_chrrom(MapperInf INF)
{
	if (MI_isINES(INF))
		return INF[5];
	return 0;
}

uint8_t MI_prgram(MapperInf INF)
{
	if (MI_isINES(INF))
		return INF[8];
	return 0;
}

uint8_t MI_battery(MapperInf INF)
{
	if (MI_isINES(INF))
		return (INF[6] & 2) > 0;
	return 0;
}

uint8_t MI_trainer(MapperInf INF)
{
	if (MI_isINES(INF))
		return (INF[6] & 4) > 0;
	return 0;
}

uint8_t MI_altnml(MapperInf INF)
{
	if (MI_isINES(INF))
		return (INF[6] & 8) > 0;
	return 0;
}

uint8_t MI_isVSU(MapperInf INF)
{
	if (MI_isINES(INF))
		return INF[7] & 1;
	return 0;
}

uint8_t MI_isPLC(MapperInf INF)
{
	if (MI_isINES(INF))
		return (INF[7] & 2) > 0;
	return 0;
}

uint8_t MI_isNTSC(MapperInf INF)
{
	if (MI_isINES(INF))
		return !(INF[9] & 1) || (INF[10] & 3) == 3 || (INF[10] & 3) == 1;
	return 0;
}

uint8_t MI_isPAL(MapperInf INF)
{
	if (MI_isINES(INF))
		return (INF[9] & 1) || (INF[10] & 3) == 3 || (INF[10] & 3) == 1;
	return 0;
}

uint8_t MI_haspram(MapperInf INF)
{
	if (MI_isINES(INF))
		return (INF[10] & 16) > 0;
	return 0;
}

uint8_t MI_busconf(MapperInf INF)
{
	if (MI_isINES(INF))
		return (INF[10] & 32) > 0;
	return 0;
}

uintmx_t MI_fprgoff(MapperInf INF)
{
	return 16 + (MI_trainer(INF) ? 512 : 0);
}

uintmx_t MI_fchroff(MapperInf INF)
{
	return MI_fprgoff(INF) + (16384 * MI_prgrom(INF));
}

uintmx_t MI_fpiroff(MapperInf INF)
{
	return MI_fchroff(INF) + (8192 * MI_chrrom(INF));
}

uintmx_t MI_fprmoff(MapperInf INF)
{
	return MI_fpiroff(INF) + (MI_isPLC(INF) ? 8192 : 0);
}

uintmx_t MI_fsenoff(MapperInf INF)
{
	return MI_fchroff(INF) + (8192 * MI_chrrom(INF));
}

void MI_destroy(MapperInf INF)
{
	free(INF);
}
