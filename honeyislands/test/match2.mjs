// node test/match2.mjs <A> <B> <games> <ms> : players = mcts | flat | random
import { readFileSync } from 'node:fs';
import { loadSolutions, Universe, MCTS, chooseMove, legalMoves } from '../analyze/engine.js';
const [A, B, games = 10, ms = 1000] = process.argv.slice(2);
const buf = readFileSync(new URL('../analyze/data/solutions.bin', import.meta.url));
const U = new Universe(loadSolutions(buf.buffer.slice(buf.byteOffset, buf.byteOffset + buf.byteLength)));
const mk = (kind, g) => {
  if (kind === 'mcts') { const m = new MCTS(U, { seed: 1000 + g }); return (st, S) => m.search(st, +ms, S); }
  if (kind === 'flat') return (st, S) => chooseMove(U.toList(S), +ms, { seed: 77 + g * 31 + st.length });
  return (st, S) => { const L = legalMoves(U.toList(S)); return { move: L[Math.floor(Math.random() * L.length)] }; };
};
let aWins = 0;
for (let g = 0; g < +games; ++g) {
  const aFirst = g % 2 === 0;
  const pa = mk(A, g), pb = mk(B, g);
  const stones = []; let S = U.root;
  while (S.n > 1) {
    const aTurn = (stones.length % 2 === 0) === aFirst;
    const r = (aTurn ? pa : pb)(stones, S);
    stones.push(r.move); S = U.child(S, r.move);
  }
  const aLast = ((stones.length - 1) % 2 === 0) === aFirst;
  if (aLast) ++aWins;
}
console.log(`${A} vs ${B}: ${A} ${aWins}/${games}`);
