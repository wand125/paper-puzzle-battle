// Interactive opening explorer (moves 1-4), ported from the solve dashboard.
// Every element with class "explorer" and data-lang="en"|"ja" gets a viewer.
// Data: results/<rule>/board.json and results/<rule>/m4/<c1>-<c2>.json (see tools/export_board.py).

const SVGNS = 'http://www.w3.org/2000/svg';
const TEXT = {
  en: {
    normal: 'Standard rule', pass: 'Redundant-hint rule', back: '← Back',
    title: ['Move 1', 'Move 2 (after move 1)', 'Move 3 (after move 2)', 'Move 4 (after move 3)'],
    note: ['Click a cell to see the replies to that first move.', 'Numbered stones are the moves so far. Click a cell for move 3.',
      'Click a cell for move 4.', 'Move 4 is colored only where the value is known from the shared results.'],
    win: 'the move wins', loss: 'the move loses', unknown: 'not recorded', dup: 'same as another cell by symmetry',
    W: 'W', L: 'L', sols: 'solutions left', loading: 'Loading…', failed: 'Could not load the data.',
    summary: (w, l) => `${w} winning and ${l} losing moves (symmetric duplicates included).`,
  },
  ja: {
    normal: '標準ルール', pass: '余剰ヒントあり', back: '← 戻る',
    title: ['1 手目', '2 手目（1 手目のあと）', '3 手目（2 手目のあと）', '4 手目（3 手目のあと）'],
    note: ['マスを押すと、その 1 手目に対する 2 手目を表示します。', '数字の石は置いた順です。マスを押すと 3 手目を表示します。',
      'マスを押すと 4 手目を表示します。', '4 手目は、共有された結果から分かったものだけ色が付きます。'],
    win: 'その手を指した側の勝ち', loss: 'その手を指した側の負け', unknown: '記録なし', dup: '対称性で他のマスと同じ',
    W: '勝', L: '負', sols: '残りの解', loading: '読み込み中…', failed: 'データを読み込めませんでした。',
    summary: (w, l) => `勝ち ${w} 手・負け ${l} 手（対称で同じ手も数えています）。`,
  },
};

// cells: y outer, x inner, axial coordinates with |x + y| <= 4 (same order as the engine)
const CELLS = [];
for (let y = -4; y <= 4; ++y) for (let x = -4; x <= 4; ++x) if (Math.abs(x + y) <= 4) CELLS.push({ x, y });
const A = 5.4, R = A * 2 / Math.sqrt(3);
const center = (c) => ({ cx: 50 + (2 * c.x + c.y) * A, cy: 50 + c.y * Math.sqrt(3) * A });
const hexPoints = ({ cx, cy }) => Array.from({ length: 6 }, (_, k) => {
  const t = Math.PI / 6 + k * Math.PI / 3;
  return `${(cx + R * Math.cos(t)).toFixed(2)},${(cy + R * Math.sin(t)).toFixed(2)}`;
}).join(' ');
const svgEl = (tag, attrs, parent) => {
  const e = document.createElementNS(SVGNS, tag);
  for (const [k, v] of Object.entries(attrs)) e.setAttribute(k, v);
  parent?.append(e);
  return e;
};

const boards = {};   // rule -> board.json promise
const m4cache = {};  // `${rule}:${c1},${c2}` -> promise
const loadBoard = (rule) => (boards[rule] ??= fetch(`results/${rule}/board.json`).then(r => r.json()));
const loadM4 = (rule, a, b) => (m4cache[`${rule}:${a},${b}`] ??= fetch(`results/${rule}/m4/${a}-${b}.json`).then(r => r.json()));

