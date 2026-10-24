use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf};

pub fn directorio_xdg(variable: &str, defecto: &str) -> PathBuf {
    std::env::var_os(variable)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(defecto)
        })
}
pub fn configuracion() -> PathBuf {
    directorio_xdg("XDG_CONFIG_HOME", ".config").join("bookos-notepad")
}
pub fn carpeta_notas() -> PathBuf {
    directorio_xdg("XDG_DATA_HOME", ".local/share").join("bookos-notepad/notas")
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Estado {
    pub theme: String,
    pub settings: Ajustes,
    #[serde(flatten)]
    pub resto: serde_json::Map<String, serde_json::Value>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Ajustes {
    pub font: String,
    pub wrap: bool,
    pub show_status: bool,
    pub file_accent: bool,
    pub zoom: u16,
    pub preview_mode: String,
    pub preview_url: String,
    pub ui_lang: String,
    pub shortcuts: HashMap<String, String>,
    pub restore_session: bool,
    pub session: Vec<Sesion>,
    pub auto_save: bool,
    pub auto_save_delay: u64,
    pub lista_oculta: bool,
    #[serde(flatten)]
    pub resto: serde_json::Map<String, serde_json::Value>,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Sesion {
    pub path: PathBuf,
    pub encoding: Option<String>,
}
impl Default for Ajustes {
    fn default() -> Self {
        Self {
            font: "sans".into(),
            wrap: true,
            show_status: true,
            file_accent: true,
            zoom: 100,
            preview_mode: "auto".into(),
            preview_url: String::new(),
            ui_lang: "auto".into(),
            shortcuts: HashMap::new(),
            restore_session: false,
            session: Vec::new(),
            auto_save: false,
            auto_save_delay: 2000,
            lista_oculta: false,
            resto: Default::default(),
        }
    }
}
pub struct Persistencia {
    ruta: PathBuf,
    pub error: Option<String>,
}
impl Persistencia {
    pub fn cargar() -> (Self, Estado) {
        Self::leer(configuracion().join("state.json"))
    }
    fn leer(ruta: PathBuf) -> (Self, Estado) {
        let resultado = match fs::read(&ruta) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| e.to_string()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Estado::default()),
            Err(e) => Err(e.to_string()),
        };
        match resultado {
            Ok(estado) => (Self { ruta, error: None }, estado),
            Err(e) => (
                Self {
                    ruta,
                    error: Some(e),
                },
                Estado::default(),
            ),
        }
    }
    pub fn guardar(&self, estado: &Estado) -> Result<(), String> {
        if let Some(e) = &self.error {
            return Err(format!("Estado original protegido: {e}"));
        }
        fs::create_dir_all(self.ruta.parent().expect("state.json tiene directorio"))
            .map_err(|e| e.to_string())?;
        let json = serde_json::to_vec_pretty(estado).map_err(|e| e.to_string())?;
        crate::archivos::escribir_bytes(&self.ruta, &json).map(|_| ())
    }
}
#[cfg(test)]
mod pruebas {
    use super::*;
    #[test]
    fn conserva_claves_desconocidas() {
        let original =
            r#"{"previewSplit":0.5,"settings":{"previewOpen":true,"shortcuts":{"save":"Alt+S"}}}"#;
        let estado: Estado = serde_json::from_str(original).expect("JSON de prueba válido");
        let salida = serde_json::to_value(estado).expect("estado serializable");
        assert_eq!(salida["previewSplit"], 0.5);
        assert_eq!(salida["settings"]["previewOpen"], true);
        assert_eq!(salida["settings"]["shortcuts"]["save"], "Alt+S");
    }
    #[test]
    fn lee_state_json_de_tauri() {
        let original = r#"{"settings":{"adblockEnabled":false,"autoSave":true,"autoSaveDelay":2000,"fileAccent":true,"font":"sans","listaOculta":false,"previewMode":"auto","previewOpen":false,"previewSplit":50.7,"previewUrl":"https://youtube.com","restoreSession":false,"session":[],"shortcuts":{},"showStatus":true,"uiLang":"es","wrap":true,"youtubeDirect":false,"youtubeNoCookie":true,"zoom":100},"theme":"auto"}"#;
        let ruta = std::env::temp_dir().join(format!("notepad-tauri-{}.json", std::process::id()));
        fs::write(&ruta, original).expect("temporal escribible");
        let (persistencia, estado) = Persistencia::leer(ruta.clone());
        assert!(persistencia.error.is_none());
        persistencia.guardar(&estado).expect("estado compatible");
        let salida: serde_json::Value =
            serde_json::from_slice(&fs::read(&ruta).expect("temporal existente"))
                .expect("JSON guardado");
        let entrada: serde_json::Value = serde_json::from_str(original).expect("JSON de Tauri");
        assert_eq!(salida, entrada);
        fs::remove_file(ruta).expect("temporal existente");
    }
    #[test]
    fn estado_roto_no_se_sobrescribe() {
        let ruta = std::env::temp_dir().join(format!("notepad-estado-{}.json", std::process::id()));
        fs::write(&ruta, b"{roto").expect("temporal escribible");
        let (persistencia, estado) = Persistencia::leer(ruta.clone());
        assert!(persistencia.guardar(&estado).is_err());
        assert_eq!(fs::read(&ruta).expect("original conservado"), b"{roto");
        fs::remove_file(ruta).expect("temporal existente");
    }
}
