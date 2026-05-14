// Tauri 2 bridge for external URLs (http://localhost:PORT).
// When the webview loads a non-tauri:// URL, Tauri does NOT inject the
// `window.__TAURI__` global automatically — only __TAURI_INTERNALS__.
// This script reconstructs the public API surface we use.
(function () {
  if (typeof window === 'undefined') return;
  if (window.__TAURI__ && window.__TAURI__.core && window.__TAURI__.window) return;

  const internals = window.__TAURI_INTERNALS__;
  if (!internals) {
    console.warn('[tauri-bridge] __TAURI_INTERNALS__ missing — IPC unavailable');
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
      open: (opts) => invoke('plugin:dialog|open', opts || {}),
      save: (opts) => invoke('plugin:dialog|save', opts || {}),
      message: (msg, opts) => invoke('plugin:dialog|message', Object.assign({ message: msg }, opts || {})),
      ask: (msg, opts) => invoke('plugin:dialog|ask', Object.assign({ message: msg }, opts || {})),
      confirm: (msg, opts) => invoke('plugin:dialog|confirm', Object.assign({ message: msg }, opts || {})),
    },
    fs: {
      readTextFile: (path) => invoke('plugin:fs|read_text_file', { path }),
      writeTextFile: (path, contents) => invoke('plugin:fs|write_text_file', { path, contents }),
      exists: (path) => invoke('plugin:fs|exists', { path }),
    },
  };

  // Tauri 2 also injects drag-region handler — verify present
  if (!document.documentElement.hasAttribute('data-tauri-bridge')) {
    document.documentElement.setAttribute('data-tauri-bridge', '1');
  }
})();
