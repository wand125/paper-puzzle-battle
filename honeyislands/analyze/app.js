import { CELLS, N } from './engine.js';
import { renderSitebar } from '../sitebar.js';

const $ = id => document.getElementById(id);
const SVGNS = 'http://www.w3.org/2000/svg';
const A = 5.4;                       // hex inradius in viewBox units
const R = A * 2 / Math.sqrt(3);      // circumradius
const store = {
  get(k, fallback) { try { return localStorage.getItem(k) ?? fallback; } catch { return fallback; } },
  set(k, value) { try { localStorage.setItem(k, value); } catch { /* private mode */ } },
};

// ---- language and preferences ---------------------------------------------
const strings = {
  en: {
    title: 'Honey Islands · Analysis', language: 'Language', theme: 'Theme', light: 'Light', dark: 'Dark',
    position: 'Position', new: 'New position', showEval: 'Win/loss of each move', heat: 'Heat map',
    showAfter: 'Solutions left after the move', stone: 'Stone', black: 'Determined black', white: 'Determined white',
    undetermined: 'Undetermined', win: 'Win', loss: 'Loss', unknown: 'Unknown',
    overlayLegend: 'Green = winning move; dashed edge = losing move; ? = unproven. B = book, S = solved. Heat: darker green = higher black share. Small numbers = solutions after playing. Amber dashed outlines = selected solution.',
    solutions: 'Remaining solutions', clearSolution: 'Clear outline', solutionsLimit: 'Thumbnail boards appear when 64 or fewer solutions remain.',
    solutionsLoading: 'Loading thumbnails…', solutionsShown: '{n} solutions · select a board to outline its black cells.', solutionLabel: 'Solution {n}',
    howTo: 'How to use',
    how1: 'Analyze mode lets you place stones for both sides. The player who leaves exactly one puzzle solution wins.',
    how2: 'With redundant hints, an empty determined-black cell can be played as a pass: solutions stay the same and the turn changes.',
    how3: 'Click a move or use ← / → / Home / End to review history. A new move replaces the continuation. Reviewing history pauses AI; Resume AI continues from there.',
    how4: 'Sort the moves table by a column heading. Select a row to highlight its cell; double-click it or press Play selected cell to play.',
    how5: 'Solve deeper uses four times the AI time. Share the current position with Copy link, or import cell indices (0–60) separated by dots or commas.',
    settings: 'Settings', mode: 'Mode', analyze: 'Analyze', humanFirst: 'vs AI (you first)', aiFirst: 'vs AI (AI first)', aiAI: 'AI vs AI',
    rule: 'Rule', standard: 'Standard', passRule: 'Redundant hints', aiTime: 'AI time (seconds)', resume: 'Resume AI',
    positionDetails: 'Position details', deeper: 'Solve deeper (4×)', kifu: 'Move list', initial: 'Initial position',
    movesTable: 'Playable moves', tableHelp: 'Select a row to highlight a cell. Double-click or use Play to place it.',
    cell: 'Cell', value: 'Value', after: 'Left', share: 'Share', playSelected: 'Play selected cell',
    sharing: 'Share & import', copyLink: 'Copy link', copyMoves: 'Copy moves', importLabel: 'Cell indices, separated by dots or commas', import: 'Import',
    loadExample: 'Load example', chooseExample: 'Choose a perfect-play line…', normalExample: 'Standard · perfect play', passExample: 'Redundant hints · perfect play',
    first: 'First player', second: 'Second player', moveNumber: 'Moves played', side: 'Side to move', remaining: 'Solutions left', passes: 'Redundant hints available', pass: 'pass',
    turn: 'Move {n} · {side} to move', finished: 'One solution · {side} wins', count: '{n} solutions left',
    loading: 'Loading solution data…', progress: 'Loading solution data · {n} MB', calculating: 'Updating position…', evaluating: 'Evaluating moves…',
    thinking: 'AI is thinking…', paused: 'AI paused while reviewing history.',
    positionValue: 'Side to move: {value}', book: 'Book', solved: 'Solved within the time limit', unproven: 'Not proven', terminal: 'Terminal position: the previous player won.',
    evalTime: 'Time limit: {n} seconds.', history: 'History', firstMove: 'Go to initial position', previousMove: 'Previous move', nextMove: 'Next move', lastMove: 'Go to last move',
    board: 'Hexagonal analysis board', cellLabel: 'Cell {n} ({x}, {y})', selected: 'Selected: {cell} · {n} solutions after this move',
    stopped: 'Import stopped before move {n}: illegal cell, repeated cell, or the game already ended.',
    imported: 'Loaded {n} moves.', copied: 'Copied.', copyFailed: 'Copy failed. Select and copy the text in the import field.',
    exampleError: 'Could not load example: {message}', error: 'Unable to load analysis: {message}',
  },
  ja: {
    title: 'ハニーアイランド・バトル · 解析', language: '言語', theme: 'テーマ', light: 'ライト', dark: 'ダーク',
    position: '盤面', new: '新しい局面', showEval: '各手の勝ち負け', heat: 'ヒートマップ',
    showAfter: '着手後の解数', stone: '置いた石', black: '黒確定', white: '白確定', undetermined: '未確定', win: '勝ち', loss: '負け', unknown: '不明',
    overlayLegend: '緑＝勝つ手、破線の縁＝負ける手、?＝未証明。「定」＝定石、「読」＝読み切り。濃い緑ほど黒率が高く、小さい数字は着手後の解数です。黄土色の破線は選択した解の黒マスです。',
    solutions: '残りの解', clearSolution: '解の輪郭を消す', solutionsLimit: '残りの解が64通り以下になると、解の一覧を表示します。',
    solutionsLoading: '解の一覧を読み込み中…', solutionsShown: '{n} 通り · 選ぶと盤面に黒マスの輪郭を表示します。', solutionLabel: '解 {n}',
    howTo: '使い方',
    how1: '「解析」では先手と後手の両方を操作します。パズルの解をちょうど1通りに絞った人の勝ちです。',
    how2: '「余剰ヒントあり」では黒確定の空きマスにも置けます。解数は変わらず、手番が相手に移ります。',
    how3: '棋譜をクリックするか、← / → / Home / Endで局面を移動します。途中で打つと続きの棋譜は置き換わります。棋譜の確認中はAIが止まり、「AIを再開」で続けられます。',
    how4: '着手一覧は列名で並べ替えられます。行を選ぶとマスが強調され、ダブルクリックか「選択したマスに打つ」で着手できます。',
    how5: '「さらに読む」はAI持ち時間の4倍で解析します。「リンクをコピー」で局面を共有できます。インポートはマス番号（0〜60）をドットかカンマで区切ります。',
    settings: '設定', mode: 'モード', analyze: '解析', humanFirst: '対AI（あなた先手）', aiFirst: '対AI（AI先手）', aiAI: 'AI同士',
    rule: 'ルール', standard: '標準', passRule: '余剰ヒントあり', aiTime: 'AIの持ち時間（秒）', resume: 'AIを再開',
    positionDetails: '局面情報', deeper: 'さらに読む（4倍）', kifu: '棋譜', initial: '初期局面',
    movesTable: '着手一覧', tableHelp: '行を選ぶとマスを強調します。ダブルクリックか着手ボタンで打てます。',
    cell: 'マス', value: '勝敗', after: '残り解数', share: '黒率', playSelected: '選択したマスに打つ',
    sharing: '共有・インポート', copyLink: 'リンクをコピー', copyMoves: '棋譜をコピー', importLabel: 'マス番号をドットかカンマで区切って入力', import: '読込',
    loadExample: '例を読み込む', chooseExample: '最善手の棋譜を選択…', normalExample: '標準 · 最善手の棋譜', passExample: '余剰ヒントあり · 最善手の棋譜',
    first: '先手', second: '後手', moveNumber: '着手数', side: '手番', remaining: '残りの解', passes: '使用可能な余剰ヒント', pass: 'パス',
    turn: '{n} 手目 · {side}の番', finished: '解が1通り · {side}の勝ち', count: '残り {n} 通り',
    loading: '解データを読み込み中…', progress: '解データを読み込み中 · {n} MB', calculating: '局面を更新中…', evaluating: '各手を解析中…',
    thinking: 'AIが考え中…', paused: '棋譜の確認中のためAIは停止しています。',
    positionValue: '手番側の勝敗：{value}', book: '定石', solved: '持ち時間内に読み切り', unproven: '未証明', terminal: '終局：直前に打った人の勝ちです。',
    evalTime: '解析時間：{n} 秒。', history: '棋譜の移動', firstMove: '初期局面へ', previousMove: '1手戻る', nextMove: '1手進む', lastMove: '最終局面へ',
    board: '六角形の解析盤面', cellLabel: 'マス {n}（{x}, {y}）', selected: '選択：{cell} · 着手後 {n} 通り',
    stopped: '{n} 手目の直前で読込を停止しました。違法なマス、重複、または終局後の着手です。',
    imported: '{n} 手を読み込みました。', copied: 'コピーしました。', copyFailed: 'コピーできませんでした。入力欄のテキストを選んでコピーしてください。',
    exampleError: '例を読み込めませんでした：{message}', error: '解析を読み込めませんでした：{message}',
  },
};
const params = new URL(location.href).searchParams;
let lang = ['ja', 'en'].includes(params.get('lang')) ? params.get('lang') : (navigator.language.startsWith('ja') ? 'ja' : 'en');
const t = (key, values = {}) => strings[lang][key].replace(/\{(\w+)\}/g, (_, name) => values[name] ?? '');
const number = n => Math.round(n).toLocaleString(lang === 'ja' ? 'ja-JP' : 'en-US');
const compact = n => n >= 1e6 ? `${+(n / 1e6).toFixed(1)}m` : n >= 1e3 ? `${+(n / 1e3).toFixed(1)}k` : String(n);
const cellLabel = c => `#${c} (${CELLS[c].x},${CELLS[c].y})`;
const sideName = p => t(p === 0 ? 'first' : 'second');
const sourceMark = source => source === 1 ? (lang === 'ja' ? '定' : 'B') : source === 2 ? (lang === 'ja' ? '読' : 'S') : '';
const valueText = v => v === 1 ? t('win') : v === 0 ? t('loss') : '?';

