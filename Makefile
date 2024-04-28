# TODO: Change src/vm6502.c to MOS6502_SRC

test_processor_opt:
	gcc tests/test_processor.c src/vm6502.c mappers/mapper.c -o test6502 -O1

test_processor:
	gcc tests/test_processor.c src/vm6502.c mappers/mapper.c -o test6502

test_cartridge:
	gcc tests/test_cartridge.c mappers/mapper.c -o testcart -I../inc
