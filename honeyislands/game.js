import { CELLS, N } from './analyze/engine.js';
import { renderSitebar } from './sitebar.js';

const $ = id => document.getElementById(id);
const board = $('board');
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
    title: 'Honey Islands Battle', analysisTool: 'Analysis tool', paper: 'Paper',
    language: 'Language', theme: 'Theme', light: 'Light', dark: 'Dark', links: 'Links',
    setupTitle: 'Choose your game', cancel: 'Cancel', opponent: 'Opponent', humanFirst: 'You first · vs AI',
    aiFirst: 'AI first · vs you', twoPlayers: 'Two players', strength: 'AI strength',
    easy: 'Easy', normal: 'Normal', strong: 'Strong', strongest: 'Strongest',
    strength1: 'Random moves, with an eye for immediate wins.',
    strength2: 'A mix of random play and a short search.',
    strength3: 'Thinks for up to 1.5 seconds per move.',
    strength4: 'Up to 6 seconds; uses the opening book and plays perfectly in most positions.',
    strengthPvp: 'Share the board and take turns.',
    rule: 'Rule', standardRule: 'Standard', passRule: 'Redundant hints',
    standardNote: 'Place stones only on undetermined cells.',
    passNote: 'Also place on empty determined-black cells to pass the turn.',
    start: 'Start game', undo: 'Undo', newGame: 'New game',
    playAgain: 'Play again', swapSides: 'Swap sides', changeSettings: 'Change settings', close: 'Close',
    countBefore: '', countAfter: 'solutions left', remaining: '{n} solutions left',
    preview: '{n} solutions would remain', passes: '{n} redundant hints available',
    loadingShort: 'Loading…', loading: 'Loading solution data (about 10 MB)…', calculating: 'Updating the board…',
    setupReady: 'Choose your settings, then start.', errorStatus: 'The game could not load.',
    error: 'Unable to load the game: {message}. Please reload to try again.',
    humanTurn: 'Your turn', aiTurn: 'AI is thinking…', firstTurn: 'First player’s turn', secondTurn: 'Second player’s turn',
    humanWin: 'You win!', aiWin: 'AI wins', firstWin: 'First player wins!', secondWin: 'Second player wins!',
    moves: '{n} moves played', move: 'Move {n}', game: 'Game', board: 'Hexagonal game board',
    cell: 'Cell {n} ({x}, {y})', stoneLabel: 'stone, move {n}',
    undetermined: 'Undetermined', determinedBlack: 'Determined black', determinedWhite: 'Determined white',
    cellTypes: 'Cell types', progress: 'Progress toward one solution',
    boardHelp: 'Tab to a playable cell, then press Enter or Space to place a stone. Arrow keys move between playable cells.',
    howTo: 'How to play',
    how1: 'The 61-cell board has 1,304,983 puzzle solutions, each with 25 black cells.',
    how2: 'Take turns placing a black stone on an undetermined cell. Only solutions with that cell black remain.',
    how3: 'The player whose stone leaves exactly one solution wins.',
    how4: 'With redundant hints, an empty determined-black cell also accepts a stone and passes the turn. Determined-white cells are never playable.',
  },
  ja: {
    title: 'ハニーアイランド・バトル', analysisTool: '解析ツール', paper: '論文',
    language: '言語', theme: 'テーマ', light: 'ライト', dark: 'ダーク', links: '関連リンク',
    setupTitle: '対戦設定', cancel: 'キャンセル', opponent: '対戦相手', humanFirst: 'あなた先手 vs AI',
    aiFirst: 'AI先手 vs あなた', twoPlayers: '2人対戦', strength: 'AIの強さ',
    easy: 'やさしい', normal: 'ふつう', strong: 'つよい', strongest: 'さいきょう',
    strength1: 'ランダムに打ちますが、すぐに勝てる手は見逃しません。',
    strength2: 'ランダムな手と短い思考を半々で使います。',
    strength3: '1手につき約1.5秒考えます。',
    strength4: '約6秒の思考と定石を使い、多くの局面で最善手を打ちます。',
    strengthPvp: '同じ盤面で交互に打ってください。',
    rule: 'ルール', standardRule: '標準', passRule: '余剰ヒントあり',
    standardNote: 'まだ色が決まっていないマスだけに石を置けます。',
    passNote: '黒確定の空きマスにも置けます。解数は変わらず、手番が移ります。',
    start: '対戦スタート', undo: '待った', newGame: '新しい対戦',
    playAgain: 'もう一度', swapSides: '先後を入れ替える', changeSettings: '設定を変える', close: '閉じる',
    countBefore: '残り', countAfter: '通り', remaining: '残り {n} 通り',
    preview: 'ここに置くと残り {n} 通り', passes: '余剰ヒント あと {n} 回',
    loadingShort: '読み込み中…', loading: '解データを読み込み中（約10 MB）…', calculating: '盤面を更新中…',
    setupReady: '設定を選んで対戦を始めましょう。', errorStatus: 'ゲームを読み込めませんでした。',
    error: 'ゲームを読み込めませんでした: {message}。ページを再読み込みしてください。',
    humanTurn: 'あなたの番', aiTurn: 'AI が考え中…', firstTurn: '先手の番', secondTurn: '後手の番',
    humanWin: 'あなたの勝ち！', aiWin: 'AI の勝ち', firstWin: '先手の勝ち', secondWin: '後手の勝ち',
    moves: '{n} 手で決着', move: '{n} 手目', game: '対戦', board: '六角形の対戦盤面',
    cell: 'マス {n}（{x}, {y}）', stoneLabel: '石、{n} 手目',
    undetermined: '未確定', determinedBlack: '黒確定', determinedWhite: '白確定',
    cellTypes: 'マスの状態', progress: '解が1通りになるまでの進み具合',
    boardHelp: 'Tabで置けるマスを選び、EnterまたはSpaceで石を置けます。矢印キーでもマスを移動できます。',
    howTo: '遊び方',
    how1: '61マスの盤面には、25マスを黒くするパズルの解が1,304,983通りあります。',
    how2: '交互に未確定のマスへ黒い石を置きます。そのマスが黒い解だけが残ります。',
    how3: '自分の石で解をちょうど1通りにした人の勝ちです。',
    how4: '「余剰ヒントあり」では黒確定の空きマスにも置け、手番だけが移ります。白確定のマスには置けません。',
  },
};
const urlLanguage = new URL(location.href).searchParams.get('lang');
let lang = ['ja', 'en'].includes(urlLanguage) ? urlLanguage : (navigator.language.startsWith('ja') ? 'ja' : 'en');
const t = (key, values = {}) => strings[lang][key].replace(/\{(\w+)\}/g, (_, name) => values[name] ?? '');
const number = n => Math.round(n).toLocaleString(lang === 'ja' ? 'ja-JP' : 'en-US');
const validated = (value, choices, fallback) => choices.includes(value) ? value : fallback;
// setup choices are radio groups shown as toggle buttons and cards
const choice = name => document.querySelector(`input[name=${name}]:checked`).value;
const setChoice = (name, value) => { document.querySelector(`input[name=${name}][value="${value}"]`).checked = true; };
setChoice('opponent', validated(store.get('hib-game-opponent', 'ai2'), ['ai2', 'ai1', 'pvp'], 'ai2'));
setChoice('strength', validated(store.get('hib-game-strength', '2'), ['1', '2', '3', '4'], '2'));
setChoice('rule', validated(store.get('hib-game-rule', 'normal'), ['normal', 'pass'], 'normal'));

