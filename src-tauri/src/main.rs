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

fn main() {
    adblock_engine::init();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            load_state,
            save_state,
            detect_system_theme,
            read_file,
            write_file,
            adblock_engine::adblock_check,
            adblock_engine::adblock_cosmetic,
            adblock_engine::adblock_set_enabled,
            adblock_engine::adblock_get_enabled
        ])
        .setup(|app| {
            use tauri::Manager;
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running app");
}
