table = [[0, 0, 0, 0]] * 256

modes = {
  'Accumulator': 0x0,
  'Immediate': 0x1,
  'Implied': 0x2,
  'Absolute': 0x4,
  'Absolute,X': 0x7,
  'Absolute,Y': 0x8,
  'Indirect': 0x6,
  'Indirect,X': 0xB,
  'Indirect,Y': 0xC,
  'Zero-Page': 0x5,
  'Zero-Page,X': 0x9,
  'Zero-Page,Y': 0xA,
  'Relative': 0x3
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

while True:
  req = input()
  if req == 'break':
    break

  d = req.split()
  print('Input:', d)
  table[int(d[3][1:], 16)] = [
    int(OpI[d[1]]), int(modes[d[0]]),
    int(d[5].replace('+', '')),
    int('+' in d[5])
  ]

  x = 0
  while x < 256:
    if x % 8 == 0:
      print('')

    print(chex(4, (table[x][3] << 1) + (table[x][2] << 2) + (table[x][1] << 5) + (table[x][0] << 9)), end=', ')
    x += 1

  print("")
