mod app;
mod archivos;
mod atajos;
mod dialogo;
mod imagenes;
mod instancia;
mod editor;
mod estado;
mod fechas;
mod html_nota;
mod markdown;
mod notas;
mod textos;
mod vista_previa;

use std::path::PathBuf;
use bookos_ui::{Recursos,entrada,ventana};
use gpui::{App,AppContext,px,size};
const APP_ID: &str = "bookos-notepad";

fn iconos(ruta:&str)->Option<&'static [u8]> {
    macro_rules! iconos {
        ($($n:literal),*) => { match ruta {
            $(concat!("iconos/",$n,".svg")=>Some(include_bytes!(concat!("../assets/iconos/",$n,".svg")).as_slice()),)*
            _=>None,
        }};
    }
    iconos!("borrar-nota","new-btn","settings-btn","open-btn","preview-btn","save-btn","tareas-btn","imagen-btn","lista-btn","close","maximize","minimize")
}
fn rutas_cli(args:impl Iterator<Item=String>)->Vec<PathBuf> {
    args.filter(|a|!a.starts_with('-')&&std::path::Path::new(a).is_file())
        .map(|p|std::fs::canonicalize(&p).unwrap_or_else(|e|{eprintln!("bookos-notepad: no se pudo resolver {p}: {e}");PathBuf::from(p)})).collect()
}
fn main() {
    let rutas=rutas_cli(std::env::args().skip(1));
    let instancia=std::env::var_os("XDG_RUNTIME_DIR").ok_or_else(||"No existe XDG_RUNTIME_DIR".to_string())
        .and_then(|d|instancia::iniciar(&PathBuf::from(d).join("bookos-notepad.sock"),&rutas));
    let (guardia,receptor)=match instancia {
        Ok(None)=>return,
        Ok(Some((guardia,rx)))=>(Some(guardia),Some(rx)),
        Err(e)=>{eprintln!("bookos-notepad: {e}; se inicia sin instancia única");(None,None)},
    };
    let conexion=(|| -> Result<(),String> {
        let nombre=std::env::var_os("WAYLAND_DISPLAY").ok_or("No hay una sesión Wayland; ejecuta la aplicación dentro del escritorio BookOS")?;
        let socket=PathBuf::from(nombre);
        let ruta=if socket.is_absolute(){socket}else{PathBuf::from(std::env::var_os("XDG_RUNTIME_DIR").ok_or("No existe XDG_RUNTIME_DIR")?).join(socket)};
        // GPUI conecta con unwrap y el perfil de publicación aborta los pánicos; hay que diagnosticar antes de entrar.
        std::os::unix::net::UnixStream::connect(&ruta).map(|_|()).map_err(|e|format!("No se puede conectar con Wayland en {}: {e}",ruta.display()))
    })();
    if let Err(e)=conexion {
        eprintln!("bookos-notepad: {e}");drop(guardia);std::process::exit(1);
    }
    // En desarrollo también se pueden diagnosticar los pánicos de inicialización de GPUI.
    let aplicacion=match std::panic::catch_unwind(gpui_platform::application) {
        Ok(aplicacion)=>aplicacion,
        Err(error)=>{
            let motivo=error.downcast_ref::<String>().map(String::as_str).or_else(||error.downcast_ref::<&str>().copied()).unwrap_or("fallo al inicializar GPUI");
            eprintln!("bookos-notepad: no se pudo iniciar la interfaz: {motivo}. Comprueba Wayland y el controlador Vulkan; RUST_BACKTRACE=1 muestra la traza.");
            std::process::exit(1);
        },
    };
    aplicacion.with_assets(Recursos(iconos)).run(move|cx:&mut App| {
        cx.bind_keys(entrada::atajos());
        let opciones=ventana::opciones(APP_ID,size(px(1100.),px(720.)),size(px(560.),px(400.)),cx);
        if let Err(e)=cx.open_window(opciones,move|w,cx|cx.new(|cx|app::Vista::nueva(rutas,guardia,receptor,w,cx))) {
            eprintln!("bookos-notepad: no se pudo abrir la ventana: {e}");std::process::exit(1);
        }
        cx.on_window_closed(|cx,_|if cx.windows().is_empty(){cx.quit()}).detach();
        cx.activate(true);
    });
}
