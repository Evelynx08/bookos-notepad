use std::{fs,io::{Read,Write},os::unix::net::{UnixListener,UnixStream},path::PathBuf,time::Duration};
use futures::channel::mpsc::{UnboundedReceiver,unbounded};
pub struct Instancia {ruta:PathBuf}
impl Drop for Instancia{fn drop(&mut self){let _=fs::remove_file(&self.ruta);}}
pub fn iniciar(rutas:&[PathBuf])->Result<Option<(Instancia,UnboundedReceiver<Vec<PathBuf>>)>,String>{
    let directorio=std::env::var_os("XDG_RUNTIME_DIR").ok_or("No existe XDG_RUNTIME_DIR")?;let ruta=PathBuf::from(directorio).join("bookos-notepad.sock");
    let contenido=serde_json::to_vec(rutas).map_err(|e|e.to_string())?;
    match UnixStream::connect(&ruta){Ok(mut conexion)=>{conexion.write_all(&contenido).map_err(|e|e.to_string())?;return Ok(None);},Err(e) if e.kind()==std::io::ErrorKind::ConnectionRefused=>{fs::remove_file(&ruta).map_err(|e|e.to_string())?;},Err(e)if e.kind()==std::io::ErrorKind::NotFound=>{},Err(e)=>return Err(format!("Instancia única: {e}"))}
    let escucha=UnixListener::bind(&ruta).map_err(|e|e.to_string())?;let(tx,rx)=unbounded();
    std::thread::spawn(move||{for conexion in escucha.incoming(){match conexion{Ok(mut conexion)=>{let _=conexion.set_read_timeout(Some(Duration::from_secs(2)));let mut datos=Vec::new();if conexion.by_ref().take(1024*1024).read_to_end(&mut datos).is_ok()&&let Ok(rutas)=serde_json::from_slice(&datos)&&tx.unbounded_send(rutas).is_err(){break;}},Err(e)=>{eprintln!("No se pudo recibir otra instancia: {e}");break;}}}});
    Ok(Some((Instancia{ruta},rx)))
}
