// BookOS Notepad — ventana de vista previa (hija de la principal).
// Pinta el documento que le manda el editor. No guarda nada: los ajustes que se
// cambian aquí vuelven al editor con `vista_previa_ajuste`, que es quien los
// escribe en state.json.
const invoke = (cmd, args) => window.__TAURI__.core.invoke(cmd, args);
const $ = (s, r=document) => r.querySelector(s);

let doc = null;       // { nombre, ext, texto, ajustes } — lo último recibido
let ajustes = {};

function ajustar(clave, valor) {
  ajustes[clave] = valor;
  invoke('vista_previa_ajuste', { clave, valor });
}

let toastTimer = null;
function toast(msg) {
  const t = $('#toast');
  t.textContent = msg;
  t.classList.add('show');
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => t.classList.remove('show'), 1600);
}

function escapeHtml(s) {
  return (s||'').replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;');
}

function effectivePreviewMode() {
  const m = ajustes.previewMode;
  if (m !== 'auto') return m;
  if (doc.ext === 'md') return 'md';
  // svg / css fall through to html render
  return 'html';
}

function youtubeEmbed(url) {
  try {
    const u = new URL(url);
    const h = u.hostname.replace(/^www\./,'');
    // If user toggled "direct" mode, pass YT URLs through unchanged (no /embed/)
    if (ajustes.youtubeDirect) {
      if (h === 'youtube.com' || h === 'm.youtube.com' || h === 'youtu.be') return url;
    }
    const host = ajustes.youtubeNoCookie ? 'https://www.youtube-nocookie.com' : 'https://www.youtube.com';
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

function mountUrlIframe(body, final) {
  const isYt = /youtube(-nocookie)?\.com\/embed\//.test(final) || /youtube\.com\/watch/.test(final) || /youtu\.be/.test(final);
  body.classList.toggle('yt-fit', isYt);
  const f = document.createElement('iframe');
  f.src = final;
  f.referrerPolicy = 'strict-origin-when-cross-origin';
  f.allow = 'accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; fullscreen';
  f.setAttribute('allowfullscreen', '');
  body.appendChild(f);
  // For YouTube embeds, add a floating action bar with PiP/external player options
  if (/youtube(-nocookie)?\.com\/embed\//.test(final) || /youtube\.com\/watch/.test(final) || /youtu\.be/.test(final)) {
    addYtActionBar(body, ajustes.previewUrl || final);
  }
}

function addYtActionBar(body, originalUrl) {
  const bar = document.createElement('div');
  bar.className = 'yt-actions';
  bar.innerHTML = `
    <button class="yt-action" id="yt-ext" title="Abrir en navegador">
      <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M13.5 6H5.25A2.25 2.25 0 0 0 3 8.25v10.5A2.25 2.25 0 0 0 5.25 21h10.5A2.25 2.25 0 0 0 18 18.75V10.5m-10.5 6L21 3m0 0h-5.25M21 3v5.25"/></svg>
      <span>Abrir en navegador</span>
    </button>`;
  body.appendChild(bar);
  bar.querySelector('#yt-ext').addEventListener('click', () => openExternal(originalUrl));
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

function showPreviewEmpty(body, msg) {
  const d = document.createElement('div');
  d.className = 'preview-empty';
  d.innerHTML = `<svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round"><path d="M2.036 12.322a1.012 1.012 0 0 1 0-.639C3.423 7.51 7.36 4.5 12 4.5c4.638 0 8.573 3.007 9.963 7.178.07.207.07.431 0 .639C20.577 16.49 16.64 19.5 12 19.5c-4.638 0-8.573-3.007-9.963-7.178Z"/><path d="M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z"/></svg><span>${escapeHtml(msg)}</span>`;
  body.appendChild(d);
}

function setActivePill(mode) {
  document.querySelectorAll('.preview-mode-pills .pill').forEach(b => {
    b.classList.toggle('on', b.dataset.pmode === mode);
  });
}

function renderPreview() {
  const body = $('#preview-body');
  // Al reescribir mientras se edita, el Markdown no vuelve arriba del todo.
  const desplazado = body.querySelector('.preview-md')?.scrollTop || 0;
  body.innerHTML = '';
  body.classList.remove('yt-fit');
  const mode = effectivePreviewMode();
  setActivePill(ajustes.previewMode);
  $('#vp-url').hidden = ajustes.previewMode !== 'url';

  if (mode === 'url' || ajustes.previewMode === 'url') {
    const url = (ajustes.previewUrl || '').trim();
    if (!url) { showPreviewEmpty(body, 'Pega una URL arriba. YouTube se embebe automáticamente.'); return; }
    const yt = youtubeEmbed(url);
    const final = yt || normalizeUrl(url);
    mountUrlIframe(body, final);
    return;
  }

  const t = doc;
  const text = doc.texto;

  if (mode === 'html') {
    const f = document.createElement('iframe');
    // OJO: nunca combinar allow-scripts + allow-same-origin en srcdoc — el
    // documento previsualizado quedaría en el MISMO origen que la app y sus
    // scripts alcanzarían el bridge de Tauri. Sin allow-same-origin el iframe
    // corre en origen opaco: scripts sí, acceso a la app no.
    f.sandbox = 'allow-scripts allow-forms allow-popups allow-modals';
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
    // Sanea el HTML embebido en el markdown: un .md ajeno no debe poder
    // ejecutar scripts dentro del webview de la app.
    div.querySelectorAll('script,iframe,object,embed,link,meta,base,form').forEach(n => n.remove());
    div.querySelectorAll('*').forEach(el => {
      for (const a of [...el.attributes]) {
        const n = a.name.toLowerCase();
        if (n.startsWith('on')) el.removeAttribute(a.name);
        else if ((n === 'href' || n === 'src' || n === 'xlink:href') && /^\s*javascript:/i.test(a.value)) el.removeAttribute(a.name);
      }
    });
    // Open external links in new context (intercept)
    div.querySelectorAll('a[href]').forEach(a => { a.target = '_blank'; a.rel = 'noopener'; });
    body.appendChild(div);
    div.scrollTop = desplazado;
    return;
  }
}

function recibir(nuevo) {
  if (!nuevo) return;
  // En modo URL lo que se ve no depende del texto: si solo cambió el
  // documento, recargar la página a cada pulsación sería un parpadeo.
  const soloTexto = doc && ajustes.previewMode === 'url'
    && JSON.stringify(nuevo.ajustes) === JSON.stringify(doc.ajustes);
  doc = nuevo;
  ajustes = { ...nuevo.ajustes };
  $('#vp-titulo').textContent = doc.nombre;
  $('#url-input').value = ajustes.previewUrl || '';
  if (!soloTexto) renderPreview();
}

function wire() {
  document.querySelectorAll('.preview-mode-pills .pill').forEach(b => {
    b.addEventListener('click', () => {
      ajustar('previewMode', b.dataset.pmode);
      renderPreview();
    });
  });
  const inp = $('#url-input');
  const go = () => {
    ajustar('previewMode', 'url');
    ajustar('previewUrl', inp.value);
    renderPreview();
  };
  inp.addEventListener('keydown', (e) => {
    if (e.key === 'Enter') { e.preventDefault(); go(); }
  });
  $('#url-go').addEventListener('click', go);
  // Recargar pide el documento otra vez: con archivos grandes el editor no lo
  // manda a cada pulsación.
  $('#preview-refresh').addEventListener('click', () => invoke('vista_previa_pedir'));
  $('#close').addEventListener('click', () => invoke('vista_previa_cerrar'));
  // Como Quick Look: Escape o Espacio la cierran, salvo escribiendo la URL.
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' || (e.key === ' ' && e.target !== inp)) {
      e.preventDefault();
      invoke('vista_previa_cerrar');
    }
  });
  document.addEventListener('contextmenu', e => e.preventDefault());
}

(async function init() {
  BookosI18n.setLang(await invoke('bookos_language').catch(() => 'es'));
  const tema = await invoke('detect_system_theme').catch(() => 'auto');
  if (tema === 'dark' || tema === 'light') document.documentElement.classList.add(tema + '-mode');
  wire();
  window.__TAURI__.event.listen('vista-previa-doc', (e) => recibir(e.payload));
  // El documento se pide, no se espera: el evento del editor pudo salir antes
  // de que esta página estuviera escuchando.
  recibir(await invoke('vista_previa_doc'));
})();
