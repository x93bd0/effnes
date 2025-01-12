# "effnes" core

[![Built With][BUILTWITH_BADGE]][BUILTWITH_LINK]
[![Stargazers][STARS_BADGE]][STARS_LINK]
[![License][LICENSE_BADGE]][LICENSE_LINK]

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

See ["enhancement"][ENHANCEMENTS_URL] labeled issues.


[ENHANCEMENTS_URL]: https://github.com/x93bd0/efnes/issues?q=is%3Aopen+is%3Aissue+label%3Aenhancement
[NESTEST_URL]: https://www.qmtpro.com/~nes/misc/nestest.nes
[BUILTWITH_BADGE]: https://img.shields.io/badge/Built_With-Rust-red?style=for-the-badge&logo=rust&logoColor=white
[BUILTWITH_LINK]: https://python.org/
[STARS_BADGE]: https://img.shields.io/github/stars/x93bd0/efnes?style=for-the-badge
[STARS_LINK]: https://github.com/x93bd0/efnes/stargazers
[LICENSE_BADGE]: https://img.shields.io/github/license/x93bd0/efnes?style=for-the-badge
[LICENSE_LINK]: https://github.com/x93bd0/efnes/blob/master/LICENSE.txt

