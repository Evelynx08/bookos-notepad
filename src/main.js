// BookOS Notepad — frontend

// El idioma y el tema se eligen en Ajustes de BookOS, no por ventana.
for (const id of ["theme-btn", "lang-btn"]) {
  const control = document.getElementById(id);
  if (control) control.hidden = true;
}
for (const id of ["ui-lang", "seg-theme"]) {
  const control = document.getElementById(id);
  control?.closest(".detail-item")?.remove();
}
const selectorTema = document.getElementById("themePicker")?.closest(".settings-row");
if (selectorTema) selectorTema.hidden = true;

const invoke = (cmd, args) => window.__TAURI__.core.invoke(cmd, args);
const tauriWin = () => window.__TAURI__.window.getCurrentWindow();
const $ = (s, r=document) => r.querySelector(s);

// Selector de archivos: el explorador de BookOS (AI-DESIGN-SYSTEM §9). Toma las
// opciones de `dialog.open/save` de Tauri para que, si el explorador no está
// instalado, pasen tal cual al diálogo de siempre.
async function elegirRuta(modo, opciones = {}) {
  const filtros = opciones.filters || [];
  const exts = filtros.flatMap(f => f.extensions);
  // Un filtro «*» es «todo»; el explorador solo aplica uno, la unión del resto.
  const patrones = exts.includes('*') ? [] : exts.map(e => '*.' + e);
  const ruta = opciones.defaultPath || '';
  const corte = ruta.lastIndexOf('/');
  try {
    const rutas = await window.__TAURI__.core.invoke('elegir_ruta', { eleccion: {
      modo: opciones.directory ? 'folder' : modo,
      multiple: !!opciones.multiple,
      patrones,
      nombreFiltro: filtros.length === 1 ? filtros[0].name : undefined,
      titulo: opciones.title,
      nombre: modo === 'save' && ruta ? ruta.slice(corte + 1) : undefined,
      inicio: corte > 0 ? ruta.slice(0, corte) : undefined,
    } });
    if (!rutas) return null;
    return opciones.multiple ? rutas : rutas[0];
  } catch (e) {
    if (e !== 'sin-explorador') throw e;
    return window.__TAURI__.dialog[modo](opciones);
  }
}

const EXT_MAP = {
  // shell
  sh:'sh', bash:'sh', zsh:'sh', fish:'sh', ksh:'sh', csh:'sh', tcsh:'sh',
  // web markup
  html:'html', htm:'html', xhtml:'html', xml:'html', svg:'html', vue:'html', astro:'html', svelte:'html',
  // styles
  css:'css', scss:'css', sass:'css', less:'css', styl:'css',
  // markdown / docs
  md:'md', markdown:'md', mdx:'md', rst:'md', adoc:'md', asciidoc:'md', tex:'md',
  // config
  cfg:'cfg', conf:'cfg', ini:'cfg', toml:'cfg', yaml:'cfg', yml:'cfg',
  desktop:'cfg', service:'cfg', env:'cfg', editorconfig:'cfg',
  gitignore:'cfg', gitattributes:'cfg', dockerignore:'cfg',
  // data
  json:'json', jsonc:'json', json5:'json',
  csv:'json', tsv:'json',
  // python
  py:'py', pyw:'py', pyi:'py',
  // javascript family
  js:'js', mjs:'js', cjs:'js', ts:'js', tsx:'js', jsx:'js',
  // rust
  rs:'rs',
  // c-family
  c:'rs', h:'rs', cpp:'rs', cc:'rs', cxx:'rs', hpp:'rs', hh:'rs',
  // other languages — fall back to nearest accent visually
  go:'rs', java:'rs', kt:'rs', swift:'rs',
  rb:'py', php:'py', pl:'py', lua:'py', r:'py',
  sql:'json', graphql:'json', gql:'json',
  // logs / plain
  log:'txt', txt:'txt'
};
const LANG_LABEL = {
  sh:'Shell', html:'HTML', css:'CSS', md:'Markdown', cfg:'Config',
  json:'JSON', py:'Python', js:'JavaScript', rs:'Código',
  txt:'Texto'
};
// Override exact label by extension when needed (else uses EXT_MAP group label).
const LANG_LABEL_BY_EXT = {
  c:'C', cpp:'C++', cc:'C++', cxx:'C++', h:'C', hpp:'C++', hh:'C++',
  go:'Go', java:'Java', kt:'Kotlin', swift:'Swift',
  rb:'Ruby', php:'PHP', pl:'Perl', lua:'Lua', r:'R',
  sql:'SQL', graphql:'GraphQL', gql:'GraphQL',
  vue:'Vue', astro:'Astro', svelte:'Svelte',
  scss:'SCSS', sass:'Sass', less:'Less', styl:'Stylus',
  toml:'TOML', yaml:'YAML', yml:'YAML', ini:'INI',
  ts:'TypeScript', tsx:'TSX', jsx:'JSX', mjs:'JavaScript',
  rst:'reST', adoc:'AsciiDoc', tex:'LaTeX', mdx:'MDX',
  log:'Log', csv:'CSV', tsv:'TSV', xml:'XML', svg:'SVG',
  desktop:'Desktop', service:'Systemd', env:'.env',
  gitignore:'Git', gitattributes:'Git',
  pyw:'Python', pyi:'Python'
};

const DEFAULT_SETTINGS = {
  font: 'sans',        // sans|serif|mono
  wrap: true,
  showStatus: true,
  fileAccent: true,
  zoom: 100,           // percent
  previewMode: 'auto', // auto|html|md|url
  previewUrl: '',
  adblockEnabled: true,
  youtubeNoCookie: true,
  youtubeDirect: false,     // try youtube.com/watch directly (skip /embed/)
  shortcuts: {},            // user overrides: { actionId: "Ctrl+Alt+B" }
  uiLang: 'auto',
  restoreSession: false,    // reopen tabs on launch
  session: [],              // [{path, encoding}] persisted tabs with on-disk path
  autoSave: false,          // auto-save tabs that have an on-disk path
  autoSaveDelay: 2000,      // debounce ms after last edit
  listaOculta: false        // lista de notas plegada
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
  state.theme = 'auto';
  state.settings = { ...DEFAULT_SETTINGS, ...(state.settings||{}) };
  state.settings.uiLang = await invoke('bookos_language').catch(() => 'es');
  if (window.BookosI18n) BookosI18n.setLang(state.settings.uiLang);
}
async function saveState() { try { await invoke('save_state', { state }); } catch(e){console.error(e);} }

// ───────── SESSION ─────────
let _sessionTimer = null;
function saveSessionDebounced(){
  clearTimeout(_sessionTimer);
  _sessionTimer = setTimeout(saveSession, 600);
}
function saveSession(){
  if (!state.settings.restoreSession) return;
  state.settings.session = tabs
    .filter(t => t.path && !t.nota)    // only persisted files
    .map(t => ({ path: t.path, encoding: t.encoding || 'utf-8' }));
  saveState();
}
async function restoreSession(){
  if (!state.settings.restoreSession) return false;
  const sess = state.settings.session || [];
  if (!sess.length) return false;
  let opened = 0;
  for (const s of sess) {
    try {
      const exists = await invoke('plugin:fs|exists', { path: s.path }).catch(() => true);
      if (exists === false) continue;
      const r = await invoke('read_file', { path: s.path, encoding: s.encoding || null });
      const name = s.path.split(/[\\/]/).pop();
      const plain = !isRichExt(rawExtOf(name));
      const tab = newTab({ name, path: s.path, content: r.text, plain });
      if (tab) { tab.encoding = r.encoding || s.encoding || 'utf-8'; tab.eol = r.eol || 'lf'; tab.mtime = r.mtime || 0; }
      opened++;
    } catch (e) { console.warn('restore skip', s.path, e); }
  }
  return opened > 0;
}

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
// raw lowercase extension as in filename (e.g. "tsx", "py", "scss")
function rawExtOf(name) {
  const m = name && name.match(/\.([A-Za-z0-9]+)$/);
  return m ? m[1].toLowerCase() : '';
}
// extension mapped to visual group (sh/html/css/md/cfg/json/py/js/rs/txt) for accent/badge
function extOf(name) {
  const r = rawExtOf(name);
  if (!r) return 'txt';
  return EXT_MAP[r] || 'txt';
}
function labelForExt(rawExt, groupExt) {
  return LANG_LABEL_BY_EXT[rawExt] || LANG_LABEL[groupExt] || (rawExt ? rawExt.toUpperCase() : 'Texto');
}

// Only HTML files round-trip rich formatting (bold/italic) as real tags.
// Everything else (.txt, .md, code, css…) is edited and saved as plain text.
function isRichExt(rawExt) {
  const e = (rawExt || '').toLowerCase();
  return e === 'html' || e === 'htm' || e === 'xhtml';
}

function newTab({name='Sin título', path=null, content='', plain=true} = {}) {
  const id = 'tab-' + (tabSeq++);
  const rawExt = rawExtOf(path || name);
  const ext = extOf(path || name);
  const rich = isRichExt(rawExt);
  // Plain-text files keep their content escaped into line blocks; only HTML
  // tabs hold raw markup. `plain=false` means content already is HTML.
  const html = (plain || !rich) ? escapeToHtml(content) : content;
  // Large files: keep editing snappy by skipping live-preview re-render on every
  // keystroke (it would flatten the whole buffer each time). ~4MB threshold.
  const big = (content ? content.length : 0) > 4_000_000;
  const tab = { id, name, path, ext, rawExt, rich, big, content: html, dirty: false, encoding: 'utf-8' };
  tabs.push(tab);
  setActive(id);
  renderTabs();
  return tab;
}

// Plain-text → editor HTML. We deliberately do NOT wrap each line in its own
// <div>: a large file (e.g. a multi-MB .log) would create hundreds of thousands
// of block elements and freeze the webview on load. Instead we keep the text as
// one escaped blob and let CSS `white-space:pre-wrap` render the newlines.
// Also normalizes CRLF / lone CR → LF (a stray '\r' under pre-wrap rendered as
// an extra blank line — the old "extra spacing" on Windows-saved files).
// htmlToPlain() reads it back correctly whether it's this blob or the mixed
// text + <div> nodes WebKit produces once the user starts editing.
function escapeToHtml(text) {
  if (!text) return '';
  return text
    .replace(/\r\n?/g, '\n')
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
}

// Flatten editor HTML to plain text. Walks the tree so it stays correct for ALL
// shapes the editor can hold: the single escaped blob (text node with literal
// \n), the mixed text + <div> nodes WebKit creates while editing, and real rich
// HTML. A block element forces a newline before its content (when the current
// line isn't empty) and before whatever follows it; <br> is a hard newline.
const _BLOCK_TAGS = new Set([
  'DIV','P','LI','H1','H2','H3','H4','H5','H6','PRE','BLOCKQUOTE',
  'SECTION','ARTICLE','HEADER','FOOTER','NAV','ASIDE','FIGURE','FIGCAPTION',
  'UL','OL','TABLE','TR','HR'
]);
function htmlToPlain(html) {
  const tmp = document.createElement('div');
  tmp.innerHTML = html;
  let out = '';
  let pendingNL = false; // a block just closed — next content opens a new line
  const walk = (node) => {
    for (const child of node.childNodes) {
      if (child.nodeType === 3) {
        const v = child.nodeValue;
        if (!v) continue;
        if (pendingNL && out && !out.endsWith('\n')) out += '\n';
        pendingNL = false;
        out += v;
      } else if (child.nodeType === 1) {
        const tag = child.tagName;
        if (tag === 'BR') { out += '\n'; pendingNL = false; continue; }
        if (_BLOCK_TAGS.has(tag)) {
          if (out && !out.endsWith('\n')) out += '\n';
          pendingNL = false;
          walk(child);
          pendingNL = true;
        } else {
          walk(child); // inline element — pass through
        }
      }
    }
  };
  walk(tmp);
  return out.replace(/\r/g, '');
}

