"""Summarize the opening-book solve into play/book/status.json.

Inputs (results/book/): jobs-<host>.txt (assigned depth-3 positions),
res-<host>.txt (solved: key value move n ms nodes stones), edges.txt
(child_depth parent_key child_key cell), hosts.json (written by the collector).
Values are for the player to move: 1 = wins, 0 = loses.
"""
import glob, json, os, statistics, sys, time

base = sys.argv[1] if len(sys.argv) > 1 else 'results/book'
out = sys.argv[2] if len(sys.argv) > 2 else 'play/book/status.json'
now = time.time()
# root key: "0" for the normal rule, "0:0" for the pass rule (keys "mask:passes")
ROOT = '0:0' if os.path.exists(f'{base}/positions.txt') and ':' in open(f'{base}/positions.txt').readline().split()[1] else '0'

hosts = json.load(open(f'{base}/hosts.json')) if os.path.exists(f'{base}/hosts.json') else {}
jobs, results = {}, {}
per_host = []
for f in sorted(glob.glob(f'{base}/jobs-*.txt')):
    h = f.rsplit('jobs-', 1)[1][:-4]
    keys = [l.split() for l in open(f) if l.strip()]
    for _, k, n, _s in keys:
        jobs[k] = (h, int(n))
    res = []
    rf = f'{base}/res-{h}.txt'
    if os.path.exists(rf):
        for l in open(rf):
            p = l.split()
            if len(p) >= 6 and p[0] in jobs:
                res.append(p)
                results[p[0]] = {'value': int(p[1]), 'move': int(p[2]), 'n': int(p[3]), 'ms': float(p[4]), 'nodes': int(p[5]), 'host': h}
    info = hosts.get(h, {})
    per_host.append({'host': h, 'assigned': len(keys), 'done': len(res), 'threads': info.get('threads'),
                     'running': info.get('running'), 'cpu': info.get('cpu'), 'checked': info.get('checked'),
                     'ok': info.get('ok'), 'cpu_hours': round(sum(float(p[4]) for p in res) / 3.6e6, 2),
                     'nodes': sum(int(p[5]) for p in res)})

# back up values: a position wins iff some child loses
children = {}
depth_of = {ROOT: 0}
for l in open(f'{base}/edges.txt'):
    d, pk, ck, _c = l.split()
    children.setdefault(pk, set()).add(ck)
    depth_of[ck] = int(d)
value = {k: r['value'] for k, r in results.items()}
for d in (2, 1, 0):
    for pk in [k for k, dd in depth_of.items() if dd == d]:
        kids = children.get(pk, ())
        vals = [value.get(c) for c in kids]
        if any(v == 0 for v in vals):
            value[pk] = 1
        elif kids and all(v == 1 for v in vals):
            value[pk] = 0
# positions still needed to decide the initial position: walk down from the root
# through undecided positions only; undecided depth-3 positions reached are needed.
needed3 = set()
stack = [ROOT] if value.get(ROOT) is None else []
seen_n = set()
while stack:
    k = stack.pop()
    if k in seen_n: continue
    seen_n.add(k)
    for c in children.get(k, ()):
        if value.get(c) is not None: continue
        if depth_of.get(c) == 3: needed3.add(c)
        else: stack.append(c)
unsolved3 = [k for k, dd in depth_of.items() if dd == 3 and k not in results]
deferred = sorted(k for k in unsolved3 if k not in needed3)
tmp = f'{base}/defer.txt.tmp'
with open(tmp, 'w') as g:
    g.writelines(k + '\n' for k in deferred)
os.replace(tmp, f'{base}/defer.txt')
# proof-number ordering of the needed positions (negamax form):
#   pn(x) = min over children of dn(c)   (x is a win if some child is a loss)
#   dn(x) = sum over children of pn(c)   (x is a loss if every child is a win)
# solved: win -> pn 0, dn inf; loss -> pn inf, dn 0; unsolved depth-3 -> 1, 1.
INF = float('inf')
pn, dn = {}, {}
def pnd(k):
    if k in pn: return pn[k], dn[k]
    v = value.get(k)
    if v == 1: r = (0, INF)
    elif v == 0: r = (INF, 0)
    elif depth_of.get(k) == 3: r = (1, 1)
    else:
        ps = [pnd(c) for c in children.get(k, ())]
        r = (min(d for _, d in ps), sum(p for p, _ in ps))
    pn[k], dn[k] = r
    return r
pnd(ROOT)
order, seen_o = [], set()
def walk(k):
    # most-proving first: children in increasing dn
    if k in seen_o or value.get(k) is not None: return
    seen_o.add(k)
    if depth_of.get(k) == 3:
        order.append(k); return
    for c in sorted(children.get(k, ()), key=lambda c: (dn[c], pn[c])):
        walk(c)
