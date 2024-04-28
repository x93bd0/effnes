# Efficient (We hope so) NES Emulator
This is a preliminary version of the effnes-emu project. Right now, we only have the Processor implemented (not completely), and we're optimizing it for the ESP32 processor.

# Features
* (Jump Table + Switch) emulation
* Correct cycle emulation in mind
* Processor can run up to 70MHz right now
* Memory efficient, taking advantage of mirroring
* Easily debuggable (at least we're trying...)

# Missing
* PPU
* APU
* Implement common cartridge's mapping
* Test processor
* And other things.
