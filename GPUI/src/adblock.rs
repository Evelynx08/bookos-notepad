use adblock::{Engine,lists::{FilterSet,ParseOptions,RuleTypes},request::Request};
use std::sync::{Mutex,OnceLock};
static MOTOR:OnceLock<Mutex<Engine>>=OnceLock::new();
pub fn comprobar(url:&str)->Result<Option<String>,String>{
    let motor=MOTOR.get_or_init(||{
        let mut filtros=FilterSet::new(false);let opciones=ParseOptions{rule_types:RuleTypes::All,..Default::default()};
        let _=filtros.add_filter_list(include_str!("../../src-tauri/resources/easylist.txt"),opciones);
        let _=filtros.add_filter_list(include_str!("../../src-tauri/resources/easyprivacy.txt"),opciones);
        let carpeta=crate::estado::configuracion().join("filters");
        match std::fs::read_dir(&carpeta){Ok(entradas)=>for entrada in entradas{match entrada {Ok(entrada)if entrada.path().extension().is_some_and(|s|s=="txt")=>match std::fs::read_to_string(entrada.path()){Ok(s)=>{let _=filtros.add_filter_list(&s,opciones);},Err(e)=>eprintln!("No se pudo leer el filtro: {e}")},Ok(_)=>{},Err(e)=>eprintln!("No se pudo leer la lista de filtros: {e}")}},Err(e)if e.kind()==std::io::ErrorKind::NotFound=>{},Err(e)=>eprintln!("No se pudieron cargar los filtros: {e}")}
        Mutex::new(Engine::from_filter_set(filtros,true))
    });
    let peticion=Request::new(url,url,"document").map_err(|e|format!("URL inválida: {e:?}"))?;
    let resultado=motor.lock().map_err(|e|e.to_string())?.check_network_request(&peticion);
    Ok(resultado.matched.then(||resultado.filter.unwrap_or_else(||"Filtro de anuncios".into())))
}
