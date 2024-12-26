#ifndef MAPPER_H
#define MAPPER_H

#include "../inc/vm6502.h"
typedef struct Mapper {
	void (*each_cycle)(VM6502*, void*);
	void* extra;

	VM6502_RamIO io;
} Mapper;

typedef char* MapperHeader;
MapperHeader	MI_fetch		(char* ROM);

// iNES flags
uint8_t		MI_isINES		(MapperHeader INF);
uint16_t	MI_mapper		(MapperHeader INF);
uint8_t		MI_nmarg		(MapperHeader INF);

uint8_t		MI_prgrom		(MapperHeader INF);
uint8_t		MI_chrrom		(MapperHeader INF);
uint8_t		MI_prgram		(MapperHeader INF);

uint8_t		MI_battery	(MapperHeader INF);
uint8_t		MI_trainer	(MapperHeader INF);
uint8_t		MI_altnml		(MapperHeader INF);
uint8_t		MI_isVSU		(MapperHeader INF);
uint8_t		MI_isPLC		(MapperHeader INF);
uint8_t		MI_isNTSC		(MapperHeader INF);
uint8_t		MI_isPAL		(MapperHeader INF);
uint8_t		MI_haspram	(MapperHeader INF);
uint8_t		MI_busconf	(MapperHeader INF);

uintmx_t	MI_fprgoff	(MapperHeader INF);
uintmx_t	MI_fchroff	(MapperHeader INF);
uintmx_t	MI_fpiroff	(MapperHeader INF);
uintmx_t	MI_fprmoff	(MapperHeader INF);
uintmx_t	MI_fsenoff	(MapperHeader INF);

void			MI_destroy	(MapperHeader INF);

#endif
