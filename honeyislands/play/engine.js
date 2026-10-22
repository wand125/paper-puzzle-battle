// Honey Islands battle engine.
//
// Board: 61 hexagonal cells (radius 4). A solution is the set of 25 black
// cells that splits the remaining 36 white cells into six islands of six.
// All 1,304,983 solutions are precomputed (data/solutions.bin, two Uint32
// words per solution: lo = cells 0..30, hi = cells 31..60).
//
// Battle rule: players alternately place a black stone on a cell that is not
// yet determined (black in some remaining solutions, white in others). The
// player whose stone leaves exactly one solution wins; the opponent has no
// legal move left.

export const RADIUS = 4;
export const CELLS = [];
for (let y = -RADIUS; y <= RADIUS; ++y) {
  for (let x = -RADIUS; x <= RADIUS; ++x) {
    if (Math.abs(x + y) <= RADIUS) CELLS.push({ x, y });
  }
}
export const N = CELLS.length; // 61

export function loadSolutions(buffer) {
  const words = new Uint32Array(buffer);
  const n = words.length >> 1;
  const lo = new Uint32Array(n), hi = new Uint32Array(n);
  for (let i = 0; i < n; ++i) { lo[i] = words[2 * i]; hi[i] = words[2 * i + 1]; }
  return { lo, hi, n };
}

const bitOf = (cell) => (cell < 31 ? [1 << cell, 0] : [0, 1 << (cell - 31)]);

// Solutions in S where `cell` is black.
export function filterBlack(S, cell) {
  const [mLo, mHi] = bitOf(cell);
  const { lo, hi, n } = S;
  let k = 0;
  if (mLo) { for (let i = 0; i < n; ++i) if (lo[i] & mLo) ++k; }
  else { for (let i = 0; i < n; ++i) if (hi[i] & mHi) ++k; }
  const nlo = new Uint32Array(k), nhi = new Uint32Array(k);
  let j = 0;
  if (mLo) { for (let i = 0; i < n; ++i) if (lo[i] & mLo) { nlo[j] = lo[i]; nhi[j++] = hi[i]; } }
  else { for (let i = 0; i < n; ++i) if (hi[i] & mHi) { nlo[j] = lo[i]; nhi[j++] = hi[i]; } }
  return { lo: nlo, hi: nhi, n: k };
}

// Cells black in every / some remaining solution.
export function masks(S) {
  const { lo, hi, n } = S;
  let aLo = 0xffffffff, aHi = 0xffffffff, oLo = 0, oHi = 0;
  for (let i = 0; i < n; ++i) { aLo &= lo[i]; aHi &= hi[i]; oLo |= lo[i]; oHi |= hi[i]; }
  if (n === 0) { aLo = aHi = 0; }
  return { aLo: aLo >>> 0, aHi: aHi >>> 0, oLo: oLo >>> 0, oHi: oHi >>> 0 };
}

export function legalMoves(S, m = masks(S)) {
  const out = [];
  const fLo = (m.oLo & ~m.aLo) >>> 0, fHi = (m.oHi & ~m.aHi) >>> 0;
  for (let c = 0; c < 31; ++c) if (fLo & (1 << c)) out.push(c);
  for (let c = 31; c < N; ++c) if (fHi & (1 << (c - 31))) out.push(c);
  return out;
}

// Per-cell count of solutions with that cell black.
export function blackCounts(S) {
  const cnt = new Uint32Array(N);
  const { lo, hi, n } = S;
  for (let i = 0; i < n; ++i) {
    let a = lo[i], b = hi[i];
    while (a) { const t = a & -a; cnt[31 - Math.clz32(t)]++; a ^= t; }
    while (b) { const t = b & -b; cnt[62 - Math.clz32(t)]++; b ^= t; }
  }
  return cnt;
}

export function cellState(S) {
  // 1 = determined black, 0 = determined white, 0.5 = undetermined (free)
  const m = masks(S);
  const st = new Array(N);
  for (let c = 0; c < N; ++c) {
    const a = c < 31 ? m.aLo & (1 << c) : m.aHi & (1 << (c - 31));
    const o = c < 31 ? m.oLo & (1 << c) : m.oHi & (1 << (c - 31));
    st[c] = a ? 1 : (o ? 0.5 : 0);
  }
  return st;
}

