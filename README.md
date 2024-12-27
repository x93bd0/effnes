# Efficient (we hope so) NES emulator
This is a preliminary version of the effnes-emu project. Right now, we only have the Processor implemented (not completely), and we're optimizing it for the ESP32 board (Xtensa LX-7 processor, 240MHz).

# Features
* (Jump Table + Switch (Match)) emulation
* Correct cycle emulation in mind
* Memory efficient, taking advantage of mirroring
* Easily debuggable (at least we're trying...)

# Missing
* PPU
* APU
* Implement (common) cartridge's mapping
* Test processor (0%)
* And other things.
* Create tests
