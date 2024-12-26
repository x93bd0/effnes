#include "mapper.h"
#include <stdlib.h>
#include <string.h>

MapperHeader MI_fetch(char* ROM)
{
	MapperHeader inf = (MapperHeader)malloc(17);
	strcpy(inf, "INV");
	if (ROM[0] != 0x4E || ROM[1] != 0x45 || ROM[2] != 0x53 || ROM[3] != 0x1A)
		return inf;
	for (uint x = 0; x < 16; x++)
		inf[x] = ROM[x];
	return inf;
}

// iNES flags
uint8_t MI_isNES2(MapperHeader INF)
{
	return (INF[7] | (3 << 2)) == INF[7];
}

uint8_t	MI_isINES(MapperHeader INF)
{
	return !MI_isNES2(INF);
}

uint16_t MI_mapper(MapperHeader INF)
{
	if (MI_isINES(INF))
		return (INF[6] >> 4) + (INF[7] & 0b11110000);
	return 0;
}

uint8_t MI_nmarg(MapperHeader INF)
{
	if (MI_isINES(INF))
		return INF[6] & 1;
	return 0;
}

uint8_t MI_prgrom(MapperHeader INF)
{
	if (MI_isINES(INF))
		return INF[4];
	return 0;
}

uint8_t MI_chrrom(MapperHeader INF)
{
	if (MI_isINES(INF))
		return INF[5];
	return 0;
}

uint8_t MI_prgram(MapperHeader INF)
{
	if (MI_isINES(INF))
		return INF[8];
	return 0;
}

uint8_t MI_battery(MapperHeader INF)
{
	if (MI_isINES(INF))
		return (INF[6] & 2) > 0;
	return 0;
}

uint8_t MI_trainer(MapperHeader INF)
{
	if (MI_isINES(INF))
		return (INF[6] & 4) > 0;
	return 0;
}

uint8_t MI_altnml(MapperHeader INF)
{
	if (MI_isINES(INF))
		return (INF[6] & 8) > 0;
	return 0;
}

uint8_t MI_isVSU(MapperHeader INF)
{
	if (MI_isINES(INF))
		return INF[7] & 1;
	return 0;
}

uint8_t MI_isPLC(MapperHeader INF)
{
	if (MI_isINES(INF))
		return (INF[7] & 2) > 0;
	return 0;
}

uint8_t MI_isNTSC(MapperHeader INF)
{
	if (MI_isINES(INF))
		return !(INF[9] & 1) || (INF[10] & 3) == 3 || (INF[10] & 3) == 1;
	return 0;
}

uint8_t MI_isPAL(MapperHeader INF)
{
	if (MI_isINES(INF))
		return (INF[9] & 1) || (INF[10] & 3) == 3 || (INF[10] & 3) == 1;
	return 0;
}

uint8_t MI_haspram(MapperHeader INF)
{
	if (MI_isINES(INF))
		return (INF[10] & 16) > 0;
	return 0;
}

uint8_t MI_busconf(MapperHeader INF)
{
	if (MI_isINES(INF))
		return (INF[10] & 32) > 0;
	return 0;
}

uintmx_t MI_fprgoff(MapperHeader INF)
{
	return 16 + (MI_trainer(INF) ? 512 : 0);
}

uintmx_t MI_fchroff(MapperHeader INF)
{
	return MI_fprgoff(INF) + (16384 * MI_prgrom(INF));
}

uintmx_t MI_fpiroff(MapperHeader INF)
{
	return MI_fchroff(INF) + (8192 * MI_chrrom(INF));
}

uintmx_t MI_fprmoff(MapperHeader INF)
{
	return MI_fpiroff(INF) + (MI_isPLC(INF) ? 8192 : 0);
}

uintmx_t MI_fsenoff(MapperHeader INF)
{
	return MI_fchroff(INF) + (8192 * MI_chrrom(INF));
}

void MI_destroy(MapperHeader INF)
{
	free(INF);
}
