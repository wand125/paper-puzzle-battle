"""Export the interactive move-1..4 board data for the paper from the solve
dashboard's status.json and m4/*.json (progress fields are dropped).
Usage: python3 tools/export_board.py <dashboard dir> results/<rule>
Writes results/<rule>/board.json ({move1, move2, move3}) and results/<rule>/m4/<c1>-<c2>.json.
Cell entries: v = value after that move for the player to move (0 = the move wins,
1 = it loses, null = not known), n = remaining solutions, dup = same position as
another cell by symmetry. 'black' / 'white' list determined cells.
"""
import json, os, sys
src, out = sys.argv[1], sys.argv[2]
KEEP = ('v', 'n', 'dup')
def clean(c):
    return None if c is None else {k: c[k] for k in KEEP if k in c and c[k] is not None or k == 'v' and k in c}
def view(v):
    r = {'cells': [clean(c) for c in v['cells']], 'black': v['black'], 'white': v['white']}
    if v.get('has4'): r['has4'] = True
    return r
b = json.load(open(f'{src}/status.json'))['board']
board = {'move1': [clean(c) for c in b['move1']],
         'move2': {k: view(v) for k, v in b['move2'].items()},
         'move3': {k: view(v) for k, v in b['move3'].items()}}
json.dump(board, open(f'{out}/board.json', 'w'), separators=(',', ':'))
os.makedirs(f'{out}/m4', exist_ok=True)
for f in os.listdir(f'{src}/m4'):
    d = json.load(open(f'{src}/m4/{f}'))
    json.dump({k: view(v) for k, v in d.items()}, open(f'{out}/m4/{f}', 'w'), separators=(',', ':'))
print(out, os.path.getsize(f'{out}/board.json'), 'bytes,', len(os.listdir(f'{out}/m4')), 'move-4 files')