function getTab(id) { return tabs.find(t => t.id === id); }

function setActive(id) {
  if (id === activeId) return;
  quitarMarcoImagen();
  // Persist current editor content into its tab before switching
  saveActiveContent();
  const anterior = getTab(activeId);
  if (anterior?.nota) soltarNota(anterior);
  prevId = activeId;
  activeId = id;
  renderEditor();
  renderTabs();
  enviarVistaPrevia();
}

function saveActiveContent() {
  if (!activeId) return;
  const t = getTab(activeId);
  if (!t) return;
  // Strip find highlights so injected <mark> never persists into saved content.
  if (typeof clearHighlights === 'function') clearHighlights();
  const ed = $('.editor-area');
  if (ed) {
    t.content = ed.innerHTML;
    if (t.nota && currentPage === 'editor') guardarVistaNota(t, ed);
  }
}

function guardarVistaNota(t, ed) {
  const sel = window.getSelection();
  const indiceHTML = (padre, hasta) => {
    let indice = 0, anteriorTexto = false;
    for (const hijo of padre.childNodes) {
      const texto = hijo.nodeType === 3;
      if (hijo === hasta) return indice - (texto && anteriorTexto ? 1 : 0);
      if (!texto || !anteriorTexto) indice++;
      anteriorTexto = texto;
    }
    return indice;
  };
  const punto = (nodo, offset) => {
    const rango = document.createRange();
    rango.selectNodeContents(ed);
    rango.setEnd(nodo, offset);
    const texto = rango.toString().length;
    if (nodo.nodeType !== 3) {
      const hijo = nodo.childNodes[offset];
      if (hijo?.nodeType === 3 && hijo.previousSibling?.nodeType === 3) {
        nodo = hijo;
        offset = 0;
      } else offset = indiceHTML(nodo, hijo);
    }
    // innerHTML une los textos contiguos. Se guarda su posición en ese
    // árbol serializado, incluidas las líneas vacías y los límites de imagen.
    while (nodo.nodeType === 3 && nodo.previousSibling?.nodeType === 3) {
      nodo = nodo.previousSibling;
      offset += nodo.length;
    }
    const camino = [];
    while (nodo !== ed) {
      camino.unshift(indiceHTML(nodo.parentNode, nodo));
      nodo = nodo.parentNode;
    }
    return { camino, offset, texto };
  };
  const seleccion = ed.contains(sel.anchorNode) && ed.contains(sel.focusNode)
    ? { inicio: punto(sel.anchorNode, sel.anchorOffset), fin: punto(sel.focusNode, sel.focusOffset) }
    : t.vista?.seleccion;
  t.vista = { arriba: ed.scrollTop, izquierda: ed.scrollLeft, seleccion };
}

function restaurarVistaNota(t, ed) {
  const vista = t.vista;
  if (!vista) return;
  const nodo = (punto) => punto.camino.reduce((padre, indice) => padre?.childNodes[indice], ed);
  if (vista.seleccion) {
    const { inicio, fin } = vista.seleccion;
    const a = nodo(inicio), b = nodo(fin);
    // La serialización puede unir nodos de texto contiguos: no se reutilizan
    // posiciones que ya no existen en el HTML reconstruido.
    if (a && b && inicio.offset <= (a.nodeType === 3 ? a.length : a.childNodes.length)
      && fin.offset <= (b.nodeType === 3 ? b.length : b.childNodes.length)) {
      window.getSelection().setBaseAndExtent(a, inicio.offset, b, fin.offset);
    } else restoreCaretOffset(ed, fin.texto);
  }
  let interactuando = false;
  for (const evento of ['pointerdown', 'wheel', 'keydown']) {
    ed.addEventListener(evento, () => { interactuando = true; }, { once: true, passive: true });
  }
  const desplazar = () => {
    if (interactuando || !ed.isConnected || activeId !== t.id) return;
    ed.scrollTop = vista.arriba;
    ed.scrollLeft = vista.izquierda;
  };
  desplazar();
  for (const img of ed.querySelectorAll('img')) {
    if (!img.complete) img.addEventListener('load', desplazar, { once: true });
  }
}

function closeTab(id) {
  const t = getTab(id);
  // Una nota no se cierra: se deja de mirar al elegir otra, o se elimina.
  if (!t || t.nota) return;
  if (t.dirty) {
    const ok = confirm(`"${t.name}" tiene cambios sin guardar. ¿Cerrar de todos modos?`);
    if (!ok) return;
  }
  const pos = tabs.filter(x => !x.nota).indexOf(t);
  tabs.splice(tabs.indexOf(t), 1);
  if (activeId === id) {
    activeId = null;
    const archivos = tabs.filter(x => !x.nota);
    const next = archivos[pos] || archivos[pos-1] || notas()[0];
    if (next) setActive(next.id);
    else { nuevaNota(); return; }
  }
  if (prevId === id) prevId = null;
  renderTabs();
  saveSessionDebounced();
}

