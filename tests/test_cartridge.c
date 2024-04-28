#include "../mappers/mapper.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

int main()
{
  FILE* f = fopen("rom.nes", "r");
  char buf[17];
  fread(buf, 16, 1, f);
  fclose(f);

  MapperInf inf = MI_fetch(buf);
  if (strcmp(inf, "INV") == 0)
  {
    printf("Invalid rom!\n");
    return 1;
  }

  uint16_t prgrom = MI_prgrom(inf) * 16;
  uint16_t chrrom = MI_chrrom(inf) * 8;
  uint16_t prgram = MI_prgram(inf);

  printf("- ROM INFO (xNES impl) -\n");
  printf("- Type    :       %s -\n", MI_isINES(inf) == 1 ? "iNES" : "NES2");
  printf("- Mapper  :       %4d -\n", MI_mapper(inf));
  printf("- Mirror. :      %c (%c) -\n", MI_nmarg(inf) ? 'H' : 'V', MI_altnml(inf) ? 'A' : '-');
  printf("- PrgRom  :    %4d KB -\n", prgrom);
  printf("- ChrRom  :    %4d KB -\n", chrrom);
  if (MI_haspram(inf))
    printf("- PrgRam  :    %4d KB -\n", prgram);
  printf("- HasBtrR :          %d -\n", MI_battery(inf));
  printf("- Trainer :          %d -\n", MI_trainer(inf));
  printf("- IsVSU   :          %d -\n", MI_isVSU(inf));
  printf("- IsPLC   :          %d -\n", MI_isPLC(inf));
  printf("- TV:    NTSC=%d; PAL:%d -\n", MI_isNTSC(inf), MI_isPAL(inf));
  printf("- Extra (byte 11-15):  -\n");
  printf("= %20s =\n", &inf[11]);
  printf("- Supposed banks loc:  -\n");
  printf("-  PRGROM:  %10d -\n", MI_fprgoff(inf));
  printf("-  CHRROM:  %10d -\n", MI_fchroff(inf));
  printf("-  DATEND:  %10d -\n", MI_fsenoff(inf));
  printf("-      Dump ended      -\n");
}
