import { WasmEngine } from './hib-wasm.js';

let E = null;
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
