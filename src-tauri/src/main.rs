#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn config_path() -> PathBuf {
    let mut p = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    p.push("bookos-notepad");
    let _ = fs::create_dir_all(&p);
    p.push("state.json");
    p
}

#[tauri::command]
fn load_state() -> serde_json::Value {
    let p = config_path();
    if let Ok(s) = fs::read_to_string(&p) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
            return v;
        }
    }
    serde_json::json!({ "theme": "auto", "settings": {} })
}

#[tauri::command]
fn save_state(state: serde_json::Value) -> Result<(), String> {
    let p = config_path();
    let s = serde_json::to_string_pretty(&state).map_err(|e| e.to_string())?;
    fs::write(&p, s).map_err(|e| e.to_string())
}

/// El tema del escritorio, preguntado al portal XDG.
///
/// Va **antes** que `kreadconfig6`: esa orden solo sabe de Plasma y lee el
/// `kdeglobals` del usuario, que en una sesión BookOS no lo escribe nadie —el
/// tema se elige en la tarjeta de Apariencia y vive en `panel.conf`—, así que
/// la app se quedaba con el tema con el que se instaló el sistema. El portal
/// lo sirve la sesión que esté corriendo: BookOS desde el propio compositor,
/// y Plasma y GNOME también, de modo que esto funciona en las tres.
///
/// `color-scheme`: 1 oscuro, 2 claro, 0 sin preferencia. `gdbus` viene con
/// GLib, que ya es dependencia de cualquier app Tauri: no añade nada.
fn portal_color_scheme() -> Option<String> {
    let salida = std::process::Command::new("gdbus")
        .args([
            "call",
            "--session",
            "--dest",
            "org.freedesktop.portal.Desktop",
            "--object-path",
            "/org/freedesktop/portal/desktop",
            "--method",
            "org.freedesktop.portal.Settings.ReadOne",
            "org.freedesktop.appearance",
            "color-scheme",
        ])
        .output()
        .ok()?;
    // Contesta «(<uint32 1>,)»; el 0 es «sin preferencia» y no es respuesta.
    let texto = String::from_utf8_lossy(&salida.stdout);
    let n = texto.split("uint32").nth(1)?.trim_start().chars().next()?;
    match n {
        '1' => Some("dark".to_string()),
        '2' => Some("light".to_string()),
        _ => None,
    }
}

#[tauri::command]
fn detect_system_theme() -> String {
    if let Some(tema) = portal_color_scheme() {
        return tema;
    }
    let kde_attempts = [
        ("kreadconfig6", &["--group", "General", "--key", "ColorScheme"][..]),
        ("kreadconfig5", &["--group", "General", "--key", "ColorScheme"][..]),
    ];
    for (bin, args) in kde_attempts {
        if let Ok(out) = Command::new(bin).args(args).output() {
            let s = String::from_utf8_lossy(&out.stdout).to_lowercase();
            if s.contains("dark") { return "dark".into(); }
            if s.contains("light") { return "light".into(); }
        }
    }
    if let Ok(out) = Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "color-scheme"])
        .output()
    {
        let s = String::from_utf8_lossy(&out.stdout).to_lowercase();
        if s.contains("dark") { return "dark".into(); }
        if s.contains("light") { return "light".into(); }
    }
    "auto".into()
}

/// Upper bound on what we'll load into memory. Bigger than this and we refuse
/// rather than let the webview OOM-kill the whole app. ~600 MB.
const MAX_OPEN_BYTES: u64 = 600 * 1024 * 1024;

/// Modification time of a file as fractional seconds since the Unix epoch.
/// Used to notice that a file changed on disk underneath an open tab.
fn mtime_secs(meta: &fs::Metadata) -> f64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

/// Lightweight stat: does the file exist, its size and mtime. The frontend
/// calls this on window focus to detect external changes without re-reading.
#[tauri::command]
fn file_meta(path: String) -> serde_json::Value {
    match fs::metadata(&path) {
        Ok(m) => serde_json::json!({
            "exists": true, "size": m.len(), "mtime": mtime_secs(&m)
        }),
        Err(_) => serde_json::json!({ "exists": false, "size": 0, "mtime": 0.0 }),
    }
}

