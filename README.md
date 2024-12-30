# "effnes" core
The "effnes" project aims to be a core for developing low RAM consuming NES
emulators. This project provides the tools for helping the developer to only
worry about the project's UI and, maybe, about the project's RAM optimization
(only on extreme cases where the implemented RAM optimizations for the
cartridge mappers are not enough).

As this project is presented in a modular manner, it's easy to only use what's
needed, making the process less painful, and allowing the developer to
implement his own methods of emulation.

> [!WARNING]
> This project is at an early development stage. It should not be used until it
> hits a stable version. Expect sudden API changes!

# Core Packages
## effnes-cpu

Currently, this is the only package that it's already implemented and almost*
fully usable. It's a memory-efficient implementation of the NES CPU (MOS6502
without _Decimal_ arithmetic). Also, it features an almost correct cycle
emulation, and it's emulation behaviour is documented in the code.

It's tested** against the [nestest][NESTEST_URL] CPU test, passing it with
everything working as intended.

*: CPU emulation is still missing some illegal opcodes.

**: Will be.

<!--

## effnes-ppu
## effnes-apu
## effness-ines & effnes-nes2
## effnes-cartridge

-->

# TODO
- [ ] Define a Project Name.
- [ ] Divide project into a Cargo workspace (effnes-cpu, effnes-ppu, effnes-apu, effnes-ines, effnes-nes2, effnes-cartridge...).
- [ ] Add unit tests.
- [ ] Emulate the Picture Processing Unit (PPU).
- [ ] Emulate the Audio Processing Unit (APU).

[NESTEST_URL]: https://www.qmtpro.com/~nes/misc/nestest.nes
