import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { loadSolutions, filterBlack, legalMoves, cellState, masks, chooseMove, Solver, N } from '../analyze/engine.js';

const buf = readFileSync(new URL('../analyze/data/solutions.bin', import.meta.url));
const ALL = loadSolutions(buf.buffer.slice(buf.byteOffset, buf.byteOffset + buf.byteLength));

test('all solutions loaded, every cell free at the start', () => {
  assert.equal(ALL.n, 1304983);
  assert.equal(N, 61);
  assert.equal(legalMoves(ALL).length, 61);
});

test('legal moves are exactly the undetermined cells, and never empty the set', () => {
  let S = ALL;
  for (const c of [30, 0, 60, 12, 45]) {
    if (!legalMoves(S).includes(c)) continue;
    S = filterBlack(S, c);
  }
  const st = cellState(S);
  const legal = legalMoves(S);
  for (let c = 0; c < N; ++c) assert.equal(legal.includes(c), st[c] === 0.5, `cell ${c}`);
  for (const c of legal) {
    const k = filterBlack(S, c);
    assert.ok(k.n >= 1 && k.n < S.n, 'a legal stone keeps at least one solution and narrows the set');
  }
});

test('a single remaining solution has no legal move (player to move has lost)', () => {
  const one = { lo: ALL.lo.slice(0, 1), hi: ALL.hi.slice(0, 1), n: 1 };
  assert.equal(legalMoves(one).length, 0);
  assert.equal(new Solver().win(one), false);
  const m = masks(one);
  assert.equal(m.aLo, one.lo[0]);
});

test('solver and AI agree on a small endgame', () => {
  let S = ALL, i = 0;
  const seq = [30, 4, 56, 21, 40, 9, 50, 14];
  while (S.n > 3000) { const l = legalMoves(S); S = filterBlack(S, l.includes(seq[i % seq.length]) ? seq[i % seq.length] : l[0]); ++i; }
  const sv = new Solver();
  const win = sv.win(S);
  const r = chooseMove(S, 20000, { solver: new Solver() });
  assert.equal(r.proven, true);
  assert.equal(r.value > 0, win);
  if (win) {
    const k = filterBlack(S, r.move);
    assert.ok(k.n === 1 || !new Solver().win(k), 'AI move leaves the opponent lost');
  }
});
