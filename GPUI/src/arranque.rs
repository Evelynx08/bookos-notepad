use std::{
    ffi::OsStr,
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
};

fn ruta_wayland(nombre: Option<&OsStr>, directorio: Option<&OsStr>) -> Result<PathBuf, String> {
    let nombre = nombre.filter(|n| !n.is_empty()).ok_or(
        "No hay una sesión Wayland; ejecuta la aplicación desde una terminal del escritorio BookOS",
    )?;
    let ruta = PathBuf::from(nombre);
    if ruta.is_absolute() {
        Ok(ruta)
    } else {
        let directorio = directorio
            .filter(|d| !d.is_empty())
            .ok_or("No existe XDG_RUNTIME_DIR")?;
        Ok(Path::new(directorio).join(ruta))
    }
}

pub fn comprobar_wayland() -> Result<PathBuf, String> {
    if std::env::var_os("ZED_HEADLESS").is_some() {
        return Err("ZED_HEADLESS activa el modo sin ventanas de GPUI; elimina esa variable para abrir la interfaz".into());
    }
    let ruta = ruta_wayland(
        std::env::var_os("WAYLAND_DISPLAY").as_deref(),
        std::env::var_os("XDG_RUNTIME_DIR").as_deref(),
    )?;
    UnixStream::connect(&ruta).map_err(|e| {
        format!(
            "No se puede conectar con Wayland en {}: {e}",
            ruta.display()
        )
    })?;
    Ok(ruta)
}

pub fn diagnosticar() -> Result<(), String> {
    println!("BookOS Notepad {}", env!("CARGO_PKG_VERSION"));
    let conexion = comprobar_wayland();
    match &conexion {
        Ok(ruta) => println!("Wayland: conexión disponible en {}", ruta.display()),
        Err(e) => eprintln!("Wayland: {e}"),
    }
    println!(
        "Configuración: {}",
        crate::estado::configuracion().display()
    );
    println!("Notas: {}", crate::estado::carpeta_notas().display());
    for programa in ["bookos-explorer", "bookos-settings", "xdg-open"] {
        use std::os::unix::fs::PermissionsExt;
        let disponible = std::env::var_os("PATH").is_some_and(|rutas| {
            std::env::split_paths(&rutas).any(|directorio| {
                std::fs::metadata(directorio.join(programa))
                    .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
            })
        });
        println!(
            "{programa}: {}",
            if disponible {
                "disponible"
            } else {
                "no encontrado en PATH"
            }
        );
    }
    println!("Para obtener la traza de un fallo: RUST_BACKTRACE=1 cargo run");
    println!(
        "La conexión Wayland no comprueba Vulkan; su inicialización se verifica al abrir la ventana."
    );
    conexion.map(|_| ())
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn socket_relativo_necesita_directorio_de_sesion() {
        assert_eq!(
            ruta_wayland(
                Some(OsStr::new("wayland-1")),
                Some(OsStr::new("/run/user/1000"))
            ),
            Ok(PathBuf::from("/run/user/1000/wayland-1")),
        );
        assert!(ruta_wayland(Some(OsStr::new("wayland-1")), None).is_err());
    }

    #[test]
    fn socket_absoluto_no_necesita_directorio_de_sesion() {
        assert_eq!(
            ruta_wayland(Some(OsStr::new("/tmp/wayland-prueba")), None),
            Ok(PathBuf::from("/tmp/wayland-prueba")),
        );
    }

    #[test]
    fn sesion_ausente_o_vacia_da_error() {
        assert!(ruta_wayland(None, Some(OsStr::new("/tmp"))).is_err());
        assert!(ruta_wayland(Some(OsStr::new("")), Some(OsStr::new("/tmp"))).is_err());
        assert!(ruta_wayland(Some(OsStr::new("wayland-1")), Some(OsStr::new(""))).is_err());
    }
}