function renderTabs() {
  const row = $('#tabs-row');
  row.innerHTML = '';
  for (const t of tabs.filter(t => !t.nota)) {
    const el = document.createElement('div');
    el.className = 'tab' + (t.id === activeId ? ' active' : '') + (t.dirty ? ' dirty' : '');
    el.dataset.id = t.id;
    el.title = t.path || t.name;
    el.innerHTML = `
      <span class="tab-dot"></span>
      <span class="tab-name">${escapeHtml(t.name)}</span>
      <button class="tab-close" title="Cerrar" aria-label="Cerrar pestaña"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M18 6 6 18M6 6l12 12"/></svg></button>`;
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
  pintarLista();
  updateTitle();
}

function escapeHtml(s) {
  return (s||'').replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;');
}

function updateTitle() {
  const t = activeTab();
  const title = t ? (t.dirty ? '● ' : '') + t.name + ' — Bloc de notas' : 'Bloc de notas';
  try { tauriWin().setTitle(title); } catch {}
  $('#tb-title').textContent = currentPage === 'settings' ? tr('settings')
    : !t ? tr('app.title') : t.nota ? tr('notas.titulo') : t.name;
}

function activeTab() { return getTab(activeId); }

// ───────── EDITOR ─────────
function renderEditor() {
  quitarMarcoImagen();
  const host = $('#editor-host');
  host.innerHTML = '';
  const t = activeTab();
  if (!t) return;
  host.className = 'editor-host';
  $('#mc').classList.toggle('modo-nota', !!t.nota);
  if (t.nota) {
    const cabecera = document.createElement('div');
    cabecera.className = 'nota-cabecera';
    const fecha = document.createElement('div');
    fecha.className = 'nota-fecha';
    fecha.textContent = fechaLarga(t.mtime);
    const estado = document.createElement('span');
    estado.className = 'nota-estado';
    estado.setAttribute('role', 'status');
    estado.setAttribute('aria-live', 'polite');
    const guardado = document.createElement('button');
    guardado.className = 'nota-guardado';
    guardado.type = 'button';
    guardado.addEventListener('click', () => guardarNota(t));
    estado.appendChild(guardado);
    cabecera.append(fecha, estado);
    host.appendChild(cabecera);
  } else {
    // Apply ext accent
    if (state.settings.fileAccent) host.classList.add('ext-' + t.ext);

    const badge = document.createElement('div');
    badge.className = 'ext-badge ext-' + t.ext;
    badge.textContent = labelForExt(t.rawExt, t.ext);
    host.appendChild(badge);
  }

  const area = document.createElement('div');
  area.className = 'editor-area font-' + state.settings.font + ' ' + (t.nota ? 'nota wrap' : state.settings.wrap ? 'wrap' : 'nowrap');
  area.contentEditable = 'true';
  area.spellcheck = false;
  area.dataset.placeholder = t.nota ? tr('notas.nueva') : 'Empieza a escribir…';
  area.style.fontSize = (15 * state.settings.zoom / 100) + 'px';
  area.innerHTML = t.content || '';
  area.addEventListener('input', onEditorInput);
  area.addEventListener('beforeinput', onEditorBeforeInput);
  area.addEventListener('keydown', onEditorKeydown);
  area.addEventListener('keyup', scheduleCaretStatus);
  area.addEventListener('click', scheduleCaretStatus);
  area.addEventListener('paste', pegarImagen);
  if (t.nota) area.addEventListener('click', alternarTarea);
  area.addEventListener('click', seleccionarImagen);
  area.addEventListener('scroll', () => marcoImagen?.colocar());
  area.addEventListener('keydown', (e) => {
    if (!['Tab', 'Control', 'Shift', 'Alt', 'Meta'].includes(e.key)) quitarMarcoImagen();
  });
  host.appendChild(area);
  area.focus({ preventScroll: true });
  if (t.nota) {
    asegurarTitulo(area);
    area.classList.toggle('vacia', notaVacia(t));
    restaurarVistaNota(t, area);
    actualizarGuardado(t);
  }
  updateStatus();
}

// Recomputing char/line counts means flattening the whole document, which is
// O(file size). Debounce it so holding a key in a multi-MB file doesn't re-scan
// the entire buffer on every keystroke — the dirty marker stays instant.
let _statusTimer = null;
function scheduleStatus() {
  clearTimeout(_statusTimer);
  _statusTimer = setTimeout(() => {
    updateStatus();
    if ($('#find-bar')?.classList.contains('open')) updateFindCount();
  }, 100);
}

function onEditorInput(e) {
  if (marcoImagen && !marcoImagen.img.isConnected) quitarMarcoImagen();
  const t = activeTab();
  if (!t) return;
  if (t.nota) { notaEditada(t, e); scheduleStatus(); return; }
  if (!t.dirty) { t.dirty = true; renderTabs(); }
  scheduleStatus();
  schedulePreview();
  scheduleAutoSave();
}

// ───────── VISTA PREVIA ─────────
// Vive en una ventana hija (preview.html), al estilo de Quick Look: aquí solo
// se le manda el documento. Los ajustes siguen teniendo un único dueño, esta
// ventana: la vista previa los recibe con cada envío y devuelve sus cambios
// por `vista-previa-ajuste`, en vez de escribir ella también en state.json.
let vistaAbierta = false;
let previewTimer = null;

async function enviarVistaPrevia() {
  const t = activeTab();
  if (!vistaAbierta || !t || t.nota) return;
  const ed = $('.editor-area');
  const { previewMode, previewUrl, adblockEnabled, youtubeNoCookie, youtubeDirect } = state.settings;
  try {
    await invoke('vista_previa', { doc: {
      nombre: t.name,
      ext: t.ext,
      // Un .html se edita como marcado real; el resto, como texto.
      texto: t.rich ? ed.innerHTML : htmlToPlain(ed.innerHTML),
      ajustes: { previewMode, previewUrl, adblockEnabled, youtubeNoCookie, youtubeDirect },
    } });
  } catch (e) {
    vistaCerrada();
    toast(tr('vp.error', { motivo: e }));
  }
}

function schedulePreview() {
  if (!vistaAbierta) return;
  // Large files: don't auto-rebuild the preview while typing (use Refresh).
  const t = activeTab();
  if (t && t.big) return;
  clearTimeout(previewTimer);
  previewTimer = setTimeout(enviarVistaPrevia, 350);
}

function togglePreview() {
  if (vistaAbierta) { invoke('vista_previa_cerrar'); return; }
  const t = activeTab();
  if (!t || t.nota) return;
  vistaAbierta = true;
  $('#preview-btn').classList.add('active');
  enviarVistaPrevia();
}

function vistaCerrada() {
  vistaAbierta = false;
  $('#preview-btn').classList.remove('active');
}

function wireVistaPrevia() {
  const ev = window.__TAURI__.event;
  ev.listen('vista-previa-cerrada', vistaCerrada);
  ev.listen('vista-previa-pide', enviarVistaPrevia);
  ev.listen('vista-previa-ajuste', (e) => {
    const [clave, valor] = e.payload;
    state.settings[clave] = valor;
    saveState();
    renderSettings();
  });
  // Sync Rust side with persisted state
  invoke('adblock_set_enabled', { enabled: !!state.settings.adblockEnabled }).catch(()=>{});
}

// Code-like extensions that get auto-close brackets/tags
const CODE_EXTS = new Set(['html','css','js','py','rs','sh','cfg','json']);

function isCodeFile() {
  const t = activeTab();
  return t && CODE_EXTS.has(t.ext);
}

// Insert text at caret and place caret at offset from the end.
// Uses execCommand('insertText') so the operation is part of the WebKit
// undo stack — Ctrl+Z reverts the full pair as one step.
function insertAtCaret(text, caretOffsetFromEnd = 0) {
  // Insert full text via the document command (undo-friendly)
  document.execCommand('insertText', false, text);
  if (caretOffsetFromEnd <= 0) return;
  // Move caret back by `caretOffsetFromEnd` chars
  const sel = window.getSelection();
  if (!sel || !sel.rangeCount) return;
  const range = sel.getRangeAt(0);
  // Walk back caretOffsetFromEnd characters
  let remaining = caretOffsetFromEnd;
  while (remaining > 0) {
    const node = range.startContainer;
    if (node.nodeType === 3 && range.startOffset > 0) {
      const take = Math.min(remaining, range.startOffset);
      range.setStart(node, range.startOffset - take);
      range.collapse(true);
      remaining -= take;
    } else {
      // Outside text node, give up
      break;
    }
  }
  sel.removeAllRanges();
  sel.addRange(range);
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

// Layout-agnostic detection of `>` insertion via beforeinput — captures the
// actual character about to be inserted regardless of keyboard layout.
function onEditorBeforeInput(e) {
  if (e.inputType !== 'insertText' || e.data !== '>') return;
  const t = activeTab();
  if (!t || t.ext !== 'html') return;
  const before = textBeforeCaret();
  const m = before.match(/<([A-Za-z][A-Za-z0-9-]*)(?:\s[^<>]*)?$/);
  if (!m) return;
  const tag = m[1].toLowerCase();
  if (VOID_TAGS.has(tag)) return;
  if (/\/\s*$/.test(before)) return;
  e.preventDefault();
  insertAtCaret('></' + m[1] + '>', m[1].length + 3);
  onEditorInput();
}

function onEditorKeydown(e) {
  if (e.key === 'Tab' && marcoImagen) return;
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

  // Auto-close HTML tag when user types `>` (handled in input event below for layout-agnostic detection)

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

// ── Line operations (move / duplicate) on the plain-text model ──
// Returns {lines, li, col} for the line the caret sits on.
function caretLineInfo(ed) {
  const off = saveCaretOffset(ed);
  const text = htmlToPlain(ed.innerHTML);
  const lines = text.split('\n');
  if (off == null) return { lines, li: lines.length - 1, col: 0, off: text.length };
  let acc = 0, li = 0;
  for (; li < lines.length; li++) {
    if (acc + lines[li].length >= off) break;
    acc += lines[li].length + 1; // +1 for the '\n'
  }
  return { lines, li: Math.min(li, lines.length - 1), col: off - acc, off };
}

// Replace the entire editor buffer with `text`. For normal files we do it via
// execCommand so the change joins the native undo stack — Ctrl+Z reverts a
// move/duplicate/replace-all in one step (a raw `innerHTML =` wipes undo
// history). Huge files keep the cheap innerHTML path: one O(n) reinsertion
// there isn't worth re-exploding the DOM into per-line blocks.
function replaceEditorContent(ed, text) {
  const t = activeTab();
  if (t && !t.rich && !t.big) {
    ed.focus();
    const sel = window.getSelection();
    const range = document.createRange();
    range.selectNodeContents(ed);
    sel.removeAllRanges();
    sel.addRange(range);
    document.execCommand('insertText', false, text);
  } else {
    ed.innerHTML = (t && t.rich) ? text : escapeToHtml(text);
  }
}

function setEditorText(ed, lines, caretOffset) {
  replaceEditorContent(ed, lines.join('\n'));
  if (caretOffset != null) restoreCaretOffset(ed, caretOffset);
  onEditorInput();
}

function moveLine(dir) {
  const ed = $('.editor-area'); if (!ed) return;
  const { lines, li, col } = caretLineInfo(ed);
  const j = li + dir;
  if (j < 0 || j >= lines.length) return;
  [lines[li], lines[j]] = [lines[j], lines[li]];
  // New caret offset = start of moved line + original column.
  let acc = 0;
  for (let k = 0; k < j; k++) acc += lines[k].length + 1;
  setEditorText(ed, lines, acc + Math.min(col, lines[j].length));
}

function duplicateLine() {
  const ed = $('.editor-area'); if (!ed) return;
  const { lines, li, col } = caretLineInfo(ed);
  lines.splice(li + 1, 0, lines[li]);
  // Keep caret on the duplicate (line below), same column.
  let acc = 0;
  for (let k = 0; k <= li; k++) acc += lines[k].length + 1;
  setEditorText(ed, lines, acc + Math.min(col, lines[li].length));
}

function updateStatus() {
  const t = activeTab();
  const sp = $('#status-path');
  const sl = $('#status-lang');
  const sc = $('#status-chars');
  const sln = $('#status-lines');
  const se = $('#status-encoding');
  const seol = $('#status-eol');
  if (!t) {
    sp.textContent='—'; sc.textContent='0 caracteres'; sln.textContent='1 línea';
    if (se) se.textContent='UTF-8';
    if (seol) seol.textContent='LF';
    return;
  }
  sp.textContent = t.path || t.name;
  sl.textContent = labelForExt(t.rawExt, t.ext);
  const ed = $('.editor-area');
  const text = ed ? htmlToPlain(ed.innerHTML) : '';
  sc.textContent = text.length + ' caracteres';
  sln.textContent = (text.split('\n').length) + ' líneas';
  const sw = $('#status-words');
  if (sw) {
    const words = (text.trim().match(/\S+/g) || []).length;
    sw.textContent = words + (words === 1 ? ' palabra' : ' palabras');
  }
  updateCaretStatus();
  if (se) se.textContent = (t.encoding || 'utf-8').toUpperCase().replace('UTF-8-BOM','UTF-8 BOM');
  if (seol) seol.textContent = (t.eol || 'lf').toUpperCase();
  $('#statusbar').classList.toggle('hidden', !state.settings.showStatus);
}

// Compute Ln/Col of the caret inside the contenteditable editor.
function updateCaretStatus() {
  const el = $('#status-caret');
  if (!el) return;
  const ed = $('.editor-area');
  const sel = window.getSelection();
  if (!ed || !sel || !sel.rangeCount || !ed.contains(sel.anchorNode)) {
    el.textContent = 'Ln 1, Col 1'; return;
  }
  // Text from editor start up to the caret, flattened to plain text.
  const r = sel.getRangeAt(0).cloneRange();
  const pre = r.cloneRange();
  pre.selectNodeContents(ed);
  pre.setEnd(r.endContainer, r.endOffset);
  const frag = document.createElement('div');
  frag.appendChild(pre.cloneContents());
  const before = htmlToPlain(frag.innerHTML);
  const lines = before.split('\n');
  el.textContent = 'Ln ' + lines.length + ', Col ' + (lines[lines.length - 1].length + 1);
}

// Caret readout walks all content before the cursor — O(offset). Debounce it so
// repeated keyup/click events in a large file don't each pay that cost.
let _caretTimer = null;
function scheduleCaretStatus() {
  clearTimeout(_caretTimer);
  _caretTimer = setTimeout(updateCaretStatus, 60);
}

const ENCODINGS = ['utf-8','utf-8-bom','utf-16le','utf-16be','latin-1','cp1252'];
function cycleEncoding() {
  const t = activeTab();
  if (!t) return;
  const cur = t.encoding || 'utf-8';
  const idx = ENCODINGS.indexOf(cur);
  const next = ENCODINGS[(idx + 1) % ENCODINGS.length];
  t.encoding = next;
  // If file has path on disk, optionally reload to interpret with new encoding
  if (t.path && !t.dirty && confirm('Recargar archivo con codificación ' + next.toUpperCase() + '?')) {
    invoke('read_file', { path: t.path, encoding: next }).then(r => {
      const ed = $('.editor-area');
      if (ed) ed.innerHTML = escapeToHtml(r.text);
      t.content = ed ? ed.innerHTML : escapeToHtml(r.text);
      t.encoding = r.encoding || next;
      t.eol = r.eol || 'lf';
      t.mtime = r.mtime || 0;
      updateStatus();
      toast('Recargado: ' + t.encoding);
    }).catch(e => toast('Error: ' + e));
  } else {
    updateStatus();
    toast('Codificación al guardar: ' + next.toUpperCase());
  }
}

// ───────── FORMAT ─────────
function exec(cmd, val=null) {
  const t = activeTab();
  // Rich formatting only applies to HTML tabs — for plain text it would inject
  // <b>/<i> tags that get stripped on save, so ignore it.
  if (t && !t.rich) { toast('Formato solo disponible en archivos HTML'); return; }
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
    // A la izquierda es lo normal: encendido siempre, parecía una opción elegida.
    if (cmd === 'justifyLeft') on = false;
    // El título de una nota es negrita por estilo, no porque se haya pulsado B.
    if (cmd === 'bold' && on) {
      const titulo = document.querySelector('.editor-area.nota > :first-child');
      if (titulo?.contains(window.getSelection().anchorNode)) on = false;
    }
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
    const sel = await elegirRuta('open', {
      multiple: true,
      filters: [
        { name: 'Todos los archivos', extensions: ['*'] },
        { name: 'Texto y notas', extensions: ['txt','md','markdown','log','rst','tex'] },
        { name: 'Código', extensions: ['sh','bash','zsh','fish','py','js','mjs','cjs','ts','tsx','jsx','rs','c','cpp','cc','h','hpp','java','go','rb','php','lua','swift','kt','dart','rb','pl','r','jl','vim','el','clj'] },
        { name: 'Web', extensions: ['html','htm','xhtml','css','scss','sass','less','xml','svg','json','jsonc','yaml','yml','vue','astro'] },
        { name: 'Config', extensions: ['cfg','conf','ini','toml','yaml','yml','desktop','env','gitignore','gitattributes','editorconfig'] },
        { name: 'Datos', extensions: ['csv','tsv','sql','log','xml'] }
      ]
    });
    if (!sel) return;
    const paths = Array.isArray(sel) ? sel : [sel];
    for (const p of paths) {
      const r = await invoke('read_file', { path: p });
      const name = p.split(/[\\/]/).pop();
      const tab = newTab({ name, path: p, content: r.text, plain: true });
      if (tab) { tab.encoding = r.encoding || 'utf-8'; tab.eol = r.eol || 'lf'; tab.mtime = r.mtime || 0; }
    }
    toast(`Abierto: ${paths.length} archivo${paths.length>1?'s':''}`);
  } catch (e) {
    console.error('openFile error:', e);
    const msg = (e && e.message) || (typeof e === 'string' ? e : JSON.stringify(e));
    toast('Error: ' + msg.slice(0, 120));
  }
}

async function saveFile(forceAs=false) {
  saveActiveContent();
  const t = activeTab();
  if (!t) return;
  if (t.nota && !forceAs) { await guardarNota(t); return; }
  let path = t.path;
  if (!path || forceAs) {
    try {
      path = await elegirRuta('save', {
        defaultPath: t.nota ? t.name.replace(/[\\/]/g, '-') + '.html' : t.name,
        filters: [
          { name: 'Texto', extensions: ['txt'] },
          { name: 'Markdown', extensions: ['md'] },
          { name: 'HTML', extensions: ['html'] }
        ]
      });
    } catch (e) { console.error('elegirRuta save failed', e); toast('Error diálogo: ' + e); return; }
    if (!path) return;
  }
  // Don't blindly clobber a file that changed on disk since we loaded it.
  if (t.path && path === t.path && await diskChanged(t)) {
    if (!confirm(`"${t.name}" cambió en disco desde que lo abriste. ¿Sobrescribir con tu versión?`)) return;
  }
  const savedRawExt = rawExtOf(path.split(/[\\/]/).pop());
  const ed = $('.editor-area');
  // Save raw HTML only when the target file is an HTML file; otherwise flatten.
  const payload = isRichExt(savedRawExt) ? ed.innerHTML : htmlToPlain(ed.innerHTML);
  try {
    const mtime = await invoke('write_file', { path, contents: payload, encoding: t.encoding || 'utf-8', eol: t.eol || 'lf' });
    // «Guardar como» en una nota saca una copia; la nota sigue en su carpeta.
    if (t.nota) { toast(tr('notas.exportada', { nombre: path.split(/[\\/]/).pop() })); return; }
    const extChanged = t.rawExt !== savedRawExt;
    t.path = path;
    t.name = path.split(/[\\/]/).pop();
    t.rawExt = savedRawExt;
    t.ext = extOf(t.name);
    t.rich = isRichExt(t.rawExt);
    t.dirty = false;
    t.mtime = mtime || 0;
    renderTabs();
    if (extChanged) renderEditor();   // re-render only if file type changed
    saveSessionDebounced();
    toast('Guardado: ' + t.name);
  } catch (e) {
    console.error('write_file failed', e);
    toast('Error al guardar: ' + e);
  }
}

// True if the file on disk has a newer mtime than when this tab last read/wrote
// it — i.e. some other program changed it underneath us.
async function diskChanged(t) {
  if (!t || !t.path || !t.mtime) return false;
  try {
    const m = await invoke('file_meta', { path: t.path });
    return !!(m && m.exists && m.mtime > t.mtime + 0.001);
  } catch { return false; }
}

// Re-read a tab's file from disk, replacing its content.
async function reloadTab(t) {
  try {
    const r = await invoke('read_file', { path: t.path, encoding: t.encoding || null });
    const html = t.rich ? r.text : escapeToHtml(r.text);
    if (activeId === t.id) { const ed = $('.editor-area'); if (ed) ed.innerHTML = html; }
    t.content = html;
    t.encoding = r.encoding || t.encoding || 'utf-8';
    t.eol = r.eol || 'lf';
    t.mtime = r.mtime || 0;
    t.dirty = false;
    renderTabs();
    if (activeId === t.id) updateStatus();
    toast('Recargado: ' + t.name);
  } catch (e) { toast('Error al recargar: ' + e); }
}

// Watch for external edits: when the window regains focus, check whether the
// active tab's file changed on disk and offer to reload (or just warn if the
// user has unsaved edits — we never discard their work automatically).
function wireFileWatch() {
  const ev = window.__TAURI__ && window.__TAURI__.event;
  if (!ev || !ev.listen) return;
  // Files from a second instance ("Open With" while already running) arrive here.
  ev.listen('open-files', async (event) => {
    const paths = (event && event.payload) || [];
    if (!Array.isArray(paths) || !paths.length) return;
    if (tabs.length === 1 && !tabs[0].path && !tabs[0].dirty) {
      const ed = $('.editor-area');
      const empty = !ed || htmlToPlain(ed.innerHTML).trim() === '';
      if (empty) { tabs.splice(0, 1); activeId = null; }
    }
    let opened = 0;
    for (const p of paths) if (await openPathAsTab(p)) opened++;
    if (opened) { toast(`Abierto: ${opened} archivo${opened>1?'s':''}`); renderTabs(); saveSessionDebounced(); }
  }).catch(() => {});
  let busy = false;
  ev.listen('tauri://focus', async () => {
    if (busy) return;
    const t = activeTab();
    // La carpeta de notas solo la escribe esta app (instancia única).
    if (!t || !t.path || t.nota) return;
    busy = true;
    try {
      if (await diskChanged(t)) {
        if (t.dirty) {
          toast('Este archivo cambió en disco y tienes cambios sin guardar');
        } else if (confirm(`"${t.name}" cambió en disco. ¿Recargar la versión nueva?`)) {
          await reloadTab(t);
        } else {
          // User declined — adopt the new mtime so we stop asking.
          const m = await invoke('file_meta', { path: t.path }).catch(() => null);
          if (m && m.mtime) t.mtime = m.mtime;
        }
      }
    } finally { busy = false; }
  }).catch(() => {});
}

// Silent auto-save: only for tabs already backed by a file on disk.
let _autoSaveTimer = null;
function scheduleAutoSave() {
  if (!state.settings.autoSave) return;
  const t = activeTab();
  if (!t || !t.path || !t.dirty) return;
  clearTimeout(_autoSaveTimer);
  _autoSaveTimer = setTimeout(async () => {
    const cur = activeTab();
    if (!cur || !cur.path || !cur.dirty) return;
    // Never let a silent auto-save stomp on an external edit.
    if (await diskChanged(cur)) { toast('Auto-guardado en pausa: el archivo cambió en disco'); return; }
    saveActiveContent();
    const payload = cur.rich ? $('.editor-area').innerHTML : htmlToPlain($('.editor-area').innerHTML);
    try {
      const mtime = await invoke('write_file', { path: cur.path, contents: payload, encoding: cur.encoding || 'utf-8', eol: cur.eol || 'lf' });
      cur.dirty = false;
      cur.mtime = mtime || 0;
      renderTabs();
    } catch (e) { console.error('auto-save failed', e); }
  }, state.settings.autoSaveDelay || 2000);
}

// ───────── PAGES ─────────
function showPage(name) {
  const wasEditor = currentPage === 'editor';
  if (wasEditor && name !== 'editor') saveActiveContent();
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
  $('#mc').classList.toggle('en-ajustes', name === 'settings');
  updateTitle();
  $('#toolbar').style.display = name === 'editor' ? '' : 'none';
  if (name === 'settings') { renderSettings(); }
  else if (!wasEditor) { renderEditor(); }
}

// ───────── SETTINGS ─────────
function renderSettings() {
  setSeg('seg-theme', state.theme);
  setSeg('seg-font', state.settings.font);
  setToggle('tg-wrap', state.settings.wrap);
  setToggle('tg-status', state.settings.showStatus);
  setToggle('tg-fileaccent', state.settings.fileAccent);
  setToggle('tg-restore', state.settings.restoreSession);
  setToggle('tg-autosave', state.settings.autoSave);
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
  wireToggle('tg-restore', 'restoreSession', () => { if (!state.settings.restoreSession) { state.settings.session = []; saveState(); } });
  wireToggle('tg-autosave', 'autoSave', () => { if (state.settings.autoSave) scheduleAutoSave(); });
  const langSel = $('#ui-lang');
  if (langSel) {
    langSel.value = state.settings.uiLang || 'auto';
    langSel.addEventListener('change', () => {
      state.settings.uiLang = langSel.value;
      saveState();
      if (window.BookosI18n) BookosI18n.setLang(langSel.value);
    });
  }
  wireToggle('tg-adblock', 'adblockEnabled', () => {
    invoke('adblock_set_enabled', { enabled: state.settings.adblockEnabled }).catch(()=>{});
    enviarVistaPrevia();
  });
  wireToggle('tg-yt-nocookie', 'youtubeNoCookie', () => {
    enviarVistaPrevia();
  });
  wireToggle('tg-yt-direct', 'youtubeDirect', () => {
    enviarVistaPrevia();
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
  // Updates section
  const upCheck = document.getElementById('update-check');
  if (upCheck) upCheck.addEventListener('click', () => checkUpdates(true));
  const upRel = document.getElementById('update-open-release');
  if (upRel) upRel.addEventListener('click', async () => {
    const url = `https://github.com/${GH_REPO}/releases`;
    try { await invoke('open_url_external', { url }); toast('Abriendo GitHub…'); }
    catch (e) { console.error(e); toast('Error: ' + e); }
  });
  const upOpen = document.getElementById('update-open-settings');
  if (upOpen) upOpen.addEventListener('click', async () => {
    try {
      await invoke('open_bookos_updates');
      toast('Abriendo BookOS Settings…');
    } catch (e) {
      console.error(e);
      toast('No se pudo abrir BookOS Settings');
    }
  });
}

// GitHub repo for releases
const GH_REPO = 'Evelynx08/bookos-notepad';

let _updateCheckPromise = null;
let _lastUpdateCheck = 0;
async function checkUpdates(force = false) {
  const statusEl = document.getElementById('update-status');
  const badge = document.getElementById('update-badge');
  if (!statusEl) return;
  // Throttle: max 1 check per 60s unless forced
  if (!force && Date.now() - _lastUpdateCheck < 60000 && _lastUpdateCheck > 0) return;
  statusEl.textContent = 'Comprobando GitHub…';
  if (badge) badge.classList.add('hidden');
  if (_updateCheckPromise) return _updateCheckPromise;
  _updateCheckPromise = (async () => {
    let info;
    try { info = await invoke('app_version_info'); }
    catch { info = { current: '0.1.0', package: 'bookos-notepad' }; }
    const cur = info.current || '0.1.0';
    const aboutV = document.getElementById('about-version');
    if (aboutV) aboutV.textContent = cur;
    try {
      const resp = await fetch(`https://api.github.com/repos/${GH_REPO}/releases/latest`, {
        headers: { 'Accept': 'application/vnd.github+json' },
        cache: 'no-cache'
      });
      if (resp.status === 404) {
        statusEl.textContent = `Versión ${cur} · Sin releases publicados aún`;
        if (badge) badge.classList.add('hidden');
        return;
      }
      if (!resp.ok) throw new Error('HTTP ' + resp.status);
      const data = await resp.json();
      const tag = (data.tag_name || data.name || '').replace(/^v/i, '').trim();
      if (!tag) {
        statusEl.textContent = `Versión ${cur} · Respuesta GitHub inválida`;
        return;
      }
      const cmp = compareVersions(tag, cur);
      if (cmp > 0) {
        statusEl.textContent = `Actualización disponible: ${cur} → ${tag}`;
        if (badge) { badge.classList.remove('hidden'); badge.className = 'update-badge available'; }
        toast('Nueva versión: ' + tag);
        statusEl.dataset.release = data.html_url || '';
        statusEl.dataset.notes = (data.body || '').slice(0, 500);
      } else {
        statusEl.textContent = `Versión ${cur} · Estás al día`;
        if (badge) { badge.classList.remove('hidden'); badge.className = 'update-badge ok'; }
      }
    } catch (e) {
      console.error('Update check failed:', e);
      statusEl.textContent = `Versión ${cur} · Sin conexión a GitHub`;
    } finally {
      _lastUpdateCheck = Date.now();
    }
  })();
  _updateCheckPromise.finally(() => { _updateCheckPromise = null; });
  return _updateCheckPromise;
}

// Semantic-ish compare: returns positive if a > b
function compareVersions(a, b) {
  const pa = a.split(/[.\-+]/).map(s => parseInt(s, 10) || 0);
  const pb = b.split(/[.\-+]/).map(s => parseInt(s, 10) || 0);
  const len = Math.max(pa.length, pb.length);
  for (let i = 0; i < len; i++) {
    const x = pa[i] || 0, y = pb[i] || 0;
    if (x !== y) return x - y;
  }
  return 0;
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
    const dirty = tabs.filter(t => t.dirty && !t.nota);
    if (dirty.length) {
      const ok = confirm(`${dirty.length} pestaña(s) sin guardar. ¿Salir igualmente?`);
      if (!ok) return;
    }
    await Promise.all(tabs.filter(t => t.nota).map(guardarNota));
    tauriWin().close();
  });
  $('#settings-btn').addEventListener('click', () => showPage(currentPage === 'settings' ? 'editor' : 'settings'));
  $('#preview-btn').addEventListener('click', () => { showPage('editor'); togglePreview(); });
  $('#new-btn').addEventListener('click', () => { showPage('editor'); nuevaNota(); });
  $('#open-btn').addEventListener('click', () => { showPage('editor'); openFile(); });
  $('#save-btn').addEventListener('click', () => saveFile(false));
}

function wireStatusbar() {
  const se = $('#status-encoding');
  if (se) se.addEventListener('click', cycleEncoding);
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
  { id:'new-tab',      sec:'Archivo',  label:'Nueva nota',        def:'Ctrl+N',     run:()=>{ showPage('editor'); nuevaNota(); } },
  { id:'new-tab-alt',  sec:'Archivo',  label:'Nueva pestaña de archivo',def:'Ctrl+T', run:()=>{ showPage('editor'); newTab(); } },
  { id:'open-file',    sec:'Archivo',  label:'Abrir archivo',     def:'Ctrl+O',     run:()=>{ showPage('editor'); openFile(); } },
  { id:'save',         sec:'Archivo',  label:'Guardar',           def:'Ctrl+S',     run:()=>saveFile(false) },
  { id:'save-as',      sec:'Archivo',  label:'Guardar como',      def:'Ctrl+Shift+S', run:()=>saveFile(true) },
  { id:'close-tab',    sec:'Archivo',  label:'Cerrar pestaña',    def:'Ctrl+W',     run:()=>{ if (activeId) closeTab(activeId); } },
  // Navigation
  { id:'next-tab-mru', sec:'Navegación',label:'Pestaña anterior usada', def:'Ctrl+Tab', run:()=>cycleTab('mru') },
  { id:'prev-tab',     sec:'Navegación',label:'Pestaña previa',   def:'Ctrl+Shift+Tab', run:()=>cycleTab('prev') },
  { id:'preview',      sec:'Vista',    label:'Vista previa', def:'Ctrl+P', run:()=>{ showPage('editor'); togglePreview(); } },
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
  { id:'find',         sec:'Edición',  label:'Buscar',            def:'Ctrl+F',     run:()=>{ if(currentPage==='editor') openFindBar(false); } },
  { id:'replace',      sec:'Edición',  label:'Buscar y reemplazar',def:'Ctrl+H',    run:()=>{ if(currentPage==='editor') openFindBar(true); } },
  { id:'find-next',    sec:'Edición',  label:'Buscar siguiente',  def:'F3',         run:()=>findNext(1) },
  { id:'move-line-up', sec:'Edición',  label:'Mover línea arriba',def:'Alt+ArrowUp',   run:()=>{ if(currentPage==='editor') moveLine(-1); } },
  { id:'move-line-dn', sec:'Edición',  label:'Mover línea abajo', def:'Alt+ArrowDown', run:()=>{ if(currentPage==='editor') moveLine(1); } },
  { id:'dup-line',     sec:'Edición',  label:'Duplicar línea',    def:'Ctrl+D',     run:()=>{ if(currentPage==='editor') duplicateLine(); } },
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
  quitarMarcoImagen();
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
      if (!$('#dialogo-borrar').hidden) {
        e.preventDefault(); $('#dialogo-borrar').hidden = true; return;
      }
      if ($('#find-bar')?.classList.contains('open')) {
        e.preventDefault(); closeFindBar(); return;
      }
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
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="m21 21-5.197-5.197m0 0A7.5 7.5 0 1 0 5.196 5.196a7.5 7.5 0 0 0 10.607 10.607Z"/></svg>
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
  // Sin sondeo: detect_system_theme lanza hasta 3 procesos por llamada y el
  // setInterval de 5s que había aquí corría siempre, también con la ventana
  // minimizada. Se comprueba al recuperar foco o visibilidad, que es cuando el
  // usuario vuelve de cambiar el tema y podría ver el desajuste.
  const recheck = () => { if (state.theme === 'auto' && !document.hidden) applyTheme(); };
  document.addEventListener('visibilitychange', recheck);
  window.addEventListener('focus', recheck);
}

// ───────── DRAG & DROP + CLI ARGS ─────────
async function openPathAsTab(p, encoding=null) {
  try {
    const r = await invoke('read_file', { path: p, encoding });
    const name = p.split(/[\\/]/).pop();
    // HTML files load as raw markup (plain:false); all others as plain text.
    const plain = !isRichExt(rawExtOf(name));
    const tab = newTab({ name, path: p, content: r.text, plain });
    if (tab) { tab.encoding = r.encoding || 'utf-8'; tab.eol = r.eol || 'lf'; tab.mtime = r.mtime || 0; }
    return true;
  } catch (e) {
    console.error('openPathAsTab', p, e);
    // Surface the backend reason (e.g. file too large) instead of silent failure.
    const msg = (e && e.message) || (typeof e === 'string' ? e : '');
    toast('No se pudo abrir ' + (p.split(/[\\/]/).pop()) + (msg ? ': ' + msg.slice(0,80) : ''));
    return false;
  }
}

// ───────── FIND & REPLACE ─────────
let _findState = { term: '', matchCase: false, regex: false };

// Build the search RegExp from the current find state. In plain mode the term is
// escaped to a literal; in regex mode it's used verbatim. Returns null if the
// term is empty or the user's regex is syntactically invalid.
function findRegex(global) {
  const term = _findState.term;
  if (!term) return null;
  const src = _findState.regex ? term : term.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const flags = (global ? 'g' : '') + (_findState.matchCase ? '' : 'i');
  try { return new RegExp(src, flags); } catch { return null; }
}

function ensureFindBar() {
  let bar = $('#find-bar');
  if (bar) return bar;
  bar = document.createElement('div');
  bar.id = 'find-bar';
  bar.innerHTML = `
    <div class="find-row">
      <input id="find-input" type="text" placeholder="Buscar" autocomplete="off" spellcheck="false">
      <button id="find-prev" class="find-btn" title="Anterior (Shift+Enter)">‹</button>
      <button id="find-next" class="find-btn" title="Siguiente (Enter / F3)">›</button>
      <button id="find-case" class="find-btn" title="Distinguir mayúsculas">Aa</button>
      <button id="find-regex" class="find-btn" title="Expresión regular">.*</button>
      <span id="find-count" class="find-count"></span>
      <button id="find-close" class="find-btn" title="Cerrar (Esc)"><svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"><path d="M6 18 18 6M6 6l12 12"/></svg></button>
    </div>
    <div class="find-row find-replace-row">
      <input id="replace-input" type="text" placeholder="Reemplazar con" autocomplete="off" spellcheck="false">
      <button id="replace-one" class="find-btn find-text">Uno</button>
      <button id="replace-all" class="find-btn find-text">Todos</button>
    </div>`;
  document.body.appendChild(bar);

  const input = bar.querySelector('#find-input');
  input.addEventListener('input', () => { _findState.term = input.value; updateFindCount(); });
  input.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') { e.preventDefault(); findNext(e.shiftKey ? -1 : 1); }
  });
  bar.querySelector('#replace-input').addEventListener('keydown', (e) => {
    if (e.key === 'Enter') { e.preventDefault(); replaceOne(); }
  });
  bar.querySelector('#find-next').addEventListener('click', () => findNext(1));
  bar.querySelector('#find-prev').addEventListener('click', () => findNext(-1));
  bar.querySelector('#find-close').addEventListener('click', closeFindBar);
  bar.querySelector('#find-case').addEventListener('click', (e) => {
    _findState.matchCase = !_findState.matchCase;
    e.currentTarget.classList.toggle('active', _findState.matchCase);
    updateFindCount();
  });
  bar.querySelector('#find-regex').addEventListener('click', (e) => {
    _findState.regex = !_findState.regex;
    e.currentTarget.classList.toggle('active', _findState.regex);
    updateFindCount();
  });
  bar.querySelector('#replace-one').addEventListener('click', replaceOne);
  bar.querySelector('#replace-all').addEventListener('click', replaceAll);
  return bar;
}

