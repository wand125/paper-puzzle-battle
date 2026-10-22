"""How well the random-playout win rate predicts the exact value (`hib playauc` output).
Usage: python3 tools/analysis/playout_auc.py <playauc output>...
AUC = probability that a won position has a higher playout win rate than a lost one (ties 1/2),
with a 95% bootstrap interval over positions.
"""
import random, sys

def auc(rows):
    pos = sorted(r[1] for r in rows if r[0] == 1)
    neg = sorted(r[1] for r in rows if r[0] == 0)
    if not pos or not neg: return float('nan')
    # rank-sum form with midranks for ties
    allv = sorted([(v, 1) for v in pos] + [(v, 0) for v in neg])
    rank_sum, i = 0.0, 0
    while i < len(allv):
        j = i
        while j < len(allv) and allv[j][0] == allv[i][0]: j += 1
        mid = (i + j + 1) / 2
        rank_sum += mid * sum(1 for t in allv[i:j] if t[1] == 1)
        i = j
    return (rank_sum - len(pos) * (len(pos) + 1) / 2) / (len(pos) * len(neg))

for path in sys.argv[1:]:
    rows = []
    for l in open(path):
        k, v, w, n = l.split()
        rows.append((int(v), int(w) / int(n)))
    rnd = random.Random(1)
    bs = sorted(auc([rows[rnd.randrange(len(rows))] for _ in rows]) for _ in range(2000))
    win = [r[1] for r in rows if r[0] == 1]; loss = [r[1] for r in rows if r[0] == 0]
    print(f'{path}: positions {len(rows)} (won {len(win)}, lost {len(loss)})  AUC {auc(rows):.3f} [{bs[50]:.3f}, {bs[1949]:.3f}]'
          f'  mean playout win rate: won {sum(win)/len(win):.3f}, lost {sum(loss)/len(loss):.3f}')