function toggleGroup(el, initial) {
  const buttons = [...el.querySelectorAll('button')];
  const group = {
    onchange: null,
    get value() { return el.dataset.value; },
    set value(value) {
      if (!buttons.some(b => b.dataset.value === value)) value = buttons[0].dataset.value;
      el.dataset.value = value;
      for (const b of buttons) b.setAttribute('aria-pressed', String(b.dataset.value === value));
    },
  };
  group.value = initial;
  for (const b of buttons) b.onclick = () => {
    if (b.dataset.value === group.value) return;
    group.value = b.dataset.value;
    group.onchange?.();
  };
  return group;
}
const mode = toggleGroup($('mode'), 'analyze');
const rule = toggleGroup($('rule'), params.get('rule') ?? store.get('hib-an-rule', 'normal'));
const seconds = () => {
  const v = Number($('ai-time').value);
  return Number.isFinite(v) && v >= .1 && v <= 600 ? v : 2;
};
$('ai-time').value = store.get('hib-an-seconds', '2');
$('ai-time').value = seconds();
for (const key of ['eval', 'heat', 'after']) {
  const box = $(`show-${key}`);
  box.checked = store.get(`hib-an-${key}`, key === 'eval' || key === 'heat' ? '1' : '0') === '1';
  box.onchange = () => { store.set(`hib-an-${key}`, box.checked ? '1' : '0'); renderBoard(); };
}

