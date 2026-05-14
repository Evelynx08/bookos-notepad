// BookOS Notepad — frontend
const invoke = (cmd, args) => window.__TAURI__.core.invoke(cmd, args);
const tauriWin = () => window.__TAURI__.window.getCurrentWindow();
const dialog = () => window.__TAURI__.dialog;
const $ = (s, r=document) => r.querySelector(s);

const EXT_MAP = {
  sh:'sh', bash:'sh', zsh:'sh', fish:'sh',
  html:'html', htm:'html', xhtml:'html', xml:'html', svg:'html',
  css:'css', scss:'css', sass:'css', less:'css',
  md:'md', markdown:'md',
  cfg:'cfg', conf:'cfg', ini:'cfg', toml:'cfg', yaml:'cfg', yml:'cfg',
  json:'json',
  py:'py',
  js:'js', mjs:'js', cjs:'js', ts:'js', tsx:'js', jsx:'js',
  rs:'rs',
  txt:'txt'
};
const LANG_LABEL = {
  sh:'Shell', html:'HTML', css:'CSS', md:'Markdown', cfg:'Config',
  json:'JSON', py:'Python', js:'JavaScript', rs:'Rust', txt:'Texto'
};

const DEFAULT_SETTINGS = {
  font: 'sans',        // sans|serif|mono
  wrap: true,
  showStatus: true,
  fileAccent: true,
  zoom: 100,           // percent
  previewOpen: false,
  previewMode: 'auto', // auto|html|md|url
  previewSplit: 50,    // editor pane % width
  previewUrl: '',
  adblockEnabled: true,
  youtubeNoCookie: true,
  youtubeDirect: false,     // try youtube.com/watch directly (skip /embed/)
  shortcuts: {}             // user overrides: { actionId: "Ctrl+Alt+B" }
};

let state = { theme:'auto', settings:{...DEFAULT_SETTINGS} };
let tabs = [];        // {id, name, path, content (html), dirty, ext, zoom, font, wrap, mruIdx}
let activeId = null;
let prevId = null;    // for Ctrl+Tab toggle
let tabSeq = 1;
let currentPage = 'editor';
let isFullscreen = false;
let fsHotzone = null;

// ───────── STATE PERSIST ─────────
async function loadState() {
  try { state = await invoke('load_state'); }
  catch { state = { theme:'auto' }; }
  if (!state.theme) state.theme = 'auto';
  state.settings = { ...DEFAULT_SETTINGS, ...(state.settings||{}) };
}
async function saveState() { try { await invoke('save_state', { state }); } catch(e){console.error(e);} }

// ───────── THEME ─────────
async function applyTheme() {
  const root = document.documentElement;
  root.classList.remove('light-mode','dark-mode');
  if (state.theme === 'light') root.classList.add('light-mode');
  else if (state.theme === 'dark') root.classList.add('dark-mode');
  else {
    try {
      const sys = await invoke('detect_system_theme');
      if (sys === 'dark') root.classList.add('dark-mode');
      else if (sys === 'light') root.classList.add('light-mode');
    } catch {}
  }
}
function cycleTheme() {
  state.theme = state.theme === 'auto' ? 'light' : state.theme === 'light' ? 'dark' : 'auto';
  saveState();
  applyTheme();
  toast('Tema: ' + (state.theme === 'auto' ? 'Automático' : state.theme === 'light' ? 'Claro' : 'Oscuro'));
}

// ───────── TOAST ─────────
let toastTimer = null;
function toast(msg) {
  const t = $('#toast');
  t.textContent = msg;
  t.classList.add('show');
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => t.classList.remove('show'), 1600);
}

// ───────── TABS ─────────
function extOf(name) {
  const m = name && name.match(/\.([A-Za-z0-9]+)$/);
  if (!m) return 'txt';
  return EXT_MAP[m[1].toLowerCase()] || 'txt';
}

function newTab({name='Sin título', path=null, content='', plain=true} = {}) {
  const id = 'tab-' + (tabSeq++);
  const ext = extOf(path || name);
  const html = plain ? escapeToHtml(content) : content;
  const tab = { id, name, path, ext, content: html, dirty: false };
  tabs.push(tab);
  setActive(id);
  renderTabs();
  return tab;
}

function escapeToHtml(text) {
  if (!text) return '';
  const div = document.createElement('div');
  // Preserve line breaks as <div> blocks (browser-friendly contenteditable)
  const lines = text.split('\n');
  return lines.map(l => {
    const safe = l.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');
    return '<div>' + (safe || '<br>') + '</div>';
  }).join('');
}

function htmlToPlain(html) {
  // Strip HTML, normalize line breaks
  const tmp = document.createElement('div');
  tmp.innerHTML = html;
  // Replace <br> with \n; block boundaries become \n
  tmp.querySelectorAll('br').forEach(br => br.replaceWith('\n'));
  // Insert \n between block-level elements
  const blocks = tmp.querySelectorAll('div,p,li,h1,h2,h3,h4,h5,h6,pre');
  blocks.forEach((b, i) => { if (i > 0) b.prepend(document.createTextNode('\n')); });
  return tmp.textContent.replace(/\r/g,'');
}

function getTab(id) { return tabs.find(t => t.id === id); }

function setActive(id) {
  if (id === activeId) return;
  // Persist current editor content into its tab before switching
  saveActiveContent();
  prevId = activeId;
  activeId = id;
  renderEditor();
  renderTabs();
  if (state.settings.previewOpen) renderPreview();
}

function saveActiveContent() {
  if (!activeId) return;
  const t = getTab(activeId);
  if (!t) return;
  const ed = $('.editor-area');
  if (ed) t.content = ed.innerHTML;
}

function closeTab(id) {
  const t = getTab(id);
  if (!t) return;
  if (t.dirty) {
    const ok = confirm(`"${t.name}" tiene cambios sin guardar. ¿Cerrar de todos modos?`);
    if (!ok) return;
  }
  const idx = tabs.indexOf(t);
  tabs.splice(idx, 1);
  if (activeId === id) {
    activeId = null;
    const next = tabs[idx] || tabs[idx-1];
    if (next) setActive(next.id);
    else { newTab(); return; }
  }
  if (prevId === id) prevId = null;
  renderTabs();
}

