import { WasmEngine } from './hib-wasm.js';
import { N, loadSolutions, filterBlack } from './engine.js';

let E = null;
let solutionBytes = null, solutionSet = null;
const RULES = ['normal', 'pass'];

// download progress over all files: {kind: 'progress', loaded, total} (total 0 if unknown)
const sizes = new Map(), done = new Map();
let lastPost = 0;
function progress(force) {
  const now = performance.now();
  if (!force && now - lastPost < 100) return;
  lastPost = now;
  const sum = m => [...m.values()].reduce((a, b) => a + b, 0);
  const unknown = [...sizes.values()].some(s => !s);
  postMessage({ kind: 'progress', loaded: sum(done), total: unknown ? 0 : sum(sizes) });
}
async function load(path, required) {
  const r = await fetch(new URL(path, import.meta.url));
  if (!r.ok) { if (required) throw new Error(`${path} ${r.status}`); return null; }
  sizes.set(path, +r.headers.get('content-length') || 0);
  done.set(path, 0);
  if (!r.body) { const b = await r.arrayBuffer(); done.set(path, b.byteLength); progress(true); return b; }
  const chunks = [];
  for (const reader = r.body.getReader(); ;) {
    const { done: end, value } = await reader.read();
    if (end) break;
    chunks.push(value);
    done.set(path, done.get(path) + value.length);
    progress(false);
  }
  const out = new Uint8Array(done.get(path));
  let o = 0;
  for (const c of chunks) { out.set(c, o); o += c.length; }
  progress(true);
  return out.buffer;
}
// opening books are optional: without them the AI only searches
const book = rule => load(`./data/book-${rule}.bin`, false).catch(() => null);

const ready = Promise.all([
  load('./wasm/hib_engine.wasm', true),
  load('./data/solutions.bin', true),
  ...RULES.map(book),
]).then(async ([wasm, data, ...books]) => {
  solutionBytes = data;
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
  } else if (data.kind === 'history') {
    // Validate imports in order; never feed an illegal prefix to the UI.
    const stones = [], moves = [];
    let a = E.analyze(stones);
    for (const c of data.stones) {
      if (!Number.isInteger(c) || c < 0 || c >= N || stones.includes(c) || a.n <= 1) break;
      const count = a.counts[c];
      if (!count || (count === a.n && data.rule !== 'pass')) break;
      moves.push({ cell: c, pass: count === a.n, n: count });
      stones.push(c);
      a = E.analyze(stones);
    }
    postMessage({ kind: 'history', id: data.id, moves, stopped: moves.length < data.stones.length });
  } else if (data.kind === 'solutions') {
    // The game never requests thumbnails, so only allocate the JS set on demand.
    solutionSet ??= loadSolutions(solutionBytes);
    let s = solutionSet;
    for (const c of data.stones) s = filterBlack(s, c);
    const limit = Number.isFinite(data.limit) ? Math.max(0, Math.min(64, Math.floor(data.limit))) : 64;
    const masks = Array.from({ length: Math.min(limit, s.n) }, (_, i) => [s.lo[i], s.hi[i]]);
    postMessage({ kind: 'solutions', id: data.id, n: s.n, masks });
  } else if (data.kind === 'eval') {
    const token = data.evalId === undefined ? {} : { evalId: data.evalId };
    postMessage({ kind: 'eval', id: data.id, ...token, ...E.evaluate(data.stones, data.ms, data.rule) });
  } else if (data.kind === 'think') {
    const r = E.think(data.stones, data.ms, undefined, data.rule);
    postMessage({ kind: 'move', id: data.id, ...r });
  }
};