/// Read a file, detect encoding (utf-8, utf-8-bom, utf-16le, utf-16be, latin-1)
/// and return { text, encoding, eol, mtime, size }.
#[tauri::command]
fn read_file(path: String, encoding: Option<String>) -> Result<serde_json::Value, String> {
    let meta = fs::metadata(&path).map_err(|e| format!("stat({}): {}", path, e))?;
    if meta.len() > MAX_OPEN_BYTES {
        return Err(format!(
            "Archivo demasiado grande ({:.0} MB). Límite: {} MB.",
            meta.len() as f64 / (1024.0 * 1024.0),
            MAX_OPEN_BYTES / (1024 * 1024)
        ));
    }
    let mtime = mtime_secs(&meta);
    let size = meta.len();
    let bytes = fs::read(&path).map_err(|e| format!("read({}): {}", path, e))?;
    // Detect from BOM unless explicit encoding requested
    let forced = encoding.as_deref();
    let (text, enc) = if forced == Some("utf-16le") {
        (decode_utf16(&bytes, true), "utf-16le".to_string())
    } else if forced == Some("utf-16be") {
        (decode_utf16(&bytes, false), "utf-16be".to_string())
    } else if forced == Some("latin-1") {
        (bytes.iter().map(|&b| b as char).collect::<String>(), "latin-1".to_string())
    } else if forced == Some("cp1252") {
        (decode_cp1252(&bytes), "cp1252".to_string())
    } else if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        (String::from_utf8_lossy(&bytes[3..]).into_owned(), "utf-8-bom".to_string())
    } else if bytes.starts_with(&[0xFF, 0xFE]) {
        (decode_utf16(&bytes[2..], true), "utf-16le".to_string())
    } else if bytes.starts_with(&[0xFE, 0xFF]) {
        (decode_utf16(&bytes[2..], false), "utf-16be".to_string())
    } else if std::str::from_utf8(&bytes).is_ok() {
        (String::from_utf8(bytes.clone()).unwrap(), "utf-8".to_string())
    } else {
        // Fallback latin-1 lossless
        (bytes.iter().map(|&b| b as char).collect::<String>(), "latin-1".to_string())
    };
    let eol = if text.contains("\r\n") { "crlf" } else { "lf" };
    Ok(serde_json::json!({ "text": text, "encoding": enc, "eol": eol, "mtime": mtime, "size": size }))
}

fn decode_utf16(bytes: &[u8], le: bool) -> String {
    let mut iter = bytes.chunks_exact(2).map(|c| {
        if le { u16::from_le_bytes([c[0], c[1]]) } else { u16::from_be_bytes([c[0], c[1]]) }
    });
    let v: Vec<u16> = iter.by_ref().collect();
    String::from_utf16_lossy(&v)
}

fn decode_cp1252(bytes: &[u8]) -> String {
    // Windows-1252 mapping (0x80-0x9F have specific chars)
    const MAP: [char; 32] = [
        '\u{20AC}','?','\u{201A}','\u{0192}','\u{201E}','\u{2026}','\u{2020}','\u{2021}',
        '\u{02C6}','\u{2030}','\u{0160}','\u{2039}','\u{0152}','?','\u{017D}','?',
        '?','\u{2018}','\u{2019}','\u{201C}','\u{201D}','\u{2022}','\u{2013}','\u{2014}',
        '\u{02DC}','\u{2122}','\u{0161}','\u{203A}','\u{0153}','?','\u{017E}','\u{0178}',
    ];
    bytes.iter().map(|&b| {
        if b < 0x80 { b as char }
        else if b < 0xA0 { MAP[(b - 0x80) as usize] }
        else { b as char }
    }).collect()
}

