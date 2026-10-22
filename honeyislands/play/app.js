import { CELLS, N } from './engine.js';

const $ = (id) => document.getElementById(id);
const board = $('board'), statusEl = $('status'), countEl = $('count');
const SVGNS = 'http://www.w3.org/2000/svg';
const A = 5.4;                       // hex inradius in viewBox units
const R = A * 2 / Math.sqrt(3);      // circumradius
const store = {
  get(k, d) { try { return localStorage.getItem(k) ?? d; } catch { return d; } },
  set(k, v) { try { localStorage.setItem(k, v); } catch { /* private mode */ } },
};

// ---- board drawing ---------------------------------------------------------
const center = (c) => ({ cx: 50 + (2 * c.x + c.y) * A, cy: 50 + c.y * Math.sqrt(3) * A });
const hexPoints = ({ cx, cy }) => Array.from({ length: 6 }, (_, k) => {
  const t = Math.PI / 6 + k * Math.PI / 3;
  return `${(cx + R * Math.cos(t)).toFixed(2)},${(cy + R * Math.sin(t)).toFixed(2)}`;
}).join(' ');

board.setAttribute('viewBox', '0 4 100 92');
const cellEls = CELLS.map((c, i) => {
  const g = document.createElementNS(SVGNS, 'g');
  const poly = document.createElementNS(SVGNS, 'polygon');
  const p = center(c);
  poly.setAttribute('points', hexPoints(p));
  poly.setAttribute('class', 'cell free');
  poly.dataset.cell = i;
  const text = document.createElementNS(SVGNS, 'text');
  text.setAttribute('x', p.cx); text.setAttribute('y', p.cy);
  const sub = document.createElementNS(SVGNS, 'text');
  sub.setAttribute('x', p.cx); sub.setAttribute('y', p.cy + 2.6);
  const dot = document.createElementNS(SVGNS, 'circle');
  dot.setAttribute('cx', p.cx); dot.setAttribute('cy', p.cy); dot.setAttribute('r', 1.1);
  dot.style.display = 'none';
  g.append(poly, dot, text, sub);
  board.append(g);
  return { poly, text, dot, sub };
});

// ---- game state ------------------------------------------------------------
let stones = [];          // cells in move order; even index = first player
let analysis = null;      // latest worker analysis for `stones`
let ready = false;
let reqId = 0;            // bumps on every state change; stale replies are dropped
let thinking = false;
const worker = new Worker(new URL('./worker.js', import.meta.url), { type: 'module' });

// a row of toggle buttons that behaves like a <select>: .value and .onchange
function toggleGroup(el, initial) {
  const buttons = [...el.querySelectorAll('button')];
  const g = {
    onchange: null,
    get value() { return el.dataset.value; },
    set value(v) {
      if (!buttons.some(b => b.dataset.value === v)) v = buttons[0].dataset.value;
      el.dataset.value = v;
      for (const b of buttons) b.setAttribute('aria-pressed', String(b.dataset.value === v));
    },
  };
  g.value = initial;
  for (const b of buttons) b.addEventListener('click', () => {
    if (b.dataset.value === g.value) return;
    g.value = b.dataset.value;
    g.onchange?.();
  });
  return g;
}
const timeEl = $('ai-time'), hintEl = $('hint'), analyzeEl = $('analyze');
const modeEl = toggleGroup($('mode'), store.get('hib-mode', 'ai2'));
const ruleEl = toggleGroup($('rule'), store.get('hib-rule', 'normal'));
timeEl.value = store.get('hib-ai-seconds', '2');
hintEl.checked = store.get('hib-hint', '0') === '1';
analyzeEl.checked = store.get('hib-analyze', '0') === '1';
let evalRes = null;       // move values for `stones` (analysis view)

const aiSeconds = () => {
  const v = parseFloat(timeEl.value);
  return Number.isFinite(v) && v >= 0.1 && v <= 600 ? v : 2;
};
const isAI = (player) => {
  const m = modeEl.value;
  return m === 'aiai' || (m === 'ai1' && player === 0) || (m === 'ai2' && player === 1);
};
const playerName = (p) => {
  const m = modeEl.value;
  if (m === 'pvp') return p === 0 ? '先手' : '後手';
  if (m === 'aiai') return p === 0 ? 'AI（先手）' : 'AI（後手）';
  return isAI(p) ? 'AI' : 'あなた';
};
const toMove = () => stones.length % 2;
const gameOver = () => analysis && analysis.n <= 1;
// pass rule: free cells already determined black can take a stone (a pass)
const passCells = () => (ruleEl.value === 'pass' && analysis
  ? analysis.state.map((v, c) => (v === 1 && !stones.includes(c) ? c : -1)).filter(c => c >= 0) : []);
const legalCells = () => (analysis ? [...analysis.legal, ...passCells()] : []);

function refresh() {
  const id = ++reqId;
  thinking = false;
  analysis = null;
  evalRes = null;
  render();
  worker.postMessage({ kind: 'analyze', id, stones });
}

worker.onmessage = ({ data }) => {
  if (data.kind === 'ready') { ready = true; refresh(); return; }
  if (data.kind === 'error') { statusEl.textContent = '解データを読み込めませんでした: ' + data.message; return; }
  if (data.id !== reqId) return;
  if (data.kind === 'eval') {
    evalRes = data;
    render();
    return;
  }
  if (data.kind === 'analysis') {
    analysis = data;
    if (analyzeEl.checked && !gameOver() && !isAI(toMove())) requestEval();
    render();
    if (!gameOver() && isAI(toMove())) {
      thinking = true;
      render();
      worker.postMessage({ kind: 'think', id: reqId, stones, ms: aiSeconds() * 1000, rule: ruleEl.value });
    }
  } else if (data.kind === 'move') {
    thinking = false;
    if (data.move == null) return;
    lastAI = data;
    stones = [...stones, data.move];
    refresh();
  }
};
let lastAI = null;

