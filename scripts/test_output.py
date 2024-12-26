with open("correct.txt") as file:
    correct_raw: list[str] = file.read().split('\n')

with open("mine.txt") as file:
    mine_raw: list[str] = file.read().split('\n')


last_cycle: list[int] = [0, 0]
last_op: str = ''
for x in range(min(len(mine_raw), len(correct_raw))):
    correct: str = correct_raw[x]
    mine: str = mine_raw[x]

    if correct[:4].lower().lstrip('0') != mine[:4].lower().lstrip(' '):
        print("DIFFERENT ADDRESS: (line " + str(x) + ")")
        print("\tcorrect:", correct)
        print("\tmine   :", mine)
        break

    gA: int = int(correct[50:52], 16)
    gX: int = int(correct[55:57], 16)
    gY: int = int(correct[60:62], 16)
    gP: int = int(correct[65:67], 16)
    gSP: int = int(correct[71:73], 16)

    mA: int = int(mine[50:52], 16)
    mX: int = int(mine[55:57], 16)
    mY: int = int(mine[60:62], 16)
    mP: int = int(mine[65:67], 16)
    mSP: int = int(mine[71:73], 16)

    if gA != mA or gX != mX or gY != mY or gP != mP or gSP != mSP:
        print("DIFFERENT REGISTERS: (line " + str(x) + ")")
        print("\tcorrect:", correct)
        print("\tmine   :", mine)
        break

    gCyc: int = int(correct[90:])
    mCyc: int = int(mine[91:].split(' ')[0])

    if gCyc - last_cycle[0] != mCyc - last_cycle[1]:
        print("CYCLE MISSMATCH (" + last_op + "): (line " + str(x) + ")")
        db: int = (gCyc - last_cycle[0]) - (mCyc - last_cycle[1])
        print("\tdiffby:", ["-", "+"][db < 0] + str(abs(db)))

    last_cycle[0] = gCyc
    last_cycle[1] = mCyc
    last_op = correct[16:19]
