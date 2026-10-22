"""Statistics of games sampled from the perfect-play tree (`hib ppgames`).
Usage: python3 tools/analysis/pp_stats.py <normal.txt> <pass.txt> [bootstrap reps]
Only finished games (with a G row) are used. Intervals are 95% percentile bootstrap
intervals over games (positions of one game are not independent).
"""
import collections, math, random, sys

def load(path):
    P, G = collections.defaultdict(list), {}
    for l in open(path):
        f = l.split()
        if f[0] == 'P':
            g, ply, n, p, d, opts, wins, win, pick_rh, rh_opt, nodes, ms = map(float, f[1:])
            P[int(g)].append(dict(ply=int(ply), n=int(n), p=int(p), d=int(d), opts=int(opts), wins=int(wins),
                                  win=int(win), pick_rh=int(pick_rh), rh_opt=int(rh_opt), nodes=nodes, ms=ms))
        elif f[0] == 'G':
            G[int(f[1])] = (int(f[2]), float(f[3]))
    return [(G[g], P[g]) for g in sorted(G)]

def boot(games, stat, reps):
    rnd = random.Random(1)
    xs = sorted(stat([games[rnd.randrange(len(games))] for _ in games]) for _ in range(reps))
    return xs[int(0.025 * reps)], xs[int(0.975 * reps) - 1]

def fmt(games, stat, reps, f='{:.2f}'):
    lo, hi = boot(games, stat, reps)
    return (f + ' [' + f + ', ' + f + ']').format(stat(games), lo, hi)

def ratio(games, sel, num, den):
    a = b = 0
    for _, ps in games:
        for q in ps:
            if sel(q): a += num(q); b += den(q)
    return a / b if b else float('nan')

reps = int(sys.argv[3]) if len(sys.argv) > 3 else 1000
for path, name in ((sys.argv[1], 'standard'), (sys.argv[2], 'redundant-hint')):
    games = load(path)
    print(f'== {name}: {len(games)} finished games')
    if not games: continue
    L = [g[0][0] for g in games]
    print('  plies: mean', fmt(games, lambda gs: sum(g[0][0] for g in gs) / len(gs), reps),
          'median', sorted(L)[len(L) // 2], 'range', min(L), max(L))
    print('  length histogram', dict(sorted(collections.Counter(L).items())))
    lw = lambda gs: math.log10(sum(10 ** g[0][1] for g in gs) / len(gs))
    print('  perfect-play games (Knuth estimate), log10:', fmt(games, lw, reps))
    print('  winning-option share at winning positions, by move number (pooled):')
    for ply in range(0, max(L)):
        sel = lambda q, ply=ply: q['ply'] == ply and q['win'] == 1
        cnt = sum(sel(q) for _, ps in games for q in ps)
        if cnt < 10: continue
        st = lambda gs, sel=sel: ratio(gs, sel, lambda q: q['wins'], lambda q: q['opts'])
        print(f'    move {ply + 1:2} ({"first" if ply % 2 == 0 else "second"} player): positions {cnt:5}  share {fmt(games, st, reps, "{:.3f}")}  mean options {ratio(games, sel, lambda q: q["opts"], lambda q: 1):.1f}')
    allw = lambda gs: ratio(gs, lambda q: q['win'] == 1 and q['ply'] >= 4, lambda q: q['wins'], lambda q: q['opts'])
    print('  share over all winning positions from ply 4:', fmt(games, allw, reps, '{:.3f}'))
    only1 = lambda gs: ratio(gs, lambda q: q['win'] == 1 and q['ply'] >= 4, lambda q: q['wins'] == 1, lambda q: 1)
    print('  winning positions (ply>=4) with a single winning option:', fmt(games, only1, reps, '{:.3f}'))
    if name == 'redundant-hint':
        has = lambda gs: sum(any(q['pick_rh'] for q in ps) for _, ps in gs) / len(gs)
        print('  games with a redundant hint played:', fmt(games, has, reps, '{:.3f}'))
        rh = [q for _, ps in games for q in ps if q['pick_rh']]
        print(f'  redundant hints played: {len(rh)}, per game {len(rh) / len(games):.2f}; by winner {sum(q["win"] for q in rh)}, by loser {sum(1 - q["win"] for q in rh)}')
        wr = [q for _, ps in games for q in ps if q['win'] and q['rh_opt'] >= 0]
        print(f'  winning positions with a redundant hint available: {len(wr)}; the hint wins in {sum(q["rh_opt"] == 1 for q in wr)}; it is the only winning option in {sum(q["rh_opt"] == 1 and q["wins"] == 1 for q in wr)}')
        print('  solutions left when a redundant hint is played:', dict(sorted(collections.Counter(min(q['n'], 99) for q in rh).items())))
        fr = [next((q['ply'] / len(ps) for q in ps if q['pick_rh']), None) for _, ps in games]
        fr = [x for x in fr if x is not None]
        if fr: print(f'  first redundant hint at fraction {sum(fr) / len(fr):.2f} of the game (games with one)')
    cost = collections.defaultdict(float)
    for _, ps in games:
        for q in ps: cost[q['ply']] += q['ms']
    print('  CPU seconds per game by ply:', {k: round(v / len(games) / 1000, 1) for k, v in sorted(cost.items()) if v / len(games) > 100})
