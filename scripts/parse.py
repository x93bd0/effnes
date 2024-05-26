import rich

with open('correct.txt') as f:
  cd: str = f.read().split('\n')

o = []
for x in cd:
  if len(x) == 0:
    break

  i = [i for i in x.split(' ') if i]
  pc = i[0].lower()
  dr = []

  l = 1
  for k in i[1:]:
    if len(k) != 2:
      break
    dr.append(hex(int(k, 16)).lower()[2:])
    l += 1

  op = i[l]

  mask = i[-1][3:9]
  sp = int(i[-1][:2], 16)
  yr = int(i[-2], 16)
  xr = int(i[-3], 16)
  ac = int(i[-4][1:], 16)

  o.append([pc, op, mask, ac, xr, yr, sp, dr])

o1 = []
with open('mine.txt') as m:
  md = [i for i in m.read().split('\n') if i]

for x in md:
  i = [i for i in x.split('|') if i]
  pc = i[0].strip().lower()
  dr = [k.lower() for k in i[1].split(' ') if k]
  op = i[2].strip()

  params = [int(l, 16) for l in i[3].split() if l]
  mask = i[-1]

  o1.append([pc, op, mask, *params, dr])


l = 0
for i in range(0, min(len(o), len(o1))):
  print(o[i], [*o1[i][:-1], o1[i][-1][:len(o[i][-1])]], o[i][:-1] != o1[i][:-1], o1[i][-1][:len(o[i][-1])] != o[i][-1])

  if o[i][:-1] != o1[i][:-1] or o1[i][-1][:len(o[i][-1])] != o[i][-1]:
    print("Collision found ", l, o[i], o1[i])
    break
  l += 1

print("compared", len(o), "with", len(o1))
