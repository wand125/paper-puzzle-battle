import sys, time, collections
sys.path.insert(0, __import__('os').path.dirname(__file__))
import count_positions as cp
sols = cp.load('play/data/solutions.bin'); T = int(sys.argv[1])
t = time.time(); cnt = collections.Counter(); stack = [(cp.closure(sols), sols, 0)]
while stack:
    A, S, y = stack.pop()
    if len(S) < T: continue
    cnt[len(S) >= T] += 1
    stack.extend(cp.children(A, S, y))
print(T, 'exact closed sets with n >=', T, ':', cnt[True], f'{time.time()-t:.0f}s')