function setupChoices() {
  return { mode: choice('opponent'), strength: +choice('strength'), rule: choice('rule') };
}
function persistChoices() {
  const choices = setupChoices();
  store.set('hib-game-opponent', choices.mode);
  store.set('hib-game-strength', String(choices.strength));
  store.set('hib-game-rule', choices.rule);
}
function renderSetup() {
  const pvp = choice('opponent') === 'pvp';
  for (const input of document.querySelectorAll('input[name=strength]')) input.disabled = pvp;
  $('strength-group').classList.toggle('off', pvp);
  $('strength-note').textContent = t(pvp ? 'strengthPvp' : `strength${choice('strength')}`);
}
function localize() {
  document.documentElement.lang = lang;
  document.title = t('title');
  for (const el of document.querySelectorAll('[data-i18n]')) el.textContent = t(el.dataset.i18n);
  for (const [id, key] of Object.entries({ 'language-toggle': 'language', 'theme-toggle': 'theme',
    game: 'game', board: 'board', legend: 'cellTypes', progress: 'progress', 'close-result': 'close' })) {
    $(id).setAttribute('aria-label', t(key));
  }
  for (const button of document.querySelectorAll('[data-language]')) {
    button.setAttribute('aria-pressed', String(button.dataset.language === lang));
  }
  renderSitebar(lang);
  $('count-before').textContent = t('countBefore');
  $('count-after').textContent = t('countAfter');
  renderSetup();
  render();
  drawCount(displayCount);
}
function renderTheme() {
  for (const button of document.querySelectorAll('[data-theme-set]')) {
    button.setAttribute('aria-pressed', String(button.dataset.themeSet === document.documentElement.dataset.theme));
  }
}