// ---- exact solver -------------------------------------------------------
// The remaining solution set is fully described by its determined-black mask
// (AND of all solutions), which gives a canonical transposition key.

export class Solver {
  constructor() { this.tt = new Map(); this.nodes = 0; this.deadline = Infinity; this.nodeLimit = Infinity; this.aborted = false; }

  // true = player to move wins. S is a solution list.
  //
  // The search re-indexes S's solutions locally and works on bitsets over
  // them: one column per cell, so every child's size is a word-wise
  // AND + popcount, and cells already determined in the parent are skipped.
  win(S) {
    if (S.n <= 1) return false;
    const W = (S.n + 31) >>> 5;
    const col = new Uint32Array(N * W);
    for (let i = 0; i < S.n; ++i) {
      let a = S.lo[i], b = S.hi[i];
      const w = i >>> 5, bit = 1 << (i & 31);
      while (a) { const t = a & -a; col[(31 - Math.clz32(t)) * W + w] |= bit; a ^= t; }
      while (b) { const t = b & -b; col[(62 - Math.clz32(t)) * W + w] |= bit; b ^= t; }
    }
    const idx = new Int32Array(W), val = new Uint32Array(W);
    for (let w = 0; w < W; ++w) { idx[w] = w; val[w] = 0xffffffff; }
    if (S.n & 31) val[W - 1] = ((2 ** (S.n & 31)) - 1) >>> 0;
    this.col = col; this.W = W;
    return this.winB(idx, val, S.n, 0, 0, 0, 0, 0);
  }

  // (bLo,bHi) / (wLo,wHi): cells known to be determined black / white.
  winB(idx, val, n, bLo, bHi, wLo, wHi, depth) {
    if (n <= 1) return false;
    const W = this.W, col = this.col, len = idx.length;
    const cnt = new Int32Array(N);
    let dLo = bLo, dHi = bHi, xLo = wLo, xHi = wHi;
    let res = false;
    for (let c = 0; c < N; ++c) {
      const inLo = c < 31, bit = inLo ? 1 << c : 1 << (c - 31);
      if (inLo ? (bLo & bit) : (bHi & bit)) { cnt[c] = n; continue; }
      if (inLo ? (wLo & bit) : (wHi & bit)) continue;
      const base = c * W;
      let k = 0;
      for (let j = 0; j < len; ++j) k += popcnt(val[j] & col[base + idx[j]]);
      cnt[c] = k;
      if (k === n) { if (inLo) dLo |= bit; else dHi |= bit; }
      else if (k === 0) { if (inLo) xLo |= bit; else xHi |= bit; }
      else if (k === 1) res = true;
    }
    const key = (dLo >>> 0) + ':' + (dHi >>> 0);
    const hit = this.tt.get(key);
    if (hit !== undefined) return hit;
    if (++this.nodes % 1024 === 0 && (performance.now() > this.deadline || this.nodes > this.nodeLimit)) { this.aborted = true; throw ABORT; }
    if (!res) {
      // a child of size 2 hands the opponent an immediate win: never worth trying
      const moves = [];
      for (let c = 0; c < N; ++c) if (cnt[c] > 2 && cnt[c] < n) moves.push(c);
      // try small children first: cheaper and more often decisive
      Solver.order(moves, cnt, n);
      const nidx = new Int32Array(len), nval = new Uint32Array(len);
      for (const c of moves) {
        const base = c * W;
        let k = 0;
        for (let j = 0; j < len; ++j) { const v = val[j] & col[base + idx[j]]; if (v) { nidx[k] = idx[j]; nval[k++] = v; } }
        if (!this.winB(nidx.slice(0, k), nval.slice(0, k), cnt[c], dLo, dHi, xLo, xHi, depth + 1)) { res = true; break; }
      }
    }
    this.tt.set(key, res);
    return res;
  }

  // Winning move for the player to move, or null if the position is lost.
  bestMove(S) {
    const moves = legalMoves(S);
    const kids = moves.map(c => [c, filterBlack(S, c)]).sort((a, b) => a[1].n - b[1].n);
    for (const [c, k] of kids) if (k.n === 1) return c;
    for (const [c, k] of kids) if (!this.win(k)) return c;
    return null;
  }
}
// Child ordering for the OR search; swappable for experiments.
Solver.order = (moves, cnt) => moves.sort((a, b) => cnt[a] - cnt[b]);
const ABORT = new Error('abort');
export const isAbort = (e) => e === ABORT;

