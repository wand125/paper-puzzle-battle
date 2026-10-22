import { readFileSync } from 'node:fs';
import { loadSolutions, filterBlack, legalMoves, Solver } from '../analyze/engine.js';
const buf = readFileSync(new URL('../analyze/data/solutions.bin', import.meta.url));
let S = loadSolutions(buf.buffer.slice(buf.byteOffset, buf.byteOffset + buf.byteLength));
console.log('all', S.n, 'moves', legalMoves(S).length);
let t = performance.now(); filterBlack(S, 30); console.log('filter ms', (performance.now() - t).toFixed(1));
let seed = 7; const rnd = () => (seed = (seed * 1103515245 + 12345) % 2147483648) / 2147483648;
for (let ply = 0; S.n > 1; ++ply) {
  const ms = legalMoves(S);
  if (S.n < 200000) {
    const sv = new Solver(); sv.deadline = performance.now() + 20000;
    t = performance.now();
    try { const w = sv.win(S); console.log(`ply ${ply} n=${S.n} moves=${ms.length} win=${w} nodes=${sv.nodes} ms=${(performance.now() - t).toFixed(0)}`); }
    catch { console.log(`ply ${ply} n=${S.n} timeout nodes=${sv.nodes}`); }
  } else console.log(`ply ${ply} n=${S.n} moves=${ms.length}`);
  S = filterBlack(S, ms[Math.floor(rnd() * ms.length)]);
}