/// Write a file. `eol` ("lf"|"crlf") restores the line ending the file had when
/// opened, so editing a Windows file doesn't silently rewrite it as LF. The
/// write is atomic: contents go to a sibling temp file that is fsync'd and then
/// renamed over the target, so a crash or full disk mid-write can't truncate or
/// corrupt the original. Returns the new mtime.
#[tauri::command]
fn write_file(
    path: String,
    contents: String,
    encoding: Option<String>,
    eol: Option<String>,
) -> Result<f64, String> {
    // Normalize to LF first, then apply the requested ending.
    let normalized = contents.replace("\r\n", "\n").replace('\r', "\n");
    let contents = if eol.as_deref() == Some("crlf") {
        normalized.replace('\n', "\r\n")
    } else {
        normalized
    };

    let enc = encoding.unwrap_or_else(|| "utf-8".into());
    let bytes: Vec<u8> = match enc.as_str() {
        "utf-8-bom" => {
            let mut v = vec![0xEF, 0xBB, 0xBF];
            v.extend_from_slice(contents.as_bytes());
            v
        }
        "utf-16le" => {
            let mut v = vec![0xFF, 0xFE];
            for u in contents.encode_utf16() { v.extend_from_slice(&u.to_le_bytes()); }
            v
        }
        "utf-16be" => {
            let mut v = vec![0xFE, 0xFF];
            for u in contents.encode_utf16() { v.extend_from_slice(&u.to_be_bytes()); }
            v
        }
        "latin-1" => contents.chars().map(|c| if (c as u32) < 256 { c as u8 } else { b'?' }).collect(),
        _ => contents.into_bytes(),
    };

    let target = PathBuf::from(&path);
    let dir = target.parent().unwrap_or_else(|| std::path::Path::new("."));
    let fname = target.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "untitled".into());
    let tmp = dir.join(format!(".{}.tmp-{}", fname, std::process::id()));

    // Write + flush + fsync the temp file before swapping it in.
    {
        use std::io::Write;
        let mut f = fs::File::create(&tmp).map_err(|e| format!("tmp create: {}", e))?;
        f.write_all(&bytes).map_err(|e| { let _ = fs::remove_file(&tmp); format!("write: {}", e) })?;
        f.flush().map_err(|e| { let _ = fs::remove_file(&tmp); format!("flush: {}", e) })?;
        let _ = f.sync_all();
    }
    if let Err(e) = fs::rename(&tmp, &target) {
        // Cross-device rename can fail; fall back to a direct copy then cleanup.
        let copy = fs::copy(&tmp, &target).map(|_| ());
        let _ = fs::remove_file(&tmp);
        copy.map_err(|_| format!("rename: {}", e))?;
    }

    let mtime = fs::metadata(&target).map(|m| mtime_secs(&m)).unwrap_or(0.0);
    Ok(mtime)
}

/// Carpeta de las notas. Cada nota es un `.html` suelto y no una entrada de un
/// índice único: así guardar una no reescribe las demás, y un fallo a medio
/// escribir estropea como mucho esa nota.
fn carpeta_notas() -> Result<PathBuf, String> {
    let carpeta = dirs::data_dir()
        .ok_or("no hay carpeta de datos del usuario")?
        .join("bookos-notepad/notas");
    fs::create_dir_all(&carpeta).map_err(|e| format!("{}: {}", carpeta.display(), e))?;
    Ok(carpeta)
}

#[derive(serde::Serialize)]
struct Nota {
    ruta: String,
    html: String,
    mtime: f64,
}

#[derive(serde::Serialize)]
struct Notas {
    carpeta: String,
    notas: Vec<Nota>,
}

