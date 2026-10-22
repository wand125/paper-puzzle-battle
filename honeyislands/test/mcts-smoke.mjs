import { readFileSync } from 'node:fs';
import { loadSolutions, Universe, MCTS } from '../play/engine.js';
const buf = readFileSync(new URL('../play/data/solutions.bin', import.meta.url));
let t = performance.now();
const U = new Universe(loadSolutions(buf.buffer.slice(buf.byteOffset, buf.byteOffset + buf.byteLength)));
console.log('universe ms', (performance.now() - t).toFixed(0));
t = performance.now(); const c = U.child(U.root, 30); console.log('bitset child ms', (performance.now() - t).toFixed(2), c.n, c.bs, c.idx?.length);
const m = new MCTS(U, { seed: 1 });
t = performance.now(); let k = 0; while (performance.now() - t < 1000) { m.playout(U.root); ++k; } console.log('playouts/s from root', k);
const stones = [];
for (let ply = 0; ply < 30; ++ply) {
  const r = m.search(stones, 1000);
  console.log(ply, r.move, r.proven, typeof r.value === 'number' ? r.value.toFixed(3) : r.value, r.info, r.ms?.toFixed(0));
  if (r.move == null) break;
  stones.push(r.move);
}
