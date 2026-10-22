// Language switch (EN default; ?lang=ja|en and a saved choice) and the first-move maps.
const KEY = 'hib-paper-language';
function pick() {
  const q = new URLSearchParams(location.search).get('lang');
  if (q === 'ja' || q === 'en') return q;
  try { if (localStorage.getItem(KEY) === 'ja') return 'ja'; } catch {}
  return 'en';
}
function apply(lang) {
  document.documentElement.lang = lang;
  for (const a of document.querySelectorAll('article[data-lang]')) a.hidden = a.dataset.lang !== lang;
  for (const b of document.querySelectorAll('[data-language]')) b.setAttribute('aria-pressed', String(b.dataset.language === lang));
  document.title = lang === 'ja' ? 'ハニーアイランド・バトルを解く' : 'Honey Islands Battle is Solved';
}
let lang = pick();
apply(lang);
for (const b of document.querySelectorAll('[data-language]')) {
  b.onclick = () => {
    lang = b.dataset.language;
    try { localStorage.setItem(KEY, lang); } catch {}
    const u = new URL(location.href); u.searchParams.set('lang', lang); history.replaceState(null, '', u);
    apply(lang);
  };
}

// cells: y outer, x inner, axial coordinates with |x + y| <= 4 (same order as the engine)
const R = 4, cells = [];
for (let y = -R; y <= R; ++y) for (let x = -R; x <= R; ++x) if (Math.abs(x + y) <= R) cells.push([x, y]);
const SIZE = 5.6;
function hex(cx, cy) {
  const pts = [];
  for (let k = 0; k < 6; ++k) { const a = Math.PI / 180 * (60 * k - 30); pts.push(`${(cx + SIZE * Math.cos(a)).toFixed(2)},${(cy + SIZE * Math.sin(a)).toFixed(2)}`); }
  return pts.join(' ');
}
async function draw(rule) {
  let move1;
  try { move1 = (await (await fetch(`results/${rule}/first-moves.json`)).json()).move1; } catch { return; }
  for (const l of ['en', 'ja']) {
    const svg = document.getElementById(`map-${rule}-${l}`);
    if (!svg) continue;
    const ns = 'http://www.w3.org/2000/svg';
    cells.forEach(([x, y], i) => {
      const cx = 50 + SIZE * Math.sqrt(3) * (x + y / 2), cy = 46 + SIZE * 1.5 * y;
      const p = document.createElementNS(ns, 'polygon');
      p.setAttribute('points', hex(cx, cy));
      const v = move1[i] && move1[i].v;
      p.setAttribute('class', v === 0 ? 'win' : v === 1 ? 'loss' : 'unknown');
      const t = document.createElementNS(ns, 'title');
      t.textContent = v === 0 ? (l === 'ja' ? '勝つ初手' : 'winning first move') : (l === 'ja' ? '負ける初手' : 'losing first move');
      p.append(t);
      svg.append(p);
    });
  }
}
draw('normal');
draw('pass');