// ---- board drawing ---------------------------------------------------------
const center = c => ({ cx: 50 + (2 * c.x + c.y) * A, cy: 50 + c.y * Math.sqrt(3) * A });
const hexPoints = ({ cx, cy }, scale = 1) => Array.from({ length: 6 }, (_, k) => {
  const a = Math.PI / 6 + k * Math.PI / 3;
  return `${(cx + R * scale * Math.cos(a)).toFixed(2)},${(cy + R * scale * Math.sin(a)).toFixed(2)}`;
}).join(' ');
function svgElement(name, attrs = {}) {
  const el = document.createElementNS(SVGNS, name);
  for (const [key, value] of Object.entries(attrs)) el.setAttribute(key, value);
  return el;
}
const cellEls = CELLS.map((c, i) => {
  const p = center(c), g = svgElement('g', { role: 'button', 'data-cell': i });
  const poly = svgElement('polygon', { points: hexPoints(p) });
  const dot = svgElement('circle', { cx: p.cx, cy: p.cy, r: .65, class: 'determined-dot' });
  const label = svgElement('text', { x: p.cx, y: p.cy });
  const source = svgElement('text', { x: p.cx, y: p.cy + 1.4 });
  const after = svgElement('text', { x: p.cx, y: p.cy + 3.1 });
  const outline = svgElement('polygon', { points: hexPoints(p, .78), class: 'solution-outline' });
  g.append(poly, dot, label, source, after, outline);
  g.onclick = () => play(i);
  g.onkeydown = event => {
    if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); play(i); }
  };
  $('board').append(g);
  return { g, poly, dot, label, source, after, outline };
});