walk(ROOT)
tmp = f'{base}/order.txt.tmp'
with open(tmp, 'w') as g:
    g.writelines(k + '\n' for k in order)
os.replace(tmp, f'{base}/order.txt')
levels = []
for d in (0, 1, 2, 3):
    ks = [k for k, dd in depth_of.items() if dd == d]
    levels.append({'depth': d, 'positions': len(ks), 'win': sum(value.get(k) == 1 for k in ks),
                   'loss': sum(value.get(k) == 0 for k in ks)})

# throughput from the collection history
hist_f = f'{base}/history.json'
hist = json.load(open(hist_f)) if os.path.exists(hist_f) else []
hist.append([now, len(results), {h['host']: h['done'] for h in per_host}, {h['host']: h['nodes'] for h in per_host}])
hist = [h for h in hist if now - h[0] < 7 * 86400]
json.dump(hist, open(hist_f, 'w'))
def rate(window):
    old = [h for h in hist if now - h[0] >= window]
    ref = old[-1] if old else hist[0]
    dt = now - ref[0]
    return (len(results) - ref[1]) / dt * 3600 if dt > 60 else None
r1 = rate(3600)
remaining = len(jobs) - len(results)
ms = [r['ms'] for r in results.values()]
slow = sorted(results.items(), key=lambda kv: -kv[1]['ms'])[:8]
# ---- board view: first move, and second move after each first move ----
R = 4
cells = [(x, y) for y in range(-R, R + 1) for x in range(-R, R + 1) if abs(x + y) <= R]
idx = {c: i for i, c in enumerate(cells)}
def perm_of(refl, r):
    out = []
    for c in cells:
        d = (c[1], c[0]) if refl else c
        for _ in range(r):
            d = (d[0] + d[1], -d[0])
        out.append(idx[d])
    return out
SYMS = [perm_of(f, r) for f in (False, True) for r in range(6)]
pos = {}
for l in open(f'{base}/positions.txt'):
    d, k, n, st, dm, xm = l.split()
    pos[k] = {'depth': int(d), 'n': int(n), 'stones': [int(c) for c in st.split(',')], 'dm': int(dm), 'xm': int(xm)}
edge_cell = {}   # (parent, cell in parent's representative orientation) -> child
for l in open(f'{base}/edges.txt'):
    d, pk, ck, c = l.split()
    edge_cell[(pk, int(c))] = ck
# depth-3 descendants of every position, for progress
desc = {}
def descendants(k):
    if k in desc: return desc[k]
    d = depth_of.get(k)
    if d == 3: r = {k}
    else:
        r = set()
        for c in children.get(k, ()): r |= descendants(c)
    desc[k] = r
    return r
def info(k):
    ds = descendants(k)
    return {'v': value.get(k), 'done': sum(1 for x in ds if x in results), 'total': len(ds), 'n': pos[k]['n'] if k in pos else 1304983}
def mask_cells(m): return [c for c in range(61) if m >> c & 1]
move1 = []
for c in range(61):
    k = edge_cell.get((ROOT, c))
    move1.append(dict(info(k), key=k) if k else None)
move2 = {}
for c in range(61):
    k1 = move1[c] and move1[c]['key']
    if not k1: continue
    rep = pos[k1]['stones']
    sig = next(p for p in SYMS if [p[x] for x in rep] == [c])   # representative -> actual orientation
    cellsv = [None] * 61
    for d in range(61):
        k2 = edge_cell.get((k1, d))
        if k2: cellsv[sig[d]] = dict(info(k2))
    move2[c] = {'cells': cellsv, 'black': sorted(sig[x] for x in mask_cells(pos[k1]['dm'])), 'white': sorted(sig[x] for x in mask_cells(pos[k1]['xm']))}
# cells equivalent by symmetry: keep one per position, mark the rest 'dup'
def in_wedge(c):
    x, y = cells[c]; z = -x - y
    return x >= y >= z
seen1 = {}
for c in sorted(range(61), key=lambda c: (not in_wedge(c), c)):
    m = move1[c]
    if m is None: continue
    if m['key'] in seen1: m['dup'] = True
    else: seen1[m['key']] = c
for c, m2 in move2.items():
    k1 = move1[c]['key']; rep = pos[k1]['stones']
    sig = next(p for p in SYMS if [p[x] for x in rep] == [c])
    inv = [0] * 61
    for a, b in enumerate(sig): inv[b] = a
    seen2 = {}
    for d in range(61):
        e = m2['cells'][d]
        if e is None: continue
        k2 = edge_cell[(k1, inv[d])]
        if k2 in seen2: e['dup'] = True
        else: seen2[k2] = d
