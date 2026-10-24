use std::{fs,path::{Path,PathBuf},time::{SystemTime,UNIX_EPOCH}};
use crate::{editor::nota::{Nota,Bloque},html_nota,estado};
pub struct NotaDisco {pub ruta:PathBuf,pub nota:Nota,pub fecha:SystemTime}
pub fn listar()->Result<Vec<NotaDisco>,String>{
    let carpeta=estado::carpeta_notas();fs::create_dir_all(&carpeta).map_err(|e|e.to_string())?;
    let mut notas=Vec::new();
    for entrada in fs::read_dir(carpeta).map_err(|e|e.to_string())? {
        let entrada=entrada.map_err(|e|e.to_string())?;let ruta=entrada.path();
        if ruta.extension().is_some_and(|s|s=="html") {let bytes=fs::read(&ruta).map_err(|e|e.to_string())?;let fecha=entrada.metadata().and_then(|m|m.modified()).map_err(|e|e.to_string())?;notas.push(NotaDisco{ruta,nota:html_nota::leer(&String::from_utf8_lossy(&bytes)),fecha});}
    }
    notas.sort_by_key(|n|std::cmp::Reverse(n.fecha));Ok(notas)
}
pub fn nombre_nuevo()->PathBuf{
    let mut numero=SystemTime::now().duration_since(UNIX_EPOCH).expect("fecha posterior a 1970").as_millis();let mut nombre=Vec::new();
    while numero>0{nombre.push(b"0123456789abcdefghijklmnopqrstuvwxyz"[(numero%36)as usize]);numero/=36;}nombre.reverse();
    estado::carpeta_notas().join(format!("{}.html",String::from_utf8(nombre).expect("dígitos ASCII")))
}
pub fn resumen(nota:&Nota)->(String,String){let texto=nota.plano();let mut lineas=texto.lines().map(str::trim).filter(|s|!s.is_empty()&&*s!="\u{fffc}");(lineas.next().unwrap_or("Nueva nota").into(),lineas.next().unwrap_or(if nota.bloques.iter().any(|b|matches!(b,Bloque::Imagen{..})){"Imagen"}else{"Sin más texto"}).into())}
pub fn borrar_en(carpeta:&Path,ruta:&Path)->Result<(),String>{
    if ruta.parent()!=Some(carpeta)||ruta.components().any(|c|matches!(c,std::path::Component::ParentDir))||ruta.extension().is_none_or(|s|s!="html"){return Err("La ruta no está directamente en la carpeta de notas".into());}
    fs::remove_file(ruta).map_err(|e|e.to_string())
}
#[cfg(test)]mod pruebas{use super::*;#[test]fn rechaza_rutas_fuera_de_la_carpeta(){assert!(borrar_en(Path::new("/tmp/notas"),Path::new("/tmp/notas/../otra.html")).is_err());assert!(borrar_en(Path::new("/tmp/notas"),Path::new("/tmp/otra.html")).is_err());}#[test]fn titulo_y_extracto_ignoran_lineas_vacias(){let n=html_nota::leer("<div>Título</div><div><br></div><div>Extracto</div>");assert_eq!(resumen(&n),("Título".into(),"Extracto".into()));}}
