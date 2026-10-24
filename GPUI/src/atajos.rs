use gpui::KeyDownEvent;
use std::collections::HashMap;
pub struct Accion{pub id:&'static str,pub seccion:&'static str,pub etiqueta:&'static str,pub tecla:&'static str}
pub const ACCIONES:&[Accion]=&[
Accion{id:"new-tab",seccion:"Archivo",etiqueta:"Nueva nota",tecla:"Ctrl+N"},
Accion{id:"new-tab-alt",seccion:"Archivo",etiqueta:"Nueva pestaña de archivo",tecla:"Ctrl+T"},
Accion{id:"open-file",seccion:"Archivo",etiqueta:"Abrir archivo",tecla:"Ctrl+O"},
Accion{id:"save",seccion:"Archivo",etiqueta:"Guardar",tecla:"Ctrl+S"},
Accion{id:"save-as",seccion:"Archivo",etiqueta:"Guardar como",tecla:"Ctrl+Shift+S"},
Accion{id:"close-tab",seccion:"Archivo",etiqueta:"Cerrar pestaña",tecla:"Ctrl+W"},
Accion{id:"next-tab-mru",seccion:"Navegación",etiqueta:"Pestaña anterior usada",tecla:"Ctrl+Tab"},
Accion{id:"prev-tab",seccion:"Navegación",etiqueta:"Pestaña previa",tecla:"Ctrl+Shift+Tab"},
Accion{id:"preview",seccion:"Vista",etiqueta:"Vista previa",tecla:"Ctrl+P"},
Accion{id:"settings",seccion:"Vista",etiqueta:"Abrir/cerrar ajustes",tecla:"Ctrl+,"},
Accion{id:"fullscreen",seccion:"Vista",etiqueta:"Pantalla completa",tecla:"F11"},
Accion{id:"shortcuts",seccion:"Vista",etiqueta:"Mostrar atajos",tecla:"Ctrl+/"},
Accion{id:"bold",seccion:"Formato",etiqueta:"Negrita",tecla:"Ctrl+B"},
Accion{id:"italic",seccion:"Formato",etiqueta:"Cursiva",tecla:"Ctrl+I"},
Accion{id:"underline",seccion:"Formato",etiqueta:"Subrayado",tecla:"Ctrl+U"},
Accion{id:"undo",seccion:"Edición",etiqueta:"Deshacer",tecla:"Ctrl+Z"},
Accion{id:"redo",seccion:"Edición",etiqueta:"Rehacer",tecla:"Ctrl+Y"},
Accion{id:"redo-alt",seccion:"Edición",etiqueta:"Rehacer (alt)",tecla:"Ctrl+Shift+Z"},
Accion{id:"select-all",seccion:"Edición",etiqueta:"Seleccionar todo",tecla:"Ctrl+A"},
Accion{id:"find",seccion:"Edición",etiqueta:"Buscar",tecla:"Ctrl+F"},
Accion{id:"replace",seccion:"Edición",etiqueta:"Buscar y reemplazar",tecla:"Ctrl+H"},
Accion{id:"find-next",seccion:"Edición",etiqueta:"Buscar siguiente",tecla:"F3"},
Accion{id:"move-line-up",seccion:"Edición",etiqueta:"Mover línea arriba",tecla:"Alt+ArrowUp"},
Accion{id:"move-line-dn",seccion:"Edición",etiqueta:"Mover línea abajo",tecla:"Alt+ArrowDown"},
Accion{id:"dup-line",seccion:"Edición",etiqueta:"Duplicar línea",tecla:"Ctrl+D"},
Accion{id:"zoom-in",seccion:"Zoom",etiqueta:"Aumentar texto",tecla:"Ctrl++"},
Accion{id:"zoom-out",seccion:"Zoom",etiqueta:"Reducir texto",tecla:"Ctrl+-"},
Accion{id:"zoom-reset",seccion:"Zoom",etiqueta:"Tamaño 100%",tecla:"Ctrl+0"},
];
pub fn combinacion(e:&KeyDownEvent)->String {
    let k=&e.keystroke;let mut partes=Vec::new();
    if k.modifiers.control||k.modifiers.platform{partes.push("Ctrl".into());}if k.modifiers.alt{partes.push("Alt".into());}if k.modifiers.shift{partes.push("Shift".into());}
    let key=match k.key.as_str(){"up"=>"ArrowUp","down"=>"ArrowDown","left"=>"ArrowLeft","right"=>"ArrowRight","space"|" "=>"Space","enter"=>"Enter","escape"=>"Escape","backspace"=>"Backspace","tab"=>"Tab",s=>s};
    partes.push(if key.len()==1{key.to_uppercase()}else{key.into()});partes.join("+")
}
fn canonica(s:&str)->String{
    let (modificadores,tecla)=if s.ends_with("++"){(&s[..s.len()-2],"+")}else{s.rsplit_once('+').unwrap_or(("",s))};
    let mut mods=modificadores.split('+').filter(|s|!s.is_empty()).map(|s|s.trim().to_lowercase()).collect::<Vec<_>>();mods.sort_unstable();format!("{}:{}",mods.join("+"),tecla.to_lowercase())
}
pub fn coincide(a:&str,b:&str)->bool{!a.is_empty()&&canonica(a)==canonica(b)}
pub fn tecla<'a>(a:&'a Accion,personalizados:&'a HashMap<String,String>)->&'a str{personalizados.get(a.id).map_or(a.tecla,String::as_str)}
pub fn conflicto<'a>(id:&str,combo:&str,personalizados:&'a HashMap<String,String>)->Option<&'static str>{ACCIONES.iter().find(|a|a.id!=id&&coincide(tecla(a,personalizados),combo)).map(|a|a.id)}
#[cfg(test)]mod pruebas {use super::*;#[test]fn modificadores_orden_mayusculas_y_mas(){assert!(coincide("Ctrl+Shift+S","shift+ctrl+s"));assert!(coincide("Ctrl++","ctrl++"));assert!(!coincide("",""));}#[test]fn personalizacion_desactivacion_y_conflictos(){let mut p=HashMap::new();p.insert("save".into(),"Alt+S".into());p.insert("open-file".into(),String::new());assert_eq!(tecla(&ACCIONES[3],&p),"Alt+S");assert_eq!(conflicto("new-tab","Alt+S",&p),Some("save"));assert_eq!(tecla(&ACCIONES[2],&p),"");}}
