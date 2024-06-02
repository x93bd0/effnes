#ifndef MAPPER_H
#define MAPPER_H

#include "../inc/vm6502.h"
typedef struct Mapper {
	void (*each_cycle)(VM6502*, void*);
	void* extra;

	LFRMethod read;
	WTRMethod write;
} Mapper;

typedef char* MapperInf;
MapperInf	MI_fetch	(char* ROM);

// iNES flags
uint8_t		MI_isINES	(MapperInf INF);
uint16_t	MI_mapper	(MapperInf INF);
uint8_t		MI_nmarg	(MapperInf INF);

uint8_t		MI_prgrom	(MapperInf INF);
uint8_t		MI_chrrom	(MapperInf INF);
uint8_t		MI_prgram	(MapperInf INF);

uint8_t		MI_battery	(MapperInf INF);
uint8_t		MI_trainer	(MapperInf INF);
uint8_t		MI_altnml	(MapperInf INF);
uint8_t		MI_isVSU	(MapperInf INF);
uint8_t		MI_isPLC	(MapperInf INF);
uint8_t		MI_isNTSC	(MapperInf INF);
uint8_t		MI_isPAL	(MapperInf INF);
uint8_t		MI_haspram	(MapperInf INF);
uint8_t		MI_busconf	(MapperInf INF);

uintmx_t	MI_fprgoff	(MapperInf INF);
uintmx_t	MI_fchroff	(MapperInf INF);
uintmx_t	MI_fpiroff	(MapperInf INF);
uintmx_t	MI_fprmoff	(MapperInf INF);
uintmx_t	MI_fsenoff	(MapperInf INF);

void		MI_destroy	(MapperInf INF);

#endif
