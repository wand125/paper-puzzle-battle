"""Convert the legacy HoneySolutions JS list into a little-endian Uint32 binary.

Each solution is two words (lo, hi): bit i of lo = cell i (0..30) is black,
bit i of hi = cell 31+i (31..60) is black. Cell ids follow the legacy
honey-viewer ordering (y outer, x inner, axial coords, radius 4).
The script also verifies every solution against the Honey Islands rules.
"""
import re, struct, sys
from collections import Counter

R = 4
cells = [(x, y) for y in range(-R, R + 1) for x in range(-R, R + 1) if abs(x + y) <= R]
idx = {c: i for i, c in enumerate(cells)}
DIRS = [(-1, 1), (-1, 0), (0, -1), (0, 1), (1, 0), (1, -1)]
nbr = [[idx[(x + dx, y + dy)] for dx, dy in DIRS if (x + dx, y + dy) in idx] for x, y in cells]

def check(lo, hi):
    black = [(lo >> i) & 1 if i < 31 else (hi >> (i - 31)) & 1 for i in range(61)]
    seen, sizes = [False] * 61, []
    for s in range(61):
        if black[s] or seen[s]:
            continue
        stack, n = [s], 0
        seen[s] = True
        while stack:
            c = stack.pop(); n += 1
            for d in nbr[c]:
                if not black[d] and not seen[d]:
                    seen[d] = True; stack.append(d)
        sizes.append(n)
    return sizes == [6] * 6

src, dst = sys.argv[1], sys.argv[2]
text = open(src).read()
pairs = [(int(a, 16), int(b, 16)) for a, b in re.findall(r"\[0x([0-9a-f]+),0x([0-9a-f]+)\]", text)]
bad = sum(1 for lo, hi in pairs if not check(lo, hi))
dup = len(pairs) - len(set(pairs))
print(f"solutions={len(pairs)} invalid={bad} duplicates={dup} max_hi_bit_ok={all(h < (1 << 30) for _, h in pairs)}")
print("black counts:", Counter(bin(lo).count('1') + bin(hi).count('1') for lo, hi in pairs))
with open(dst, 'wb') as f:
    for lo, hi in pairs:
        f.write(struct.pack('<II', lo, hi))