function play(cell) {
  if (!ready || !analysis || thinking || gameOver() || isAI(toMove())) return;
  if (!legalCells().includes(cell)) return;
  lastAI = null;
  stones = [...stones, cell];
  refresh();
}

board.addEventListener('click', (e) => {
  const c = e.target.dataset?.cell;
  if (c !== undefined) play(+c);
});
$('new').onclick = () => { stones = []; lastAI = null; refresh(); };
$('undo').onclick = () => {
  if (!stones.length) return;
  // in AI games, undo back to the human's previous turn
  stones = stones.slice(0, -1);
  if (modeEl.value !== 'aiai') while (stones.length && isAI(toMove())) stones = stones.slice(0, -1);
  lastAI = null;
  refresh();
};
modeEl.onchange = () => { store.set('hib-mode', modeEl.value); refresh(); };
ruleEl.onchange = () => { store.set('hib-rule', ruleEl.value); stones = []; lastAI = null; refresh(); };
timeEl.onchange = () => {
  const v = parseFloat(timeEl.value);
  if (Number.isFinite(v) && v >= 0.1 && v <= 600) store.set('hib-ai-seconds', String(v));
  else timeEl.value = store.get('hib-ai-seconds', '2');
};
hintEl.onchange = () => { store.set('hib-hint', hintEl.checked ? '1' : '0'); render(); };
analyzeEl.onchange = () => {
  store.set('hib-analyze', analyzeEl.checked ? '1' : '0');
  if (analyzeEl.checked && analysis && !gameOver() && !isAI(toMove()) && !evalRes) requestEval();
  render();
};
// ask the worker for the value of every move (book first, then solving within the AI time)
function requestEval() {
  worker.postMessage({ kind: 'eval', id: reqId, stones, ms: aiSeconds() * 1000, rule: ruleEl.value });
}

function render() {
  const st = analysis?.state;
  const order = new Map(stones.map((c, i) => [c, i]));
  const last = stones[stones.length - 1];
  for (let i = 0; i < N; ++i) {
    const { poly, text, dot } = cellEls[i];
    let cls = 'cell free';
    text.textContent = ''; text.setAttribute('class', '');
    cellEls[i].sub.textContent = '';
    dot.style.display = 'none';
    if (order.has(i)) {
      cls = 'cell stone';
      text.textContent = order.get(i) + 1;
      text.setAttribute('class', 'num');
    } else if (st && st[i] === 1) {
      cls = ruleEl.value === 'pass' ? 'cell fb pass' : 'cell fb'; dot.style.display = ''; dot.setAttribute('class', 'dot-b');
    } else if (st && st[i] === 0) {
      cls = 'cell fw'; dot.style.display = ''; dot.setAttribute('class', 'dot-w');
    } else if (st && hintEl.checked) {
      text.textContent = Math.round(analysis.ratio[i] * 100);
      text.setAttribute('class', 'ratio');
    }
    // analysis view: playable cells (including pass cells) show the value of the move
    const ev = analyzeEl.checked && evalRes && !order.has(i) ? evalRes.value[i] : -1;
    if (ev >= 0) {
      const src = evalRes.source[i] === 1 ? '定' : evalRes.source[i] === 2 ? '読' : '';
      text.textContent = ev === 1 ? '勝' : ev === 0 ? '負' : '？';
      text.setAttribute('class', 'mv' + (st && st[i] === 1 ? ' onblack' : ''));
      if (src) {
        const sub = cellEls[i].sub;
        sub.textContent = src;
        sub.setAttribute('class', 'mvsrc' + (st && st[i] === 1 ? ' onblack' : ''));
      }
      if (ev === 1) cls += ' mwin';
    }
    if (i === last) cls += ' last';
    poly.setAttribute('class', cls);
  }
  board.classList.toggle('locked', !analysis || thinking || gameOver() || isAI(toMove()));

  statusEl.className = '';
  if (!ready) { statusEl.textContent = '解データを読み込み中…'; countEl.textContent = ''; return; }
  if (!analysis) { statusEl.textContent = '計算中…'; return; }
  countEl.textContent = `残りの解: ${analysis.n.toLocaleString()} 通り ／ 置けるマス: ${analysis.legal.length}`
    + (ruleEl.value === 'pass' ? ` ／ パス: あと ${passCells().length} 回` : '');
  if (gameOver()) {
    const winner = (stones.length - 1) % 2;
    statusEl.textContent = stones.length ? `解が 1 つに決まりました。${playerName(winner)}の勝ち！` : '';
    statusEl.className = 'win';
    return;
  }
  if (analyzeEl.checked) {
    if (!evalRes) countEl.textContent += ' ／ 解析中…';
    else {
      const cells = [...legalCells()];
      const k = v => cells.filter(c => evalRes.value[c] === v).length;
      countEl.textContent += ` ／ 解析: 勝ち ${k(1)}・負け ${k(0)}` + (k(2) ? `・？ ${k(2)}` : '')
        + (k(1) ? '（手番の人の勝ち）' : k(2) ? '' : '（手番の人の負け）');
    }
  }
  const p = toMove();
  let s = `${stones.length + 1} 手目：${playerName(p)}の番`;
  if (thinking) s += '（考え中…）';
  if (lastAI && lastAI.book) s += ' ― AI は定石で勝ちを読み切っています';
  else if (lastAI && lastAI.proven) s += lastAI.value > 0 ? ' ― AI は勝ちを読み切っています' : ' ― AI は負けを読み切っています';
  statusEl.textContent = s;
}

render();
