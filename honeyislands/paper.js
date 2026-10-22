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

// Theme switch: light by default, a saved choice of dark (set early by the inline script in <head>).
const THEME = 'hib-paper-theme';
function theme(t) {
  if (t === 'dark') document.documentElement.dataset.theme = 'dark'; else delete document.documentElement.dataset.theme;
  for (const b of document.querySelectorAll('[data-theme-set]')) b.setAttribute('aria-pressed', String(b.dataset.themeSet === t));
}
theme(document.documentElement.dataset.theme === 'dark' ? 'dark' : 'light');
for (const b of document.querySelectorAll('[data-theme-set]')) {
  b.onclick = () => { try { localStorage.setItem(THEME, b.dataset.themeSet); } catch {} theme(b.dataset.themeSet); };
}