/// Todas las notas con su contenido, en una sola llamada: la lista necesita el
/// título y el extracto de cada una, que salen del propio texto.
#[tauri::command]
fn notas_listar() -> Result<Notas, String> {
    let carpeta = carpeta_notas()?;
    let notas = fs::read_dir(&carpeta)
        .map_err(|e| format!("{}: {}", carpeta.display(), e))?
        .filter_map(Result::ok)
        .map(|entrada| entrada.path())
        .filter(|ruta| ruta.extension().is_some_and(|ext| ext == "html"))
        .map(|ruta| {
            let bytes = fs::read(&ruta).map_err(|e| format!("{}: {}", ruta.display(), e))?;
            let meta = fs::metadata(&ruta).map_err(|e| format!("{}: {}", ruta.display(), e))?;
            Ok(Nota {
                ruta: ruta.to_string_lossy().into_owned(),
                // Con pérdida y no `read_to_string`: una nota con bytes rotos
                // se abre con «�» en vez de dejar la lista entera sin cargar.
                html: String::from_utf8_lossy(&bytes).into_owned(),
                mtime: mtime_secs(&meta),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(Notas { carpeta: carpeta.to_string_lossy().into_owned(), notas })
}

/// Borra una nota. La ruta viene del frontend, así que solo se acepta si cae
/// directamente dentro de la carpeta de notas.
#[tauri::command]
fn nota_borrar(ruta: String) -> Result<(), String> {
    let ruta = PathBuf::from(ruta);
    if ruta.parent() != Some(carpeta_notas()?.as_path()) {
        return Err(format!("{} no es una nota", ruta.display()));
    }
    fs::remove_file(&ruta).map_err(|e| format!("{}: {}", ruta.display(), e))
}

/// Los bytes de una imagen, para insertarla en una nota. Respuesta binaria y no
/// `Vec<u8>`: serializado como JSON viaja como una lista de números decimales.
#[tauri::command]
fn leer_imagen(ruta: String) -> Result<tauri::ipc::Response, String> {
    fs::read(&ruta)
        .map(tauri::ipc::Response::new)
        .map_err(|e| format!("{}: {}", ruta, e))
}

// ── Vista previa ─────────────────────────────────────────────────────────
// Una ventana hija de la principal, al estilo de Quick Look. El editor manda el
// documento con `vista_previa`; la ventana lo recibe por evento si ya existe, y
// si se está creando lo pide ella con `vista_previa_doc` al terminar de cargar:
// el evento habría salido antes de que hubiera nadie escuchando.

const VISTA_PREVIA: &str = "vista-previa";

/// El último documento enviado a la vista previa.
#[derive(Default)]
struct VistaPrevia(std::sync::Mutex<Option<serde_json::Value>>);

// `async` para que la ventana no se cree en el hilo principal desde un comando
// síncrono, que es donde la documentación de Tauri avisa de bloqueos.
#[tauri::command]
async fn vista_previa(
    app: tauri::AppHandle,
    estado: tauri::State<'_, VistaPrevia>,
    doc: serde_json::Value,
) -> Result<(), String> {
    use tauri::{Emitter, Manager, WebviewUrl, WebviewWindowBuilder};
    let existe = app.get_webview_window(VISTA_PREVIA).is_some();
    if existe {
        app.emit("vista-previa-doc", &doc).map_err(|e| e.to_string())?;
    }
    *estado.0.lock().map_err(|e| e.to_string())? = Some(doc);
    if existe {
        return Ok(());
    }
    let principal = app.get_webview_window("main").ok_or("no hay ventana principal")?;
    let ventana = WebviewWindowBuilder::new(&app, VISTA_PREVIA, WebviewUrl::App("preview.html".into()))
        .title("Vista previa")
        .inner_size(760.0, 560.0)
        .min_inner_size(440.0, 320.0)
        .decorations(false)
        .transparent(true)
        .parent(&principal)
        .map_err(|e| e.to_string())?
        .initialization_script(include_str!("../tauri-api.js"))
        .initialization_script(include_str!("../tauri-bridge.js"))
        .build()
        .map_err(|e| e.to_string())?;
    // También se cierra desde el escritorio, sin pasar por su botón: el editor
    // tiene que enterarse para apagar el suyo y dejar de mandar el documento.
    ventana.on_window_event(move |evento| {
        if matches!(evento, tauri::WindowEvent::Destroyed) {
            let _ = app.emit("vista-previa-cerrada", ());
        }
    });
    Ok(())
}

#[tauri::command]
fn vista_previa_doc(estado: tauri::State<'_, VistaPrevia>) -> Result<Option<serde_json::Value>, String> {
    // Se clona: la copia guardada sirve también si la ventana se recarga.
    Ok(estado.0.lock().map_err(|e| e.to_string())?.clone())
}

#[tauri::command]
fn vista_previa_cerrar(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    match app.get_webview_window(VISTA_PREVIA) {
        Some(ventana) => ventana.close().map_err(|e| e.to_string()),
        None => Ok(()),
    }
}

/// La vista previa pide al editor el documento tal como está ahora.
#[tauri::command]
fn vista_previa_pedir(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Emitter;
    app.emit("vista-previa-pide", ()).map_err(|e| e.to_string())
}

/// Un ajuste cambiado en la vista previa vuelve al editor, que es el único que
/// escribe state.json.
#[tauri::command]
fn vista_previa_ajuste(app: tauri::AppHandle, clave: String, valor: serde_json::Value) -> Result<(), String> {
    use tauri::Emitter;
    app.emit("vista-previa-ajuste", (clave, valor)).map_err(|e| e.to_string())
}

#[tauri::command]
fn open_url_external(url: String) -> Result<(), String> {
    Command::new("xdg-open")
        .arg(&url)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Launch bookos-settings on the "updates" page. Uses the temp-file
/// single-instance signal that bookos-settings reads at startup.
#[tauri::command]
fn open_bookos_updates() -> Result<(), String> {
    // Write the temp file FIRST so a running bookos-settings picks it up.
    let _ = std::fs::write("/tmp/bookos-start-page", "actualizacion");
    // Launch (or focus if already running — bookos-settings is single-instance).
    Command::new("bookos-settings")
        .args(["--page", "actualizacion"])
        .spawn()
        .map(|_| ())
        .map_err(|e| {
            // Fallback: try xdg-open .desktop
            let _ = Command::new("xdg-open").arg("application://bookos-settings.desktop").spawn();
            format!("bookos-settings launch failed: {}", e)
        })
}

/// Files passed on the command line (e.g. "Open With → bookos-notepad foo.py").
/// Pull model: the frontend calls this once on boot, so it's immune to the
/// startup race a fire-once `emit` had (on cold start the webview often isn't
/// listening yet at launch, dropping the event and the file never opened).
#[tauri::command]
fn take_cli_files() -> Vec<String> {
    std::env::args()
        .skip(1)
        .filter(|a| !a.starts_with('-') && std::path::Path::new(a).is_file())
        .map(|p| std::fs::canonicalize(&p)
            .map(|c| c.to_string_lossy().to_string())
            .unwrap_or(p))
        .collect()
}

/// Returns current version + package name; frontend does GitHub Releases API call.
#[tauri::command]
fn app_version_info() -> serde_json::Value {
    serde_json::json!({
        "current": env!("CARGO_PKG_VERSION"),
        "package": env!("CARGO_PKG_NAME"),
    })
}

/// Launch mpv as a floating window (acts like PiP — always-on-top, no border).
/// Requires yt-dlp for YouTube URLs.
/// Optional geometry: x, y, w, h in screen pixels — places mpv exactly over that rect.
#[tauri::command]
fn play_in_mpv(url: String, x: Option<i32>, y: Option<i32>, w: Option<i32>, h: Option<i32>) -> Result<(), String> {
    let geometry = match (x, y, w, h) {
        (Some(x), Some(y), Some(w), Some(h)) if w > 0 && h > 0 => {
            format!("{}x{}+{}+{}", w.max(160), h.max(120), x.max(0), y.max(0))
        }
        _ => "480x270-30-30".to_string(),
    };
    Command::new("mpv")
        .args([
            "--ontop",
            "--no-border",
            &format!("--geometry={}", geometry),
            "--save-position-on-quit",
            "--keep-open=no",
            "--ytdl-format=bestvideo[height<=?720]+bestaudio/best[height<=?720]",
            &url,
        ])
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("mpv falló: {}. ¿yt-dlp instalado?", e))
}

#[tauri::command]
fn check_player_available() -> serde_json::Value {
    let has_mpv = Command::new("which").arg("mpv").output()
        .map(|o| o.status.success()).unwrap_or(false);
    let has_ytdlp = Command::new("which").arg("yt-dlp").output()
        .map(|o| o.status.success()).unwrap_or(false);
    serde_json::json!({
        "mpv": has_mpv,
        "ytdlp": has_ytdlp,
        "canPlayYoutube": has_mpv && has_ytdlp
    })
}

/// Quita el zoom de página que WebKitGTK hace con el pellizco del touchpad.
///
/// No hay ajuste para eso ni en WebKit ni en Tauri, y la página no se entera:
/// el pellizco lo consume un `GtkGestureZoom` de la propia vista y llega como
/// `setMagnification`, no como `wheel` ni `touch*`, así que ningún
/// `preventDefault` lo frena. Se apaga ese gesto y ningún otro: con fase `None`
/// GTK deja de pasarle eventos (gtkeventcontroller.c, 3.24). Va en
/// `on_page_load` para cubrir también las ventanas que se abran después.
fn desactivar_zoom_por_pellizco<R: tauri::Runtime>(
    webview: &tauri::Webview<R>,
    _: &tauri::webview::PageLoadPayload<'_>,
) {
    #[cfg(target_os = "linux")]
    let _ = webview.with_webview(|webview| {
        use gtk::glib::translate::from_glib_none;
        use gtk::prelude::*;

        // WebKitGTK guarda ahí el gesto (WebKitWebViewBase.cpp, 2.52.5). No es
        // API pública: si la clave cambia, no se encuentra y el zoom vuelve.
        // Lo guardado es un puntero C a un GObject, no un tipo de Rust.
        let Some(gesto) = (unsafe { webview.inner().data::<gtk::ffi::GtkGesture>("wk-view-zoom-gesture") }) else {
            return;
        };
        let gesto: gtk::Gesture = unsafe { from_glib_none(gesto.as_ptr()) };
        gesto.set_propagation_phase(gtk::PropagationPhase::None);
    });
}

/// Qué quiere elegir el frontend. Cada campo es una bandera `--pick-*` de
/// `bookos-explorer`.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Eleccion {
    modo: ModoEleccion,
    #[serde(default)]
    multiple: bool,
    #[serde(default)]
    patrones: Vec<String>,
    nombre_filtro: Option<String>,
    titulo: Option<String>,
    nombre: Option<String>,
    inicio: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "lowercase")]
enum ModoEleccion {
    Open,
    Save,
    Folder,
}

/// Selector de archivos del sistema: `bookos-explorer` en modo portal
/// (AI-DESIGN-SYSTEM §9), nunca el diálogo de GTK. `None` es que el usuario
/// canceló. Sin el explorador instalado devuelve el error `sin-explorador` y
/// el frontend cae al diálogo de Tauri.
#[tauri::command]
async fn elegir_ruta(eleccion: Eleccion) -> Result<Option<Vec<String>>, String> {
    let salida = std::env::temp_dir().join(format!(
        "{}-pick-{}-{}.txt",
        env!("CARGO_PKG_NAME"),
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos()
    ));
    let mut orden = std::process::Command::new("bookos-explorer");
    orden.arg("--bookos-portal").arg(&salida).arg("--pick-mode").arg(match eleccion.modo {
        ModoEleccion::Open => "open",
        ModoEleccion::Save => "save",
        ModoEleccion::Folder => "folder",
    });
    if eleccion.multiple {
        orden.arg("--pick-multiple");
    }
    for patron in &eleccion.patrones {
        orden.arg("--pick-pattern").arg(patron);
    }
    for (bandera, valor) in [
        ("--pick-filter-name", &eleccion.nombre_filtro),
        ("--pick-title", &eleccion.titulo),
        ("--pick-name", &eleccion.nombre),
        ("--pick-start", &eleccion.inicio),
    ] {
        if let Some(valor) = valor {
            orden.arg(bandera).arg(valor);
        }
    }
    let mut hijo = match orden.spawn() {
        Ok(hijo) => hijo,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Err("sin-explorador".into()),
        Err(e) => return Err(e.to_string()),
    };
    // Se espera a que el explorador salga y solo entonces se lee: escribe el
    // fichero justo antes de `exit`, y leerlo en cuanto aparece (lo que hace
    // el Viewer) puede pillarlo vacío y tomarlo por una cancelación. Es lo
    // mismo que hace el portal del compositor.
    tauri::async_runtime::spawn_blocking(move || {
        let fin = hijo.wait();
        let texto = std::fs::read_to_string(&salida);
        // Sin fichero es que se canceló: no hay nada que borrar.
        let _ = std::fs::remove_file(&salida);
        fin.map_err(|e| e.to_string())?;
        let rutas: Vec<String> = texto.unwrap_or_default().lines().map(str::to_owned).collect();
        Ok((!rutas.is_empty()).then_some(rutas))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn bookos_language() -> String {
    let locale = std::env::var("LANG").unwrap_or_else(|_| "en".into());
    let Some(home) = std::env::var_os("HOME") else {
        return if locale.to_lowercase().starts_with("es") { "es" } else { "en" }.into();
    };
    let path = std::path::PathBuf::from(home).join(".config/bookos/settings.json");
    let language = std::fs::read_to_string(path)
        .ok()
        .and_then(|json| serde_json::from_str::<serde_json::Value>(&json).ok())
        .and_then(|settings| settings.get("AppLanguage").and_then(|value| value.as_str()).map(str::to_owned));
    match language.as_deref() {
        Some("es") => "es".into(),
        Some("en") => "en".into(),
        _ if locale.to_lowercase().starts_with("es") => "es".into(),
        _ => "en".into(),
    }
}

fn main() {
    // Force dark GTK theme for native dialogs (open/save file pickers).
    // The webkit2gtk file chooser inherits the GTK theme; without this it
    // shows the system light theme. Respect an existing GTK_THEME override.
    if std::env::var_os("GTK_THEME").is_none() {
        std::env::set_var("GTK_THEME", "catppuccin-bookos-dark-blue-standard+default");
    }
    let resultado = tauri::Builder::default()
        .on_page_load(desactivar_zoom_por_pellizco)
        // Single instance MUST be registered first. A second launch (e.g. a new
        // "Open With") routes its files into the already-running window as tabs
        // instead of spawning a duplicate process/window.
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            use tauri::{Manager, Emitter};
            let files: Vec<String> = args.into_iter()
                .skip(1)
                .filter(|a| !a.starts_with('-') && std::path::Path::new(a).is_file())
                .map(|p| std::fs::canonicalize(&p)
                    .map(|c| c.to_string_lossy().to_string())
                    .unwrap_or(p))
                .collect();
            if let Some(win) = app.get_webview_window("main") {
                let _ = win.unminimize();
                let _ = win.set_focus();
                if !files.is_empty() { let _ = win.emit("open-files", files); }
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(VistaPrevia::default())
        .invoke_handler(tauri::generate_handler![
            elegir_ruta,
            bookos_palette_css,
            load_state,
            save_state,
            bookos_language,
            detect_system_theme,
            read_file,
            write_file,
            file_meta,
            notas_listar,
            nota_borrar,
            leer_imagen,
            vista_previa,
            vista_previa_doc,
            vista_previa_cerrar,
            vista_previa_pedir,
            vista_previa_ajuste,
            open_url_external,
            play_in_mpv,
            check_player_available,
            open_bookos_updates,
            app_version_info,
            take_cli_files
        ])
        .setup(|app| {
            use tauri::{WebviewWindowBuilder, WebviewUrl};
            let win = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("Bloc de notas")
                // Con la lista de notas a la izquierda (306px), a 900 de ancho
                // la barra de formato de un archivo (~677px) no cabía en una fila.
                .inner_size(1040.0, 680.0)
                .min_inner_size(560.0, 400.0)
                .decorations(false)
                .transparent(true)
                .resizable(true)
                .visible(false)
                .initialization_script(include_str!("../tauri-api.js"))
                .initialization_script(include_str!("../tauri-bridge.js"))
                .build()?;

            // CLI files are delivered via the `take_cli_files` command the
            // frontend pulls on boot — no fragile fire-once emit/race here.
            // Sin la principal la vista previa no pinta nada, y viva mantendría
            // el proceso abierto.
            let app = app.handle().clone();
            win.on_window_event(move |evento| {
                if matches!(evento, tauri::WindowEvent::Destroyed) {
                    use tauri::Manager;
                    if let Some(vista) = app.get_webview_window(VISTA_PREVIA) {
                        let _ = vista.close();
                    }
                }
            });
            win.show()?;
            Ok(())
        })
        .run(tauri::generate_context!());
    if let Err(error) = resultado {
        eprintln!("bookos-notepad: no se pudo iniciar la interfaz: {error}");
        std::process::exit(1);
    }
}

// ── Color dinámico ───────────────────────────────────────────────────────
// La paleta la genera BookOS Settings desde el fondo de pantalla y la deja en
// ~/.config/bookos/palette.css. Aquí solo se lee: quien tiñe es la hoja, que
// el cliente bookos-palette.js inyecta al final de <head>.
#[tauri::command]
fn bookos_palette_css() -> String {
    std::env::var("HOME").ok()
        .map(|h| std::path::Path::new(&h).join(".config/bookos/palette.css"))
        .and_then(|p| std::fs::read_to_string(p).ok())
        .unwrap_or_default()
}

