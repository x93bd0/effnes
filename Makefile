# TODO: Change src/vm6502.c to MOS6502_SRC

CC=gcc
CC_FLAGS=-Wall -std=c11

# CC_FLAGS_RELEASE=${CC_FLAGS} -02

PROJECT_DIR=./
OUTPUT_DIR=${PROJECT_DIR}out/
OUTPUT_TEST_DIR=${OUTPUT_DIR}tests/
OUTPUT_DEBUG_DIR=${OUTPUT_DIR}debug/
OUTPUT_RELEASE_DIR=${OUTPUT_DIR}release/

test_all: clear_scr test_processor test_processor_opt test_cartridge

	
test_processor_opt: 
	mkdir -p ${OUTPUT_TEST_DIR}
	${CC} ${CC_FLAGS} tests/test_processor.c src/vm6502.c mappers/mapper.c -o ${OUTPUT_TEST_DIR}test6502_opt -O3

test_processor: 
	mkdir -p ${OUTPUT_TEST_DIR}
	${CC} ${CC_FLAGS} tests/test_processor.c src/vm6502.c mappers/mapper.c -o ${OUTPUT_TEST_DIR}test6502 -O0

test_cartridge: 
	mkdir -p ${OUTPUT_TEST_DIR}
	${CC} ${CC_FLAGS} tests/test_cartridge.c mappers/mapper.c -o ${OUTPUT_TEST_DIR}testcart -I../inc -O0
	
clear_scr:
	clear