function renderTabs() {
  const row = $('#tabs-row');
  row.innerHTML = '';
  // #5 hide tabs row when only 1 tab
  row.classList.toggle('single-tab', tabs.length < 2);
  for (const t of tabs) {
    const el = document.createElement('div');
    el.className = 'tab' + (t.id === activeId ? ' active' : '') + (t.dirty ? ' dirty' : '');
    el.dataset.id = t.id;
    el.title = t.path || t.name;
    el.innerHTML = `
      <span class="tab-dot"></span>
      <span class="tab-name">${escapeHtml(t.name)}</span>
      <button class="tab-close" title="Cerrar" aria-label="Cerrar pestaña">×</button>`;
    el.addEventListener('mousedown', (e) => {
      if (e.button === 1) { e.preventDefault(); closeTab(t.id); return; }
      if (e.target.closest('.tab-close')) return;
      setActive(t.id);
    });
    el.querySelector('.tab-close').addEventListener('click', (e) => {
      e.stopPropagation();
      closeTab(t.id);
    });
    row.appendChild(el);
  }
  updateTitle();
}

function escapeHtml(s) {
  return (s||'').replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;');
}

function updateTitle() {
  const t = activeTab();
  const title = t ? (t.dirty ? '● ' : '') + t.name + ' — Bloc de notas' : 'Bloc de notas';
  try { tauriWin().setTitle(title); } catch {}
  $('#tb-title').textContent = t ? t.name : 'Bloc de notas';
}

function activeTab() { return getTab(activeId); }

// ───────── EDITOR ─────────
function renderEditor() {
  const host = $('#editor-host');
  host.innerHTML = '';
  const t = activeTab();
  if (!t) return;
  // Apply ext accent
  host.className = 'editor-host';
  if (state.settings.fileAccent) host.classList.add('ext-' + t.ext);

  const badge = document.createElement('div');
  badge.className = 'ext-badge ext-' + t.ext;
  badge.textContent = LANG_LABEL[t.ext] || t.ext.toUpperCase();
  host.appendChild(badge);

  const area = document.createElement('div');
  area.className = 'editor-area font-' + state.settings.font + ' ' + (state.settings.wrap ? 'wrap' : 'nowrap');
  area.contentEditable = 'true';
  area.spellcheck = false;
  area.dataset.placeholder = 'Empieza a escribir…';
  area.style.fontSize = (15 * state.settings.zoom / 100) + 'px';
  area.innerHTML = t.content || '';
  area.addEventListener('input', onEditorInput);
  area.addEventListener('keydown', onEditorKeydown);
  host.appendChild(area);
  area.focus();
  updateStatus();
}

function onEditorInput() {
  const t = activeTab();
  if (!t) return;
  if (!t.dirty) { t.dirty = true; renderTabs(); }
  updateStatus();
  schedulePreview();
}

// ───────── PREVIEW ─────────
let previewTimer = null;
let previewToken = 0;        // increments each render — async callbacks check before mount
function schedulePreview() {
  if (!state.settings.previewOpen) return;
  // URL mode is independent of editor content — don't re-render on every keystroke
  if (effectivePreviewMode() === 'url') return;
  clearTimeout(previewTimer);
  previewTimer = setTimeout(renderPreview, 350);
}

function effectivePreviewMode() {
  const m = state.settings.previewMode;
  if (m !== 'auto') return m;
  const t = activeTab();
  if (!t) return 'md';
  if (t.ext === 'html') return 'html';
  if (t.ext === 'md') return 'md';
  // svg / css fall through to html render
  return 'html';
}

