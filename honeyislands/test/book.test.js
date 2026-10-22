import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { WasmEngine } from '../analyze/hib-wasm.js';
import { loadSolutions, filterBlack, Solver } from '../analyze/engine.js';

const read = p => readFileSync(new URL(p, import.meta.url));
const data = read('../analyze/data/solutions.bin');
const E = await WasmEngine.create(read('../analyze/wasm/hib_engine.wasm'), data, 1);
const nNormal = E.loadBook(read('../analyze/data/book-normal.bin'), 'normal');
const nPass = E.loadBook(read('../analyze/data/book-pass.bin'), 'pass');
const ALL = loadSolutions(data.buffer.slice(data.byteOffset, data.byteOffset + data.byteLength));
// dashboard boards: move1[c].v = value of the position after first move c for the player to move
const move1 = rule => JSON.parse(read(`../results/${rule}/first-moves.json`)).move1;

test('books load', () => {
  assert.ok(nNormal > 200000, String(nNormal));
  assert.ok(nPass > 70000, String(nPass));
});

for (const rule of ['normal', 'pass']) {
  test(`${rule}: the first move comes from the book and wins`, () => {
    const m1 = move1(rule);
    for (let seed = 1; seed <= 20; ++seed) {
      const r = E.think([], 2000, seed, rule);
      assert.ok(r.book && r.proven && r.value > 0, JSON.stringify(r));
      assert.ok(r.ms < 500, `book move took ${r.ms} ms`);
      assert.equal(m1[r.move].v, 0, `first move ${r.move} should leave the opponent lost`);
    }
  });
}

test('normal: book reply on a small position agrees with the JS solver', () => {
  const stones = [14, 18, 0, 60];   // 3,136 solutions, winning
  const r = E.think(stones, 8000, 3, 'normal');
  assert.ok(r.proven && r.value > 0, JSON.stringify(r));
  let S = ALL;
  for (const c of stones) S = filterBlack(S, c);
  const k = filterBlack(S, r.move);
  assert.ok(k.n === 1 || !new Solver().win(k), 'the move leaves the opponent lost');
});

test('normal: the book follows a game through move 4', () => {
  // AI moves first and second; the opponent answers with the book move too
  const stones = [];
  for (let i = 0; i < 4; ++i) {
    const r = E.think(stones, 2000, 10 + i, 'normal');
    stones.push(r.move);
    if (i % 2 === 0) assert.ok(r.book, `AI move ${i + 1}: ${JSON.stringify(r)}`);
  }
});

for (const rule of ['normal', 'pass']) {
  test(`${rule}: analysis of the first move matches the book results`, () => {
    const m1 = move1(rule);
    const r = E.evaluate([], 1000, rule);
    for (let c = 0; c < 61; ++c) {
      assert.equal(r.source[c], 1, `cell ${c} should come from the book`);
      assert.equal(r.value[c], m1[c].v === 0 ? 1 : 0, `cell ${c}`);
    }
  });
}

test('normal: analysis of a small position agrees with the JS solver', () => {
  const stones = [14, 18, 0, 60];
  let S = ALL;
  for (const c of stones) S = filterBlack(S, c);
  const r = E.evaluate(stones, 20000, 'normal');
  let wins = 0;
  for (let c = 0; c < 61; ++c) {
    if (r.value[c] < 0) continue;
    const k = filterBlack(S, c);
    const ref = k.n === 1 || !new Solver().win(k) ? 1 : 0;
    assert.equal(r.value[c], ref, `cell ${c}`);
    wins += ref;
  }
  assert.ok(wins > 0);
});