// ---- AI ----------------------------------------------------------------

function randomPlayout(S, rand) {
  // returns true if the player to move at S wins under random play
  let toMoveWins = false; // after the loop: whoever made the last move wins
  let cur = S, turn = 0;
  while (cur.n > 1) {
    const moves = legalMoves(cur);
    cur = filterBlack(cur, moves[Math.floor(rand() * moves.length)]);
    turn ^= 1;
  }
  // last mover is the player of parity (turn ^ 1); player to move at S is 0
  toMoveWins = (turn === 1);
  return toMoveWins;
}

function mulberry(seed) {
  return () => { seed |= 0; seed = seed + 0x6d2b79f5 | 0; let t = Math.imul(seed ^ seed >>> 15, 1 | seed); t = t + Math.imul(t ^ t >>> 7, 61 | t) ^ t; return ((t ^ t >>> 14) >>> 0) / 4294967296; };
}

// Choose a move within `ms` milliseconds. Returns {move, proven, value, info}.
// value: +1 proven win, -1 proven loss, otherwise estimated win rate.
export function chooseMove(S, ms, { solver = new Solver(), seed = Date.now(), exactLimit = 6000 } = {}) {
  const t0 = performance.now();
  const end = t0 + ms;
  const rand = mulberry(seed);
  const moves = legalMoves(S);
  if (moves.length === 0) return { move: null, proven: true, value: -1 };
  const kids = moves.map(c => ({ c, S: filterBlack(S, c), win: 0, n: 0, proven: null }));
  for (const k of kids) if (k.S.n === 1) return { move: k.c, proven: true, value: 1, info: 'unique' };

  // 1. exact solve where the child is small enough
  solver.deadline = end;
  kids.sort((a, b) => a.S.n - b.S.n);
  try {
    for (const k of kids) {
      if (k.S.n > exactLimit) break;
      const oppWins = solver.win(k.S);
      k.proven = !oppWins;
      if (k.proven) return { move: k.c, proven: true, value: 1, info: 'solved', ms: performance.now() - t0 };
    }
  } catch (e) { if (!isAbort(e)) throw e; }
  solver.deadline = Infinity;

  const open = kids.filter(k => k.proven === null);
  if (open.length === 0) {
    // every move loses against perfect play: pick the largest child (most room for error)
    const k = kids.reduce((a, b) => (b.S.n > a.S.n ? b : a));
    return { move: k.c, proven: true, value: -1, info: 'lost' };
  }

  // 2. flat Monte Carlo with UCB over the unresolved moves
  let total = 0;
  while (performance.now() < end || total < open.length) {
    let best = open[0], bestU = -Infinity;
    for (const k of open) {
      const u = k.n === 0 ? Infinity : k.win / k.n + Math.sqrt(2 * Math.log(total + 1) / k.n);
      if (u > bestU) { bestU = u; best = k; }
    }
    // child is the opponent's position: we win if the opponent loses the playout
    if (!randomPlayout(best.S, rand)) best.win++;
    best.n++; total++;
  }
  const pick = open.reduce((a, b) => (b.n > a.n ? b : a));
  return { move: pick.c, proven: false, value: pick.win / pick.n, info: `mc ${total}`, ms: performance.now() - t0 };
}

// ---- bitset representation (fast for large solution sets) ----------------
//
// A "Set" is either a list {lo, hi, n} (as above) or a bitset over solution
// indices {idx, val, n, bs: true} holding only the non-zero 32-bit words.
// Large sets (opening) use bitsets: filtering is a word-wise AND with the
// per-cell column, independent of how many solutions survive.

export const LIST_LIMIT = 20000;

const popcnt = (x) => { x -= (x >>> 1) & 0x55555555; x = (x & 0x33333333) + ((x >>> 2) & 0x33333333); return (Math.imul((x + (x >>> 4)) & 0x0f0f0f0f, 0x01010101) >>> 24); };

