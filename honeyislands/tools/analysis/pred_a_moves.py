"""Prediction A on sampled mid-game positions (output of `hib moves`).
Within positions where the mover wins, compare dD (increase of determined-black
cells) of winning moves and losing moves, per rule. Also: win rate by dD.
"""
import collections, statistics, sys
rows = []
for f in sys.argv[1:]:
    for l in open(f):
        s, rule, ply, n, p, dd, kn, ip, cv, pv = map(int, l.split())
        rows.append((f, s, rule, ply, n, p, dd, kn, ip, cv, pv))
for rule in (0, 1):
    R = [r for r in rows if r[2] == rule]
    pos = collections.defaultdict(list)
    for r in R: pos[(r[0], r[1])].append(r)
    winpos = [v for v in pos.values() if v[0][10] == 1]
    w = [r[6] for v in winpos for r in v if r[8] == 0 and r[9] == 0]
    l = [r[6] for v in winpos for r in v if r[8] == 0 and r[9] == 1]
    # per-position comparison: mean dD of winning moves minus mean dD of all regular moves
    diffs = []
    for v in winpos:
        reg = [r for r in v if r[8] == 0]
        ww = [r[6] for r in reg if r[9] == 0]
        if ww and len(ww) < len(reg):
            diffs.append(statistics.mean(ww) - statistics.mean(r[6] for r in reg))
    wins_by_pass = sum(1 for v in winpos for r in v if r[8] == 1 and r[9] == 0)
    hist = collections.defaultdict(lambda: [0, 0])
    for v in winpos:
        for r in v:
            if r[8] == 0: hist[min(r[6], 8)][0 if r[9] == 0 else 1] += 1
    odd = [r for v in winpos for r in v if r[8] == 0]
    def rate(sel): return sum(1 for r in sel if r[9] == 0) / max(1, len(sel))
    print(f'rule {"pass" if rule else "normal"}: positions {len(pos)}, mover wins in {len(winpos)} ({len(winpos)/len(pos):.1%})')
    print(f'  winning moves {len(w)} mean dD {statistics.mean(w):.2f}; losing moves {len(l)} mean dD {statistics.mean(l):.2f}')
    print(f'  per position (winning-move mean dD - all-move mean dD): mean {statistics.mean(diffs):+.2f}, '
          f'positive {sum(d > 0 for d in diffs)}/{len(diffs)}, negative {sum(d < 0 for d in diffs)}')
    print(f'  win rate of a move by dD (winning positions): ' + '  '.join(f'{k}{"+" if k == 8 else ""}:{a/(a+b):.2f}({a+b})' for k, (a, b) in sorted(hist.items())))
    print(f'  win rate by parity of dD: odd {rate([r for r in odd if r[6] % 2]):.3f}, even {rate([r for r in odd if r[6] % 2 == 0]):.3f}; pass moves that win: {wins_by_pass}')