// ---- board drawing ---------------------------------------------------------
const center = c => ({ cx: 50 + (2 * c.x + c.y) * A, cy: 50 + c.y * Math.sqrt(3) * A });
const hexPoints = ({ cx, cy }) => Array.from({ length: 6 }, (_, k) => {
  const angle = Math.PI / 6 + k * Math.PI / 3;
  return `${(cx + R * Math.cos(angle)).toFixed(2)},${(cy + R * Math.sin(angle)).toFixed(2)}`;
}).join(' ');

board.setAttribute('viewBox', '0 4 100 92');
const cellEls = CELLS.map((cell, i) => {
  const g = document.createElementNS(SVGNS, 'g');
  g.dataset.cell = i;
  g.setAttribute('role', 'button');
  const poly = document.createElementNS(SVGNS, 'polygon');
  const ring = document.createElementNS(SVGNS, 'polygon');
  const p = center(cell);
  poly.setAttribute('points', hexPoints(p));
  ring.setAttribute('points', hexPoints(p));
  ring.setAttribute('class', 'focus-ring');
  const text = document.createElementNS(SVGNS, 'text');
  text.setAttribute('x', p.cx);
  text.setAttribute('y', p.cy);
  g.append(poly, ring, text);
  g.addEventListener('animationend', () => {
    g.classList.remove('placed');
    if (animateCell === i) animateCell = null;
  });
  board.append(g);
  return { g, poly, text };
});

// ---- game state and worker -------------------------------------------------
let settings = setupChoices();
let stones = [];          // cells in move order; even index = first player
let analysis = null;      // only valid for the current `stones`
let visualAnalysis = null;
let total = 1304983;
let ready = false;
let active = false;
let failure = '';
let reqId = 0;            // every state change invalidates older worker replies
let thinking = false;
let aiStarted = 0;
let aiTimer = null;
let animateCell = null;
let hoverCell = null;
let focusCell = null;
let restoreBoardFocus = false;
let resultShown = false;
let worker;
const reducedMotion = matchMedia('(prefers-reduced-motion: reduce)');
let displayCount = null;
let targetCount = null;
let countFrame = null;

const toMove = () => stones.length % 2;
const isAI = player => (settings.mode === 'ai1' && player === 0) || (settings.mode === 'ai2' && player === 1);
const gameOver = () => analysis && analysis.n === 1;
const passCells = () => (settings.rule === 'pass' && analysis
  ? analysis.state.map((value, c) => value === 1 && !stones.includes(c) ? c : -1).filter(c => c >= 0) : []);
const legalCells = () => analysis ? [...analysis.legal, ...passCells()] : [];
const canPlay = () => ready && active && analysis && !failure && !thinking && !gameOver() && !isAI(toMove());
const remainingAfter = c => analysis.state[c] === 1 ? analysis.n : Math.round(analysis.ratio[c] * analysis.n);
const canUndo = () => active && !failure && stones.some((_, i) => !isAI(i % 2));