function openFindBar(withReplace) {
  const bar = ensureFindBar();
  bar.classList.add('open');
  bar.classList.toggle('show-replace', !!withReplace);
  const input = bar.querySelector('#find-input');
  // Prefill with current selection
  const sel = String(window.getSelection() || '');
  if (sel && !sel.includes('\n')) { input.value = sel; _findState.term = sel; }
  input.focus(); input.select();
  updateFindCount();
}

function closeFindBar() {
  const bar = $('#find-bar');
  if (bar) bar.classList.remove('open');
  clearHighlights();
  const ed = $('.editor-area');
  if (ed) ed.focus();
}

// ── Highlight all matches via CSS Custom Highlight API (no DOM mutation) ──
const _findHL = (typeof Highlight !== 'undefined' && CSS.highlights) ? new Highlight() : null;
if (_findHL) CSS.highlights.set('find-match', _findHL);

let _domHLActive = false;

function clearHighlights() {
  if (_findHL) { _findHL.clear(); return; }
  // DOM fallback: unwrap any <mark.find-hl> we injected.
  if (!_domHLActive) return;
  const ed = $('.editor-area');
  if (ed) {
    ed.querySelectorAll('mark.find-hl').forEach(m => {
      const txt = document.createTextNode(m.textContent);
      m.replaceWith(txt);
    });
    ed.normalize();
  }
  _domHLActive = false;
}