// ---- position, history and worker ------------------------------------------
let history = [];        // full linear line, including the redo continuation
let cursor = 0;
let analysis = null, evalRes = null, solutionMasks = null;
let selectedCell = null, selectedSolution = null;
let ready = false, failure = '', loadMB = null;
let reqId = 0, evalId = 0, evalPending = false, evalSeconds = 0;
let thinking = false, paused = false, aiTimer = null;
let notice = null, importing = false, exampleRequest = 0;
let sort = { key: 'cell', direction: 1 };
let pendingImport = parseMoves(params.get('moves') ?? '');
const stones = () => history.slice(0, cursor).map(m => m.cell);
const toMove = () => cursor % 2;
const gameOver = () => analysis && analysis.n <= 1;
const isAI = p => mode.value === 'aiai' || (mode.value === 'ai1' && p === 0) || (mode.value === 'ai2' && p === 1);
const passCells = () => rule.value === 'pass' && analysis ? analysis.state.flatMap((v, c) => v === 1 && !stones().includes(c) ? [c] : []) : [];
const legalCells = () => analysis && !gameOver() ? [...analysis.legal, ...passCells()] : [];
const canPlay = () => ready && analysis && !failure && !importing && !thinking && !gameOver() && (paused || !isAI(toMove()));
const afterCount = c => Math.round(analysis.ratio[c] * analysis.n);
const worker = new Worker(new URL('./worker.js', import.meta.url), { type: 'module' });

