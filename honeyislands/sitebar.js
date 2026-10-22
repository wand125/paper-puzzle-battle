// Thin site bar: <nav class="sitebar" data-current="game|paper|analyze" data-base="./|../"></nav>
// Labels follow ?lang (or the page language); links keep ?lang.
const LABELS = {
  ja: { title: 'ハニーアイランド・バトル', game: 'ゲーム', analyze: '解析ツール', paper: '論文' },
  en: { title: 'Honey Islands Battle', game: 'Game', analyze: 'Analysis', paper: 'Paper' },
};
const PATHS = { game: '', analyze: 'analyze/', paper: 'paper/' };
// tab icons (24×24, stroked), shown in the bottom bar on phones
const ICONS = {
  game: '<path d="M12 3l7.8 4.5v9L12 21l-7.8-4.5v-9z"/><circle cx="12" cy="12" r="3" fill="currentColor"/>',
  analyze: '<circle cx="10.5" cy="10.5" r="6"/><path d="M15 15l5.5 5.5M7.5 12l2-2.5 2 1.5 2-3"/>',
  paper: '<path d="M6 3h8.5L19 7.5V21H6z"/><path d="M14 3v5h5M9 12.5h7M9 16h7"/>',
};

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
  for (const k of ['game', 'paper', 'analyze']) {
    const a = Object.assign(document.createElement('a'), { className: 'sb-tab', href: base + PATHS[k] + q });
    a.innerHTML = `<svg class="sb-icon" viewBox="0 0 24 24" aria-hidden="true">${ICONS[k]}</svg>`;
    a.append(Object.assign(document.createElement('span'), { textContent: L[k] }));
    if (bar.dataset.current === k) a.setAttribute('aria-current', 'page');
    bar.append(a);
  }
}
renderSitebar();
