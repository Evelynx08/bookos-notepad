#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod adblock_engine;

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

#[tauri::command]
fn detect_system_theme() -> String {
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

#[tauri::command]
fn read_file(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn write_file(path: String, contents: String) -> Result<(), String> {
    fs::write(&path, contents).map_err(|e| e.to_string())
}

#[tauri::command]
fn open_url_external(url: String) -> Result<(), String> {
    Command::new("xdg-open")
        .arg(&url)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
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

fn main() {
    adblock_engine::init();
    // Fixed port for stable session storage / cookies. Falls back to random if taken.
    const PREFERRED_PORT: u16 = 17841;
    let port = if portpicker::is_free_tcp(PREFERRED_PORT) {
        PREFERRED_PORT
    } else {
        portpicker::pick_unused_port().unwrap_or(1430)
    };
    let url = format!("http://localhost:{}", port).parse().unwrap();
    tauri::Builder::default()
        .plugin(tauri_plugin_localhost::Builder::new(port).build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            load_state,
            save_state,
            detect_system_theme,
            read_file,
            write_file,
            open_url_external,
            play_in_mpv,
            check_player_available,
            adblock_engine::adblock_check,
            adblock_engine::adblock_cosmetic,
            adblock_engine::adblock_set_enabled,
            adblock_engine::adblock_get_enabled
        ])
        .setup(move |app| {
            use tauri::{WebviewWindowBuilder, WebviewUrl};
            let win = WebviewWindowBuilder::new(app, "main", WebviewUrl::External(url))
                .title("Bloc de notas")
                .inner_size(900.0, 640.0)
                .min_inner_size(560.0, 400.0)
                .decorations(false)
                .transparent(true)
                .resizable(true)
                .visible(false)
                .initialization_script(include_str!("../tauri-bridge.js"))
                .build()?;
            let _ = win.show();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running app");
}