function explorer(root) {
  const T = TEXT[root.dataset.lang] || TEXT.en;
  root.innerHTML = `
    <div class="ex-head">
      <div class="ex-rule" role="group"><button type="button" data-rule="normal">${T.normal}</button><button type="button" data-rule="pass">${T.pass}</button></div>
      <button type="button" class="ex-back" hidden>${T.back}</button>
    </div>
    <p class="ex-title"></p>
    <svg class="ex-board" viewBox="0 4 100 92" role="img"></svg>
    <p class="ex-summary"></p>
    <p class="ex-note"></p>
    <p class="ex-legend"><span class="sw ex-win"></span>${T.win} <span class="sw ex-loss"></span>${T.loss} <span class="sw ex-dup"></span>${T.dup} <span class="sw ex-unk"></span>${T.unknown}</p>`;
  const $ = (s) => root.querySelector(s);
  const state = { rule: 'normal', stones: [] };

  async function draw() {
    for (const b of root.querySelectorAll('[data-rule]')) b.setAttribute('aria-pressed', String(b.dataset.rule === state.rule));
    const st = state.stones, depth = st.length;
    $('.ex-back').hidden = depth === 0;
    $('.ex-title').textContent = T.title[depth];
    $('.ex-note').textContent = T.note[depth];
    const svg = $('.ex-board');
    let b, view = null;
    try {
      b = await loadBoard(state.rule);
      if (depth === 1) view = b.move2[st[0]];
      if (depth === 2) view = b.move3[`${st[0]},${st[1]}`];
      if (depth === 3) view = (await loadM4(state.rule, st[0], st[1]))[st[2]];
    } catch { $('.ex-summary').textContent = T.failed; return; }
    svg.textContent = '';
    let w = 0, l = 0;
    CELLS.forEach((c, i) => {
      const p = center(c);
      let info = null, cls = 'ex-unk', label = '', dot = false, clickable = false;
      if (depth === 0) {
        info = b.move1[i];
        clickable = !!info;
      } else if (st.includes(i)) {
        cls = 'ex-stone'; label = String(st.indexOf(i) + 1);
      } else if (view && view.cells[i]) {
        info = view.cells[i];          // under the pass rule this can be a free determined-black cell (a pass)
        dot = view.black.includes(i);
        clickable = depth === 1 ? !!b.move3[`${st[0]},${i}`] : depth === 2 ? !!view.has4 : false;
      } else if (view && view.black.includes(i)) {
        cls = 'ex-black'; dot = true;
      } else if (view && view.white.includes(i)) {
        cls = 'ex-white'; dot = true;
      }
      if (info) {
        if (info.v === 0) w++; else if (info.v === 1) l++;
        if (info.dup) { cls = 'ex-dup'; clickable = false; }
        else if (info.v === 0) { cls = 'ex-win'; label = T.W; }
        else if (info.v === 1) { cls = 'ex-loss'; label = T.L; }
      }
      const poly = svgEl('polygon', { points: hexPoints(p), class: `ex-cell ${cls}${clickable ? ' ex-click' : ''}` }, svg);
      if (info && info.n) svgEl('title', {}, poly).textContent = `${T.sols}: ${info.n.toLocaleString()}`;
      if (clickable) poly.addEventListener('click', () => { state.stones = [...st, i]; draw(); });
      if (dot) svgEl('circle', { cx: p.cx, cy: p.cy, r: 1.1, class: 'ex-dot' }, svg);
      if (label) svgEl('text', { x: p.cx, y: p.cy, class: 'ex-label' }, svg).textContent = label;
    });
    $('.ex-summary').textContent = T.summary(w, l);
  }
  for (const b of root.querySelectorAll('[data-rule]')) b.onclick = () => { state.rule = b.dataset.rule; state.stones = []; draw(); };
  $('.ex-back').onclick = () => { state.stones = state.stones.slice(0, -1); draw(); };
  $('.ex-summary').textContent = T.loading;
  draw();
}

for (const root of document.querySelectorAll('.explorer')) explorer(root);

