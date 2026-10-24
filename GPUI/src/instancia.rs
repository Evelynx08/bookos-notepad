use std::{fs,io::{Read,Write},os::unix::net::{UnixListener,UnixStream},path::{Path,PathBuf},time::Duration};
use futures::channel::mpsc::{UnboundedReceiver,unbounded};
pub type Receptor = UnboundedReceiver<Vec<PathBuf>>;
pub struct Instancia {ruta:PathBuf}
impl Drop for Instancia {
    fn drop(&mut self) {
        if let Err(e)=fs::remove_file(&self.ruta) { eprintln!("bookos-notepad: no se pudo retirar el socket: {e}"); }
    }
}
pub fn iniciar(ruta:&Path,rutas:&[PathBuf])->Result<Option<(Instancia,Receptor)>,String>{
    let contenido=serde_json::to_vec(rutas).map_err(|e|e.to_string())?;
    match UnixStream::connect(ruta) {
        Ok(mut conexion)=>{conexion.write_all(&contenido).map_err(|e|e.to_string())?;return Ok(None);},
        Err(e) if e.kind()==std::io::ErrorKind::ConnectionRefused=>{fs::remove_file(ruta).map_err(|e|e.to_string())?;},
        Err(e) if e.kind()==std::io::ErrorKind::NotFound=>{},
        Err(e)=>return Err(format!("Instancia única: {e}")),
    }
    let escucha=UnixListener::bind(ruta).map_err(|e|e.to_string())?;
    let(tx,rx)=unbounded();
    std::thread::spawn(move|| {
        for conexion in escucha.incoming() {
            let resultado=(|| -> Result<(),String> {
                let mut conexion=conexion.map_err(|e|e.to_string())?;
                conexion.set_read_timeout(Some(Duration::from_secs(2))).map_err(|e|e.to_string())?;
                let mut datos=Vec::new();
                Read::by_ref(&mut conexion).take(1024*1024).read_to_end(&mut datos).map_err(|e|e.to_string())?;
                let rutas=serde_json::from_slice(&datos).map_err(|e|e.to_string())?;
                tx.unbounded_send(rutas).map_err(|e|e.to_string())
            })();
            if let Err(e)=resultado { eprintln!("bookos-notepad: no se pudo recibir otra instancia: {e}"); }
            if tx.is_closed() { break; }
        }
    });
    Ok(Some((Instancia{ruta:ruta.into()},rx)))
}
#[cfg(test)]
mod pruebas {
    use super::*;
    use futures::StreamExt;
    #[test]
    fn segunda_instancia_entrega_rutas() {
        let ruta=std::env::temp_dir().join(format!("notepad-instancia-{}.sock",std::process::id()));
        let (primera,mut rx)=iniciar(&ruta,&[]).expect("socket temporal").expect("primera instancia");
        let rutas=vec![PathBuf::from("/tmp/archivo con espacios.txt")];
        assert!(iniciar(&ruta,&rutas).expect("segunda instancia").is_none());
        assert_eq!(futures::executor::block_on(rx.next()),Some(rutas));
        drop(primera);
        assert!(!ruta.exists());
    }
}