function updateURL() {
  const url = new URL(location.href);
  url.searchParams.set('lang', lang);
  url.searchParams.set('rule', rule.value);
  url.searchParams.set('moves', stones().join('.'));
  window.history.replaceState(null, '', url);
}
function refresh() {
  ++reqId;
  clearTimeout(aiTimer);
  thinking = false;
  analysis = evalRes = solutionMasks = null;
  selectedCell = selectedSolution = null;
  evalPending = false;
  evalSeconds = 0;
  render();
  updateURL();
  if (ready && !failure) worker.postMessage({ kind: 'analyze', id: reqId, stones: stones() });
}
function parseMoves(text) {
  // Keep a malformed token as NaN so validation stops there, rather than skipping it.
  return text.trim() ? text.trim().split(/[.,]/).slice(0, 62).map(s => /^\s*\d+\s*$/.test(s) ? Number(s) : NaN) : [];
}
function importMoves(moves) {
  clearTimeout(aiTimer);
  paused = true;
  thinking = false;
  importing = true;
  analysis = evalRes = solutionMasks = null;
  selectedCell = selectedSolution = null;
  evalPending = false;
  notice = null;
  ++reqId;
  render();
  if (!ready) { pendingImport = moves; return; }
  worker.postMessage({ kind: 'history', id: reqId, stones: moves, rule: rule.value });
}
function requestEval(multiplier = 1) {
  if (!analysis || gameOver() || failure) return;
  clearTimeout(aiTimer);
  evalPending = true;
  evalSeconds = seconds() * multiplier;
  worker.postMessage({ kind: 'eval', id: reqId, evalId: ++evalId, stones: stones(), ms: evalSeconds * 1000, rule: rule.value });
  render();
}
function scheduleAI() {
  if (!analysis || gameOver() || paused || !isAI(toMove()) || evalPending || thinking || failure) return;
  const id = reqId;
  // Leave a short window to inspect or pause even when the book replies immediately.
  aiTimer = setTimeout(() => {
    if (id !== reqId || paused) return;
    thinking = true;
    render();
    worker.postMessage({ kind: 'think', id, stones: stones(), ms: seconds() * 1000, rule: rule.value });
  }, 450);
}
worker.onmessage = ({ data }) => {
  if (data.kind === 'progress') { loadMB = (data.loaded / 1e6).toFixed(1); renderStatus(); return; }
  if (data.kind === 'ready') {
    ready = true;
    const moves = pendingImport;
    pendingImport = null;
    if (moves?.length) importMoves(moves);
    else { importing = false; refresh(); }
    return;
  }
  if (data.kind === 'error') { fail(data.message); return; }
  if (data.id !== reqId) return;
  if (data.kind === 'history') {
    importing = false;
    history = data.moves;
    cursor = history.length;
    notice = { key: data.stopped ? 'stopped' : 'imported', values: { n: data.stopped ? cursor + 1 : cursor } };
    refresh();
  } else if (data.kind === 'analysis') {
    analysis = data;
    render();
    if (analysis.n <= 64) worker.postMessage({ kind: 'solutions', id: reqId, stones: stones(), limit: 64 });
    if (!gameOver()) requestEval();
  } else if (data.kind === 'eval') {
    if (data.evalId !== evalId) return;
    evalRes = data;
    evalPending = false;
    render();
    scheduleAI();
  } else if (data.kind === 'solutions') {
    solutionMasks = data.masks;
    renderSolutions();
  } else if (data.kind === 'move') {
    thinking = false;
    if (data.move !== null && legalCells().includes(data.move)) appendMove(data.move);
    else { paused = true; render(); }
  }
};
worker.onerror = event => fail(event.message);
worker.onmessageerror = () => fail('Worker message could not be read');
function fail(message) {
  failure = String(message);
  clearTimeout(aiTimer);
  thinking = evalPending = false;
  render();
}
function appendMove(c) {
  ++exampleRequest;
  const move = { cell: c, pass: analysis.state[c] === 1, n: afterCount(c) };
  history = [...history.slice(0, cursor), move];
  ++cursor;
  notice = null;
  refresh();
}
function play(c) {
  if (!canPlay() || !legalCells().includes(c)) return;
  paused = false;
  appendMove(c);
}
function jump(next) {
  if (importing || !ready || failure) return;
  next = Math.max(0, Math.min(history.length, next));
  paused = true;
  notice = null;
  cursor = next;
  refresh();
}
$('first').onclick = $('initial').onclick = () => jump(0);
$('previous').onclick = () => jump(cursor - 1);
$('next').onclick = () => jump(cursor + 1);
$('last').onclick = () => jump(history.length);
$('new').onclick = () => { ++exampleRequest; history = []; cursor = 0; notice = null; paused = false; importing = false; pendingImport = []; refresh(); };
$('resume').onclick = () => {
  paused = false;
  // Continuing AI from a reviewed position replaces the old continuation on its first move.
  if (!evalRes && !evalPending) requestEval();
  else scheduleAI();
  render();
};
mode.onchange = () => { ++exampleRequest; paused = false; refresh(); };
rule.onchange = () => {
  ++exampleRequest;
  store.set('hib-an-rule', rule.value);
  // Revalidate the entire line: pass moves may be illegal under the standard rule.
  importMoves(history.map(m => m.cell));
};
$('ai-time').onchange = () => {
  $('ai-time').value = seconds();
  store.set('hib-an-seconds', $('ai-time').value);
  if (!thinking) requestEval();
};
$('deeper').onclick = () => requestEval(4);
document.addEventListener('keydown', event => {
  if (event.altKey || event.ctrlKey || event.metaKey || event.shiftKey || event.target.closest('input, select, textarea, [contenteditable=true]')) return;
  const targets = { ArrowLeft: cursor - 1, ArrowRight: cursor + 1, Home: 0, End: history.length };
  if (event.key in targets) { event.preventDefault(); jump(targets[event.key]); }
});