function youtubeEmbed(url) {
  try {
    const u = new URL(url);
    const h = u.hostname.replace(/^www\./,'');
    // If user toggled "direct" mode, pass YT URLs through unchanged (no /embed/)
    if (state.settings.youtubeDirect) {
      if (h === 'youtube.com' || h === 'm.youtube.com' || h === 'youtu.be') return url;
    }
    const host = state.settings.youtubeNoCookie ? 'https://www.youtube-nocookie.com' : 'https://www.youtube.com';
    // Privacy + less spam params:
    //  rel=0  → no related videos at end
    //  modestbranding=1 → smaller YT logo
    //  iv_load_policy=3 → no annotations
    //  fs=1 fullscreen allowed
    //  cc_load_policy=0 default cc state
    const extra = 'rel=0&modestbranding=1&iv_load_policy=3&fs=1';
    const build = (id, start) => {
      let q = extra;
      if (start) q += '&start=' + start;
      return `${host}/embed/${id}?${q}`;
    };
    if (h === 'youtube.com' || h === 'm.youtube.com') {
      const v = u.searchParams.get('v');
      const t = parseInt(u.searchParams.get('t') || u.searchParams.get('start') || '0') || 0;
      if (v) return build(v, t);
      if (u.pathname.startsWith('/shorts/')) return build(u.pathname.split('/')[2], 0);
      if (u.pathname.startsWith('/embed/')) {
        const id = u.pathname.split('/')[2];
        return build(id, parseInt(u.searchParams.get('start')||'0')||0);
      }
    }
    if (h === 'youtu.be') {
      const id = u.pathname.replace(/^\//,'').split('/')[0];
      const t = parseInt(u.searchParams.get('t')||'0')||0;
      return build(id, t);
    }
  } catch {}
  return null;
}

function normalizeUrl(raw) {
  raw = (raw || '').trim();
  if (!raw) return '';
  // Common typos: ttps:// → https://, ttp:// → http://
  if (/^ttps:\/\//i.test(raw)) raw = 'h' + raw;
  else if (/^ttp:\/\//i.test(raw)) raw = 'h' + raw;
  if (/^https?:\/\//i.test(raw)) return raw;
  if (/^localhost(:\d+)?($|\/)/i.test(raw)) return 'http://' + raw;
  if (/^\d{1,3}(\.\d{1,3}){3}(:\d+)?($|\/)/.test(raw)) return 'http://' + raw;
  if (/^[\w.-]+\.[a-z]{2,}/i.test(raw)) return 'https://' + raw;
  return raw;
}

function renderPreview() {
  const body = $('#preview-body');
  if (!body) return;
  const myToken = ++previewToken;
  body.innerHTML = '';
  const mode = effectivePreviewMode();
  setActivePill(state.settings.previewMode);

  if (mode === 'url' || state.settings.previewMode === 'url') {
    const url = (state.settings.previewUrl || '').trim();
    if (!url) { showPreviewEmpty(body, 'Pega una URL arriba. YouTube se embebe automáticamente.'); return; }
    const yt = youtubeEmbed(url);
    const final = yt || normalizeUrl(url);
    // Adblock pre-check
    if (state.settings.adblockEnabled) {
      invoke('adblock_check', { url: final, sourceUrl: 'about:blank' }).then(res => {
        if (myToken !== previewToken) return; // stale render — bail
        body.innerHTML = '';
        if (res && res.blocked) {
          showAdblockBlocked(body, final, res.rule || '');
        } else {
          mountUrlIframe(body, final);
        }
      }).catch(() => {
        if (myToken !== previewToken) return;
        body.innerHTML = '';
        mountUrlIframe(body, final);
      });
    } else {
      mountUrlIframe(body, final);
    }
    return;
  }

  const t = activeTab();
  if (!t) { showPreviewEmpty(body, 'Sin pestaña activa'); return; }
  const ed = $('.editor-area');
  const text = ed ? htmlToPlain(ed.innerHTML) : '';

  if (mode === 'html') {
    const f = document.createElement('iframe');
    f.sandbox = 'allow-scripts allow-same-origin allow-forms allow-popups allow-modals';
    f.referrerPolicy = 'no-referrer';
    // For SVG: wrap in HTML container if it's just <svg>
    let doc = text;
    if (t.ext !== 'html' && /^\s*<svg/i.test(text)) {
      doc = `<!doctype html><html><head><meta charset="utf-8"><style>html,body{margin:0;background:#fff;height:100%;display:flex;align-items:center;justify-content:center}svg{max-width:100%;max-height:100%}</style></head><body>${text}</body></html>`;
    } else if (t.ext === 'css') {
      doc = `<!doctype html><html><head><meta charset="utf-8"><style>${text}</style></head><body><h1>Heading</h1><p>Lorem ipsum dolor sit amet, <a href="#">link</a>, <strong>bold</strong>, <em>italic</em>.</p><button>Botón</button> <input placeholder="input"> <ul><li>uno</li><li>dos</li></ul></body></html>`;
    }
    f.srcdoc = doc;
    body.appendChild(f);
    return;
  }

  if (mode === 'md') {
    const div = document.createElement('div');
    div.className = 'preview-md';
    let html = '';
    if (window.marked) {
      try {
        window.marked.setOptions({ breaks: true, gfm: true });
        html = window.marked.parse(text || '');
      } catch (e) { html = '<pre>' + escapeHtml(text) + '</pre>'; }
    } else {
      html = '<pre>' + escapeHtml(text) + '</pre>';
    }
    div.innerHTML = html;
    // Open external links in new context (intercept)
    div.querySelectorAll('a[href]').forEach(a => { a.target = '_blank'; a.rel = 'noopener'; });
    body.appendChild(div);
    return;
  }
}

function mountUrlIframe(body, final) {
  const isYt = /youtube(-nocookie)?\.com\/embed\//.test(final) || /youtube\.com\/watch/.test(final) || /youtu\.be/.test(final);
  body.classList.toggle('yt-fit', isYt);
  const f = document.createElement('iframe');
  f.src = final;
  f.referrerPolicy = 'strict-origin-when-cross-origin';
  f.allow = 'accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; fullscreen';
  f.setAttribute('allowfullscreen', '');
  body.appendChild(f);
  updateShieldBadge(true, final);
  // For YouTube embeds, add a floating action bar with PiP/external player options
  if (/youtube(-nocookie)?\.com\/embed\//.test(final) || /youtube\.com\/watch/.test(final) || /youtu\.be/.test(final)) {
    addYtActionBar(body, state.settings.previewUrl || final);
  }
}

function addYtActionBar(body, originalUrl) {
  const bar = document.createElement('div');
  bar.className = 'yt-actions';
  bar.innerHTML = `
    <button class="yt-action" id="yt-ext" title="Abrir en navegador">
      <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/><polyline points="15 3 21 3 21 9"/><line x1="10" y1="14" x2="21" y2="3"/></svg>
      <span>Abrir en navegador</span>
    </button>`;
  body.appendChild(bar);
  bar.querySelector('#yt-ext').addEventListener('click', () => openExternal(originalUrl));
}

async function playFloating(url) {
  try {
    const avail = await invoke('check_player_available');
    if (!avail.canPlayYoutube) {
      const missing = [];
      if (!avail.mpv) missing.push('mpv');
      if (!avail.ytdlp) missing.push('yt-dlp');
      toast('Falta instalar: ' + missing.join(' + ') + ' (sudo pacman -S ' + missing.join(' ') + ')');
      return;
    }
    // Calculate screen position of the preview-body so mpv overlays it
    const rect = $('#preview-body').getBoundingClientRect();
    let winPos = { x: 0, y: 0 };
    try {
      const pos = await tauriWin().outerPosition();
      winPos = { x: pos.x, y: pos.y };
    } catch {}
    const dpr = window.devicePixelRatio || 1;
    const args = {
      url: normalizeUrl(url),
      x: Math.round(winPos.x + rect.left * dpr),
      y: Math.round(winPos.y + rect.top * dpr),
      w: Math.round(rect.width * dpr),
      h: Math.round(rect.height * dpr)
    };
    await invoke('play_in_mpv', args);
    toast('Abierto en mpv encima del preview');
  } catch (e) {
    toast('Error mpv: ' + e);
  }
}

async function openExternal(url) {
  try {
    const fixed = normalizeUrl(url);
    await invoke('open_url_external', { url: fixed });
    toast('Abriendo en navegador…');
  } catch (e) {
    toast('Error: ' + e);
  }
}

function showAdblockBlocked(body, url, rule) {
  const d = document.createElement('div');
  d.className = 'preview-empty';
  d.innerHTML = `
    <svg width="56" height="56" viewBox="0 0 24 24" fill="none" stroke="var(--red)" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="9"/><path d="M5.6 5.6l12.8 12.8"/></svg>
    <div style="font-weight:600;color:var(--red);font-size:14px">URL bloqueada por filtro</div>
    <div style="opacity:.8;font-size:11.5px;max-width:340px;word-break:break-all;line-height:1.5">${escapeHtml(url)}</div>
    <div style="opacity:.5;font-size:11px;font-family:'JetBrains Mono',monospace;max-width:340px;word-break:break-all">${escapeHtml(rule || 'regla desconocida')}</div>
    <button class="tool-btn" id="adblock-bypass" style="margin-top:8px;background:var(--sbg);padding:6px 14px;opacity:1">Cargar igualmente</button>`;
  body.appendChild(d);
  const btn = d.querySelector('#adblock-bypass');
  if (btn) btn.addEventListener('click', () => {
    body.innerHTML = '';
    mountUrlIframe(body, url);
  });
  updateShieldBadge(false, url);
}

function updateShieldBadge(ok, url) {
  const sh = $('#shield-badge');
  if (!sh) return;
  const SHIELD_OK = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>`;
  const SHIELD_BLOCK = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/><line x1="9" y1="9" x2="15" y2="15"/><line x1="15" y1="9" x2="9" y2="15"/></svg>`;
  const SHIELD_OFF = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" stroke-dasharray="3 3"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>`;
  if (!state.settings.adblockEnabled) {
    sh.innerHTML = SHIELD_OFF;
    sh.title = 'Adblock desactivado';
    sh.className = 'tool-btn shield-badge off';
    return;
  }
  sh.innerHTML = ok ? SHIELD_OK : SHIELD_BLOCK;
  sh.title = ok ? 'Adblock activo · permitido' : 'Adblock bloqueó: ' + url;
  sh.className = 'tool-btn shield-badge ' + (ok ? 'on' : 'block');
}

function showPreviewEmpty(body, msg) {
  const d = document.createElement('div');
  d.className = 'preview-empty';
  d.innerHTML = `<svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg><span>${escapeHtml(msg)}</span>`;
  body.appendChild(d);
}

function setActivePill(mode) {
  document.querySelectorAll('.preview-mode-pills .pill').forEach(b => {
    b.classList.toggle('on', b.dataset.pmode === mode);
  });
}

function togglePreview(force) {
  state.settings.previewOpen = (force === undefined) ? !state.settings.previewOpen : !!force;
  applyPreviewLayout();
  saveState();
  $('#preview-btn').classList.toggle('active', state.settings.previewOpen);
  if (state.settings.previewOpen) renderPreview();
}

function applyPreviewLayout() {
  const pane = $('#preview-pane');
  const splitter = $('#splitter');
  const open = state.settings.previewOpen;
  pane.classList.toggle('hidden', !open);
  splitter.style.display = open ? '' : 'none';
  applySplit();
}

function applySplit() {
  const pct = Math.max(20, Math.min(80, state.settings.previewSplit || 50));
  const editor = $('#editor-pane');
  const preview = $('#preview-pane');
  if (state.settings.previewOpen) {
    editor.style.flex = `0 0 ${pct}%`;
    preview.style.flex = `1 1 auto`;
  } else {
    editor.style.flex = '1 1 auto';
  }
}

function wireSplitter() {
  const s = $('#splitter');
  let dragging = false;
  let startX = 0;
  let startPct = 50;
  const onMove = (e) => {
    if (!dragging) return;
    const split = $('#split');
    const rect = split.getBoundingClientRect();
    const x = (e.touches ? e.touches[0].clientX : e.clientX) - rect.left;
    const pct = Math.max(20, Math.min(80, (x / rect.width) * 100));
    state.settings.previewSplit = pct;
    applySplit();
  };
  const onUp = () => {
    if (!dragging) return;
    dragging = false;
    s.classList.remove('drag');
    document.body.style.cursor = '';
    saveState();
    window.removeEventListener('mousemove', onMove);
    window.removeEventListener('mouseup', onUp);
  };
  s.addEventListener('mousedown', (e) => {
    dragging = true;
    startX = e.clientX;
    startPct = state.settings.previewSplit || 50;
    s.classList.add('drag');
    document.body.style.cursor = 'col-resize';
    window.addEventListener('mousemove', onMove);
    window.addEventListener('mouseup', onUp);
  });
}

function wirePreviewBar() {
  document.querySelectorAll('.preview-mode-pills .pill').forEach(b => {
    b.addEventListener('click', () => {
      state.settings.previewMode = b.dataset.pmode;
      setActivePill(b.dataset.pmode);
      saveState();
      renderPreview();
    });
  });
  const inp = $('#url-input');
  const go = () => {
    state.settings.previewMode = 'url';
    state.settings.previewUrl = inp.value;
    setActivePill('url');
    saveState();
    renderPreview();
  };
  inp.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') { e.preventDefault(); go(); }
  });
  $('#url-go').addEventListener('click', go);
  $('#preview-refresh').addEventListener('click', renderPreview);
  $('#preview-close').addEventListener('click', () => togglePreview(false));
  $('#shield-badge').addEventListener('click', () => {
    state.settings.adblockEnabled = !state.settings.adblockEnabled;
    invoke('adblock_set_enabled', { enabled: state.settings.adblockEnabled }).catch(()=>{});
    saveState();
    toast('Adblock: ' + (state.settings.adblockEnabled ? 'ON' : 'OFF'));
    renderPreview();
  });
  // Restore saved URL
  if (state.settings.previewUrl) inp.value = state.settings.previewUrl;
  // Sync Rust side with persisted state
  invoke('adblock_set_enabled', { enabled: !!state.settings.adblockEnabled }).catch(()=>{});
}

// Code-like extensions that get auto-close brackets/tags
const CODE_EXTS = new Set(['html','css','js','py','rs','sh','cfg','json']);

function isCodeFile() {
  const t = activeTab();
  return t && CODE_EXTS.has(t.ext);
}

// Insert text at caret and place caret at offset from the end
function insertAtCaret(text, caretOffsetFromEnd = 0) {
  const sel = window.getSelection();
  if (!sel || !sel.rangeCount) return;
  const range = sel.getRangeAt(0);
  range.deleteContents();
  const node = document.createTextNode(text);
  range.insertNode(node);
  // Move caret to text.length - caretOffsetFromEnd
  const pos = text.length - caretOffsetFromEnd;
  const newRange = document.createRange();
  newRange.setStart(node, Math.max(0, Math.min(pos, text.length)));
  newRange.collapse(true);
  sel.removeAllRanges();
  sel.addRange(newRange);
}

// Get plain text content of editor up to caret (for tag matching)
function textBeforeCaret() {
  const ed = $('.editor-area');
  if (!ed) return '';
  const sel = window.getSelection();
  if (!sel || !sel.rangeCount) return '';
  const range = sel.getRangeAt(0).cloneRange();
  range.setStart(ed, 0);
  return range.toString();
}

// HTML void elements that should NOT auto-close
const VOID_TAGS = new Set(['area','base','br','col','embed','hr','img','input','keygen','link','meta','param','source','track','wbr','!doctype','!--']);

function onEditorKeydown(e) {
  // Tab — insert 2 spaces (or 4 for non-code)
  if (e.key === 'Tab') {
    e.preventDefault();
    document.execCommand('insertText', false, isCodeFile() ? '  ' : '    ');
    return;
  }

  if (!isCodeFile()) return;

  // Bracket / quote pairing — insert pair + caret in middle
  const PAIRS = { '(': ')', '[': ']', '{': '}', '"': '"', "'": "'", '`': '`' };
  if (PAIRS[e.key]) {
    // Don't auto-close quotes if next char is alphanumeric (likely apostrophe in text)
    e.preventDefault();
    insertAtCaret(e.key + PAIRS[e.key], 1);
    onEditorInput();
    return;
  }

  // Auto-close HTML tag when user types `>`
  if (e.key === '>') {
    const t = activeTab();
    if (!t || t.ext !== 'html') return;
    const before = textBeforeCaret();
    // Match last opening tag like `<tagname` or `<tagname attr="...">` (no `/` at start, no `/` at end)
    const m = before.match(/<([A-Za-z][A-Za-z0-9-]*)(?:\s[^<>]*)?$/);
    if (!m) return;
    const tag = m[1].toLowerCase();
    if (VOID_TAGS.has(tag)) return;
    // Check if already self-closing (ends with /)
    if (/\/\s*$/.test(before)) return;
    e.preventDefault();
    insertAtCaret('></' + m[1] + '>', m[1].length + 3);
    onEditorInput();
    return;
  }

  // Skip-over closing pair if user types the same closing char already present
  if ([')', ']', '}', '"', "'", '`'].includes(e.key)) {
    const sel = window.getSelection();
    if (!sel || !sel.rangeCount || !sel.isCollapsed) return;
    const r = sel.getRangeAt(0);
    const node = r.startContainer;
    if (node.nodeType === 3 && node.textContent.charAt(r.startOffset) === e.key) {
      e.preventDefault();
      const nr = document.createRange();
      nr.setStart(node, r.startOffset + 1);
      nr.collapse(true);
      sel.removeAllRanges();
      sel.addRange(nr);
    }
  }
}

function updateStatus() {
  const t = activeTab();
  const sp = $('#status-path');
  const sl = $('#status-lang');
  const sc = $('#status-chars');
  const sln = $('#status-lines');
  if (!t) { sp.textContent='—'; sc.textContent='0 caracteres'; sln.textContent='1 línea'; return; }
  sp.textContent = t.path || t.name;
  sl.textContent = LANG_LABEL[t.ext] || 'Texto';
  const ed = $('.editor-area');
  const text = ed ? htmlToPlain(ed.innerHTML) : '';
  sc.textContent = text.length + ' caracteres';
  sln.textContent = (text.split('\n').length) + ' líneas';
  $('#statusbar').classList.toggle('hidden', !state.settings.showStatus);
}

// ───────── FORMAT ─────────
function exec(cmd, val=null) {
  const ed = $('.editor-area');
  if (!ed) return;
  ed.focus();
  document.execCommand(cmd, false, val);
  refreshToolbarState();
  onEditorInput();
}

function refreshToolbarState() {
  for (const btn of document.querySelectorAll('.tool-btn[data-cmd]')) {
    const cmd = btn.dataset.cmd;
    let on = false;
    try { on = document.queryCommandState(cmd); } catch {}
    btn.classList.toggle('active', on);
  }
}

// ───────── ZOOM ─────────
function setZoom(z) {
  z = Math.max(50, Math.min(300, Math.round(z)));
  state.settings.zoom = z;
  $('#zoom-val').textContent = z + '%';
  const ed = $('.editor-area');
  if (ed) ed.style.fontSize = (15 * z / 100) + 'px';
  saveState();
}
function zoomBy(d) { setZoom(state.settings.zoom + d); }
function zoomReset() { setZoom(100); }

// ───────── FILE IO ─────────
async function openFile() {
  try {
    const sel = await dialog().open({
      multiple: true,
      filters: [
        { name: 'Texto', extensions: ['txt','md','markdown','log','rst'] },
        { name: 'Código', extensions: ['sh','bash','zsh','fish','py','js','mjs','ts','tsx','jsx','rs','c','cpp','h','hpp','java','go','rb','php','lua','swift','kt'] },
        { name: 'Web', extensions: ['html','htm','css','scss','sass','less','xml','svg','json'] },
        { name: 'Config', extensions: ['cfg','conf','ini','toml','yaml','yml','desktop'] },
        { name: 'Todos', extensions: ['*'] }
      ]
    });
    if (!sel) return;
    const paths = Array.isArray(sel) ? sel : [sel];
    for (const p of paths) {
      const txt = await invoke('read_file', { path: p });
      const name = p.split(/[\\/]/).pop();
      newTab({ name, path: p, content: txt, plain: true });
    }
    toast(`Abierto: ${paths.length} archivo${paths.length>1?'s':''}`);
  } catch (e) {
    console.error(e);
    toast('Error al abrir');
  }
}

async function saveFile(forceAs=false) {
  saveActiveContent();
  const t = activeTab();
  if (!t) return;
  let path = t.path;
  if (!path || forceAs) {
    try {
      path = await dialog().save({
        defaultPath: t.name,
        filters: [
          { name: 'Texto', extensions: ['txt'] },
          { name: 'Markdown', extensions: ['md'] },
          { name: 'HTML', extensions: ['html'] },
          { name: 'Todos', extensions: ['*'] }
        ]
      });
    } catch (e) { console.error(e); return; }
    if (!path) return;
  }
  const ext = (path.match(/\.([A-Za-z0-9]+)$/) || [,''])[1].toLowerCase();
  const ed = $('.editor-area');
  let payload;
  if (ext === 'html' || ext === 'htm') payload = ed.innerHTML;
  else payload = htmlToPlain(ed.innerHTML);
  try {
    await invoke('write_file', { path, contents: payload });
    t.path = path;
    t.name = path.split(/[\\/]/).pop();
    t.ext = extOf(t.name);
    t.dirty = false;
    renderTabs();
    renderEditor();
    toast('Guardado: ' + t.name);
  } catch (e) {
    console.error(e);
    toast('Error al guardar');
  }
}

// ───────── PAGES ─────────
function showPage(name) {
  currentPage = name;
  const pages = { editor: $('#page-editor'), settings: $('#page-settings') };
  for (const k in pages) {
    const p = pages[k]; if (!p) continue;
    if (k === name) {
      p.classList.remove('hidden','slide-in-r','slide-in-l');
      void p.offsetWidth;
      p.classList.add(name === 'settings' ? 'slide-in-r' : 'slide-in-l');
    } else p.classList.add('hidden');
  }
  $('#settings-btn').classList.toggle('active', name === 'settings');
  $('#toolbar').style.display = name === 'editor' ? '' : 'none';
  if (name === 'settings') renderSettings();
  else { renderEditor(); }
}

// ───────── SETTINGS ─────────
function renderSettings() {
  setSeg('seg-theme', state.theme);
  setSeg('seg-font', state.settings.font);
  setToggle('tg-wrap', state.settings.wrap);
  setToggle('tg-status', state.settings.showStatus);
  setToggle('tg-fileaccent', state.settings.fileAccent);
  setToggle('tg-adblock', state.settings.adblockEnabled);
  setToggle('tg-yt-nocookie', state.settings.youtubeNoCookie);
  setToggle('tg-yt-direct', state.settings.youtubeDirect);
}
function setSeg(id, value) {
  const root = document.getElementById(id);
  if (!root) return;
  root.querySelectorAll('button').forEach(b => b.classList.toggle('on', b.dataset.v === value));
}
function setToggle(id, on) {
  const el = document.getElementById(id);
  if (!el) return;
  el.classList.toggle('active', !!on);
  el.setAttribute('aria-checked', on ? 'true' : 'false');
}
function wireSettings() {
  document.querySelectorAll('.seg').forEach(seg => {
    seg.addEventListener('click', (e) => {
      const btn = e.target.closest('button');
      if (!btn) return;
      const key = seg.dataset.key;
      const val = btn.dataset.v;
      if (key === 'theme') { state.theme = val; applyTheme(); }
      else if (key === 'font') { state.settings.font = val; }
      setSeg(seg.id, val);
      saveState();
    });
  });
  const wireToggle = (id, key, after) => {
    const el = document.getElementById(id);
    if (!el) return;
    const fire = () => {
      state.settings[key] = !state.settings[key];
      setToggle(id, state.settings[key]);
      saveState();
      if (after) after();
    };
    el.addEventListener('click', fire);
    el.addEventListener('keydown', (e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); fire(); } });
  };
  wireToggle('tg-wrap', 'wrap');
  wireToggle('tg-status', 'showStatus', updateStatus);
  wireToggle('tg-fileaccent', 'fileAccent');
  wireToggle('tg-adblock', 'adblockEnabled', () => {
    invoke('adblock_set_enabled', { enabled: state.settings.adblockEnabled }).catch(()=>{});
    if (state.settings.previewOpen) renderPreview();
  });
  wireToggle('tg-yt-nocookie', 'youtubeNoCookie', () => {
    if (state.settings.previewOpen) renderPreview();
  });
  wireToggle('tg-yt-direct', 'youtubeDirect', () => {
    if (state.settings.previewOpen) renderPreview();
  });
  const openSc = document.getElementById('open-shortcuts');
  if (openSc) openSc.addEventListener('click', () => toggleShortcutsPalette(true));
  const resetSc = document.getElementById('reset-shortcuts');
  if (resetSc) resetSc.addEventListener('click', () => {
    if (!confirm('¿Restablecer TODOS los atajos a su valor por defecto?')) return;
    state.settings.shortcuts = {};
    saveState();
    toast('Atajos restablecidos');
  });
}

