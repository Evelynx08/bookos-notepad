use std::sync::OnceLock;
#[derive(Clone, Copy)]
pub struct Textos { pub ingles: bool }
impl Textos {
    pub fn cargar() -> Self {
        let idioma = std::fs::read(crate::estado::directorio_xdg("XDG_CONFIG_HOME", ".config").join("bookos/settings.json"))
            .ok().and_then(|s| serde_json::from_slice::<serde_json::Value>(&s).ok())
            .and_then(|v| v.get("AppLanguage").and_then(|s| s.as_str()).map(str::to_owned))
            .unwrap_or_else(|| std::env::var("LANG").unwrap_or_default());
        Self { ingles: !idioma.to_lowercase().starts_with("es") }
    }
    pub fn t(self, clave: &str) -> String {
        static TABLAS: OnceLock<serde_json::Value> = OnceLock::new();
        let tablas = TABLAS.get_or_init(|| serde_json::from_str(include_str!("../assets/textos.json")).expect("tablas incluidas válidas"));
        tablas[if self.ingles {"en"} else {"es"}][clave].as_str().unwrap_or(clave).to_owned()
    }
    pub fn elegir(self, es: &str, en: &str) -> String { if self.ingles { en } else { es }.into() }
}
