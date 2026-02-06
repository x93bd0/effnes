table = [[0 for _ in range(16)] for _ in range(16)]
mnemons = set()

for x in range(0, 256):
    code, mnemonic = input().split()
    mnemons.add(mnemonic)
    code = int(code, 16)
    table[code // 16][code % 16] = mnemonic

mnemons = list(mnemons)
mnemons.sort()
print("enum Mnemonics {")
print('\n'.join([f'    {mn.capitalize()},' for mn in mnemons]))
print("}")

print('\n'.join([', '.join([mn.capitalize() for mn in l]) + ', ' for l in table]))