// ---- game animation for the rules section: elements with class "playanim", data-rule and data-lang ----
const PT = {
  en: { play: 'Play', pause: 'Pause', btn: ['⏮', '◀', '▶', '⏭'], speed: 'Speed', step: 'Move', labels: ['To the start', 'Previous move', 'Next move', 'To the end'], start: 'Start: 1,304,983 solutions, every cell undetermined.',
    move: (k, who, pass, n, p) => `Move ${k} (${who})${pass ? ', a redundant hint' : ''}: ${n.toLocaleString()} solution${n === 1 ? '' : 's'} left` + (p == null ? '.' : `, ${p} redundant hint${p === 1 ? '' : 's'} available.`),
    win: (who) => ` One solution remains: the ${who} wins.`, first: 'first player', second: 'second player',
    legend: ['stone (move number)', 'determined black', 'determined white', 'undetermined'] },
  ja: { play: '再生', pause: '停止', btn: ['⏮', '◀', '▶', '⏭'], speed: '速さ', step: '手', labels: ['最初へ', '1 手戻る', '1 手進む', '最後へ'], start: '開始：解 1,304,983 通り、すべてのマスが未確定。',
    move: (k, who, pass, n, p) => `${k} 手目（${who}）${pass ? '・余剰ヒント' : ''}：残りの解 ${n.toLocaleString()} 通り` + (p == null ? '。' : `、使える余剰ヒント ${p} 回。`),
    win: (who) => `解が 1 つに決まり、${who}の勝ち。`, first: '先手', second: '後手',
    legend: ['石（数字は手順）', '黒に確定', '白に確定', '未確定'] },
};
async function playanim(root) {
  const L = PT[root.dataset.lang] || PT.en, pass = root.dataset.rule === 'pass';
  let line;
  try { line = (await (await fetch(`results/${root.dataset.rule}/best-line.json`)).json()).moves; } catch { return; }
  root.innerHTML = `<svg class="ex-board" viewBox="0 4 100 92" role="img"></svg>
    <p class="ex-summary pa-info"></p>
    <div class="pa-ctrl">
      <div class="pa-row">
        <button type="button" data-go="first" aria-label="${L.labels[0]}" title="${L.labels[0]}">${L.btn[0]}</button>
        <button type="button" data-go="prev" aria-label="${L.labels[1]}" title="${L.labels[1]}">${L.btn[1]}</button>
        <button type="button" class="pa-btn"></button>
        <button type="button" data-go="next" aria-label="${L.labels[2]}" title="${L.labels[2]}">${L.btn[2]}</button>
        <button type="button" data-go="last" aria-label="${L.labels[3]}" title="${L.labels[3]}">${L.btn[3]}</button>
      </div>
      <input type="range" class="bl-range" min="0" max="${line.length}" value="0" aria-label="${L.step}">
      <div class="pa-row pa-speed" role="group" aria-label="${L.speed}">${L.speed}
        ${[0.5, 1, 2].map(v => `<button type="button" data-speed="${v}">${v}×</button>`).join('')}
      </div>
    </div>`;
  const svg = root.querySelector('svg'), info = root.querySelector('.pa-info'), btn = root.querySelector('.pa-btn');
  const range = root.querySelector('input');
  let k = 0, timer = null, speed = 1;
  function draw() {
    svg.textContent = '';
    range.value = k;
    const m = k ? line[k - 1] : null;
    const black = new Set(m ? m.black : []), white = new Set(m ? m.white : []);
    const order = new Map(line.slice(0, k).map((x, i) => [x.cell, i]));
    CELLS.forEach((c, i) => {
      const p = center(c), j = order.get(i);
      let cls = 'ex-unk', dot = false;
      if (j != null) cls = line[j].pass ? 'ex-stone bl-pass' : 'ex-stone';
      else if (black.has(i)) { cls = 'ex-black'; dot = true; }
      else if (white.has(i)) { cls = 'ex-white'; dot = true; }
      svgEl('polygon', { points: hexPoints(p), class: `ex-cell ${cls}` }, svg);
      if (dot) svgEl('circle', { cx: p.cx, cy: p.cy, r: 1.1, class: 'ex-dot' }, svg);
      if (j != null) svgEl('text', { x: p.cx, y: p.cy, class: `ex-label${j === k - 1 ? ' bl-cur' : ''}` }, svg).textContent = String(j + 1);
    });
    if (!m) { info.textContent = L.start; return; }
    const who = (k - 1) % 2 === 0 ? L.first : L.second;
    const left = pass ? (k < line.length ? line[k].passes : 0) : null;
    info.textContent = L.move(k, who, m.pass, m.after, left) + (m.after === 1 ? L.win(who) : '');
  }
  function tick() {
    k = k >= line.length ? 0 : k + 1;
    draw();
    timer = setTimeout(tick, (k === line.length ? 3500 : k === 0 ? 1500 : 1100) / speed);
  }
  function set(on) {
    clearTimeout(timer); timer = null;
    if (on) timer = setTimeout(tick, 600);
    btn.textContent = on ? L.pause : L.play;
  }
  btn.onclick = () => set(!timer);
  const go = (to) => { set(false); k = Math.max(0, Math.min(line.length, to)); draw(); };
  const moves = { first: () => 0, prev: () => k - 1, next: () => k + 1, last: () => line.length };
  for (const b of root.querySelectorAll('[data-go]')) b.onclick = () => go(moves[b.dataset.go]());
  range.oninput = () => go(+range.value);
  const speeds = root.querySelectorAll('[data-speed]');
  const setSpeed = (v) => { speed = v; for (const b of speeds) b.setAttribute('aria-pressed', String(+b.dataset.speed === v)); };
  for (const b of speeds) b.onclick = () => setSpeed(+b.dataset.speed);
  setSpeed(1);
  const still = matchMedia('(prefers-reduced-motion: reduce)').matches;
  if (still) k = line.length;
  draw();
  set(!still);
}
for (const root of document.querySelectorAll('.playanim')) playanim(root);
