// node test/match.mjs <games> <ms> [exactLimit] [solverNodes] [c]
// MCTS vs the old flat Monte Carlo AI, alternating the first player.
import { readFileSync } from 'node:fs';
import { loadSolutions, Universe, MCTS, chooseMove } from '../play/engine.js';
const [games = 10, ms = 1000, exactLimit = 3000, solverNodes = 2000, c = 0.8, maxDepth = 64, smart = 1, proofShare = 0.5, proofLimit = 6000, offset = 0] = process.argv.slice(2).map(Number);
const buf = readFileSync(new URL('../play/data/solutions.bin', import.meta.url));
const ALL = loadSolutions(buf.buffer.slice(buf.byteOffset, buf.byteOffset + buf.byteLength));
const U = new Universe(ALL);
let mctsWins = 0;
for (let g = 0; g < games; ++g) {
  const m = new MCTS(U, { seed: 1000 + g + offset, exactLimit, solverNodes, c, maxDepth, smartPlayout: !!smart, proofShare, proofLimit, handoff: +(process.env.HANDOFF ?? 30000), exactLimit: +(process.env.MEXACT ?? exactLimit) });
  const mctsFirst = g % 2 === 0;
  const stones = [];
  let S = U.root;
  for (;;) {
    const mctsTurn = (stones.length % 2 === 0) === mctsFirst;
    let r;
    if (mctsTurn) r = m.search(stones, ms, S);
    else r = chooseMove(U.toList(S), ms, { seed: 77 + (g + offset) * 31 + stones.length, exactLimit: +(process.env.FLATEXACT ?? 6000) });
    if (process.env.V) console.log(`  ${stones.length} ${mctsTurn ? "M" : "F"} n=${S.n} move=${r.move} proven=${r.proven} v=${typeof r.value === "number" ? r.value.toFixed(2) : r.value} ${r.info}`);
    if (r.move == null) break;
    stones.push(r.move);
    S = U.child(S, r.move);
    if (S.n <= 1) break;
  }
  const lastMoverIsMcts = ((stones.length - 1) % 2 === 0) === mctsFirst;
  if (lastMoverIsMcts) ++mctsWins;
  console.log(`game ${g} mctsFirst=${mctsFirst} len=${stones.length} winner=${lastMoverIsMcts ? 'MCTS' : 'flat'}`);
}
console.log(`MCTS ${mctsWins}/${games}`);
