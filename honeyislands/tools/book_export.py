"""Export the opening book for the AI (play/data/book-<rule>.bin).

Usage: python3 tools/book_export.py results/book play/data/book-normal.bin [max_pop]
Records of 10 bytes: u64 LE key (canonical determined-black mask), i8 passes
left (0 for the normal rule), u8 value (1 = the player to move wins).
Contents: every position up to depth 3 (values backed up from the solved
depth-3 positions), the winning depth-4 replies (winkeys.txt), and the shared
known positions whose determined-black mask has at most max_pop cells.
"""
import glob, struct, sys

base, out = sys.argv[1], sys.argv[2]
max_pop = int(sys.argv[3]) if len(sys.argv) > 3 else 5

def parse(k):
    a, _, b = k.partition(':')
    return int(a), int(b or 0)

value = {}
for f in (glob.glob(f'{base}/res-*.txt') + glob.glob(f'{base}/depth3-results.txt')):
    for l in open(f):
        p = l.split()
        if len(p) >= 2:
            value[p[0]] = int(p[1])
children, depth_of = {}, {}
for l in open(f'{base}/edges.txt'):
    d, pk, ck, _c = l.split()
    children.setdefault(pk, set()).add(ck)
    depth_of[ck] = int(d)
    depth_of.setdefault(pk, int(d) - 1)
for d in (2, 1, 0):
    for pk in [k for k, dd in depth_of.items() if dd == d]:
        vals = [value.get(c) for c in children.get(pk, ())]
        if any(v == 0 for v in vals):
            value[pk] = 1
        elif vals and all(v == 1 for v in vals):
            value[pk] = 0
book = {k: v for k, v in value.items() if k in depth_of}
for l in open(f'{base}/winkeys.txt'):
    p = l.split()
    if len(p) == 2:
        book[p[1]] = 0
extra = 0
import os
for l in (open(f'{base}/known-all.txt') if os.path.exists(f'{base}/known-all.txt') else []):
    p = l.split()
    if len(p) >= 2 and p[0] not in book and bin(parse(p[0])[0]).count('1') <= max_pop:
        book[p[0]] = int(p[1])
        extra += 1
with open(out, 'wb') as g:
    for k, v in sorted(book.items(), key=lambda kv: parse(kv[0])):
        m, p = parse(k)
        g.write(struct.pack('<QbB', m, p, v))
print(f'{out}: {len(book)} positions ({len(book) - extra} from the book, {extra} known), {len(book) * 10} bytes')
