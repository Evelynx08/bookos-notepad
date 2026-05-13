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
  youtubeDirect: false      // try youtube.com/watch directly (skip /embed/)
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
  const f = document.createElement('iframe');
  f.src = final;
  f.referrerPolicy = 'strict-origin-when-cross-origin';
  f.allow = 'accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; fullscreen';
  f.setAttribute('allowfullscreen', '');
  body.appendChild(f);
  updateShieldBadge(true, final);
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
  if (!state.settings.adblockEnabled) {
    sh.textContent = '○';
    sh.title = 'Adblock desactivado';
    sh.className = 'shield-badge off';
    return;
  }
  sh.textContent = ok ? '🛡' : '⊘';
  sh.title = ok ? 'Adblock activo · permitido' : 'Adblock bloqueó: ' + url;
  sh.className = 'shield-badge ' + (ok ? 'on' : 'block');
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

function onEditorKeydown(e) {
  // Tab inserts spaces in mono mode, indent otherwise
  if (e.key === 'Tab') {
    e.preventDefault();
    document.execCommand('insertText', false, '    ');
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

function wireKeys() {
  document.addEventListener('keydown', (e) => {
    if (e.key === 'F11') { e.preventDefault(); toggleFullscreen(); return; }
    if (e.key === 'Escape' && isFullscreen) { toggleFullscreen(); return; }
    const ctrl = e.ctrlKey || e.metaKey;
    if (!ctrl) {
      if (e.key === 'Escape' && currentPage !== 'editor') { showPage('editor'); }
      return;
    }
    const k = e.key.toLowerCase();
    if (k === 't' || k === 'n') { e.preventDefault(); showPage('editor'); newTab(); return; }
    if (k === 'o') { e.preventDefault(); showPage('editor'); openFile(); return; }
    if (k === 's') {
      e.preventDefault();
      saveFile(e.shiftKey);
      return;
    }
    if (k === 'w') {
      e.preventDefault();
      if (activeId) closeTab(activeId);
      return;
    }
    if (e.key === 'Tab') {
      e.preventDefault();
      if (tabs.length < 2) return;
      if (e.shiftKey) {
        const idx = tabs.findIndex(t => t.id === activeId);
        const next = tabs[(idx - 1 + tabs.length) % tabs.length];
        setActive(next.id);
      } else {
        // Toggle to last-used (Ctrl+Tab → MRU pair)
        if (prevId && getTab(prevId)) setActive(prevId);
        else {
          const idx = tabs.findIndex(t => t.id === activeId);
          setActive(tabs[(idx + 1) % tabs.length].id);
        }
      }
      return;
    }
    if (e.key === '+' || e.key === '=') { e.preventDefault(); zoomBy(+10); return; }
    if (e.key === '-' || e.key === '_') { e.preventDefault(); zoomBy(-10); return; }
    if (e.key === '0') { e.preventDefault(); zoomReset(); return; }
    if (k === 'b' || k === 'i' || k === 'u') {
      e.preventDefault();
      if (currentPage !== 'editor') return;
      exec(k === 'b' ? 'bold' : k === 'i' ? 'italic' : 'underline');
      return;
    }
    if (k === 'p') {
      e.preventDefault();
      showPage('editor');
      togglePreview();
      return;
    }
    if (k === 'z') {
      // Ctrl+Z undo, Ctrl+Shift+Z redo
      if (currentPage !== 'editor') return;
      const ed = $('.editor-area');
      if (!ed) return;
      e.preventDefault();
      ed.focus();
      document.execCommand(e.shiftKey ? 'redo' : 'undo');
      refreshToolbarState();
      onEditorInput();
      return;
    }
    if (k === 'y') {
      if (currentPage !== 'editor') return;
      const ed = $('.editor-area');
      if (!ed) return;
      e.preventDefault();
      ed.focus();
      document.execCommand('redo');
      refreshToolbarState();
      onEditorInput();
      return;
    }
    if (k === 'a') {
      // Let native select-all work inside editor
      if (currentPage !== 'editor') return;
      const ed = $('.editor-area');
      if (!ed) return;
      e.preventDefault();
      const r = document.createRange();
      r.selectNodeContents(ed);
      const sel = window.getSelection();
      sel.removeAllRanges();
      sel.addRange(r);
      return;
    }
    if (k === ',' || k === '.') {
      e.preventDefault();
      showPage(currentPage === 'settings' ? 'editor' : 'settings');
      return;
    }
  });
  // Ctrl+wheel for zoom
  window.addEventListener('wheel', (e) => {
    if (e.ctrlKey || e.metaKey) {
      e.preventDefault();
      zoomBy(e.deltaY < 0 ? +10 : -10);
    }
  }, { passive:false });
  // Pinch zoom block
  window.addEventListener('gesturestart', e => e.preventDefault());
  document.addEventListener('contextmenu', e => e.preventDefault());
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
