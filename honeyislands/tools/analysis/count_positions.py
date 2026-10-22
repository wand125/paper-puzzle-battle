"""Estimate the number of positions of Honey Islands Battle.

A position is determined by its determined-black set D (Section 3), and the reachable sets D
are exactly the closed sets of the closure system given by the solutions: D = intersection of
all solutions containing D, with at least one such solution. (Given such a D, placing stones on
its cells in any order, skipping cells already determined, reaches D.) Close-by-One generates
every closed set exactly once as a node of a tree; Knuth's estimator on that tree gives an
unbiased estimate of the number of nodes, here split by the number of remaining solutions.

Usage:
  python3 count_positions.py estimate <solutions.bin> <walks> [seed]
  python3 count_positions.py raw <solutions.bin> <walks> <seed> > walks.jsonl   (one walk per line)
  python3 count_positions.py combine walks*.jsonl
  python3 count_positions.py check <solutions.bin> <subsample size> <walks> [seed]
      (exact enumeration on a random subsample of solutions, compared with the estimate)
"""
import sys, random, math, collections
import numpy as np

N = 61
ALPHA, BETA = 1.0, 0.0
ONE = np.uint64(1)

def load(path):
    r = np.fromfile(path, dtype='<u8')
    return (r & np.uint64(0xffffffff)) | ((r >> np.uint64(32)) << np.uint64(31))

def closure(S):
    return int(np.bitwise_and.reduce(S)) if len(S) else None

def children(A, S, y):
    """Canonical Close-by-One children of the closed set A (solutions S) from cell y on."""
    out = []
    for i in range(y, N):
        if A >> i & 1:
            continue
        S2 = S[(S >> np.uint64(i)) & ONE == ONE]
        if len(S2) == 0:
            continue
        B = closure(S2)
        low = (1 << i) - 1
        if B & low == A & low:
            out.append((B, S2, i + 1))
    return out

def bucket(n):
    return 0 if n <= 1 else int(math.log10(n)) + 1   # 0: n=1, 1: 2-9, 2: 10-99, ...

def walk(sols, rnd):
    est = collections.Counter()
    A, S, y, w = closure(sols), sols, 0, 1.0
    while True:
        est[bucket(len(S))] += w
        ch = children(A, S, y)
        if not ch:
            return est
        # importance sampling: pick a child with probability proportional to
        # (its solutions)^ALPHA * 2^(cells it may still add); weight by 1/probability
        sc = [len(c[1]) ** ALPHA * 2.0 ** (BETA * (N - c[2])) for c in ch]
        tot = sum(sc)
        j = rnd.choices(range(len(ch)), weights=sc)[0]
        w *= tot / sc[j]
        A, S, y = ch[j]

def exact(sols):
    cnt = collections.Counter()
    stack = [(closure(sols), sols, 0)]
    while stack:
        A, S, y = stack.pop()
        cnt[bucket(len(S))] += 1
        stack.extend(children(A, S, y))
    return cnt

def estimate(sols, walks, seed):
    rnd = random.Random(seed)
    runs = [walk(sols, rnd) for _ in range(walks)]
    keys = sorted(set(k for r in runs for k in r))
    res = {}
    for k in keys + ['all']:
        xs = [sum(r.values()) if k == 'all' else r.get(k, 0.0) for r in runs]
        m = sum(xs) / len(xs)
        se = (sum((x - m) ** 2 for x in xs) / (len(xs) - 1) / len(xs)) ** 0.5
        res[k] = (m, se)
    return res

def label(k):
    return 'all' if k == 'all' else ('n=1' if k == 0 else f'n=10^{k-1}..10^{k}-1' if k > 1 else 'n=2..9')

if __name__ == '__main__':
    import os
    ALPHA, BETA = float(os.environ.get('ALPHA', 1.0)), float(os.environ.get('BETA', 0.0))
    mode = sys.argv[1]
    if mode == 'combine':
        import json
        runs = [collections.Counter({int(k): v for k, v in json.loads(l).items()}) for f in sys.argv[2:] for l in open(f)]
        print(f'{len(runs)} walks (ALPHA, BETA as when generated)')
        keys = sorted(set(k for r in runs for k in r))
        for k in keys + ['all']:
            xs = [sum(r.values()) if k == 'all' else r.get(k, 0.0) for r in runs]
            m = sum(xs) / len(xs)
            se = (sum((x - m) ** 2 for x in xs) / (len(xs) - 1) / len(xs)) ** 0.5
            print(f'{label(k):22} {m:.4g} ± {se:.2g}')
        sys.exit()
    sols = load(sys.argv[2])
    if mode == 'raw':
        import json
        rnd = random.Random(int(sys.argv[4]))
        for _ in range(int(sys.argv[3])):
            print(json.dumps(walk(sols, rnd)), flush=True)
        sys.exit()
    if mode == 'estimate':
        walks, seed = int(sys.argv[3]), int(sys.argv[4]) if len(sys.argv) > 4 else 1
        for k, (m, se) in estimate(sols, walks, seed).items():
            print(f'{label(k):22} {m:.4g} ± {se:.2g}')
    else:
        size, walks, seed = int(sys.argv[3]), int(sys.argv[4]), int(sys.argv[5]) if len(sys.argv) > 5 else 1
        sub = np.array(random.Random(seed).sample(list(sols), size), dtype=np.uint64)
        ex = exact(sub)
        est = estimate(sub, walks, seed + 1)
        for k in sorted(ex) + ['all']:
            e = sum(ex.values()) if k == 'all' else ex[k]
            m, se = est.get(k, (0, 0))
            print(f'{label(k):22} exact {e:>10}  estimate {m:.4g} ± {se:.2g}  z={(m - e) / se if se else float("nan"):+.2f}')
