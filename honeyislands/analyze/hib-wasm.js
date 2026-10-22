// Thin wrapper over the Rust engine (web/wasm/hib_engine.wasm).
import { N } from './engine.js';

export class WasmEngine {
  // wasmBytes / dataBytes: ArrayBuffer or typed array
  static async create(wasmBytes, dataBytes, seed = 1) {
    const { instance } = await WebAssembly.instantiate(wasmBytes, { env: { hib_now: () => performance.now() } });
    const e = new WasmEngine();
    e.x = instance.exports;
    const data = new Uint8Array(dataBytes);
    const ptr = e.x.hib_alloc(data.length);
    new Uint8Array(e.x.memory.buffer, ptr, data.length).set(data);
    e.total = e.x.hib_init(ptr, data.length, seed >>> 0);
    return e;
  }

  // opening book (data/book-<rule>.bin); returns the number of positions
  loadBook(bytes, rule = 'normal') {
    const data = new Uint8Array(bytes);
    const ptr = this.x.hib_alloc(data.length);
    new Uint8Array(this.x.memory.buffer, ptr, data.length).set(data);
    return this.x.hib_book(ptr, data.length, rule === 'pass' ? 1 : 0);
  }

  setStones(stones) {
    if (stones.length > 64) throw new Error('too many stones');
    new Uint8Array(this.x.memory.buffer, this.x.hib_stones(), 64).set(stones);
  }

  // {n, legalCount, counts[61]}
  analyze(stones) {
    this.setStones(stones);
    const p = this.x.hib_analyze(stones.length);
    const o = new Uint32Array(this.x.memory.buffer, p, 2 + N);
    return { n: o[0], legalCount: o[1], counts: Array.from(o.subarray(2)) };
  }

  // value of every move: {value[61], source[61]}
  // value: -1 not playable, 0 loses, 1 wins, 2 unknown; source: 0 none, 1 book, 2 solved now
  evaluate(stones, ms, rule = 'normal') {
    this.setStones(stones);
    const p = this.x.hib_eval(stones.length, ms, rule === 'pass' ? 1 : 0);
    const o = new Int8Array(this.x.memory.buffer, p, 2 * N);
    return { value: Array.from(o.subarray(0, N)), source: Array.from(o.subarray(N)) };
  }

  // {move, proven, value, info, ms, book}  (book: the move came from the opening book)
  // rule: 'normal', or 'pass' (stones may also go on cells determined black)
  think(stones, ms, seed = (Math.random() * 2 ** 32) >>> 0, rule = 'normal') {
    this.setStones(stones);
    const p = this.x.hib_think(stones.length, ms, seed >>> 0, rule === 'pass' ? 1 : 0);
    const o = new Float64Array(this.x.memory.buffer, p, 8);
    const move = o[0] < 0 ? null : o[0];
    return { move, proven: o[1] === 1, value: o[2], info: o[6] === 1 ? 'book' : `wasm ${o[3]} it, ${o[4]} playouts`, ms: o[5], book: o[6] === 1 };
  }
}