// ---- position and board rendering ------------------------------------------
function renderBoard() {
  const order = new Map(stones().map((c, i) => [c, i]));
  const legal = new Set(legalCells());
  const playable = canPlay();
  for (let c = 0; c < N; ++c) {
    const { g, poly, dot, label, source, after, outline } = cellEls[c];
    const placed = order.has(c), st = analysis?.state[c];
    const kind = placed ? 'stone' : st === 1 ? 'fb' : st === 0 ? 'fw' : 'free';
    const v = legal.has(c) && $('show-eval').checked ? evalRes?.value[c] ?? 2 : -1;
    poly.setAttribute('class', `cell ${kind}${c === stones().at(-1) ? ' last' : ''}${v === 1 ? ' mwin' : v === 0 ? ' mlose' : ''}${c === selectedCell ? ' selected' : ''}`);
    poly.style.fill = $('show-heat').checked && kind === 'free' && analysis
      ? `color-mix(in srgb, var(--accent) ${Math.round(analysis.ratio[c] * 90)}%, var(--cell))` : '';
    g.setAttribute('class', playable && legal.has(c) ? 'playable' : '');
    g.setAttribute('tabindex', playable && legal.has(c) ? '0' : '-1');
    g.setAttribute('aria-disabled', String(!playable || !legal.has(c)));
    const cellName = t('cellLabel', { n: c, x: CELLS[c].x, y: CELLS[c].y });
    g.setAttribute('aria-label', placed ? `${cellName}, ${t('stone')} ${order.get(c) + 1}` : `${cellName}, ${t(st === 1 ? 'black' : st === 0 ? 'white' : 'undetermined')}${legal.has(c) ? `, ${valueText(evalRes?.value[c])}, ${number(afterCount(c))}` : ''}`);
    const onblack = kind === 'fb' ? ' onblack' : '';
    label.setAttribute('class', placed ? 'num' : `move-value${onblack}`);
    label.setAttribute('y', center(CELLS[c]).cy + (placed || v < 0 ? 0 : -1.2));
    label.textContent = placed ? order.get(c) + 1 : v >= 0 ? valueText(v) : '';
    source.setAttribute('class', `source${onblack}`);
    source.textContent = v >= 0 ? sourceMark(evalRes?.source[c]) : '';
    after.setAttribute('class', `after${onblack}`);
    after.textContent = legal.has(c) && $('show-after').checked ? compact(afterCount(c)) : '';
    dot.style.display = !placed && (st === 0 || st === 1) && v < 0 && !(legal.has(c) && $('show-after').checked) ? '' : 'none';
    outline.style.display = selectedSolution !== null && maskHas(solutionMasks?.[selectedSolution], c) ? '' : 'none';
  }
  $('selection').textContent = analysis && selectedCell !== null && legal.has(selectedCell)
    ? t('selected', { cell: cellLabel(selectedCell), n: number(afterCount(selectedCell)) }) : '';
  $('play-selected').disabled = !playable || !legal.has(selectedCell);
}
function renderStatus() {
  let status = !ready ? loadMB === null ? t('loading') : t('progress', { n: loadMB })
    : !analysis || importing ? t('calculating')
      : gameOver() ? t('finished', { side: sideName((cursor - 1) % 2) })
        : t('turn', { n: cursor + 1, side: sideName(toMove()) });
  if (analysis && !gameOver()) {
    if (thinking) status += ` · ${t('thinking')}`;
    else if (paused && isAI(toMove())) status += ` · ${t('paused')}`;
    else if (evalPending) status += ` · ${t('evaluating')}`;
  }
  $('status').textContent = failure ? t('error', { message: failure }) : status;
  $('count').textContent = analysis ? t('count', { n: number(analysis.n) }) : '—';
  $('notice').textContent = notice ? t(notice.key, notice.values) : '';
  $('error').hidden = !failure;
  $('error').textContent = failure;
}
function detail(key, value) {
  const dt = document.createElement('dt'), dd = document.createElement('dd');
  dt.textContent = t(key); dd.textContent = value;
  return [dt, dd];
}
function renderPosition() {
  const list = $('position-details');
  list.replaceChildren(...detail('moveNumber', cursor), ...detail('side', gameOver() ? '—' : sideName(toMove())),
    ...detail('remaining', analysis ? number(analysis.n) : '—'));
  for (const [key, value] of [['black', 1], ['white', 0], ['undetermined', .5]]) {
    list.append(...detail(key, analysis ? analysis.state.filter(v => v === value).length : '—'));
  }
  if (rule.value === 'pass') list.append(...detail('passes', analysis ? passCells().length : '—'));
  const cells = legalCells();
  const wins = cells.filter(c => evalRes?.value[c] === 1);
  const losing = cells.length > 0 && cells.every(c => evalRes?.value[c] === 0);
  const v = gameOver() ? 0 : wins.length ? 1 : losing ? 0 : 2;
  $('position-value').textContent = t('positionValue', { value: v === 2 ? t('unknown') : valueText(v) });
  // A winning witness proves a win; proving loss needs every move to lose.
  const book = wins.length ? wins.some(c => evalRes.source[c] === 1) : losing && cells.every(c => evalRes.source[c] === 1);
  const source = gameOver() ? 'terminal' : v === 2 ? 'unproven' : book ? 'book' : 'solved';
  $('value-source').textContent = `${t(source)}${evalPending ? ` · ${t('evaluating')}` : ''}${evalSeconds && analysis && !gameOver() ? ` · ${t('evalTime', { n: evalSeconds })}` : ''}`;
  $('deeper').disabled = !analysis || gameOver() || evalPending || thinking || !!failure;
  $('resume').hidden = !paused || mode.value === 'analyze' || !isAI(toMove()) || gameOver();
  $('resume').disabled = !analysis || !!failure;
}
function renderHistory() {
  $('history-position').textContent = `${cursor} / ${history.length}`;
  for (const [id, disabled] of [['first', !cursor], ['previous', !cursor], ['next', cursor === history.length], ['last', cursor === history.length], ['initial', !cursor]]) {
    $(id).disabled = disabled || !ready || importing || !!failure;
  }
  $('initial').setAttribute('aria-current', cursor === 0 ? 'step' : 'false');
  $('kifu').replaceChildren(...history.map((m, i) => {
    const li = document.createElement('li'), button = document.createElement('button'), count = document.createElement('span');
    button.type = 'button';
    button.textContent = `${cellLabel(m.cell)}${m.pass ? ` · ${t('pass')}` : ''}`;
    button.setAttribute('aria-current', i + 1 === cursor ? 'step' : 'false');
    count.className = 'move-count'; count.textContent = number(m.n);
    button.append(count); button.onclick = () => jump(i + 1);
    li.classList.toggle('future', i >= cursor);
    li.append(button); return li;
  }));
}
function selectCell(c) {
  selectedCell = c;
  renderBoard();
  for (const row of $('move-rows').children) row.setAttribute('aria-selected', String(+row.dataset.cell === c));
}
function renderTable() {
  const cells = legalCells();
  const rank = c => evalRes?.value[c] === 1 ? 2 : evalRes?.value[c] === 0 ? 0 : 1;
  const sortValue = c => sort.key === 'cell' ? c : sort.key === 'value' ? rank(c) : afterCount(c);
  cells.sort((a, b) => sort.direction * (sortValue(a) - sortValue(b)) || a - b);
  $('move-rows').replaceChildren(...cells.map(c => {
    const row = document.createElement('tr');
    row.dataset.cell = c; row.tabIndex = 0; row.setAttribute('aria-selected', String(c === selectedCell));
    const cell = document.createElement('td'), value = document.createElement('td'), count = document.createElement('td'), share = document.createElement('td');
    cell.textContent = `#${c}${analysis.state[c] === 1 ? ` · ${t('pass')}` : ''}`;
    const coords = document.createElement('small'); coords.textContent = `(${CELLS[c].x},${CELLS[c].y})`; cell.append(coords);
    value.textContent = valueText(evalRes?.value[c]);
    const src = document.createElement('span'); src.className = 'value-source'; src.textContent = sourceMark(evalRes?.source[c]); value.append(src);
    count.textContent = number(afterCount(c));
    share.textContent = `${(analysis.ratio[c] * 100).toFixed(1)}%`;
    row.append(cell, value, count, share);
    row.onclick = () => selectCell(c);
    row.ondblclick = () => play(c);
    row.onkeydown = e => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); selectCell(c); } };
    return row;
  }));
  for (const b of document.querySelectorAll('[data-sort]')) {
    if (b.dataset.sort === sort.key) b.parentElement.setAttribute('aria-sort', sort.direction === 1 ? 'ascending' : 'descending');
    else b.parentElement.removeAttribute('aria-sort');
  }
}
for (const b of document.querySelectorAll('[data-sort]')) b.onclick = () => {
  sort = { key: b.dataset.sort, direction: sort.key === b.dataset.sort ? -sort.direction : 1 };
  renderTable();
};
$('play-selected').onclick = () => play(selectedCell);
function maskHas(mask, c) { return !!mask && !!(c < 31 ? mask[0] & (1 << c) : mask[1] & (1 << (c - 31))); }
function renderSolutions() {
  $('solutions-grid').replaceChildren();
  $('clear-solution').hidden = selectedSolution === null;
  $('solutions-note').textContent = !analysis || analysis.n > 64 ? t('solutionsLimit')
    : !solutionMasks ? t('solutionsLoading') : t('solutionsShown', { n: number(analysis.n) });
  if (!solutionMasks) return;
  const fragment = document.createDocumentFragment();
  solutionMasks.forEach((mask, i) => {
    const button = document.createElement('button'); button.type = 'button';
    button.setAttribute('aria-label', t('solutionLabel', { n: i + 1 }));
    button.setAttribute('aria-pressed', String(i === selectedSolution));
    const svg = svgElement('svg', { viewBox: '0 4 100 92', class: 'thumbnail', 'aria-hidden': true });
    for (let c = 0; c < N; ++c) svg.append(svgElement('polygon', { points: hexPoints(center(CELLS[c])), class: maskHas(mask, c) ? 'stone' : 'free' }));
    button.append(svg, String(i + 1));
    button.onclick = () => {
      selectedSolution = selectedSolution === i ? null : i;
      $('clear-solution').hidden = selectedSolution === null;
      for (const [j, b] of [...$('solutions-grid').children].entries()) b.setAttribute('aria-pressed', String(j === selectedSolution));
      renderBoard();
    };
    fragment.append(button);
  });
  $('solutions-grid').append(fragment);
}
$('clear-solution').onclick = () => { selectedSolution = null; renderSolutions(); renderBoard(); };
function render() {
  renderStatus(); renderBoard(); renderPosition(); renderHistory(); renderTable(); renderSolutions();
  $('import').disabled = !ready || !!failure;
  $('example').disabled = !ready || !!failure;
  for (const button of $('mode').querySelectorAll('button')) button.disabled = importing || !!failure;
}

