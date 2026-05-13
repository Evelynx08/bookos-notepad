use adblock::Engine;
use adblock::lists::{FilterSet, ParseOptions, RuleTypes};
use adblock::request::Request;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

// Bundled lists shipped with the binary
const EASYLIST: &str = include_str!("../resources/easylist.txt");
const EASYPRIVACY: &str = include_str!("../resources/easyprivacy.txt");

static ENGINE: OnceLock<Mutex<Option<Engine>>> = OnceLock::new();
static ENABLED: OnceLock<Mutex<bool>> = OnceLock::new();

fn user_filter_dir() -> PathBuf {
    let mut p = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    p.push("bookos-notepad");
    p.push("filters");
    let _ = std::fs::create_dir_all(&p);
    p
}

fn build_engine() -> Engine {
    let mut set = FilterSet::new(false);
    let opts = ParseOptions {
        rule_types: RuleTypes::All,
        ..ParseOptions::default()
    };
    let _ = set.add_filter_list(EASYLIST, opts);
    let _ = set.add_filter_list(EASYPRIVACY, opts);
    // Load user lists in ~/.config/bookos-notepad/filters/*.txt
    if let Ok(rd) = std::fs::read_dir(user_filter_dir()) {
        for entry in rd.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("txt") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let _ = set.add_filter_list(&content, opts);
                }
            }
        }
    }
    Engine::from_filter_set(set, true)
}

pub fn init() {
    let _ = ENABLED.set(Mutex::new(true));
    let _ = ENGINE.set(Mutex::new(None));
    // Lazy build on first check so startup stays fast
}

fn engine_guard() -> &'static Mutex<Option<Engine>> {
    ENGINE.get_or_init(|| Mutex::new(None))
}

fn enabled_guard() -> &'static Mutex<bool> {
    ENABLED.get_or_init(|| Mutex::new(true))
}

#[tauri::command]
pub fn adblock_set_enabled(enabled: bool) {
    if let Ok(mut g) = enabled_guard().lock() {
        *g = enabled;
    }
}

#[tauri::command]
pub fn adblock_get_enabled() -> bool {
    enabled_guard().lock().map(|g| *g).unwrap_or(true)
}

#[tauri::command]
pub fn adblock_check(url: String, source_url: String) -> serde_json::Value {
    let on = enabled_guard().lock().map(|g| *g).unwrap_or(true);
    if !on {
        return serde_json::json!({ "blocked": false, "rule": null });
    }
    let mut guard = match engine_guard().lock() {
        Ok(g) => g,
        Err(_) => return serde_json::json!({ "blocked": false, "rule": null }),
    };
    if guard.is_none() {
        *guard = Some(build_engine());
    }
    let eng = guard.as_ref().unwrap();
    let req = match Request::new(&url, &source_url, "document") {
        Ok(r) => r,
        Err(_) => return serde_json::json!({ "blocked": false, "rule": null }),
    };
    let res = eng.check_network_request(&req);
    serde_json::json!({
        "blocked": res.matched,
        "rule": res.filter,
        "exception": res.exception,
    })
}

#[tauri::command]
pub fn adblock_cosmetic(url: String) -> Vec<String> {
    let on = enabled_guard().lock().map(|g| *g).unwrap_or(true);
    if !on {
        return Vec::new();
    }
    let mut guard = match engine_guard().lock() {
        Ok(g) => g,
        Err(_) => return Vec::new(),
    };
    if guard.is_none() {
        *guard = Some(build_engine());
    }
    let eng = guard.as_ref().unwrap();
    let res = eng.url_cosmetic_resources(&url);
    // Return CSS selectors to hide
    res.hide_selectors.into_iter().collect()
}