# third move: boards3.txt gives, for wedge first moves, the key of each third move
move3 = {}
if os.path.exists(f'{base}/boards3.txt'):
    for l in open(f'{base}/boards3.txt'):
        c1, c2, dm, xm, parts = l.split()
        c1, c2 = int(c1), int(c2)
        m1 = move1[c1]
        if m1 is None or m1.get('dup'): continue
        e2 = move2[c1]['cells'][c2]
        if e2 is None or e2.get('dup'): continue
        cellsv = [None] * 61
        seen3 = {}
        for part in parts.split(','):
            c3, k3 = part.split(':', 1)
            c3 = int(c3)
            r = results.get(k3)
            e = {'v': r['value'] if r else None, 'n': pos[k3]['n'], 'done': 1 if r else 0, 'total': 1, 'key': k3}
            if k3 in seen3: e['dup'] = True
            else: seen3[k3] = c3
            cellsv[c3] = e
        move3[f'{c1},{c2}'] = {'cells': cellsv, 'black': mask_cells(int(dm)), 'white': mask_cells(int(xm))}
# fourth move: values of depth-4 positions known so far
v4 = {}
if os.path.exists(f'{base}/known-all.txt'):
    for l in open(f'{base}/known-all.txt'):
        k, v = l.split()[:2]
        v4[k] = int(v)
if os.path.exists(f'{base}/winkeys.txt'):
    for l in open(f'{base}/winkeys.txt'):
        k3, k4 = l.split()
        v4[k4] = 0                      # the winning reply leaves the opponent lost
lost3 = {k for k, r in results.items() if r['value'] == 0}
m4dir = os.path.join(os.path.dirname(out), 'm4')
os.makedirs(m4dir, exist_ok=True)
if os.path.exists(f'{base}/boards4.txt'):
    files = {}
    for l in open(f'{base}/boards4.txt'):
        c1, c2, c3, dm, xm, parts = l.split()
        key = f'{c1},{c2}'
        if key not in move3: continue
        e3 = move3[key]['cells'][int(c3)]
        if e3 is None or e3.get('dup'): continue
        cellsv = [None] * 61
        seen4 = {}
        for part in parts.split(','):
            c4, k4 = part.split(':', 1)
            c4 = int(c4)
            e = {'v': v4.get(k4)}
            if k4 in seen4: e['dup'] = True
            else: seen4[k4] = c4
            cellsv[c4] = e
        files.setdefault(key, {})[c3] = {'cells': cellsv, 'black': mask_cells(int(dm)), 'white': mask_cells(int(xm)), 'k3': e3.get('key')}
    for key, boards in files.items():
        for c3, bd in boards.items():
            # the player to move at a lost depth-3 position loses with every reply:
            # each depth-4 position is then a win for its player to move
            if bd.pop('k3') in lost3:
                for e in bd['cells']:
                    if e is not None and e['v'] is None: e['v'] = 1
        tmp = f'{m4dir}/{key.replace(",", "-")}.json.tmp'
        json.dump(boards, open(tmp, 'w'), separators=(',', ':'))
        os.replace(tmp, tmp[:-4])
    for e in move3.values():
        e['has4'] = True
for bd in move3.values():
    for e in bd['cells']:
        if e is not None: e.pop('key', None)
board = {'move1': move1, 'move2': move2, 'move3': move3, 'root': info(ROOT)}

status = {
    'updated': now,
    'total': len(jobs), 'done': len(results), 'remaining': remaining,
    'win': sum(r['value'] == 1 for r in results.values()), 'loss': sum(r['value'] == 0 for r in results.values()),
    'cpu_hours': round(sum(ms) / 3.6e6, 2),
    'nodes': sum(r['nodes'] for r in results.values()),
    'known': sum(1 for _ in open(f'{base}/known-all.txt')) if os.path.exists(f'{base}/known-all.txt') else 0,
    'mean_s': round(statistics.mean(ms) / 1000, 1) if ms else None,
    'median_s': round(statistics.median(ms) / 1000, 1) if ms else None,
    'rate_per_hour': round(r1, 1) if r1 else None,
    'eta_hours': round(remaining / r1, 1) if r1 else None,
    'root': value.get(ROOT),
    'needed': len(needed3), 'deferred': len(deferred),
    # proof numbers: infinity (decided) is written as null, which JSON can carry
    'root_pn': None if pn.get(ROOT) == INF else pn.get(ROOT),
    'root_dn': None if dn.get(ROOT) == INF else dn.get(ROOT),
    'levels': levels,
    'hosts': per_host,
    'slowest': [{'n': r['n'], 's': round(r['ms'] / 1000), 'value': r['value'], 'host': r['host']} for _, r in slow],
    'history': [[round(e[0]), e[1], e[2] if len(e) > 2 else {}, e[3] if len(e) > 3 else {}] for e in hist[-2000:]],
    'board': board,
}
os.makedirs(os.path.dirname(out), exist_ok=True)
tmp = out + '.tmp'
json.dump(status, open(tmp, 'w'), ensure_ascii=False, allow_nan=False)
os.replace(tmp, out)