// ───────── FULLSCREEN ─────────
async function toggleFullscreen() {
  isFullscreen = !isFullscreen;
  const mc = $('#mc');
  mc.classList.toggle('fullscreen-note', isFullscreen);
  mc.classList.toggle('windowed', !isFullscreen);
  if (isFullscreen) setupFsChrome();
  else teardownFsChrome();
  try {
    await tauriWin().setFullscreen(isFullscreen);
  } catch (e) {
    console.error('setFullscreen failed, falling back to maximize:', e);
    try { await tauriWin().toggleMaximize(); } catch (e2) { console.error(e2); }
  }
}
function setupFsChrome() {
  const mc = $('#mc');
  const titlebar = $('.titlebar');
  const toolbar = $('#toolbar');
  if (fsHotzone) fsHotzone.remove();
  fsHotzone = document.createElement('div');
  fsHotzone.className = 'fs-hotzone';
  document.body.append(fsHotzone);
  const show = () => mc.classList.add('show-chrome');
  const hide = () => mc.classList.remove('show-chrome');
  fsHotzone.addEventListener('mouseenter', show);
  titlebar.addEventListener('mouseenter', show);
  toolbar.addEventListener('mouseenter', show);
  titlebar.addEventListener('mouseleave', (e) => {
    // Only hide if leaving downward past toolbar
    if (e.relatedTarget !== toolbar) hide();
  });
  toolbar.addEventListener('mouseleave', hide);
}
function teardownFsChrome() {
  const mc = $('#mc');
  mc.classList.remove('show-chrome');
  if (fsHotzone) { fsHotzone.remove(); fsHotzone = null; }
}