// Walk editor text nodes, collect Ranges for every occurrence (matched per node;
// matches don't span node boundaries — same scope as before, now regex-capable).
function collectMatchRanges() {
  const ed = $('.editor-area');
  const ranges = [];
  const re = findRegex(true);
  if (!ed || !re) return ranges;
  const walker = document.createTreeWalker(ed, NodeFilter.SHOW_TEXT, null);
  let node;
  while ((node = walker.nextNode())) {
    const raw = node.nodeValue;
    re.lastIndex = 0;
    let m;
    while ((m = re.exec(raw)) !== null) {
      const len = m[0].length;
      if (len === 0) { re.lastIndex++; continue; } // avoid infinite loop on empty match
      const r = document.createRange();
      r.setStart(node, m.index);
      r.setEnd(node, m.index + len);
      ranges.push(r);
    }
  }
  return ranges;
}

function highlightMatches() {
  if (_findHL) {
    _findHL.clear();
    const ranges = collectMatchRanges();
    for (const r of ranges) _findHL.add(r);
    return ranges.length;
  }
  // DOM fallback for older WebKit without Custom Highlight API.
  return domHighlight();
}

// Save caret offset, wrap matches in <mark>, restore caret. Plain editors only
// keep simple structure so unwrapping on edit is lossless.
function domHighlight() {
  const ed = $('.editor-area');
  if (!ed) return 0;
  clearHighlights();
  const re = findRegex(true);
  if (!re) return 0;
  const caret = saveCaretOffset(ed);
  let count = 0;
  // Snapshot text nodes first (live wrapping changes the tree).
  const nodes = [];
  const walker = document.createTreeWalker(ed, NodeFilter.SHOW_TEXT, {
    acceptNode: (n) => n.parentElement && n.parentElement.tagName === 'MARK'
      ? NodeFilter.FILTER_REJECT : NodeFilter.FILTER_ACCEPT
  });
  let node; while ((node = walker.nextNode())) nodes.push(node);
  for (const tn of nodes) {
    const raw = tn.nodeValue;
    re.lastIndex = 0;
    if (!re.test(raw)) continue;
    re.lastIndex = 0;
    const frag = document.createDocumentFragment();
    let last = 0, m;
    while ((m = re.exec(raw)) !== null) {
      const len = m[0].length;
      if (len === 0) { re.lastIndex++; continue; }
      if (m.index > last) frag.appendChild(document.createTextNode(raw.slice(last, m.index)));
      const mark = document.createElement('mark');
      mark.className = 'find-hl';
      mark.textContent = raw.slice(m.index, m.index + len);
      frag.appendChild(mark);
      last = m.index + len;
      count++;
    }
    if (last < raw.length) frag.appendChild(document.createTextNode(raw.slice(last)));
    tn.replaceWith(frag);
  }
  _domHLActive = count > 0;
  if (caret != null) restoreCaretOffset(ed, caret);
  return count;
}