function refresh() {
  const id = ++reqId;
  clearTimeout(aiTimer);
  thinking = false;
  visualAnalysis = analysis ?? visualAnalysis;
  analysis = null;
  hoverCell = focusCell = null;
  render();
  if (ready && !failure) worker.postMessage({ kind: 'analyze', id, stones: [...stones] });
}

function fail(message) {
  failure = String(message);
  ++reqId;
  clearTimeout(aiTimer);
  thinking = false;
  render();
}

function onWorkerMessage({ data }) {
  if (data.kind === 'error') { fail(data.message); return; }
  if (failure) return;
  if (data.kind === 'ready') {
    ready = true;
    total = data.total;
    updateCount(total);
    refresh();
    return;
  }
  if (data.id !== reqId) return;
  if (data.kind === 'analysis') {
    analysis = data;
    visualAnalysis = data;
    updateCount(data.n);
    render();
    if (active && gameOver()) {
      restoreBoardFocus = false;
      if (!resultShown) {
        resultShown = true;
        $('result').showModal();
        $('play-again').focus();
      }
    } else if (active && isAI(toMove())) {
      beginAI();
    } else if (restoreBoardFocus && canPlay()) {
      restoreBoardFocus = false;
      cellEls[legalCells()[0]]?.g.focus({ preventScroll: true });
    }
  } else if (data.kind === 'move' && thinking) {
    scheduleAI(data.move, reqId);
  }
}

// Easy uses public counts only: take an immediate win, otherwise avoid two.
function easyMove() {
  const legal = legalCells();
  const wins = legal.filter(c => remainingAfter(c) === 1);
  const safe = legal.filter(c => remainingAfter(c) !== 2);
  const choices = wins.length ? wins : safe.length ? safe : legal;
  return choices[Math.floor(Math.random() * choices.length)];
}

function beginAI() {
  thinking = true;
  aiStarted = performance.now();
  render();
  if (settings.strength === 1 || (settings.strength === 2 && Math.random() < .5)) {
    scheduleAI(easyMove(), reqId);
  } else {
    const ms = { 2: 300, 3: 1500, 4: 6000 }[settings.strength];
    worker.postMessage({ kind: 'think', id: reqId, stones: [...stones], ms, rule: settings.rule });
  }
}

function scheduleAI(move, id) {
  clearTimeout(aiTimer);
  aiTimer = setTimeout(() => {
    if (id !== reqId || !active || !thinking || failure || !analysis || gameOver() || !isAI(toMove())) return;
    const cell = legalCells().includes(move) ? move : easyMove();
    if (cell !== undefined) placeStone(cell);
  }, Math.max(0, 400 - (performance.now() - aiStarted)));
}

function placeStone(cell) {
  animateCell = cell;
  stones = [...stones, cell];
  refresh();
}
function play(cell) {
  if (!canPlay() || !legalCells().includes(cell)) return;
  restoreBoardFocus = board.contains(document.activeElement);
  placeStone(cell);
}
function closeResult() {
  if ($('result').open) $('result').close();
}
function startGame(next = setupChoices()) {
  if (!ready || failure) return;
  closeResult();
  settings = { ...next };
  stones = [];
  analysis = visualAnalysis = null;
  animateCell = null;
  resultShown = false;
  restoreBoardFocus = true;
  active = true;
  if ($('setup').open) $('setup').close();
  refresh();
  $('game').scrollIntoView({ block: 'start' });
}
// the setup popup; while a game is on, cancelling it returns to that game
function showSetup() {
  closeResult();
  if (!$('setup').open) $('setup').showModal();
  render();
  document.querySelector('input[name=opponent]:checked').focus();
}

