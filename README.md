# Efficient (We hope so) NES Emulator
This is a preliminary version of the effnes-emu project. Right now, we only have the Processor implemented (not completely), and we're optimizing it for the ESP32 board (Xtensa LX-7 processor, 240MHz).

# Features
* (Jump Table + Switch) emulation
* Correct cycle emulation in mind
* Processor can run up to 110MHz on 2.7GHz right now
* Memory efficient, taking advantage of mirroring
* Easily debuggable (at least we're trying...)

# Missing
* PPU
* APU
* Implement common cartridge's mapping
* Test processor (50%)
* And other things.
* Refactorize tests

# How to test
```bash
# For compiling the test suite for the processor (u can add _opt for a faster test)
make test_processor
# Running
./test6502
```