// Caret position as a plain-text character offset within the editor.
function saveCaretOffset(ed) {
  const sel = window.getSelection();
  if (!sel || !sel.rangeCount || !ed.contains(sel.anchorNode)) return null;
  const r = sel.getRangeAt(0).cloneRange();
  const pre = r.cloneRange();
  pre.selectNodeContents(ed);
  pre.setEnd(r.endContainer, r.endOffset);
  return pre.toString().length;
}

function restoreCaretOffset(ed, offset) {
  const walker = document.createTreeWalker(ed, NodeFilter.SHOW_TEXT, null);
  let node, acc = 0;
  while ((node = walker.nextNode())) {
    const len = node.nodeValue.length;
    if (acc + len >= offset) {
      const sel = window.getSelection();
      const r = document.createRange();
      r.setStart(node, offset - acc);
      r.collapse(true);
      sel.removeAllRanges();
      sel.addRange(r);
      return;
    }
    acc += len;
  }
}

function findNext(dir) {
  if (!_findState.term) return;
  const ed = $('.editor-area');
  if (ed) ed.focus();
  // window.find can't do regex — navigate our collected ranges in that mode.
  if (_findState.regex) {
    findNextRange(dir);
  } else {
    // webkit2gtk native: window.find(str, caseSensitive, backwards, wrap)
    const found = window.find(_findState.term, _findState.matchCase, dir < 0, true);
    if (!found) toast('Sin coincidencias');
  }
  // keep find input focused for repeated Enter
  const fi = $('#find-input'); if (fi) fi.focus();
}

// Select the next/previous match relative to the current caret (regex mode).
function findNextRange(dir) {
  const ed = $('.editor-area'); if (!ed) return;
  const ranges = collectMatchRanges();
  if (!ranges.length) { toast('Sin coincidencias'); return; }
  const sel = window.getSelection();
  const anchor = (sel && sel.rangeCount && ed.contains(sel.anchorNode)) ? sel.getRangeAt(0) : null;
  let pick = null;
  if (!anchor) {
    pick = dir > 0 ? ranges[0] : ranges[ranges.length - 1];
  } else if (dir > 0) {
    pick = ranges.find(r => r.compareBoundaryPoints(Range.START_TO_START, anchor) > 0) || ranges[0];
  } else {
    for (let i = ranges.length - 1; i >= 0; i--) {
      if (ranges[i].compareBoundaryPoints(Range.END_TO_END, anchor) < 0) { pick = ranges[i]; break; }
    }
    if (!pick) pick = ranges[ranges.length - 1];
  }
  sel.removeAllRanges();
  sel.addRange(pick);
  const host = pick.startContainer.parentElement;
  if (host && host.scrollIntoView) host.scrollIntoView({ block: 'nearest' });
}

function updateFindCount() {
  const el = $('#find-count');
  if (!el) return;
  if (!_findState.term) { clearHighlights(); el.textContent = ''; return; }
  // Invalid regex: flag it instead of silently showing 0.
  if (_findState.regex && !findRegex(true)) { clearHighlights(); el.textContent = '⚠ regex'; return; }
  el.textContent = highlightMatches() + ' result.';
}

function replaceOne() {
  if (!_findState.term) return;
  const repl = ($('#replace-input') || {}).value || '';
  const sel = window.getSelection();
  const selText = String(sel);
  let isMatch = false, out = repl;
  if (selText && sel.rangeCount) {
    if (_findState.regex) {
      const re = findRegex(false);
      if (re) {
        // Anchor so only a full-selection match counts; honor capture groups in repl.
        const anchored = new RegExp('^(?:' + re.source + ')$', re.flags.replace('g', ''));
        if (anchored.test(selText)) { isMatch = true; out = selText.replace(re, repl); }
      }
    } else {
      isMatch = _findState.matchCase ? selText === _findState.term
        : selText.toLowerCase() === _findState.term.toLowerCase();
    }
  }
  if (isMatch) { document.execCommand('insertText', false, out); onEditorInput(); }
  findNext(1);
  updateFindCount();
}

function replaceAll() {
  if (!_findState.term) return;
  const t = activeTab();
  if (!t) return;
  const repl = ($('#replace-input') || {}).value || '';
  const ed = $('.editor-area');
  if (!ed) return;
  const re = findRegex(true);
  if (!re) { toast('Regex inválida'); return; }
  const plain = htmlToPlain(ed.innerHTML);
  const matches = (plain.match(re) || []).length;
  if (!matches) { toast('Sin coincidencias'); return; }
  const out = plain.replace(re, repl);
  replaceEditorContent(ed, out); // undoable for normal files
  onEditorInput();
  updateFindCount();
  toast(matches + ' reemplazos');
}

function wireDragDrop() {
  // Visual hint on drag (Tauri intercepts the actual drop event)
  window.addEventListener('dragover', (e) => { e.preventDefault(); document.body.classList.add('dropping'); }, false);
  window.addEventListener('dragleave', (e) => { if (e.relatedTarget === null) document.body.classList.remove('dropping'); }, false);
  window.addEventListener('drop', (e) => { e.preventDefault(); document.body.classList.remove('dropping'); }, false);

  const ev = window.__TAURI__ && window.__TAURI__.event;
  if (!ev || !ev.listen) {
    console.warn('Tauri event API missing — drag-drop/CLI disabled');
    return;
  }
  // Native Tauri drag-drop with absolute file paths
  ev.listen('tauri://drag-drop', async (event) => {
    document.body.classList.remove('dropping');
    const paths = (event && event.payload && event.payload.paths) || [];
    if (!Array.isArray(paths) || paths.length === 0) return;
    // Remove untitled empty default tab if present
    if (tabs.length === 1 && !tabs[0].path && !tabs[0].dirty) {
      const ed = $('.editor-area');
      const empty = !ed || htmlToPlain(ed.innerHTML).trim() === '';
      if (empty) { tabs.splice(0, 1); activeId = null; }
    }
    let opened = 0;
    for (const p of paths) {
      // Una imagen soltada se inserta; abrirla como texto no sirve.
      if (esImagen(p)) await insertarImagenDeRuta(p, rangoEnEditor());
      else if (await openPathAsTab(p)) opened++;
    }
    if (opened) {
      toast(`Abierto: ${opened} archivo${opened>1?'s':''}`);
      renderTabs();
      saveSessionDebounced();
    }
  }).catch(e => console.warn('drag-drop listen failed:', e));
  ev.listen('tauri://drag-enter', () => document.body.classList.add('dropping')).catch(()=>{});
  ev.listen('tauri://drag-leave', () => document.body.classList.remove('dropping')).catch(()=>{});
  // CLI files ("Open With") are pulled on boot via take_cli_files — see init().
}

// ───────── NOTAS ─────────
// Una nota es una pestaña más (`nota:true`): así el editor, el formato, la
// búsqueda y el zoom sirven igual. Lo que cambia es que vive en la lista
// lateral en vez de en la fila de pestañas y que se guarda sola en su carpeta.
const tr = (clave, vars) => BookosI18n.t(clave, vars);
let carpetaNotas = '';   // vacía mientras no se haya podido leer la carpeta
let errorNotas = '';
let filtroNotas = '';

function notas() {
  return tabs.filter(t => t.nota).sort((a, b) => b.mtime - a.mtime);
}

// Título y extracto salen del texto: la primera línea con algo y la siguiente.
function resumirNota(t, texto, imagen) {
  const lineas = texto.split('\n').map(l => l.trim()).filter(Boolean);
  t.texto = lineas.join('\n');
  t.name = lineas[0] || tr(imagen ? 'notas.soloImagen' : 'notas.nueva');
  t.extracto = lineas[1] || '';
  if (imagen === t.imagen) return;
  t.imagen = imagen;
  t.miniatura = '';
  if (!imagen) return;
  // La fila enseña una copia de 80px y no la imagen de la nota: la lista se
  // repinta al escribir, y cada vez habría que decodificar el original entero.
  const img = new Image();
  img.onload = () => {
    if (t.imagen !== imagen) return;
    const lado = Math.min(img.naturalWidth, img.naturalHeight);
    const lienzo = document.createElement('canvas');
    lienzo.width = lienzo.height = 80;
    lienzo.getContext('2d').drawImage(img, (img.naturalWidth - lado) / 2, (img.naturalHeight - lado) / 2, lado, lado, 0, 0, 80, 80);
    t.miniatura = lienzo.toDataURL('image/jpeg', .8);
    pintarLista();
  };
  img.src = imagen;
}

// Una nota vacía no tiene texto ni imagen.
const notaVacia = (t) => !t.texto && !t.imagen;

function crearNota(ruta, html, mtime, enDisco) {
  const t = { id: 'tab-' + (tabSeq++), nota: true, enDisco, path: ruta, ext: 'html', rawExt: 'html',
              rich: true, big: false, content: html, dirty: false, encoding: 'utf-8', mtime,
              estadoGuardado: enDisco ? 'guardado' : 'nuevo' };
  // Sin las <img>: htmlToPlain monta el HTML en un <div> y las cargaría todas.
  resumirNota(t, htmlToPlain(html.replace(/<img[^>]*>/g, '')), /<img[^>]+src="([^"]+)"/.exec(html)?.[1] || '');
  tabs.push(t);
  return t;
}

async function cargarNotas() {
  try {
    const r = await invoke('notas_listar');
    carpetaNotas = r.carpeta;
    errorNotas = '';
    for (const n of r.notas) crearNota(n.ruta, n.html, n.mtime, true);
  } catch (e) {
    errorNotas = String(e);
  }
}

function nuevaNota() {
  // Sin carpeta de notas no hay dónde guardarla: queda el archivo sin título.
  if (!carpetaNotas) return newTab();
  // Una nota vacía que aún no está en disco ya es «la nueva»: no se apilan.
  const actual = activeTab();
  if (actual?.nota && !actual.enDisco && notaVacia(actual)) { $('.editor-area')?.focus(); return actual; }
  if (filtroNotas) { filtroNotas = ''; $('#lista-buscar').value = ''; }
  const t = crearNota(`${carpetaNotas}/${Date.now().toString(36)}.html`, '', Date.now() / 1000, false);
  setActive(t.id);
  return t;
}

function abrirNotaReciente() {
  const t = notas()[0];
  if (t) setActive(t.id); else nuevaNota();
}

