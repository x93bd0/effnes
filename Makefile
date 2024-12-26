# TODO: Change src/vm6502.c to MOS6502_SRC

all:
	gcc main.c src/vm6502.c mappers/mapper.c -o test6502 -fsanitize=address
	[ -f mine.txt ] && rm mine.txt
	echo 10000 | ./test6502 > mine.txt || echo
	python3 scripts/test_output.py

test_processor_opt:
	gcc tests/test_processor.c src/vm6502.c mappers/mapper.c -o test6502 -O3

test_processor:
	gcc tests/test_processor.c src/vm6502.c mappers/mapper.c -o test6502

test_cartridge:
	gcc tests/test_cartridge.c mappers/mapper.c -o testcart -I../inc