// ---- counter and rendering -------------------------------------------------
function drawCount(value) {
  $('count').textContent = value === null ? '—' : number(value);
  const progress = value === null ? 0 : Math.max(0, Math.min(100, 100 * (1 - Math.log(Math.max(1, value)) / Math.log(total))));
  $('progress-fill').style.width = `${progress}%`;
}
function updateCount(n) {
  if (targetCount === n) return;
  targetCount = n;
  cancelAnimationFrame(countFrame);
  $('count-announcement').textContent = t('remaining', { n: number(n) });
  const progress = Math.max(0, Math.min(100, 100 * (1 - Math.log(Math.max(1, n)) / Math.log(total))));
  $('progress').setAttribute('aria-valuenow', String(Math.round(progress)));
  $('progress').setAttribute('aria-valuetext', t('remaining', { n: number(n) }));
  if (displayCount === null || reducedMotion.matches) {
    displayCount = n;
    drawCount(n);
    return;
  }
  const from = displayCount;
  const start = performance.now();
  const step = now => {
    const fraction = reducedMotion.matches ? 1 : Math.min(1, (now - start) / 300);
    displayCount = from + (n - from) * (1 - (1 - fraction) ** 3);
    drawCount(displayCount);
    if (fraction < 1) countFrame = requestAnimationFrame(step);
  };
  countFrame = requestAnimationFrame(step);
}
function winnerText() {
  const winner = (stones.length - 1) % 2;
  return t(settings.mode === 'pvp' ? (winner === 0 ? 'firstWin' : 'secondWin') : isAI(winner) ? 'aiWin' : 'humanWin');
}
function renderPreview() {
  const cell = hoverCell ?? focusCell;
  $('preview').textContent = canPlay() && cell !== null && legalCells().includes(cell)
    ? t('preview', { n: number(remainingAfter(cell)) }) : '';
}
function render() {
  const state = (analysis ?? visualAnalysis)?.state;
  const order = new Map(stones.map((c, i) => [c, i]));
  const playable = new Set(canPlay() ? legalCells() : []);
  for (let i = 0; i < N; ++i) {
    const { g, poly, text } = cellEls[i];
    const placed = order.has(i);
    const kind = placed ? 'stone' : state?.[i] === 1 ? 'fb' : state?.[i] === 0 ? 'fw' : 'free';
    poly.setAttribute('class', `cell ${kind}${i === stones.at(-1) ? ' last' : ''}`);
    g.setAttribute('class', [playable.has(i) ? 'playable' : '', kind === 'fb' ? 'pass' : '', animateCell === i ? 'placed' : ''].filter(Boolean).join(' '));
    g.setAttribute('tabindex', playable.has(i) ? '0' : '-1');
    g.setAttribute('aria-disabled', String(!playable.has(i)));
    text.textContent = placed ? order.get(i) + 1 : '';
    const label = t('cell', { n: i + 1, x: CELLS[i].x, y: CELLS[i].y });
    const description = placed ? t('stoneLabel', { n: order.get(i) + 1 })
      : t(kind === 'fb' ? 'determinedBlack' : kind === 'fw' ? 'determinedWhite' : 'undetermined');
    const preview = playable.has(i) ? `. ${t('preview', { n: number(remainingAfter(i)) })}` : '';
    g.setAttribute('aria-label', `${label}: ${description}${preview}`);
  }
  if (!active && !$('setup').open) $('setup').showModal();
  $('setup-cancel').hidden = !active;
  $('game-actions').hidden = !active;
  $('start').disabled = !ready || !!failure;
  $('start').textContent = t(ready || failure ? 'start' : 'loadingShort');
  $('undo').disabled = !canUndo();
  $('passes').hidden = !active || settings.rule !== 'pass';
  $('passes').textContent = analysis ? t('passes', { n: passCells().length }) : '';
  $('error').hidden = !failure;
  $('error').textContent = failure ? t('error', { message: failure }) : '';
  $('status').classList.toggle('thinking', thinking);
  let status;
  if (failure) status = t('errorStatus');
  else if (!ready) status = t('loading');
  else if (!active) status = t('setupReady');
  else if (!analysis) status = t('calculating');
  else if (gameOver()) status = winnerText();
  else if (thinking || isAI(toMove())) status = t('aiTurn');
  else status = t(settings.mode === 'pvp' ? (toMove() === 0 ? 'firstTurn' : 'secondTurn') : 'humanTurn');
  $('status').textContent = status;
  $('game-label').textContent = active
    ? `${t(settings.rule === 'pass' ? 'passRule' : 'standardRule')} · ${t('move', { n: gameOver() ? stones.length : stones.length + 1 })}` : '';
  $('result-title').textContent = stones.length && gameOver() ? winnerText() : '';
  $('result-moves').textContent = t('moves', { n: number(stones.length) });
  $('swap').hidden = settings.mode === 'pvp';
  $('count-announcement').textContent = targetCount === null ? '' : t('remaining', { n: number(targetCount) });
  if (targetCount !== null) $('progress').setAttribute('aria-valuetext', t('remaining', { n: number(targetCount) }));
  renderPreview();
}

