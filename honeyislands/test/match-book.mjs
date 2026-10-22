// Book AI vs the same AI without the book. usage: node test/match-book.mjs <rule> <games> <ms>
import { readFileSync } from 'node:fs';
import { WasmEngine } from '../analyze/hib-wasm.js';

const [rule = 'normal', games = 8, ms = 1000] = process.argv.slice(2);
const read = p => readFileSync(new URL(p, import.meta.url));
const wasm = read('../analyze/wasm/hib_engine.wasm'), data = read('../analyze/data/solutions.bin');
const B = await WasmEngine.create(wasm, data, 1);
B.loadBook(read(`../analyze/data/book-${rule}.bin`), rule);
const P = await WasmEngine.create(wasm, data, 2);
const score = { first: [0, 0], second: [0, 0] };
for (let g = 0; g < +games; ++g) {
  const bookFirst = g % 2 === 0;
  const stones = [];
  let bookMoves = 0;
  while (B.analyze(stones).n > 1) {
    const bookTurn = (stones.length % 2 === 0) === bookFirst;
    const r = (bookTurn ? B : P).think(stones, +ms, 100 * g + stones.length, rule);
    if (bookTurn && r.book) ++bookMoves;
    stones.push(r.move);
  }
  const lastByBook = ((stones.length - 1) % 2 === 0) === bookFirst;
  const k = bookFirst ? 'first' : 'second';
  score[k][lastByBook ? 0 : 1]++;
  console.log(`game ${g + 1}: book AI ${bookFirst ? 'first' : 'second'}, ${lastByBook ? 'wins' : 'loses'} in ${stones.length} moves (book moves ${bookMoves})`);
}
console.log(rule, `book AI first: ${score.first[0]}-${score.first[1]}, second: ${score.second[0]}-${score.second[1]}`);