// ───────── WIRE ─────────
function wireWindow() {
  $('#minimize').addEventListener('click', () => tauriWin().minimize());
  $('#maximize').addEventListener('click', () => tauriWin().toggleMaximize());
  $('#close').addEventListener('click', async () => {
    const dirty = tabs.filter(t => t.dirty);
    if (dirty.length) {
      const ok = confirm(`${dirty.length} pestaña(s) sin guardar. ¿Salir igualmente?`);
      if (!ok) return;
    }
    tauriWin().close();
  });
  $('#theme-btn').addEventListener('click', cycleTheme);
  $('#fullscreen-btn').addEventListener('click', toggleFullscreen);
  const scBtn = document.getElementById('shortcuts-btn');
  if (scBtn) scBtn.addEventListener('click', () => toggleShortcutsPalette());
  $('#settings-btn').addEventListener('click', () => showPage(currentPage === 'settings' ? 'editor' : 'settings'));
  $('#preview-btn').addEventListener('click', () => { showPage('editor'); togglePreview(); });
  $('#new-btn').addEventListener('click', () => { showPage('editor'); newTab(); });
  $('#open-btn').addEventListener('click', () => { showPage('editor'); openFile(); });
  $('#save-btn').addEventListener('click', () => saveFile(false));
}

function wireToolbar() {
  $('#toolbar').addEventListener('click', (e) => {
    const b = e.target.closest('.tool-btn[data-cmd]');
    if (!b) return;
    exec(b.dataset.cmd);
  });
  $('#zoom-in').addEventListener('click', () => zoomBy(+10));
  $('#zoom-out').addEventListener('click', () => zoomBy(-10));
  $('#color-text').addEventListener('input', (e) => exec('foreColor', e.target.value));
  $('#color-bg').addEventListener('input', (e) => exec('hiliteColor', e.target.value));
  // Refresh toolbar state on selection change
  document.addEventListener('selectionchange', () => {
    if (currentPage === 'editor') refreshToolbarState();
  });
}

