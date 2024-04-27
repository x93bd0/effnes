table = [[0, 0, 0, 0]] * 256

modes = {
  'Accumulator': '_ACC',
  'Immediate': '',
  'Implied': '',
  'Absolute': '_ABS',
  'Absolute,X': '_ABX',
  'Absolute,Y': '_ABY',
  'Indirect': '_IND',
  'Indirect,X': '_INX',
  'Indirect,Y': '_INY',
  'Zero-Page': '_ZPG',
  'Zero-Page,X': '_ZPX',
  'Zero-Page,Y': '_ZPY',
  'Relative': '_REL'
}

OpI = {
  'ADC':  1, 'AND':  2, 'ASL':  3, 'BCC':  4,
  'BCS':  5, 'BEQ':  6, 'BIT':  7, 'BMI':  8,
  'BNE':  9, 'BPL': 10, 'BRK': 11, 'BVC': 12,
  'BVS': 13, 'CLC': 14, 'CLD': 15, 'CLI': 16,
  'CLV': 17, 'CMP': 18, 'CPX': 19, 'CPY': 20,
  'DEC': 21, 'DEX': 22, 'DEY': 23, 'EOR': 24,
  'INC': 25, 'INX': 26, 'INY': 27, 'JMP': 28,
  'JSR': 29, 'LDA': 30, 'LDX': 31, 'LDY': 32,
  'LSR': 33, 'NOP': 34, 'ORA': 35, 'PHA': 36,
  'PHP': 37, 'PLA': 38, 'PLP': 39, 'ROL': 40,
  'ROR': 41, 'RTI': 42, 'RTS': 43, 'SBC': 44,
  'SEC': 45, 'SED': 46, 'SEI': 47, 'STA': 48, 
  'STX': 49, 'STY': 50, 'TAX': 51, 'TAY': 52,
  'TSX': 53, 'TXA': 54, 'TXS': 55, 'TYA': 56
}

def chex(c, n):
  c = (c / 2) * 8
  return ' '*int(c - (len(bin(n)) - 2)) + bin(n)

ops = {}

while True:
  req = input()
  if req == 'break':
    break

  d = req.split()
  print('Input:', d)

  ops[d[1] + modes[d[0]]] = '0x' + d[3][1:]

for x,y in ops.items():
  print(f"#define {x} {y}")
