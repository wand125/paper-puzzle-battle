"""Compare independent verifier results with the book, and back up the
verified depth-3 values alone to see which book claims they prove.
Usage: python3 tools/analysis/verify_compare.py results/book results/verify/res-normal.txt
"""
import collections, glob, sys
base, vf = sys.argv[1], sys.argv[2]
book = {}
for f in (glob.glob(f'{base}/res-*.txt') + glob.glob(f'{base}/depth3-results.txt')):
    for l in open(f):
        p = l.split()
        if len(p) >= 2: book[p[0]] = int(p[1])
ver = {}
for l in open(vf):
    p = l.split()
    if len(p) >= 2: ver[p[0]] = int(p[1])
bad = [k for k in ver if book.get(k) != ver[k]]
print(f'verified {len(ver)} depth-3 positions: agree {len(ver) - len(bad)}, disagree {len(bad)}')
for k in bad[:20]: print('  MISMATCH', k, 'book', book.get(k), 'verifier', ver[k])
kids, depth = collections.defaultdict(set), {}
for l in open(f'{base}/edges.txt'):
    d, pk, ck, _ = l.split()
    kids[pk].add(ck); depth[ck] = int(d); depth.setdefault(pk, int(d) - 1)
def backup(vals):
    v = dict(vals)
    for d in (2, 1, 0):
        for k in [k for k, dd in depth.items() if dd == d]:
            cs = [v.get(c) for c in kids[k]]
            if 0 in cs: v[k] = 1
            elif cs and all(x == 1 for x in cs): v[k] = 0
    return v
full = backup(book)
proved = backup(ver)
for d in (0, 1, 2):
    ks = [k for k, dd in depth.items() if dd == d]
    pr = [k for k in ks if proved.get(k) is not None]
    print(f'depth {d}: {len(ks)} positions, proved by the verifier {len(pr)}, '
          f'agreeing with the book {sum(proved[k] == full[k] for k in pr)}')