// ───────── SHORTCUTS REGISTRY ─────────
// Each action: id, label, default binding, run(e), section.
// Bindings are strings like "Ctrl+N", "Ctrl+Shift+S", "F11", "Ctrl+Tab".
const ACTIONS = [
  // File
  { id:'new-tab',      sec:'Archivo',  label:'Nueva pestaña',     def:'Ctrl+N',     run:()=>{ showPage('editor'); newTab(); } },
  { id:'new-tab-alt',  sec:'Archivo',  label:'Nueva pestaña (alt)',def:'Ctrl+T',    run:()=>{ showPage('editor'); newTab(); } },
  { id:'open-file',    sec:'Archivo',  label:'Abrir archivo',     def:'Ctrl+O',     run:()=>{ showPage('editor'); openFile(); } },
  { id:'save',         sec:'Archivo',  label:'Guardar',           def:'Ctrl+S',     run:()=>saveFile(false) },
  { id:'save-as',      sec:'Archivo',  label:'Guardar como',      def:'Ctrl+Shift+S', run:()=>saveFile(true) },
  { id:'close-tab',    sec:'Archivo',  label:'Cerrar pestaña',    def:'Ctrl+W',     run:()=>{ if (activeId) closeTab(activeId); } },
  // Navigation
  { id:'next-tab-mru', sec:'Navegación',label:'Pestaña anterior usada', def:'Ctrl+Tab', run:()=>cycleTab('mru') },
  { id:'prev-tab',     sec:'Navegación',label:'Pestaña previa',   def:'Ctrl+Shift+Tab', run:()=>cycleTab('prev') },
  { id:'preview',      sec:'Vista',    label:'Mostrar/ocultar preview', def:'Ctrl+P', run:()=>{ showPage('editor'); togglePreview(); } },
  { id:'settings',     sec:'Vista',    label:'Abrir/cerrar ajustes', def:'Ctrl+,',   run:()=>showPage(currentPage === 'settings' ? 'editor' : 'settings') },
  { id:'fullscreen',   sec:'Vista',    label:'Pantalla completa', def:'F11',        run:()=>toggleFullscreen() },
  { id:'shortcuts',    sec:'Vista',    label:'Mostrar atajos',    def:'Ctrl+/',     run:()=>toggleShortcutsPalette() },
  // Format
  { id:'bold',         sec:'Formato',  label:'Negrita',           def:'Ctrl+B',     run:()=>{ if (currentPage==='editor') exec('bold'); } },
  { id:'italic',       sec:'Formato',  label:'Cursiva',           def:'Ctrl+I',     run:()=>{ if (currentPage==='editor') exec('italic'); } },
  { id:'underline',    sec:'Formato',  label:'Subrayado',         def:'Ctrl+U',     run:()=>{ if (currentPage==='editor') exec('underline'); } },
  // Edit
  { id:'undo',         sec:'Edición',  label:'Deshacer',          def:'Ctrl+Z',     run:()=>doUndoRedo(false) },
  { id:'redo',         sec:'Edición',  label:'Rehacer',           def:'Ctrl+Y',     run:()=>doUndoRedo(true) },
  { id:'redo-alt',     sec:'Edición',  label:'Rehacer (alt)',     def:'Ctrl+Shift+Z', run:()=>doUndoRedo(true) },
  { id:'select-all',   sec:'Edición',  label:'Seleccionar todo',  def:'Ctrl+A',     run:()=>doSelectAll() },
  // Zoom
  { id:'zoom-in',      sec:'Zoom',     label:'Aumentar texto',    def:'Ctrl++',     run:()=>zoomBy(+10) },
  { id:'zoom-out',     sec:'Zoom',     label:'Reducir texto',     def:'Ctrl+-',     run:()=>zoomBy(-10) },
  { id:'zoom-reset',   sec:'Zoom',     label:'Tamaño 100%',       def:'Ctrl+0',     run:()=>zoomReset() },
];

