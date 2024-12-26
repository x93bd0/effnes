from typing import List, Dict, TypeAlias, Optional
from dataclasses import dataclass
from common import modes, mnemonics, modes_suffix


@dataclass
class Instruction:
    mnemonic: str
    mnemonic_id: int
    mode: str
    mode_id: int
    length: int
    cycles: int
    special_cycles: bool


InstructionSet: TypeAlias = Dict[int, Instruction]


def load_instruction_set(instruction_set: InstructionSet) -> None:
    while True:
        raw_data: str = input()
        if raw_data == "break":
            break

        data: List[str] = [x for x in raw_data.split(" ") if x]
        print(data)

        # Sintax: mode mnemonic [usage] opcode length cycles[+]
        mode: str = data[0]
        mnemonic: str = data[1]
        opcode: int = int(data[-3][1:], 16)
        length: int = int(data[-2])
        cycles: int = int(data[-1][: (-1 if data[-1].endswith("+") else 1)])
        special_cycles: bool = data[-1].endswith("+")

        instruction_set[opcode] = Instruction(
            mnemonic,
            mnemonics[mnemonic],
            mode,
            modes[mode],
            length,
            cycles,
            special_cycles,
        )


def custom_bin(c, n) -> str:
    c = (c / 2) * 8
    return " " * int(c - (len(bin(n)) - 2)) + bin(n)


def create_bin_table(instruction_set: InstructionSet) -> str:
    bin_table: str = ""
    opcode: int = 0
    while opcode < 256:
        if opcode % 16 == 0:
            bin_table += "\n"

        # Format:
        #  Op Codes Jump Table
        #  100010000101000
        #  ^^^^^^    ^^^ ^
        #  OpCode^^^^Tim^U
        #        AdMd   E
        #
        #  AdMd: Addressing Mode
        #  Tim:  Execution Time - 1
        #  E:    Extra Time if Page Boundary Crossed
        #  U:    Unused

        table_entry: int = 0
        if opcode in instruction_set:
            table_entry = (
                (int(instruction_set[opcode].special_cycles) << 1)
                + ((instruction_set[opcode].cycles - 1) << 2)
                + (instruction_set[opcode].mode_id << 5)
                + (instruction_set[opcode].mnemonic_id << 9)
            )

        bin_table += custom_bin(4, table_entry) + ", "

        opcode += 1
    return bin_table


def create_testasm_mnemonic_defs(instruction_set: InstructionSet) -> None:
    output: List[str] = []

    opcode: int
    ins: Instruction
    for opcode, ins in instruction_set.items():
        output.append(
            ins.mnemonic
            + (("_" + modes_suffix[ins.mode]) if modes_suffix[ins.mode] else "")
            + " 0x"
            + hex(opcode)[2:].upper()
        )

    output.sort()
    for value in output:
        print("#define " + value)


def create_testasm_mnemonic_array(instruction_set: InstructionSet) -> str:
    opcodes: List[Optional[str]] = []
    for x in range(0, 256):
        opcodes.append(None)

    opcode: int
    ins: Instruction
    for opcode, ins in instruction_set.items():
        opcodes[opcode] = ins.mnemonic

    array: str = ''
    for x, mnemonic in enumerate(opcodes):
        array += '"' + (mnemonic or "ILG") + '"' + ", "
        if (x + 1) % 16 == 0:
            array += "\n"

    return array


def create_opcode_defs() -> str:
    output: List[str] = []
    for mnemonic, opcode in mnemonics.items():
        output.append("#define OP_" + mnemonic + " 0x" + hex(opcode)[2:].upper())

    output.sort()
    return "\n".join(output)


if __name__ == "__main__":
    iset: InstructionSet = {}
    load_instruction_set(iset)

    with open("templates/ops6502.h") as file:
        template_ops: str = file.read()

    with open("output/ops6502.h", "w") as file:
        file.write(
            template_ops.format(
                opcodes_defs=create_opcode_defs(), jumptable=create_bin_table(iset)
            )
        )

    with open("templates/testasm.h") as file:
        template_testasm: str = file.read()

    with open("output/testasm.h", "w") as file:
        file.write(template_testasm.format(fromasm_table=create_testasm_mnemonic_array(iset)))

# create_table(iset)
# create_opcode_defs()
# create_testasm_mnemonic_array(iset)
