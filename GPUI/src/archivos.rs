use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::SystemTime,
};

pub const LIMITE: u64 = 600 * 1024 * 1024;
pub const CODIFICACIONES: &[&str] = &[
    "utf-8",
    "utf-8-bom",
    "utf-16le",
    "utf-16be",
    "latin-1",
    "cp1252",
];
const CP1252: [char; 32] = [
    '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8d}', 'Ž', '\u{8f}',
    '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9d}', 'ž', 'Ÿ',
];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FinLinea {
    Lf,
    Crlf,
}
impl FinLinea {
    pub fn etiqueta(self) -> &'static str {
        match self {
            Self::Lf => "LF",
            Self::Crlf => "CRLF",
        }
    }
}
pub struct Archivo {
    pub texto: String,
    pub codificacion: String,
    pub eol: FinLinea,
    pub fecha: SystemTime,
}
pub fn leer(ruta: &Path, forzada: Option<&str>) -> Result<Archivo, String> {
    let mut fichero = fs::File::open(ruta).map_err(|e| format!("{}: {e}", ruta.display()))?;
    let meta = fichero.metadata().map_err(|e| e.to_string())?;
    if meta.len() > LIMITE {
        return Err("Archivo demasiado grande. Límite: 600 MB".into());
    }
    let mut bytes = Vec::with_capacity(meta.len() as usize);
    (&mut fichero)
        .take(LIMITE + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > LIMITE {
        return Err("Archivo demasiado grande. Límite: 600 MB".into());
    }
    let (mut texto, codificacion) = decodificar(bytes, forzada)?;
    let eol = if texto.contains("\r\n") {
        FinLinea::Crlf
    } else {
        FinLinea::Lf
    };
    if texto.contains('\r') {
        texto = texto.replace("\r\n", "\n").replace('\r', "\n");
    }
    Ok(Archivo {
        texto,
        codificacion,
        eol,
        fecha: meta.modified().map_err(|e| e.to_string())?,
    })
}
fn decodificar(mut bytes: Vec<u8>, forzada: Option<&str>) -> Result<(String, String), String> {
    let detectada = if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        "utf-8-bom"
    } else if bytes.starts_with(&[0xff, 0xfe]) {
        "utf-16le"
    } else if bytes.starts_with(&[0xfe, 0xff]) {
        "utf-16be"
    } else if std::str::from_utf8(&bytes).is_ok() {
        "utf-8"
    } else {
        "latin-1"
    };
    let codificacion = forzada.unwrap_or(detectada);
    let texto = match codificacion {
        "utf-8" | "utf-8-bom" => {
            if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
                bytes.drain(..3);
            }
            String::from_utf8(bytes).map_err(|e| format!("UTF-8 inválido: {e}"))?
        }
        "utf-16le" | "utf-16be" => {
            let datos = if bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0xfe, 0xff]) {
                &bytes[2..]
            } else {
                &bytes
            };
            if datos.len() % 2 != 0 {
                return Err("UTF-16 con un byte incompleto".into());
            }
            let unidades = datos.as_chunks::<2>().0.iter().map(|c| {
                if codificacion == "utf-16le" {
                    u16::from_le_bytes([c[0], c[1]])
                } else {
                    u16::from_be_bytes([c[0], c[1]])
                }
            });
            char::decode_utf16(unidades)
                .collect::<Result<String, _>>()
                .map_err(|e| e.to_string())?
        }
        "latin-1" => bytes.into_iter().map(char::from).collect(),
        "cp1252" => bytes
            .into_iter()
            .map(|b| {
                if (0x80..0xa0).contains(&b) {
                    CP1252[(b - 0x80) as usize]
                } else {
                    char::from(b)
                }
            })
            .collect(),
        _ => return Err(format!("Codificación desconocida: {codificacion}")),
    };
    Ok((texto, codificacion.into()))
}
pub fn escribir(
    ruta: &Path,
    texto: &str,
    codificacion: &str,
    eol: FinLinea,
) -> Result<SystemTime, String> {
    let normalizado;
    let texto = if texto.contains('\r') {
        normalizado = texto.replace("\r\n", "\n").replace('\r', "\n");
        &normalizado
    } else {
        texto
    };
    let crlf;
    let texto = if eol == FinLinea::Crlf {
        crlf = texto.replace('\n', "\r\n");
        &crlf
    } else {
        texto
    };
    if codificacion == "utf-8" {
        return escribir_bytes(ruta, texto.as_bytes());
    }
    let mut bytes = Vec::new();
    match codificacion {
        "utf-8-bom" => {
            bytes.extend_from_slice(&[0xef, 0xbb, 0xbf]);
            bytes.extend_from_slice(texto.as_bytes());
        }
        "utf-16le" | "utf-16be" => {
            let le = codificacion == "utf-16le";
            bytes.extend_from_slice(if le { &[0xff, 0xfe] } else { &[0xfe, 0xff] });
            for u in texto.encode_utf16() {
                bytes.extend_from_slice(&if le { u.to_le_bytes() } else { u.to_be_bytes() });
            }
        }
        "latin-1" | "cp1252" => {
            for c in texto.chars() {
                let byte = if codificacion == "cp1252" {
                    CP1252
                        .iter()
                        .position(|&v| v == c)
                        .map(|i| i as u8 + 0x80)
                        .or_else(|| {
                            u8::try_from(c as u32)
                                .ok()
                                .filter(|b| !(0x80..0xa0).contains(b))
                        })
                } else {
                    u8::try_from(c as u32).ok()
                };
                bytes.push(byte.ok_or_else(|| {
                    format!("«{c}» no cabe en {codificacion}; guarda como UTF-8")
                })?);
            }
        }
        _ => return Err(format!("Codificación desconocida: {codificacion}")),
    }
    escribir_bytes(ruta, &bytes)
}
pub fn escribir_bytes(ruta: &Path, bytes: &[u8]) -> Result<SystemTime, String> {
    let padre = ruta.parent().unwrap_or(Path::new("."));
    let nombre = ruta
        .file_name()
        .ok_or("La ruta necesita un nombre")?
        .to_string_lossy();
    let temporal = padre.join(format!(".{nombre}.tmp-{}", std::process::id()));
    let resultado = (|| -> std::io::Result<SystemTime> {
        let mut fichero = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporal)?;
        if let Ok(meta) = fs::metadata(ruta) {
            fichero.set_permissions(meta.permissions())?;
        }
        fichero.write_all(bytes)?;
        fichero.sync_all()?;
        fs::rename(&temporal, ruta)?;
        fs::File::open(padre)?.sync_all()?;
        fs::metadata(ruta)?.modified()
    })();
    if resultado.is_err()
        && let Err(e) = fs::remove_file(&temporal)
        && e.kind() != std::io::ErrorKind::NotFound
    {
        eprintln!(
            "bookos-notepad: no se pudo limpiar {}: {e}",
            temporal.display()
        );
    }
    resultado.map_err(|e| format!("{}: {e}", ruta.display()))
}
pub fn elegir(modo: &str, nombre: Option<&Path>, imagenes: bool) -> Result<Vec<PathBuf>, String> {
    let salida = std::env::temp_dir().join(format!(
        "notepad-pick-{}-{}.txt",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos()
    ));
    let mut orden = std::process::Command::new("bookos-explorer");
    orden
        .arg("--bookos-portal")
        .arg(&salida)
        .args(["--pick-mode", modo]);
    if modo == "open" {
        orden.arg("--pick-multiple");
    }
    if imagenes {
        for patron in [
            "*.png", "*.jpg", "*.jpeg", "*.webp", "*.gif", "*.bmp", "*.svg",
        ] {
            orden.args(["--pick-pattern", patron]);
        }
    }
    if let Some(ruta) = nombre {
        if let Some(padre) = ruta.parent() {
            orden.arg("--pick-start").arg(padre);
        }
        if let Some(nombre) = ruta.file_name() {
            orden.arg("--pick-name").arg(nombre);
        }
    }
    let estado = orden
        .status()
        .map_err(|e| format!("No se pudo abrir bookos-explorer: {e}"))?;
    let rutas = match fs::read_to_string(&salida) {
        Ok(s) => s.lines().map(PathBuf::from).collect(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => return Err(e.to_string()),
    };
    if let Err(e) = fs::remove_file(&salida)
        && e.kind() != std::io::ErrorKind::NotFound
    {
        eprintln!(
            "bookos-notepad: no se pudo limpiar {}: {e}",
            salida.display()
        );
    }
    if !estado.success() {
        return Err("El selector de archivos terminó con error".into());
    }
    Ok(rutas)
}
#[cfg(test)]
mod pruebas {
    use super::*;
    fn ruta(nombre: &str) -> PathBuf {
        std::env::temp_dir().join(format!("notepad-{}-{nombre}", std::process::id()))
    }
    #[test]
    fn codificaciones_y_finales_de_linea_se_conservan() {
        for codificacion in CODIFICACIONES {
            let ruta = ruta(codificacion);
            let texto = if *codificacion == "cp1252" {
                "España €\nsegunda"
            } else {
                "España\nsegunda"
            };
            escribir(&ruta, texto, codificacion, FinLinea::Crlf).expect("temporal escribible");
            let leido = leer(&ruta, Some(codificacion)).expect("codificación válida");
            assert_eq!(leido.texto, texto);
            assert_eq!(leido.eol, FinLinea::Crlf);
            assert!(
                !ruta
                    .with_file_name(format!(
                        ".{}.tmp-{}",
                        ruta.file_name().expect("nombre").to_string_lossy(),
                        std::process::id()
                    ))
                    .exists()
            );
            fs::remove_file(ruta).expect("temporal existente");
        }
    }
    #[test]
    fn rechaza_archivos_mayores_del_limite() {
        let ruta = ruta("grande");
        let fichero = fs::File::create(&ruta).expect("temporal");
        fichero.set_len(LIMITE + 1).expect("fichero disperso");
        assert!(leer(&ruta, None).is_err());
        fs::remove_file(ruta).expect("temporal");
    }
    #[test]
    fn no_sustituye_caracteres_ni_trunca_el_original() {
        let ruta = ruta("protegido");
        fs::write(&ruta, b"original").expect("temporal");
        assert!(escribir(&ruta, "𝄞", "latin-1", FinLinea::Lf).is_err());
        assert_eq!(fs::read(&ruta).expect("original"), b"original");
        fs::remove_file(ruta).expect("temporal");
    }
    #[test]
    fn normaliza_retorno_suelto() {
        let ruta = ruta("retorno");
        fs::write(&ruta, b"a\rb\r\nc").expect("temporal");
        assert_eq!(leer(&ruta, None).expect("texto").texto, "a\nb\nc");
        fs::remove_file(ruta).expect("temporal");
    }
}