// El título es el primer bloque de la nota. Con el editor vacío WebKit deja el
// primer renglón como texto suelto en la raíz, sin elemento al que dar estilo,
// así que siempre hay un <div> donde cae lo que se escribe. Se descartó
// ::first-line, que no necesita nada de esto: con un título de dos renglones
// solo ponía en negrita el primero.
function asegurarTitulo(ed) {
  const esBloque = (n) => n.nodeType === 1 && _BLOCK_TAGS.has(n.tagName);
  if (ed.firstChild && esBloque(ed.firstChild)) return;
  const enfocado = document.activeElement === ed;
  const cursor = enfocado ? saveCaretOffset(ed) : null;
  const titulo = document.createElement('div');
  while (ed.firstChild && !esBloque(ed.firstChild)) titulo.appendChild(ed.firstChild);
  if (!titulo.firstChild) titulo.appendChild(document.createElement('br'));
  ed.prepend(titulo);
  if (!enfocado) return;
  if (titulo.textContent) { restoreCaretOffset(ed, cursor ?? 0); return; }
  const sel = window.getSelection();
  sel.removeAllRanges();
  const rango = document.createRange();
  rango.setStart(titulo, 0);
  sel.addRange(rango);
}

function notaEditada(t, e) {
  const antes = t.name + '\n' + t.extracto + '\n' + t.imagen;
  const primerCambio = !t.dirty;
  const ed = $('.editor-area');
  asegurarTitulo(ed);
  // Intro tras una tarea hecha: WebKit clona el <li> con su clase, y la tarea
  // nueva nacía ya tachada.
  if (e?.inputType === 'insertParagraph') elementoDelCursor()?.closest('li.hecha')?.classList.remove('hecha');
  t.dirty = true;
  t.estadoGuardado = t.guardadoEnCurso ? 'guardando' : 'pendiente';
  actualizarGuardado(t);
  t.mtime = Date.now() / 1000;
  resumirNota(t, ed.innerText, ed.querySelector('img')?.src || '');
  ed.classList.toggle('vacia', notaVacia(t));
  // La lista solo cambia si cambia lo que enseña, o si la nota sube arriba.
  if (primerCambio || antes !== t.name + '\n' + t.extracto + '\n' + t.imagen) {
    pintarLista();
    updateTitle();
    $('.nota-fecha').textContent = fechaLarga(t.mtime);
  }
  clearTimeout(t.temporizador);
  t.temporizador = setTimeout(() => guardarNota(t), 600);
}

function actualizarGuardado(t) {
  if (activeId !== t.id) return;
  const boton = $('.nota-guardado');
  if (!boton) return;
  boton.textContent = tr('notas.guardado.' + t.estadoGuardado);
  boton.disabled = t.estadoGuardado !== 'error';
  boton.classList.toggle('error', t.estadoGuardado === 'error');
  boton.title = t.estadoGuardado === 'error'
    ? tr('notas.guardado.reintentar', { motivo: t.errorGuardado }) : '';
}

async function guardarNota(t) {
  clearTimeout(t.temporizador);
  if (t.guardadoEnCurso) return t.guardadoEnCurso;
  // Vacía y sin archivo todavía: no se crea un .html en blanco.
  if (!t.dirty) return;
  if (notaVacia(t) && !t.enDisco) {
    t.estadoGuardado = 'nuevo';
    actualizarGuardado(t);
    return;
  }
  // Una sola escritura por nota: una respuesta lenta no puede sobrescribir
  // una versión más reciente ni anunciar «Guardado» con cambios pendientes.
  t.guardadoEnCurso = (async () => {
    try {
      do {
        if (activeId === t.id) saveActiveContent();
        t.dirty = false;
        t.estadoGuardado = 'guardando';
        actualizarGuardado(t);
        await invoke('write_file', { path: t.path, contents: t.content, encoding: 'utf-8', eol: 'lf' });
        t.enDisco = true;
      } while (t.dirty);
      t.estadoGuardado = 'guardado';
    } catch (e) {
      t.dirty = true;
      t.estadoGuardado = 'error';
      t.errorGuardado = String(e);
      toast(tr('notas.error.guardar', { motivo: e }));
    } finally {
      t.guardadoEnCurso = null;
      actualizarGuardado(t);
    }
  })();
  return t.guardadoEnCurso;
}

// Al dejar de mirar una nota: se guarda, o se descarta si nunca tuvo nada.
function soltarNota(t) {
  if (notaVacia(t) && !t.enDisco) tabs.splice(tabs.indexOf(t), 1);
  else guardarNota(t);
}

function pedirBorrado() {
  const t = activeTab();
  if (!t?.nota) return;
  if (notaVacia(t)) { borrarNota(t); return; }
  const nombre = t.name.length > 60 ? t.name.slice(0, 60) + '…' : t.name;
  $('#dialogo-borrar-texto').textContent = tr('notas.borrar.texto', { nombre });
  $('#dialogo-borrar').hidden = false;
  $('#borrar-no').focus();
}

async function borrarNota(t) {
  clearTimeout(t.temporizador);
  t.dirty = false;
  if (t.enDisco) {
    try { await invoke('nota_borrar', { ruta: t.path }); }
    catch (e) { toast(tr('notas.error.borrar', { motivo: e })); return; }
  }
  const orden = notas();
  const i = orden.indexOf(t);
  const siguiente = orden[i + 1] || orden[i - 1];
  tabs.splice(tabs.indexOf(t), 1);
  if (activeId !== t.id) { pintarLista(); return; }
  activeId = null;
  if (siguiente) setActive(siguiente.id); else nuevaNota();
}

function diasDesde(mtime) {
  return Math.ceil((new Date().setHours(0, 0, 0, 0) - mtime * 1000) / 86400000);
}
function grupoDe(mtime) {
  const dias = diasDesde(mtime);
  if (dias <= 0) return tr('notas.hoy');
  if (dias === 1) return tr('notas.ayer');
  if (dias <= 7) return tr('notas.semana');
  if (dias <= 30) return tr('notas.mes');
  return String(new Date(mtime * 1000).getFullYear());
}
function fechaCorta(mtime) {
  const fecha = new Date(mtime * 1000);
  const idioma = BookosI18n.getLang();
  return diasDesde(mtime) <= 0
    ? fecha.toLocaleTimeString(idioma, { timeStyle: 'short' })
    : fecha.toLocaleDateString(idioma, { day: 'numeric', month: 'numeric', year: '2-digit' });
}
function fechaLarga(mtime) {
  return new Date(mtime * 1000).toLocaleString(BookosI18n.getLang(), { dateStyle: 'long', timeStyle: 'short' });
}

function avisoLista(texto) {
  const aviso = document.createElement('div');
  aviso.className = 'lista-aviso';
  aviso.textContent = texto;
  $('#lista-notas').appendChild(aviso);
  return aviso;
}

function pintarLista() {
  const cont = $('#lista-notas');
  const todas = notas();
  cont.textContent = '';
  $('#lista-cuenta').textContent = errorNotas ? ''
    : tr(todas.length === 1 ? 'notas.cuenta.una' : 'notas.cuenta', { n: todas.length });
  if (errorNotas) {
    const reintentar = document.createElement('button');
    reintentar.textContent = tr('notas.reintentar');
    reintentar.addEventListener('click', async () => {
      await cargarNotas();
      if (errorNotas) pintarLista(); else abrirNotaReciente();
    });
    avisoLista(tr('notas.error.cargar', { motivo: errorNotas })).appendChild(reintentar);
    return;
  }
  const busca = filtroNotas.toLowerCase();
  const visibles = busca ? todas.filter(t => t.texto.toLowerCase().includes(busca)) : todas;
  if (!visibles.length) { avisoLista(tr('notas.sinResultados', { texto: filtroNotas })); return; }
  let etiqueta = null;
  for (const t of visibles) {
    const suya = grupoDe(t.mtime);
    if (suya !== etiqueta) {
      etiqueta = suya;
      const cab = document.createElement('div');
      cab.className = 'grupo-cab';
      cab.textContent = suya;
      cont.appendChild(cab);
    }
    const item = document.createElement('button');
    item.className = 'nota-item' + (t.id === activeId ? ' sel' : '');
    item.dataset.id = t.id;
    const textos = document.createElement('span');
    textos.className = 'nota-textos';
    const nombre = document.createElement('span');
    nombre.className = 'nota-nombre';
    nombre.textContent = t.name;
    const cuando = document.createElement('span');
    cuando.className = 'nota-cuando';
    cuando.textContent = fechaCorta(t.mtime);
    const extracto = document.createElement('span');
    extracto.className = 'nota-extracto';
    extracto.append(cuando, t.extracto || tr('notas.sinTexto'));
    textos.append(nombre, extracto);
    item.appendChild(textos);
    if (t.miniatura) {
      const mini = document.createElement('img');
      mini.className = 'nota-mini';
      mini.alt = '';
      mini.src = t.miniatura;
      item.appendChild(mini);
    }
    cont.appendChild(item);
  }
}

function wireNotas() {
  $('#mc').classList.toggle('sin-lista', state.settings.listaOculta);
  $('#lista-btn').addEventListener('click', () => {
    state.settings.listaOculta = !state.settings.listaOculta;
    $('#mc').classList.toggle('sin-lista', state.settings.listaOculta);
    saveState();
  });
  $('#lista-notas').addEventListener('click', (e) => {
    const item = e.target.closest('.nota-item');
    if (!item) return;
    setActive(item.dataset.id);
    if (currentPage !== 'editor') showPage('editor');
    $('.editor-area')?.focus();
  });
  $('#lista-buscar').addEventListener('input', (e) => {
    filtroNotas = e.target.value.trim();
    pintarLista();
  });
  $('#borrar-nota').addEventListener('click', pedirBorrado);
  $('#imagen-btn').addEventListener('click', elegirImagenes);
  $('#tareas-btn').addEventListener('click', insertarTareas);
  $('#borrar-no').addEventListener('click', () => { $('#dialogo-borrar').hidden = true; });
  $('#borrar-si').addEventListener('click', () => {
    $('#dialogo-borrar').hidden = true;
    const t = activeTab();
    if (t?.nota) borrarNota(t);
  });
  // El escritorio puede cerrar la ventana sin pasar por el botón de la barra:
  // al perder el foco no se espera a los 600ms del temporizador.
  window.addEventListener('blur', () => {
    const t = activeTab();
    if (t?.nota) guardarNota(t);
  });
}

// ───────── INSERTAR EN LA NOTA ─────────
const EXT_IMAGEN = ['png', 'jpg', 'jpeg', 'webp', 'gif', 'bmp', 'svg'];
const esImagen = (ruta) => EXT_IMAGEN.includes(rawExtOf(ruta));

function elementoDelCursor() {
  const nodo = window.getSelection().anchorNode;
  return nodo && (nodo.nodeType === 1 ? nodo : nodo.parentElement);
}

// La imagen va dentro del .html de la nota como data:, y la nota sigue siendo
// un solo archivo que se puede copiar o exportar entero. Por eso se reduce a
// 1600px de lado: notas_listar lee todas las notas completas al arrancar.
async function insertarImagen(blob, rango) {
  const ed = $('.editor-area');
  // En un archivo de texto la imagen se vería en el editor y desaparecería al
  // guardar: solo se guarda el texto. Mejor no dejarla entrar.
  if (!ed || !activeTab()?.rich) { toast(tr('notas.error.imagenTexto')); return; }
  const img = new Image();
  const url = URL.createObjectURL(blob);
  img.src = url;
  try { await img.decode(); }
  catch { toast(tr('notas.error.imagen')); return; }
  finally { URL.revokeObjectURL(url); }
  const escala = Math.min(1, 1600 / Math.max(img.naturalWidth, img.naturalHeight));
  const lienzo = document.createElement('canvas');
  lienzo.width = Math.round(img.naturalWidth * escala);
  lienzo.height = Math.round(img.naturalHeight * escala);
  lienzo.getContext('2d').drawImage(img, 0, 0, lienzo.width, lienzo.height);
  // JPEG solo para lo que ya era JPEG: los demás formatos pueden traer transparencia.
  const datos = lienzo.toDataURL(blob.type === 'image/jpeg' ? 'image/jpeg' : 'image/png', .85);
  ed.focus();
  // El selector de archivos se lleva el foco y con él la selección: se repone
  // la de antes de abrirlo, o el final de la nota si no había.
  const sel = window.getSelection();
  if (!rango) { rango = document.createRange(); rango.selectNodeContents(ed); rango.collapse(false); }
  sel.removeAllRanges();
  sel.addRange(rango);
  document.execCommand('insertImage', false, datos);
}

