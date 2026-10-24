use futures::channel::mpsc::{UnboundedReceiver, unbounded};
use std::{
    fs,
    io::{Read, Write},
    os::unix::net::{UnixListener, UnixStream},
    path::{Path, PathBuf},
    time::Duration,
};
pub type Receptor = UnboundedReceiver<Vec<PathBuf>>;
pub struct Instancia {
    ruta: PathBuf,
}
impl Drop for Instancia {
    fn drop(&mut self) {
        if let Err(e) = fs::remove_file(&self.ruta) {
            eprintln!("bookos-notepad: no se pudo retirar el socket: {e}");
        }
    }
}
fn leer_rutas(conexion: impl Read) -> Result<Vec<PathBuf>, String> {
    let mut datos = Vec::new();
    conexion
        .take(1024 * 1024)
        .read_to_end(&mut datos)
        .map_err(|e| e.to_string())?;
    serde_json::from_slice(&datos).map_err(|e| e.to_string())
}
pub fn iniciar(ruta: &Path, rutas: &[PathBuf]) -> Result<Option<(Instancia, Receptor)>, String> {
    let contenido = serde_json::to_vec(rutas).map_err(|e| e.to_string())?;
    match UnixStream::connect(ruta) {
        Ok(mut conexion) => {
            conexion.write_all(&contenido).map_err(|e| e.to_string())?;
            return Ok(None);
        }
        Err(e) if e.kind() == std::io::ErrorKind::ConnectionRefused => {
            fs::remove_file(ruta).map_err(|e| e.to_string())?;
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("Instancia única: {e}")),
    }
    let escucha = UnixListener::bind(ruta).map_err(|e| e.to_string())?;
    let (tx, rx) = unbounded();
    std::thread::spawn(move || {
        for conexion in escucha.incoming() {
            let resultado = (|| -> Result<(), String> {
                let conexion = conexion.map_err(|e| e.to_string())?;
                conexion
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .map_err(|e| e.to_string())?;
                let rutas = leer_rutas(conexion)?;
                tx.unbounded_send(rutas).map_err(|e| e.to_string())
            })();
            if let Err(e) = resultado {
                eprintln!("bookos-notepad: no se pudo recibir otra instancia: {e}");
            }
            if tx.is_closed() {
                break;
            }
        }
    });
    Ok(Some((Instancia { ruta: ruta.into() }, rx)))
}
#[cfg(test)]
mod pruebas {
    use super::*;
    use futures::StreamExt;
    #[test]
    fn mensaje_entrega_rutas_sin_necesitar_socket() {
        let rutas = vec![PathBuf::from("/tmp/archivo con espacios y ñ.txt")];
        let datos = serde_json::to_vec(&rutas).expect("rutas serializables como JSON");
        let (tx, mut rx) = unbounded();
        tx.unbounded_send(leer_rutas(datos.as_slice()).expect("mensaje JSON de rutas"))
            .expect("receptor abierto");
        assert_eq!(futures::executor::block_on(rx.next()), Some(rutas));
        assert_eq!(leer_rutas(b"[]".as_slice()), Ok(Vec::new()));
    }

    #[test]
    fn mensaje_invalido_propaga_el_error() {
        assert!(leer_rutas(b"incompleto".as_slice()).is_err());
        assert!(leer_rutas(b"[1]".as_slice()).is_err());
        assert_eq!(leer_rutas(LectorFallido), Err("lectura denegada".into()));
    }

    struct LectorFallido;

    impl Read for LectorFallido {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "lectura denegada",
            ))
        }
    }

    #[test]
    #[ignore = "requiere permiso para crear sockets UNIX; ejecutar con --ignored"]
    fn segunda_instancia_entrega_rutas() {
        let ruta =
            std::env::temp_dir().join(format!("notepad-instancia-{}.sock", std::process::id()));
        let (primera, mut rx) = iniciar(&ruta, &[])
            .expect("socket temporal")
            .expect("primera instancia");
        let rutas = vec![PathBuf::from("/tmp/archivo con espacios.txt")];
        assert!(iniciar(&ruta, &rutas).expect("segunda instancia").is_none());
        assert_eq!(futures::executor::block_on(rx.next()), Some(rutas));
        drop(primera);
        assert!(!ruta.exists());
    }
}
