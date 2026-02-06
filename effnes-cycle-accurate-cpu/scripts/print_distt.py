# aaabbbcc

def mark(low, high):
    if high & 0b0001 and not low:
        return "REL"
    if low in [0x0, 0x2]:
        return "IMP" if high <= 0x7 else "###"
    if low in [0x1, 0x3]:
        return ["XIn", "InY"][high % 2]
    if low in [0x4, 0x5, 0x6, 0x7]:
        return ["Zpg", "ZpX"][high % 2]
    if low in [0x8, 0xA]:
        return "IMP"
    if low in [0x9, 0xB]:
        return ["###", "AbY"][high % 2]
    if low in [0xC, 0xD, 0xE, 0xF]:
        return ["Abs", "AbX"][high % 2]
    return "---"

table = [[ mark(l, h) for l in range(16) ] for h in range(16)]
print("    " + '    '.join([f"-{hex(id)[2:]}" for id in range(16)]))
print('\n'.join([f"{hex(id)[2:]}-  " + '   '.join([d.upper() for d in t]) for id, t in enumerate(table)]))
