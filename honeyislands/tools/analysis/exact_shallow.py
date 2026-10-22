"""Exact comparison of the two rules on the complete game tree up to depth 3.
Uses results/<rule>/{positions,edges,depth3-results}.txt; no sampling.
Edges are moves (one per cell), so multiplicities count moves, not distinct children.
Usage: python3 tools/analysis/exact_shallow.py
"""
import collections

def load(rule):
    pos = {}
    for l in open(f'results/{rule}/positions.txt'):
        d, k, n, *_ = l.split()
        pos[k] = (int(d), int(n))
    edges = collections.defaultdict(list)
    for l in open(f'results/{rule}/edges.txt'):
        d, a, b, c = l.split()
        edges[a].append((b, int(c)))
    root = '0:0' if rule == 'pass' else '0'
    pos[root] = (0, 1304983)
    val, nodes = {}, {}
    for l in open(f'results/{rule}/depth3-results.txt'):
        k, v, _, _, _, nd, _ = l.split()
        val.setdefault(k, int(v)); nodes.setdefault(k, int(nd))
    for depth in (2, 1, 0):
        for k, (d, _) in pos.items():
            if d == depth:
                val[k] = int(any(val[b] == 0 for b, _ in edges[k]))
    return pos, edges, val

R = {r: load(r) for r in ('normal', 'pass')}
strip = lambda k: k.split(':')[0]
for depth in range(4):
    keys = {strip(k): k for k, (d, _) in R['pass'][0].items() if d == depth}
    nk = [k for k, (d, _) in R['normal'][0].items() if d == depth]
    same_moves = sum(1 for k in nk if k in keys and len(R['normal'][1][k]) == len(R['pass'][1][keys[k]]))
    t = collections.Counter((R['normal'][2][k], R['pass'][2][keys[k]]) for k in nk)
    print(f'depth {depth}: {len(nk)} positions (paired {sum(k in keys for k in nk)}); '
          f'mover wins (std,rh): ' + ', '.join(f'{a}{b}:{t[(a, b)]}' for a in (1, 0) for b in (1, 0)) +
          (f'; same number of moves {same_moves}' if depth < 3 else ''))
    if depth < 3:
        for r in ('normal', 'pass'):
            pos, edges, val = R[r]
            ks = [k for k, (d, _) in pos.items() if d == depth]
            win = [k for k in ks if val[k] == 1]
            tot = sum(len(edges[k]) for k in win)
            wm = sum(sum(val[b] == 0 for b, _ in edges[k]) for k in win)
            per = sorted(sum(val[b] == 0 for b, _ in edges[k]) / len(edges[k]) for k in win)
            print(f'   {r:6}: winning positions {len(win)}, winning moves {wm}/{tot} = {wm / max(tot, 1):.3f}'
                  + (f', per position median {per[len(per) // 2]:.3f} min {per[0]:.3f} max {per[-1]:.3f}' if per else ''))
