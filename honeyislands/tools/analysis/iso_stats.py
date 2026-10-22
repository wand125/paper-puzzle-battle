"""Paired cost of solving the same depth-3 position under both rules (`hib isocost` output).
Usage: python3 tools/analysis/iso_stats.py <iso-jobs.txt> <isocost output>... 
Per stratum (standard value, redundant-hint value; 1 = side to move wins): geometric mean of the
node ratio rh/std with a 95% bootstrap interval, and the median nodes and seconds of each rule.
"""
import math, random, sys
strat = {l.split()[0]: l.split()[2] for l in open(sys.argv[1])}
rows = []
for path in sys.argv[2:]:
    for l in open(path):
        f = l.split()
        if len(f) == 9: rows.append(f)
rnd = random.Random(1)
gm = lambda xs: math.exp(sum(xs) / len(xs))
med = lambda xs: sorted(xs)[len(xs) // 2]
bad = [r[0] for r in rows if r[3] + r[6] != strat[r[0]]]
print(f'positions {len(rows)}; values disagreeing with the stratum label: {len(bad)}')
for s in ('11', '10', '01', '00'):
    rs = [r for r in rows if strat[r[0]] == s]
    lr = [math.log(max(1, int(r[7])) / max(1, int(r[4]))) for r in rs]
    bs = sorted(gm([lr[rnd.randrange(len(lr))] for _ in lr]) for _ in range(5000))
    sn, rn = [int(r[4]) for r in rs], [int(r[7]) for r in rs]
    st, rt = [int(r[5]) / 1000 for r in rs], [int(r[8]) / 1000 for r in rs]
    less = sum(b < a for a, b in zip(sn, rn))
    print(f'std={s[0]} rh={s[1]}: n {len(rs):3}  nodes rh/std {gm(lr):7.3f} [{bs[125]:.3f}, {bs[4874]:.3f}]'
          f'  rh fewer in {less}/{len(rs)}  median nodes std {med(sn):>10} rh {med(rn):>10}'
          f'  median s std {med(st):7.2f} rh {med(rt):7.2f}  max s std {max(st):7.1f} rh {max(rt):7.1f}')