export class Universe {
  constructor(ALL) {
    this.ALL = ALL;
    const W = (ALL.n + 31) >>> 5;
    this.W = W;
    this.col = Array.from({ length: N }, () => new Uint32Array(W));
    for (let i = 0; i < ALL.n; ++i) {
      let a = ALL.lo[i], b = ALL.hi[i];
      const w = i >>> 5, bit = 1 << (i & 31);
      while (a) { const t = a & -a; this.col[31 - Math.clz32(t)][w] |= bit; a ^= t; }
      while (b) { const t = b & -b; this.col[62 - Math.clz32(t)][w] |= bit; b ^= t; }
    }
    const idx = new Int32Array(W), val = new Uint32Array(W);
    for (let w = 0; w < W; ++w) { idx[w] = w; val[w] = w === W - 1 && (ALL.n & 31) ? (2 ** (ALL.n & 31) - 1) >>> 0 : 0xffffffff; }
    this.root = { bs: true, idx, val, n: ALL.n };
  }

  count(S, c) {
    if (!S.bs) {
      const { lo, hi, n } = S;
      let k = 0;
      if (c < 31) { const m = 1 << c; for (let i = 0; i < n; ++i) if (lo[i] & m) ++k; }
      else { const m = 1 << (c - 31); for (let i = 0; i < n; ++i) if (hi[i] & m) ++k; }
      return k;
    }
    const col = this.col[c], { idx, val } = S;
    let k = 0;
    for (let j = 0; j < idx.length; ++j) k += popcnt(val[j] & col[idx[j]]);
    return k;
  }

  counts(S) {
    if (!S.bs) return blackCounts(S);
    const out = new Uint32Array(N);
    for (let c = 0; c < N; ++c) out[c] = this.count(S, c);
    return out;
  }

  child(S, c) {
    if (!S.bs) return filterBlack(S, c);
    const col = this.col[c], { idx, val } = S;
    const nidx = new Int32Array(idx.length), nval = new Uint32Array(idx.length);
    let k = 0, n = 0;
    for (let j = 0; j < idx.length; ++j) {
      const v = val[j] & col[idx[j]];
      if (v) { nidx[k] = idx[j]; nval[k++] = v; n += popcnt(v); }
    }
    const out = { bs: true, idx: nidx.subarray(0, k), val: nval.subarray(0, k), n };
    return n <= LIST_LIMIT ? this.toList(out) : out;
  }

  toList(S) {
    if (!S.bs) return S;
    const lo = new Uint32Array(S.n), hi = new Uint32Array(S.n);
    let k = 0;
    for (let j = 0; j < S.idx.length; ++j) {
      let v = S.val[j];
      const base = S.idx[j] << 5;
      while (v) { const t = v & -v; const i = base + 31 - Math.clz32(t); lo[k] = this.ALL.lo[i]; hi[k++] = this.ALL.hi[i]; v ^= t; }
    }
    return { lo, hi, n: S.n };
  }

  // Set for a sequence of stones.
  play(stones) { let S = this.root; for (const c of stones) S = this.child(S, c); return S; }
}

// ---- Monte Carlo tree search with exact endgame proofs (MCTS-Solver) -------
//
// Nodes are shared between move orders (keyed by the set of stones).
// node.proven: +1 = player to move wins, -1 = player to move loses, 0 = open.
// node.w / node.v: wins / visits for the player who moved INTO the node.

export const EXACT_LIMIT = 3000;

const stoneKey = (stLo, stHi) => stLo + ':' + stHi;

export class MCTS {
  constructor(U, { seed = Date.now(), exactLimit = EXACT_LIMIT, solverNodes = 2000, c = 0.8, proofShare = 0.5, proofLimit = 20000, maxDepth = 64, smartPlayout = true, ttMax = 300000, handoff = 30000 } = {}) {
    this.ttMax = ttMax; this.handoff = handoff;
    this.proofShare = proofShare; this.proofLimit = proofLimit; this.maxDepth = maxDepth; this.smartPlayout = smartPlayout;
    this.U = U; this.rand = mulberry(seed); this.exactLimit = exactLimit; this.solverNodes = solverNodes; this.c = c;
    this.nodes = new Map(); this.solver = new Solver(); this.playouts = 0;
  }

  node(stLo, stHi, n) {
    const key = stoneKey(stLo, stHi);
    let nd = this.nodes.get(key);
    if (!nd) { nd = { stLo, stHi, n, v: 0, w: 0, proven: n <= 1 ? -1 : 0, moves: null, kids: null, nextExact: 0 }; this.nodes.set(key, nd); }
    return nd;
  }

