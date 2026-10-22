"""Solver time per position in `hib ppgames` output (time to evaluate every option).
Usage: python3 tools/analysis/pp_time.py <file>...
"""
import collections, sys
for path in sys.argv[1:]:
    rows = [l.split() for l in open(path) if l.startswith('P ')]
    ms = sorted(float(r[12]) for r in rows)
    by = collections.defaultdict(list)
    for r in rows: by[int(r[2])].append(float(r[12]))
    q = lambda p: ms[min(len(ms) - 1, int(p * len(ms)))] / 1000
    print(f'{path}: positions {len(ms)}  median {q(.5):.3f}s  p99 {q(.99):.1f}s  max {ms[-1]/1000:.1f}s')
    for ply in sorted(by):
        v = sorted(by[ply])
        print(f'  ply {ply:2}: n {len(v):4}  median {v[len(v)//2]/1000:8.3f}s  max {v[-1]/1000:8.1f}s')
