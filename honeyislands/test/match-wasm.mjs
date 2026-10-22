// node test/match-wasm.mjs <games> <ms> [offset] : new wasm AI vs previous JS AI
import { readFileSync } from 'node:fs';
import { WasmEngine } from '../play/hib-wasm.js';
import { loadSolutions, filterBlack, chooseMove } from '../play/engine.js';
const [games = 10, ms = 1000, offset = 0] = process.argv.slice(2).map(Number);
const data = readFileSync(new URL('../play/data/solutions.bin', import.meta.url));
const wasm = readFileSync(new URL('../play/wasm/hib_engine.wasm', import.meta.url));
const ALL = loadSolutions(data.buffer.slice(data.byteOffset, data.byteOffset + data.byteLength));
let wins = 0;
for (let g = 0; g < games; ++g) {
  const E = await WasmEngine.create(wasm, data, 1000 + g + offset);
  const wasmFirst = g % 2 === 0;
  const stones = []; let S = ALL;
  while (S.n > 1) {
    const wTurn = (stones.length % 2 === 0) === wasmFirst;
    const r = wTurn ? E.think(stones, ms, 5 + g + offset + stones.length) : chooseMove(S, ms, { seed: 77 + (g + offset) * 31 + stones.length });
    stones.push(r.move); S = filterBlack(S, r.move);
  }
  const wLast = ((stones.length - 1) % 2 === 0) === wasmFirst;
  if (wLast) ++wins;
  console.log(`game ${g} wasmFirst=${wasmFirst} len=${stones.length} winner=${wLast ? 'wasm' : 'js'}`);
}
console.log(`wasm ${wins}/${games}`);
