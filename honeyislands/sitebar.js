// Thin site bar: <nav class="sitebar" data-current="game|analyze|paper" data-base="./|../"></nav>
// Labels follow ?lang (or the page language); links keep ?lang.
const LABELS = {
  ja: { title: 'ハニーアイランド・バトル', game: 'ゲーム', analyze: '解析ツール', paper: '論文' },
  en: { title: 'Honey Islands Battle', game: 'Game', analyze: 'Analysis', paper: 'Paper' },
};
const PATHS = { game: '', analyze: 'analyze/', paper: 'paper/' };

export function renderSitebar(lang) {
  const bar = document.querySelector('nav.sitebar');
  if (!bar) return;
  const explicit = lang ?? new URLSearchParams(location.search).get('lang');
  lang = explicit || (document.documentElement.lang.startsWith('ja') ? 'ja' : 'en');
  const L = LABELS[lang] ?? LABELS.en;
  const base = bar.dataset.base || './';
  const q = explicit ? `?lang=${lang}` : '';
  bar.setAttribute('aria-label', lang === 'ja' ? 'ページ' : 'Pages');
  bar.replaceChildren();
  const title = Object.assign(document.createElement('a'), { className: 'sb-title', href: base + q, textContent: L.title });
  bar.append(title);
  for (const k of ['game', 'analyze', 'paper']) {
    const a = Object.assign(document.createElement('a'), { className: 'sb-tab', href: base + PATHS[k] + q, textContent: L[k] });
    if (bar.dataset.current === k) a.setAttribute('aria-current', 'page');
    bar.append(a);
  }
}
renderSitebar();