  expand(nd, S) {
    const cnt = this.U.counts(S);
    const moves = [];
    for (let c = 0; c < N; ++c) if (cnt[c] > 0 && cnt[c] < S.n) moves.push(c);
    nd.moves = moves; nd.cnt = moves.map(c => cnt[c]); nd.kids = new Array(moves.length).fill(null);
    if (nd.cnt.some(k => k === 1)) nd.proven = 1;
  }

  kid(nd, i) {
    let k = nd.kids[i];
    if (!k) {
      const c = nd.moves[i];
      const sLo = c < 31 ? (nd.stLo | (1 << c)) >>> 0 : nd.stLo, sHi = c < 31 ? nd.stHi : (nd.stHi | (1 << (c - 31))) >>> 0;
      k = nd.kids[i] = this.node(sLo, sHi, nd.cnt[i]);
    }
    return k;
  }

  // Update nd.proven from its children; returns the new value.
  settle(nd) {
    if (nd.proven || !nd.moves) return nd.proven;
    let allWin = true;
    for (let i = 0; i < nd.moves.length; ++i) {
      const k = nd.kids[i];
      if (k && k.proven === -1) return (nd.proven = 1);
      if (!k || k.proven !== 1) allWin = false;
    }
    if (allWin) nd.proven = -1;
    return nd.proven;
  }

  select(nd) {
    const lnN = Math.log(nd.v + 1);
    let best = -1, bestU = -Infinity;
    for (let i = 0; i < nd.moves.length; ++i) {
      const k = nd.kids[i];
      if (k && k.proven === 1) continue;          // opponent wins there: never choose
      if (k && k.proven === -1) return i;         // winning move
      let u;
      if (!k || k.v === 0) u = 1e9 + this.rand(); // try every move once (random order)
      else u = k.w / k.v + this.c * Math.sqrt(lnN / k.v);
      if (u > bestU) { bestU = u; best = i; }
    }
    return best;
  }

  // Exact result for small sets; null when over budget or time.
  exact(S, budget) {
    const sv = this.solver;
    sv.nodes = 0; sv.nodeLimit = budget; sv.deadline = this.end;
    try { return sv.win(S) ? 1 : -1; }
    catch (e) { if (!isAbort(e)) throw e; return null; }
  }

  // Proof attempts with a budget that doubles with the node's visit count;
  // completed sub-results stay in the solver's table, so retries resume.
  tryExact(nd, S) {
    if (nd.proven || nd.n > this.exactLimit || nd.v < nd.nextExact) return;
    const v = Math.max(1, nd.v);
    nd.nextExact = v * 2;
    const r = this.exact(S.bs ? this.U.toList(S) : S, this.solverNodes * v);
    if (r !== null) nd.proven = r;
  }

  // Playout: true if the player to move at S wins.
  playout(S) {
    const U = this.U, rand = this.rand;
    let cur = S, turn = 0;
    const dead = new Uint8Array(N); // determined cells stay determined further down
    while (cur.n > 1) {
      if (this.smartPlayout && !cur.bs && cur.n <= 256) {
        // small sets: take an immediate win, avoid handing one over
        const cnt = blackCounts(cur);
        let win = -1; const ok = [], poor = [];
        for (let c = 0; c < N; ++c) {
          if (cnt[c] === 1) { win = c; break; }
          if (cnt[c] > 2 && cnt[c] < cur.n) ok.push(c); else if (cnt[c] === 2) poor.push(c);
        }
        if (win >= 0) { turn ^= 1; break; }
        const pool = ok.length ? ok : poor;
        cur = filterBlack(cur, pool[Math.floor(rand() * pool.length)]);
        turn ^= 1;
        continue;
      }
      let c, k;
      for (;;) {
        c = Math.floor(rand() * N);
        if (dead[c]) continue;
        k = U.count(cur, c);
        if (k > 0 && k < cur.n) break;
        dead[c] = 1;
      }
      cur = U.child(cur, c); dead[c] = 1;
      turn ^= 1;
    }
    this.playouts++;
    return turn === 1;
  }