function getBinding(actionId) {
  const custom = (state.settings.shortcuts || {})[actionId];
  if (custom === '') return '';   // user disabled
  if (custom) return custom;
  const a = ACTIONS.find(x => x.id === actionId);
  return a ? a.def : '';
}

function setBinding(actionId, combo) {
  state.settings.shortcuts = state.settings.shortcuts || {};
  state.settings.shortcuts[actionId] = combo;
  saveState();
}

function resetBinding(actionId) {
  if (state.settings.shortcuts) {
    delete state.settings.shortcuts[actionId];
    saveState();
  }
}

// Normalize a KeyboardEvent into a binding string like "Ctrl+Shift+B"
function eventToCombo(e) {
  const parts = [];
  if (e.ctrlKey || e.metaKey) parts.push('Ctrl');
  if (e.altKey) parts.push('Alt');
  if (e.shiftKey) parts.push('Shift');
  let k = e.key;
  // Normalize special names
  if (k === ' ') k = 'Space';
  else if (k.length === 1) k = k.toUpperCase();
  // Filter out lone modifiers
  if (['Control','Shift','Alt','Meta'].includes(k)) return null;
  parts.push(k);
  return parts.join('+');
}

// Compare two combos in canonical order
function comboMatches(combo, eventCombo) {
  if (!combo) return false;
  // Normalize both to canonical form (sorted modifiers + final key)
  const norm = c => {
    const segs = c.split('+');
    const final = segs.pop();
    const mods = segs.map(s => s.trim()).sort().join('+');
    const finalNorm = final.length === 1 ? final.toUpperCase() : final;
    return (mods ? mods + '+' : '') + finalNorm;
  };
  return norm(combo).toLowerCase() === norm(eventCombo).toLowerCase();
}

// Helpers
function cycleTab(mode) {
  if (tabs.length < 2) return;
  const idx = tabs.findIndex(t => t.id === activeId);
  if (mode === 'mru') {
    if (prevId && getTab(prevId)) setActive(prevId);
    else setActive(tabs[(idx + 1) % tabs.length].id);
  } else if (mode === 'prev') {
    setActive(tabs[(idx - 1 + tabs.length) % tabs.length].id);
  }
}

function doUndoRedo(redo) {
  if (currentPage !== 'editor') return;
  const ed = $('.editor-area');
  if (!ed) return;
  ed.focus();
  document.execCommand(redo ? 'redo' : 'undo');
  refreshToolbarState();
  onEditorInput();
}

function doSelectAll() {
  if (currentPage !== 'editor') return;
  const ed = $('.editor-area');
  if (!ed) return;
  const r = document.createRange();
  r.selectNodeContents(ed);
  const sel = window.getSelection();
  sel.removeAllRanges();
  sel.addRange(r);
}

let capturingCombo = false;

function wireKeys() {
  document.addEventListener('keydown', (e) => {
    if (capturingCombo) return;   // combo capture has its own handler
    const combo = eventToCombo(e);
    if (!combo) return;
    // Escape special — close modal/return to editor
    if (e.key === 'Escape') {
      if ($('#shortcuts-palette')?.classList.contains('open')) {
        e.preventDefault(); toggleShortcutsPalette(false); return;
      }
      if (isFullscreen) { e.preventDefault(); toggleFullscreen(); return; }
      if (currentPage !== 'editor') { e.preventDefault(); showPage('editor'); return; }
      return;
    }
    for (const a of ACTIONS) {
      if (comboMatches(getBinding(a.id), combo)) {
        e.preventDefault();
        try { a.run(e); } catch (err) { console.error(err); }
        return;
      }
    }
  });
  // Ctrl+wheel zoom
  window.addEventListener('wheel', (e) => {
    if (e.ctrlKey || e.metaKey) {
      e.preventDefault();
      zoomBy(e.deltaY < 0 ? +10 : -10);
    }
  }, { passive:false });
  window.addEventListener('gesturestart', e => e.preventDefault());
  document.addEventListener('contextmenu', e => e.preventDefault());
}

