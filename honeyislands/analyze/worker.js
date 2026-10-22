import { WasmEngine } from './hib-wasm.js';

let E = null;
const RULES = ['normal', 'pass'];
// opening books are optional: without them the AI only searches
const book = rule => fetch(new URL(`./data/book-${rule}.bin`, import.meta.url))
  .then(r => (r.ok ? r.arrayBuffer() : null)).catch(() => null);

const ready = Promise.all([
  fetch(new URL('./wasm/hib_engine.wasm', import.meta.url)).then(r => { if (!r.ok) throw new Error('hib_engine.wasm ' + r.status); return r.arrayBuffer(); }),
  fetch(new URL('./data/solutions.bin', import.meta.url)).then(r => { if (!r.ok) throw new Error('solutions.bin ' + r.status); return r.arrayBuffer(); }),
  ...RULES.map(book),
]).then(async ([wasm, data, ...books]) => {
  E = await WasmEngine.create(wasm, data, (Math.random() * 2 ** 32) >>> 0);
  books.forEach((b, i) => { if (b) E.loadBook(b, RULES[i]); });
  postMessage({ kind: 'ready', total: E.total });
}).catch(e => postMessage({ kind: 'error', message: String(e) }));

onmessage = async ({ data }) => {
  await ready;
  if (!E) return;
  if (data.kind === 'analyze') {
    const a = E.analyze(data.stones);
    const state = a.counts.map(c => (c === a.n ? 1 : c === 0 ? 0 : 0.5));
    const legal = [];
    for (let c = 0; c < state.length; ++c) if (state[c] === 0.5) legal.push(c);
    postMessage({ kind: 'analysis', id: data.id, n: a.n, state, ratio: a.counts.map(c => (a.n ? c / a.n : 0)), legal });
  } else if (data.kind === 'eval') {
    postMessage({ kind: 'eval', id: data.id, ...E.evaluate(data.stones, data.ms, data.rule) });
  } else if (data.kind === 'think') {
    const r = E.think(data.stones, data.ms, undefined, data.rule);
    postMessage({ kind: 'move', id: data.id, ...r });
  }
};