// El tamaño se guarda como `width` en el <img> y los píxeles del data: no se
// tocan: encoger y volver a agrandar es reversible. No se deja pasar del
// tamaño natural, porque más allá solo se estiraría el mapa de bits.
let marcoImagen = null;

function anchoDisponibleImagen(img) {
  const padre = img.parentElement, estilo = getComputedStyle(padre);
  return padre.clientWidth - parseFloat(estilo.paddingLeft) - parseFloat(estilo.paddingRight);
}

function confirmarCambioImagen(img, estiloAnterior) {
  if (img.getAttribute('style') === estiloAnterior) return;
  const ed = img.closest('.editor-area');
  const arriba = ed.scrollTop, izquierda = ed.scrollLeft;
  const indice = [...ed.querySelectorAll('img')].indexOf(img);
  const copia = img.cloneNode();
  if (estiloAnterior === null) img.removeAttribute('style');
  else img.setAttribute('style', estiloAnterior);
  quitarMarcoImagen();
  const rango = document.createRange();
  rango.selectNode(img);
  const sel = window.getSelection();
  sel.removeAllRanges();
  sel.addRange(rango);
  ed.focus({ preventScroll: true });
  // El arrastre solo previsualiza: sustituir la imagen al soltar registra
  // una operación nativa, junto al texto, en vez de un historial paralelo.
  document.execCommand('insertHTML', false, copia.outerHTML);
  ed.scrollTop = arriba;
  ed.scrollLeft = izquierda;
  seleccionarImagen({ target: ed.querySelectorAll('img')[indice] });
}

function accionesImagen(img) {
  const barra = document.createElement('div');
  barra.className = 'img-acciones';
  barra.setAttribute('role', 'toolbar');
  barra.setAttribute('aria-label', tr('imagen.acciones'));
  barra.addEventListener('pointerdown', (e) => e.preventDefault());
  const acciones = [
    ['original', () => { img.style.width = img.naturalWidth + 'px'; }],
    ['ajustar', () => { img.style.width = Math.min(img.naturalWidth, anchoDisponibleImagen(img)) + 'px'; }],
    ['izquierda', () => { img.style.marginLeft = '0'; img.style.marginRight = 'auto'; }],
    ['centro', () => { img.style.marginLeft = img.style.marginRight = 'auto'; }],
    ['derecha', () => { img.style.marginLeft = 'auto'; img.style.marginRight = '0'; }]
  ];
  const trazos = {
    izquierda: 'M3 5h18M3 10h10M3 15h18M3 20h10',
    centro: 'M3 5h18M7 10h10M3 15h18M7 20h10',
    derecha: 'M3 5h18M11 10h10M3 15h18M11 20h10'
  };
  for (const [nombre, aplicar] of acciones) {
    const boton = document.createElement('button');
    boton.type = 'button';
    boton.dataset.accion = nombre;
    boton.title = tr('imagen.' + nombre);
    boton.setAttribute('aria-label', boton.title);
    if (trazos[nombre]) {
      boton.innerHTML = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="${trazos[nombre]}"/></svg>`;
      const alineacion = img.style.marginLeft === 'auto' ? (img.style.marginRight === 'auto' ? 'centro' : 'derecha') : 'izquierda';
      boton.setAttribute('aria-pressed', nombre === alineacion ? 'true' : 'false');
    } else boton.textContent = boton.title;
    boton.addEventListener('click', () => {
      const enfocado = document.activeElement === boton;
      const anterior = img.getAttribute('style');
      aplicar();
      confirmarCambioImagen(img, anterior);
      if (enfocado) $('.img-acciones [data-accion="' + nombre + '"]').focus({ preventScroll: true });
    });
    barra.appendChild(boton);
  }
  const ed = img.closest('.editor-area');
  ed.parentElement.insertBefore(barra, ed);
  return barra;
}

function quitarMarcoImagen() {
  if (!marcoImagen) return;
  marcoImagen.cancelarArrastre?.();
  marcoImagen.observador.disconnect();
  marcoImagen.capa.remove();
  marcoImagen.barra.remove();
  marcoImagen = null;
}
function seleccionarImagen(e) {
  quitarMarcoImagen();
  const img = e.target;
  if (img.tagName !== 'IMG') return;
  const host = $('#editor-host');
  const ed = img.closest('.editor-area');
  const capa = document.createElement('div');
  capa.className = 'img-capa';
  const marco = document.createElement('div');
  marco.className = 'img-marco';
  capa.appendChild(marco);
  host.appendChild(capa);
  const colocar = () => {
    const r = img.getBoundingClientRect(), a = ed.getBoundingClientRect(), h = host.getBoundingClientRect();
    Object.assign(capa.style, { left: a.left - h.left + 'px', top: a.top - h.top + 'px', width: ed.clientWidth + 'px', height: ed.clientHeight + 'px' });
    Object.assign(marco.style, { left: r.left - a.left + 'px', top: r.top - a.top + 'px', width: r.width + 'px', height: r.height + 'px' });
  };
  const observador = new ResizeObserver(colocar);
  observador.observe(img);
  observador.observe(ed);
  const barra = accionesImagen(img);
  marcoImagen = { img, capa, barra, colocar, observador };
  colocar();
  for (const [esquina, sx, sy] of [
    ['sup-izq', -1, -1], ['sup-der', 1, -1],
    ['inf-izq', -1, 1], ['inf-der', 1, 1]
  ]) {
    const asa = document.createElement('div');
    asa.className = 'img-asa ' + esquina;
    marco.appendChild(asa);
    asa.addEventListener('pointerdown', (ev) => {
      if (ev.button !== 0) return;
      ev.preventDefault();
      asa.setPointerCapture(ev.pointerId);
      const x0 = ev.clientX, y0 = ev.clientY;
      const { width: w0, height: h0 } = img.getBoundingClientRect();
      const estiloAnterior = img.getAttribute('style');
      const max = Math.min(img.naturalWidth, anchoDisponibleImagen(img));
      const limpiar = () => {
        asa.onpointermove = asa.onpointerup = asa.onpointercancel = asa.onlostpointercapture = null;
      };
      marcoImagen.cancelarArrastre = () => {
        limpiar();
        if (estiloAnterior === null) img.removeAttribute('style');
        else img.setAttribute('style', estiloAnterior);
      };
      asa.onpointermove = (m) => {
        const dx = m.clientX - x0, dy = m.clientY - y0;
        const cambio = Math.abs(dx) >= Math.abs(dy) ? sx * dx : sy * dy * w0 / h0;
        img.style.width = Math.round(Math.min(max, Math.max(40, w0 + cambio))) + 'px';
        colocar();
      };
      const terminar = () => {
        limpiar();
        marcoImagen.cancelarArrastre = null;
        confirmarCambioImagen(img, estiloAnterior);
      };
      asa.onpointerup = asa.onpointercancel = asa.onlostpointercapture = terminar;
    });
  }
}

function rangoEnEditor() {
  const sel = window.getSelection();
  const ed = $('.editor-area');
  return sel.rangeCount && ed?.contains(sel.anchorNode) ? sel.getRangeAt(0).cloneRange() : null;
}

async function insertarImagenDeRuta(ruta, rango) {
  try {
    const bytes = await invoke('leer_imagen', { ruta });
    const ext = rawExtOf(ruta);
    const tipo = 'image/' + (ext === 'jpg' ? 'jpeg' : ext === 'svg' ? 'svg+xml' : ext);
    await insertarImagen(new Blob([new Uint8Array(bytes)], { type: tipo }), rango);
  } catch (e) {
    toast(tr('notas.error.leerImagen', { motivo: e }));
  }
}

async function elegirImagenes() {
  const rango = rangoEnEditor();
  let rutas;
  try {
    rutas = await elegirRuta('open', { multiple: true, filters: [{ name: tr('notas.imagenes'), extensions: EXT_IMAGEN }] });
  } catch (e) { toast('Error diálogo: ' + e); return; }
  // Solo la primera va a la selección guardada; las demás siguen al cursor.
  for (const [i, ruta] of (rutas || []).entries()) await insertarImagenDeRuta(ruta, i ? rangoEnEditor() : rango);
}

function pegarImagen(e) {
  const imagen = [...e.clipboardData.files].find(f => f.type.startsWith('image/'));
  if (!imagen) return;
  e.preventDefault();
  insertarImagen(imagen, rangoEnEditor());
}

// Lista de tareas: una <ul class="tareas"> normal, con el círculo dibujado en
// el margen de cada <li>. Así Intro, Tab y borrar se comportan como en
// cualquier lista sin escribir nada de eso aquí.
function insertarTareas() {
  const ed = $('.editor-area.nota');
  if (!ed) return;
  ed.focus();
  const lista = elementoDelCursor()?.closest('ul');
  if (lista && ed.contains(lista)) lista.classList.toggle('tareas');
  else {
    document.execCommand('insertUnorderedList');
    elementoDelCursor()?.closest('ul')?.classList.add('tareas');
  }
  onEditorInput();
}

function alternarTarea(e) {
  // El círculo vive en los 30px de relleno izquierdo del <li>.
  if (!e.target.matches('ul.tareas > li') || e.offsetX >= 30) return;
  e.target.classList.toggle('hecha');
  onEditorInput();
}

// ───────── BOOT ─────────
(async function init() {
  await loadState();
  await applyTheme();
  wireWindow();
  wireToolbar();
  wireSettings();
  wireKeys();
  wireVistaPrevia();
  wireStatusbar();
  wireSystemTheme();
  wireDragDrop();
  wireFileWatch();
  wireNotas();
  await cargarNotas();
  // Restore previous session if enabled
  await restoreSession();
  // Open files passed on the command line ("Open With → bookos-notepad").
  // Pull model — race-free vs the old fire-once emit.
  let cliOpened = 0;
  try {
    const cliFiles = await invoke('take_cli_files');
    if (Array.isArray(cliFiles) && cliFiles.length) {
      // Drop the empty default tab if it's the only thing open.
      if (tabs.length === 1 && !tabs[0].path && !tabs[0].dirty) {
        const ed = $('.editor-area');
        const empty = !ed || htmlToPlain(ed.innerHTML).trim() === '';
        if (empty) { tabs.splice(0, 1); activeId = null; }
      }
      for (const p of cliFiles) if (await openPathAsTab(p)) cliOpened++;
      if (cliOpened) { renderTabs(); saveSessionDebounced(); }
    }
  } catch (e) { console.warn('take_cli_files failed:', e); }
  // Sin archivo pedido por línea de órdenes, la app abre en las notas; las
  // pestañas de la sesión anterior quedan detrás, en la barra.
  // Si la carpeta de notas falló y ya hay pestañas, no se añade otra vacía.
  if (!cliOpened && (carpetaNotas || !tabs.length)) abrirNotaReciente();
  setZoom(state.settings.zoom);
  showPage('editor');
})();