// ───────── SHORTCUTS PALETTE (modal) ─────────
function ensurePaletteDom() {
  if ($('#shortcuts-palette')) return;
  const p = document.createElement('div');
  p.id = 'shortcuts-palette';
  p.className = 'sc-palette';
  p.innerHTML = `
    <div class="sc-backdrop"></div>
    <div class="sc-card" role="dialog" aria-modal="true" aria-label="Atajos">
      <div class="sc-head">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>
        <input class="sc-search" id="sc-search" placeholder="Buscar atajo… (acción o tecla)" autocomplete="off" spellcheck="false">
        <button class="sc-close" id="sc-close" title="Cerrar (Esc)">×</button>
      </div>
      <div class="sc-list" id="sc-list"></div>
      <div class="sc-foot">
        <span>↵ ejecutar · click derecho editar · doble click resetear</span>
        <button class="sc-reset-all" id="sc-reset-all">Restablecer todos</button>
      </div>
    </div>`;
  document.body.appendChild(p);
  $('#sc-close').addEventListener('click', () => toggleShortcutsPalette(false));
  p.querySelector('.sc-backdrop').addEventListener('click', () => toggleShortcutsPalette(false));
  $('#sc-search').addEventListener('input', renderShortcutsList);
  $('#sc-reset-all').addEventListener('click', () => {
    if (!confirm('¿Restablecer TODOS los atajos a su valor por defecto?')) return;
    state.settings.shortcuts = {};
    saveState();
    renderShortcutsList();
  });
}

function toggleShortcutsPalette(force) {
  ensurePaletteDom();
  const p = $('#shortcuts-palette');
  const open = force === undefined ? !p.classList.contains('open') : !!force;
  p.classList.toggle('open', open);
  if (open) {
    renderShortcutsList();
    setTimeout(() => $('#sc-search').focus(), 50);
  }
}

function renderShortcutsList() {
  const list = $('#sc-list');
  if (!list) return;
  const q = ($('#sc-search')?.value || '').toLowerCase().trim();
  // Group by section
  const groups = {};
  for (const a of ACTIONS) {
    const binding = getBinding(a.id);
    if (q && !a.label.toLowerCase().includes(q) && !a.sec.toLowerCase().includes(q) && !binding.toLowerCase().includes(q)) continue;
    (groups[a.sec] = groups[a.sec] || []).push({ ...a, binding });
  }
  list.innerHTML = '';
  if (Object.keys(groups).length === 0) {
    list.innerHTML = '<div class="sc-empty">Sin resultados.</div>';
    return;
  }
  for (const sec of Object.keys(groups)) {
    const h = document.createElement('div');
    h.className = 'sc-section';
    h.textContent = sec;
    list.appendChild(h);
    for (const a of groups[sec]) {
      const row = document.createElement('div');
      row.className = 'sc-row';
      row.tabIndex = 0;
      row.dataset.id = a.id;
      const customized = (state.settings.shortcuts || {})[a.id] !== undefined;
      row.innerHTML = `
        <span class="sc-label">${escapeHtml(a.label)}${customized ? ' <span class="sc-dot" title="Personalizado">●</span>' : ''}</span>
        <span class="sc-combo">${renderCombo(a.binding)}</span>`;
      row.addEventListener('click', () => {
        toggleShortcutsPalette(false);
        try { a.run(); } catch (err) { console.error(err); }
      });
      row.addEventListener('contextmenu', (e) => {
        e.preventDefault();
        startCaptureFor(a.id, row);
      });
      row.addEventListener('dblclick', () => {
        resetBinding(a.id);
        renderShortcutsList();
        toast('Atajo restablecido');
      });
      row.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') { row.click(); }
        else if (e.key === 'F2' || e.key === ' ') { e.preventDefault(); startCaptureFor(a.id, row); }
      });
      list.appendChild(row);
    }
  }
}

function renderCombo(combo) {
  if (!combo) return '<em class="sc-disabled">(sin asignar)</em>';
  return combo.split('+').map(k => `<kbd>${escapeHtml(k)}</kbd>`).join('<span class="sc-plus">+</span>');
}

function startCaptureFor(actionId, row) {
  const combo = row.querySelector('.sc-combo');
  combo.innerHTML = '<em class="sc-capturing">Pulsa combinación…</em>';
  capturingCombo = true;
  const onKey = (e) => {
    e.preventDefault();
    e.stopPropagation();
    if (e.key === 'Escape') {
      capturingCombo = false;
      window.removeEventListener('keydown', onKey, true);
      renderShortcutsList();
      return;
    }
    if (e.key === 'Backspace' || e.key === 'Delete') {
      capturingCombo = false;
      window.removeEventListener('keydown', onKey, true);
      setBinding(actionId, '');
      renderShortcutsList();
      toast('Atajo desactivado');
      return;
    }
    const c = eventToCombo(e);
    if (!c) return;  // wait for non-modifier
    capturingCombo = false;
    window.removeEventListener('keydown', onKey, true);
    // Check conflict
    const conflict = ACTIONS.find(a => a.id !== actionId && comboMatches(getBinding(a.id), c));
    if (conflict) {
      if (!confirm(`"${c}" ya se usa para "${conflict.label}". ¿Sobrescribir?`)) {
        renderShortcutsList();
        return;
      }
      setBinding(conflict.id, '');
    }
    setBinding(actionId, c);
    renderShortcutsList();
    toast('Atajo: ' + c);
  };
  window.addEventListener('keydown', onKey, true);
}

function wireSystemTheme() {
  const mq = window.matchMedia('(prefers-color-scheme: dark)');
  const handler = () => { if (state.theme === 'auto') applyTheme(); };
  if (mq.addEventListener) mq.addEventListener('change', handler);
  setInterval(() => { if (state.theme === 'auto') applyTheme(); }, 5000);
}

// ───────── BOOT ─────────
(async function init() {
  await loadState();
  await applyTheme();
  wireWindow();
  wireToolbar();
  wireSettings();
  wireKeys();
  wireSplitter();
  wirePreviewBar();
  wireSystemTheme();
  // open first tab
  newTab();
  setZoom(state.settings.zoom);
  applyPreviewLayout();
  $('#preview-btn').classList.toggle('active', state.settings.previewOpen);
  showPage('editor');
  if (state.settings.previewOpen) renderPreview();
})();