// ---- input -----------------------------------------------------------------
const eventCell = event => {
  const g = event.target.closest('[data-cell]');
  return g && board.contains(g) ? +g.dataset.cell : null;
};
board.addEventListener('click', event => {
  const cell = eventCell(event);
  if (cell !== null) play(cell);
});
board.addEventListener('keydown', event => {
  const cell = eventCell(event);
  if (cell === null || !canPlay()) return;
  if (event.key === 'Enter' || event.key === ' ') {
    event.preventDefault();
    play(cell);
  } else if (['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(event.key)) {
    event.preventDefault();
    const direction = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] }[event.key];
    const p = center(CELLS[cell]);
    const candidates = legalCells().filter(c => c !== cell).map(c => {
      const q = center(CELLS[c]);
      const dx = q.cx - p.cx, dy = q.cy - p.cy;
      const forward = dx * direction[0] + dy * direction[1];
      return { c, forward, score: Math.hypot(dx, dy) + 2 * Math.abs(dx * direction[1] - dy * direction[0]) };
    }).filter(c => c.forward > .1).sort((a, b) => a.score - b.score);
    if (candidates.length) cellEls[candidates[0].c].g.focus({ preventScroll: true });
  }
});
board.addEventListener('pointerover', event => { hoverCell = eventCell(event); renderPreview(); });
board.addEventListener('pointerleave', () => { hoverCell = null; renderPreview(); });
board.addEventListener('focusin', event => { focusCell = eventCell(event); renderPreview(); });
board.addEventListener('focusout', () => { focusCell = null; renderPreview(); });
$('setup-form').addEventListener('change', () => { persistChoices(); renderSetup(); });
$('setup-form').addEventListener('submit', event => {
  event.preventDefault();
  persistChoices();
  startGame();
});
$('undo').addEventListener('click', () => {
  if (!canUndo()) return;
  closeResult();
  stones = stones.slice(0, -1);
  while (stones.length && isAI(toMove())) stones = stones.slice(0, -1);
  animateCell = null;
  resultShown = false;
  restoreBoardFocus = false;
  refresh();
});
$('new-game').addEventListener('click', showSetup);
$('change-settings').addEventListener('click', showSetup);
$('play-again').addEventListener('click', () => startGame(settings));
$('swap').addEventListener('click', () => {
  const mode = settings.mode === 'ai1' ? 'ai2' : 'ai1';
  setChoice('opponent', mode);
  persistChoices();
  renderSetup();
  startGame({ ...settings, mode });
});
$('close-result').addEventListener('click', closeResult);
$('setup-cancel').addEventListener('click', () => $('setup').close());
$('setup').addEventListener('cancel', event => { if (!active) event.preventDefault(); });
$('result').addEventListener('close', () => { if (active) $('new-game').focus({ preventScroll: true }); });
for (const button of document.querySelectorAll('[data-language]')) {
  button.addEventListener('click', () => {
    lang = button.dataset.language;
    const url = new URL(location.href);
    url.searchParams.set('lang', lang);
    history.replaceState(null, '', url);
    localize();
  });
}
for (const button of document.querySelectorAll('[data-theme-set]')) {
  button.addEventListener('click', () => {
    document.documentElement.dataset.theme = button.dataset.themeSet;
    store.set('hib-paper-theme', button.dataset.themeSet);
    renderTheme();
  });
}

localize();
renderTheme();
try {
  worker = new Worker(new URL('./analyze/worker.js', import.meta.url), { type: 'module' });
  worker.onmessage = onWorkerMessage;
  worker.onerror = event => { event.preventDefault(); fail(event.message || 'Worker error'); };
  worker.onmessageerror = () => fail('Worker message error');
} catch (error) {
  fail(error.message);
}
