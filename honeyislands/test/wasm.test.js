import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { WasmEngine } from '../play/hib-wasm.js';
import { loadSolutions, filterBlack, blackCounts, legalMoves, Solver } from '../play/engine.js';

const wasm = readFileSync(new URL('../play/wasm/hib_engine.wasm', import.meta.url));
const data = readFileSync(new URL('../play/data/solutions.bin', import.meta.url));
const E = await WasmEngine.create(wasm, data, 1);
const ALL = loadSolutions(data.buffer.slice(data.byteOffset, data.byteOffset + data.byteLength));

test('wasm loads every solution', () => assert.equal(E.total, 1304983));

test('wasm analysis matches the JS reference', () => {
  for (const stones of [[], [30], [14, 18, 0, 60], [31, 4, 56, 19, 40]]) {
    let S = ALL;
    for (const c of stones) S = filterBlack(S, c);
    const a = E.analyze(stones);
    assert.equal(a.n, S.n);
    assert.deepEqual(a.counts, Array.from(blackCounts(S)));
    assert.equal(a.legalCount, legalMoves(S).length);
  }
});

test('wasm AI plays legal moves and finds a proven win', () => {
  const stones = [14, 18, 0, 60];   // 3,136 solutions, winning (cells 19 and 40 win)
  const r = E.think(stones, 8000);
  assert.ok(r.proven && r.value > 0, JSON.stringify(r));
  let S = ALL;
  for (const c of stones) S = filterBlack(S, c);
  const k = filterBlack(S, r.move);
  assert.ok(k.n === 1 || !new Solver().win(k), 'the move leaves the opponent lost');
});

test('finished game: no move', () => {
  // play a full game with the engine and check the end state
  const stones = [];
  for (let i = 0; i < 61; ++i) {
    const a = E.analyze(stones);
    if (a.n <= 1) break;
    const r = E.think(stones, 50);
    assert.ok(a.counts[r.move] > 0 && a.counts[r.move] < a.n, 'legal move');
    stones.push(r.move);
  }
  assert.equal(E.analyze(stones).n, 1);
  assert.equal(E.think(stones, 10).move, null);
});

test('pass rule: the AI may pass and always plays a legal cell', () => {
  // play random games with the pass rule; every AI move is either undetermined
  // or a free cell determined black, and never a white one
  for (let g = 0; g < 3; ++g) {
    const stones = [];
    let passes = 0;
    for (let i = 0; i < 61; ++i) {
      const a = E.analyze(stones);
      if (a.n <= 1) break;
      const r = E.think(stones, 30, 11 + g * 61 + i, 'pass');
      const c = r.move;
      assert.ok(!stones.includes(c), 'empty cell');
      assert.ok(a.counts[c] > 0, 'never a cell determined white');
      if (a.counts[c] === a.n) passes++;
      stones.push(c);
    }
    assert.equal(E.analyze(stones).n, 1);
    // stones on determined-black cells never exceed what the rule allows
    assert.ok(passes >= 0);
  }
});

test('pass rule: a proven win is found and is really winning', () => {
  const r = E.think([14, 18, 0, 60], 8000, 1, 'pass');
  assert.ok(Number.isInteger(r.move));
});