  iterate(rootNode, rootS) {
    const path = [rootNode];
    let nd = rootNode, S = rootS;
    for (;;) {
      this.tryExact(nd, S);
      if (nd.proven) break;
      if (!nd.moves) { this.expand(nd, S); break; }
      const i = this.select(nd);
      if (i < 0) { this.settle(nd); break; }
      const k = this.kid(nd, i);
      S = this.U.child(S, nd.moves[i]);
      path.push(k);
      nd = k;
      if (k.v === 0 || path.length > this.maxDepth) { this.tryExact(k, S); break; }
    }
    // result for the player to move at the leaf
    let r;
    if (nd.proven) r = nd.proven === 1 ? 1 : 0;
    else r = this.playout(S) ? 1 : 0;
    for (let j = path.length - 1; j >= 0; --j) {
      const p = path[j];
      p.v++; p.w += 1 - r;     // stats are for the player who moved into p
      r = 1 - r;
      if (j < path.length - 1) this.settle(p);
    }
  }

  // Search from `stones` for `ms` milliseconds.
  search(stones, ms, S = this.U.play(stones)) {
    const t0 = performance.now(), end = t0 + ms;
    this.end = end;
    if (this.solver.tt.size > this.ttMax) this.solver = new Solver();
    let stLo = 0, stHi = 0;
    for (const c of stones) { if (c < 31) stLo = (stLo | (1 << c)) >>> 0; else stHi = (stHi | (1 << (c - 31))) >>> 0; }
    const root = this.node(stLo, stHi, S.n);
    if (S.n <= 1) return { move: null, proven: true, value: -1 };
    if (S.n <= this.handoff) {
      // tactical zone: proofs over the whole budget (flat search), shared table
      const L = S.bs ? this.U.toList(S) : S;
      const r = chooseMove(L, ms, { solver: this.solver, seed: Math.floor(this.rand() * 2 ** 31), exactLimit: this.proofLimit });
      return { ...r, info: 'endgame ' + r.info };
    }
    if (!root.moves) this.expand(root, S);
    // Phase A: like the flat AI, try to prove each small child outright
    // (smallest first) with part of the time; proofs feed the tree below.
    if (!root.proven) {
      // near the tactical zone almost all of the time goes to proofs
      this.end = t0 + ms * this.proofShare;
      const order = root.moves.map((c, i) => i).filter(i => root.cnt[i] <= this.proofLimit).sort((a, b) => root.cnt[a] - root.cnt[b]);
      for (const i of order) {
        if (performance.now() > this.end) break;
        const k = this.kid(root, i);
        if (k.proven) { if (k.proven === -1) break; continue; }
        const r = this.exact(this.U.child(S, root.moves[i]), Infinity);
        if (r === null) break;
        k.proven = r;
        if (r === -1) break;
      }
      this.settle(root);
      this.end = end;
    }
    const p0 = this.playouts;
    let it = 0;
    while (!root.proven && (performance.now() < end || it < 8)) {
      for (let k = 0; k < 16 && !root.proven; ++k) { this.iterate(root, S); ++it; }
    }
    this.settle(root);
    const stats = root.moves.map((c, i) => ({ c, k: root.kids[i], n: root.cnt[i] }));
    let pick;
    if (root.proven === 1) {
      pick = stats.find(s => s.n === 1) || stats.find(s => s.k && s.k.proven === -1);
      if (!pick) {
        // proven by the exact solver at this node: ask it for the move
        this.solver.nodeLimit = Infinity; this.solver.deadline = Infinity;
        const c = this.solver.bestMove(this.U.toList(S));
        pick = stats.find(s => s.c === c);
      }
    } else if (root.proven === -1) {
      pick = stats.reduce((a, b) => (b.n > a.n ? b : a)); // every move loses: keep the most room
    } else {
      const open = stats.filter(s => s.k && s.k.proven !== 1 && s.k.v > 0);
      if (open.length) pick = open.reduce((a, b) => (b.k.v > a.k.v ? b : a));
      else pick = stats.reduce((a, b) => (b.n > a.n ? b : a)); // all lose: keep the most room
    }
    const value = root.proven ? root.proven : (pick.k ? pick.k.w / pick.k.v : 0.5);
    if (this.nodes.size > 2e6) this.nodes.clear();
    return { move: pick.c, proven: !!root.proven, value, info: `mcts ${it} it, ${this.playouts - p0} playouts, ${this.nodes.size} nodes`, ms: performance.now() - t0 };
  }
}
