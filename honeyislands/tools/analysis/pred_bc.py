"""Predictions B and C.
B (lengths.txt from `hib lengths`): game length under perfect play (winner fastest,
loser slowest) for the same positions under both rules.
C (plines.txt from `hib plines`): where passes occur in best lines under the pass rule.
Usage: python3 tools/analysis/pred_bc.py lengths.txt plines.txt
"""
import collections, statistics as st, sys
L = collections.defaultdict(dict)
for l in open(sys.argv[1]):
    s, ply, n, p, rule, w, plies, nodes, ms = l.split()
    L[s][int(rule)] = (int(w), int(plies), int(nodes), float(ms), int(n), int(p))
pairs = [v for v in L.values() if 0 in v and 1 in v]
print(f'B: {len(pairs)} positions (200-3,000 solutions) solved for game length under both rules')
for r, name in ((0, 'standard'), (1, 'pass')):
    w = [v[r][1] for v in pairs if v[r][0] == 1]
    l = [v[r][1] for v in pairs if v[r][0] == 0]
    print(f'  {name}: mover wins {len(w)}/{len(pairs)}; plies to the end: wins mean {st.mean(w):.2f} median {st.median(w)}, '
          f'losses mean {st.mean(l) if l else 0:.2f}; all mean {st.mean(w + l):.2f}; nodes median {st.median(v[r][2] for v in pairs)}')
d = [v[1][1] - v[0][1] for v in pairs]
print(f'  pass minus standard plies: mean {st.mean(d):+.2f}; shorter {sum(x < 0 for x in d)}, equal {sum(x == 0 for x in d)}, longer {sum(x > 0 for x in d)}')
same = [v for v in pairs if v[0][0] == v[1][0]]
ds = [v[1][1] - v[0][1] for v in same]
print(f'  same value under both rules: {len(same)}; plies difference mean {st.mean(ds):+.2f}; differing value: {len(pairs) - len(same)}')
hp = [v for v in pairs if v[1][5] > 0]
print(f'  positions with passes available: {len(hp)}; value flips among them {sum(v[0][0] != v[1][0] for v in hp)}')

rows = [list(map(int, l.split())) for l in open(sys.argv[2])]
# sample ply plies_left mover_wins is_pass passes_left n regular_win
games = collections.defaultdict(list)
for r in rows: games[r[0]].append(r)
passes = [r for r in rows if r[4] == 1]
print(f'C: {len(games)} best lines under the pass rule, {len(rows)} plies, {len(passes)} passes')
print(f'  lines with at least one pass: {sum(any(r[4] for r in g) for g in games.values())}; '
      f'lines where passes were available at the start: {sum(g[0][5] > 0 for g in games.values())}')
by_left = collections.Counter(min(r[2], 12) for r in passes)
print('  passes by plies left to the end: ' + ' '.join(f'{k}{"+" if k == 12 else ""}:{by_left[k]}' for k in sorted(by_left)))
by_n = collections.Counter(min(r[6], 9) for r in passes)
print('  passes by remaining solutions: ' + ' '.join(f'{k}{"+" if k == 9 else ""}:{by_n[k]}' for k in sorted(by_n)))
win_p = [r for r in passes if r[3] == 1]
print(f'  passes by the winner {len(win_p)} (a regular move would also have won: {sum(r[7] for r in win_p)}; pass was the only win: {sum(1 - r[7] for r in win_p)}); by the loser {len(passes) - len(win_p)}')
first = [next((r for r in g if r[4]), None) for g in games.values()]
first = [(r[1], r[1] + r[2]) for r in first if r]
if first:
    print(f'  first pass at ply (from the sample position) mean {st.mean(a for a, _ in first):.2f}, as a fraction of the line {st.mean(a / t for a, t in first):.2f}')
