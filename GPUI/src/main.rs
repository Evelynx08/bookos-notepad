mod app;
mod archivos;
mod arranque;
mod atajos;
mod dialogo;
mod editor;
mod estado;
mod fechas;
mod html_nota;
mod imagenes;
mod instancia;
mod markdown;
mod notas;
mod textos;
mod vista_previa;

use bookos_ui::{Recursos, entrada, ventana};
use gpui::{App, AppContext, px, size};
use std::path::PathBuf;
const APP_ID: &str = "bookos-notepad";

fn iconos(ruta: &str) -> Option<&'static [u8]> {
    macro_rules! iconos {
        ($($n:literal),*) => { match ruta {
            $(concat!("iconos/",$n,".svg")=>Some(include_bytes!(concat!("../assets/iconos/",$n,".svg")).as_slice()),)*
            _=>None,
        }};
    }
    iconos!(
        "borrar-nota",
        "new-btn",
        "settings-btn",
        "open-btn",
        "preview-btn",
        "save-btn",
        "tareas-btn",
        "imagen-btn",
        "lista-btn",
        "close",
        "maximize",
        "minimize",
        "list",
        "number",
        "left",
        "center",
        "right",
        "clear",
        "caracteres",
        "lineas",
        "palabras"
    )
}
fn rutas_cli(args: impl Iterator<Item = String>) -> Vec<PathBuf> {
    args.filter(|a| !a.starts_with('-') && std::path::Path::new(a).is_file())
        .map(|p| {
            std::fs::canonicalize(&p).unwrap_or_else(|e| {
                eprintln!("bookos-notepad: no se pudo resolver {p}: {e}");
                PathBuf::from(p)
            })
        })
        .collect()
}
fn main() {
    if std::env::args().skip(1).any(|a| a == "--diagnostico") {
        if arranque::diagnosticar().is_err() {
            std::process::exit(1);
        }
        return;
    }
    if std::env::args().skip(1).any(|a| a == "--help" || a == "-h") {
        println!(
            "Uso: bookos-notepad [ARCHIVOS…]\n     bookos-notepad --diagnostico\n\n--diagnostico comprueba el entorno sin abrir ventanas ni modificar las notas."
        );
        return;
    }
    let rutas = rutas_cli(std::env::args().skip(1));
    let instancia = std::env::var_os("XDG_RUNTIME_DIR")
        .ok_or_else(|| "No existe XDG_RUNTIME_DIR".to_string())
        .and_then(|d| instancia::iniciar(&PathBuf::from(d).join("bookos-notepad.sock"), &rutas));
    let (guardia, receptor) = match instancia {
        Ok(None) => {
            eprintln!("bookos-notepad: solicitud enviada a la instancia abierta");
            return;
        }
        Ok(Some((guardia, rx))) => (Some(guardia), Some(rx)),
        Err(e) => {
            eprintln!("bookos-notepad: {e}; se inicia sin instancia única");
            (None, None)
        }
    };
    if let Err(e) = arranque::comprobar_wayland() {
        eprintln!(
            "bookos-notepad: {e}. Ejecuta cargo run -- --diagnostico para revisar el entorno."
        );
        drop(guardia);
        std::process::exit(1);
    }
    // En desarrollo también se pueden diagnosticar los pánicos de inicialización de GPUI.
    let aplicacion = match std::panic::catch_unwind(gpui_platform::application) {
        Ok(aplicacion) => aplicacion,
        Err(error) => {
            let motivo = error
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| error.downcast_ref::<&str>().copied())
                .unwrap_or("fallo al inicializar GPUI");
            eprintln!(
                "bookos-notepad: no se pudo iniciar la interfaz: {motivo}. Comprueba Wayland y el controlador Vulkan; RUST_BACKTRACE=1 muestra la traza."
            );
            drop(guardia);
            std::process::exit(1);
        }
    };
    aplicacion
        .with_assets(Recursos(iconos))
        .run(move |cx: &mut App| {
            cx.bind_keys(entrada::atajos());
            let opciones = ventana::opciones(
                APP_ID,
                size(px(1100.), px(720.)),
                size(px(560.), px(400.)),
                cx,
            );
            if let Err(e) = cx.open_window(opciones, move |w, cx| {
                cx.new(|cx| app::Vista::nueva(rutas, guardia, receptor, w, cx))
            }) {
                eprintln!("bookos-notepad: no se pudo abrir la ventana: {e}");
                std::process::exit(1);
            }
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    cx.quit()
                }
            })
            .detach();
            cx.activate(true);
        });
}
