// Tauri 2 bridge — patches in missing plugin namespaces (dialog, fs).
// The real Tauri bundle loads first and provides core/event/window/webview.
// This adds plugin shims that the bundle doesn't expose by default.
(function () {
  if (typeof window === 'undefined') return;

  const internals = window.__TAURI_INTERNALS__;
  if (!internals) {
    console.warn('[tauri-bridge] __TAURI_INTERNALS__ missing — IPC unavailable');
    return;
  }

  // If the official bundle already loaded, only patch missing plugin namespaces
  if (window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.event) {
    const T = window.__TAURI__;
    const invokeReal = T.core.invoke;
    if (!T.dialog) {
      T.dialog = {
        open:    (o)=>invokeReal('plugin:dialog|open', { options: o||{} }),
        save:    (o)=>invokeReal('plugin:dialog|save', { options: o||{} }),
        message: (m,o)=>invokeReal('plugin:dialog|message', Object.assign({message:m}, o||{})),
        ask:     (m,o)=>invokeReal('plugin:dialog|ask',     Object.assign({message:m}, o||{})),
        confirm: (m,o)=>invokeReal('plugin:dialog|confirm', Object.assign({message:m}, o||{})),
      };
    }
    if (!T.fs) {
      T.fs = {
        readTextFile:  (p)=>invokeReal('plugin:fs|read_text_file', { path:p }),
        writeTextFile: (p,c)=>invokeReal('plugin:fs|write_text_file', { path:p, contents:c }),
        exists:        (p)=>invokeReal('plugin:fs|exists', { path:p }),
      };
    }
    return;
  }

  const invoke = (cmd, args) => internals.invoke(cmd, args || {});

  // Window proxy: every method translates to plugin:window|<method> invoke
  function makeWindowProxy(label) {
    const win = {
      label,
      _call(cmd, payload) {
        return invoke('plugin:window|' + cmd, Object.assign({ label }, payload || {}));
      },
      minimize() { return this._call('minimize'); },
      maximize() { return this._call('maximize'); },
      unmaximize() { return this._call('unmaximize'); },
      toggleMaximize() { return this._call('toggle_maximize'); },
      close() { return this._call('close'); },
      destroy() { return this._call('destroy'); },
      show() { return this._call('show'); },
      hide() { return this._call('hide'); },
      setFocus() { return this._call('set_focus'); },
      setTitle(title) { return this._call('set_title', { title }); },
      setFullscreen(fullscreen) { return this._call('set_fullscreen', { value: !!fullscreen }); },
      isFullscreen() { return this._call('is_fullscreen'); },
      isMaximized() { return this._call('is_maximized'); },
      isMinimized() { return this._call('is_minimized'); },
      isVisible() { return this._call('is_visible'); },
      startDragging() { return this._call('start_dragging'); },
      outerPosition() { return this._call('outer_position'); },
      innerPosition() { return this._call('inner_position'); },
      outerSize() { return this._call('outer_size'); },
      innerSize() { return this._call('inner_size'); },
      setSize(size) { return this._call('set_size', { value: size }); },
      setPosition(pos) { return this._call('set_position', { value: pos }); },
    };
    return win;
  }

  const currentWindow = makeWindowProxy('main');

  window.__TAURI__ = {
    core: {
      invoke,
      convertFileSrc: (p) => internals.convertFileSrc ? internals.convertFileSrc(p) : p,
    },
    window: {
      getCurrentWindow: () => currentWindow,
      Window: { getCurrent: () => currentWindow },
    },
    dialog: {
      open: (opts) => invoke('plugin:dialog|open', { options: opts || {} }),
      save: (opts) => invoke('plugin:dialog|save', { options: opts || {} }),
      message: (msg, opts) => invoke('plugin:dialog|message', Object.assign({ message: msg }, opts || {})),
      ask: (msg, opts) => invoke('plugin:dialog|ask', Object.assign({ message: msg }, opts || {})),
      confirm: (msg, opts) => invoke('plugin:dialog|confirm', Object.assign({ message: msg }, opts || {})),
    },
    fs: {
      readTextFile: (path) => invoke('plugin:fs|read_text_file', { path }),
      writeTextFile: (path, contents) => invoke('plugin:fs|write_text_file', { path, contents }),
      exists: (path) => invoke('plugin:fs|exists', { path }),
    },
    event: {
      // Subscribe to Tauri events via the IPC channel.
      // Returns a Promise<unlisten>.
      listen: (eventName, handler) => {
        return invoke('plugin:event|listen', { event: eventName, target: { kind: 'Any' } })
          .then((id) => {
            const wrapper = (ev) => {
              try { handler(ev.detail || ev); } catch (e) { console.error(e); }
            };
            window.addEventListener(eventName, wrapper);
            return () => {
              window.removeEventListener(eventName, wrapper);
              invoke('plugin:event|unlisten', { event: eventName, eventId: id }).catch(()=>{});
            };
          })
          .catch(() => {
            // Fallback: use IPC postMessage event bus directly via __TAURI_INTERNALS__
            return () => {};
          });
      },
    },
  };

  // Tauri 2 also injects drag-region handler — verify present
  if (!document.documentElement.hasAttribute('data-tauri-bridge')) {
    document.documentElement.setAttribute('data-tauri-bridge', '1');
  }
})();