// ---- sharing, imports, examples and preferences ----------------------------
$('import-form').onsubmit = event => {
  event.preventDefault();
  ++exampleRequest;
  importMoves(parseMoves($('import-moves').value));
};
async function copy(link) {
  updateURL();
  const text = link ? location.href : stones().join('.');
  try {
    await navigator.clipboard.writeText(text);
    notice = { key: 'copied' };
  } catch {
    $('import-moves').value = text;
    $('import-moves').focus(); $('import-moves').select();
    notice = { key: 'copyFailed' };
  }
  renderStatus();
}
$('copy-link').onclick = () => copy(true);
$('copy-moves').onclick = () => copy(false);
$('example').onchange = async () => {
  const chosen = $('example').value;
  if (!['normal', 'pass'].includes(chosen)) return;
  const request = ++exampleRequest;
  try {
    const response = await fetch(`../results/${chosen}/best-line.json`);
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const data = await response.json();
    if (request !== exampleRequest) return;
    rule.value = chosen; mode.value = 'analyze';
    store.set('hib-an-rule', chosen);
    importMoves(data.moves.map(m => m.cell));
  } catch (error) {
    if (request !== exampleRequest) return;
    notice = { key: 'exampleError', values: { message: error.message } }; renderStatus();
  } finally {
    if (request === exampleRequest) $('example').value = '';
  }
};
function localize() {
  document.documentElement.lang = lang;
  document.title = t('title');
  for (const el of document.querySelectorAll('[data-i18n]')) el.textContent = t(el.dataset.i18n);
  for (const [id, key] of Object.entries({ 'language-toggle': 'language', 'theme-toggle': 'theme', navigation: 'history', board: 'board', mode: 'mode', rule: 'rule', first: 'firstMove', previous: 'previousMove', next: 'nextMove', last: 'lastMove' })) {
    $(id).setAttribute('aria-label', t(key));
    if (['first', 'previous', 'next', 'last'].includes(id)) $(id).title = t(key);
  }
  for (const b of document.querySelectorAll('[data-language]')) b.setAttribute('aria-pressed', String(b.dataset.language === lang));
  for (const b of document.querySelectorAll('[data-theme-set]')) b.setAttribute('aria-pressed', String(b.dataset.themeSet === document.documentElement.dataset.theme));
  renderSitebar(lang);
  render();
}
for (const b of document.querySelectorAll('[data-language]')) b.onclick = () => { lang = b.dataset.language; localize(); updateURL(); };
for (const b of document.querySelectorAll('[data-theme-set]')) b.onclick = () => {
  document.documentElement.dataset.theme = b.dataset.themeSet;
  store.set('hib-paper-theme', b.dataset.themeSet);
  localize();
};
window.addEventListener('popstate', () => {
  ++exampleRequest;
  const p = new URL(location.href).searchParams;
  if (['ja', 'en'].includes(p.get('lang'))) lang = p.get('lang');
  rule.value = p.get('rule') ?? 'normal';
  mode.value = 'analyze';
  importMoves(parseMoves(p.get('moves') ?? ''));
  localize();
});
localize();
