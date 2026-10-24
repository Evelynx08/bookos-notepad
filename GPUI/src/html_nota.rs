use crate::editor::nota::{Alineacion, Bloque, Formato, Lista, Nota, Tramo};
use std::sync::{Arc,OnceLock};
use regex::Regex;
fn entidades(s:&str)->String {
    static RE:OnceLock<Regex>=OnceLock::new();
    RE.get_or_init(||Regex::new(r"&(#x[0-9a-fA-F]+|#[0-9]+|[a-zA-Z]+);").expect("regex fija válida"))
        .replace_all(s,|c:&regex::Captures<'_>|match &c[1] {
            "amp"=>"&".into(),"lt"=>"<".into(),"gt"=>">".into(),"quot"=>"\"".into(),"apos"=>"'".into(),"nbsp"=>"\u{a0}".into(),
            n if n.starts_with('#')=>{let valor=if let Some(hex)=n.strip_prefix("#x"){u32::from_str_radix(hex,16).ok()}else{n[1..].parse().ok()};valor.and_then(char::from_u32).map_or_else(||c[0].into(),|v|v.to_string())},
            _=>c[0].into(),
        }).into_owned()
}
fn escapar(s:&str)->String {s.replace('&',"&amp;").replace('<',"&lt;").replace('>',"&gt;").replace('"',"&quot;")}
fn atributos(s:&str)->Vec<(String,String)> {
    static RE:OnceLock<Regex>=OnceLock::new();
    RE.get_or_init(||Regex::new(r#"([\w-]+)\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]+))"#).expect("regex fija válida"))
        .captures_iter(s).map(|c|(c[1].to_lowercase(),entidades(c.get(2).or(c.get(3)).or(c.get(4)).expect("valor del atributo").as_str()))).collect()
}
fn atributo<'a>(attrs:&'a [(String,String)],clave:&str)->Option<&'a str>{attrs.iter().find(|(k,_)|k==clave).map(|(_,v)|v.as_str())}
fn estilo(attrs:&[(String,String)],clave:&str)->Option<String>{atributo(attrs,"style")?.split(';').filter_map(|s|s.split_once(':')).find(|(k,_)|k.trim()==clave).map(|(_,v)|v.trim().into())}
fn alineacion(attrs:&[(String,String)])->Alineacion {match estilo(attrs,"text-align").as_deref(){Some("center")=>Alineacion::Centro,Some("right")=>Alineacion::Derecha,_=>Alineacion::Izquierda}}
pub fn leer(html:&str)->Nota {
    let mut bloques=Vec::new();let mut actual=Bloque::vacio();let mut pila:Vec<(String,Formato)>=Vec::new();let mut formato=Formato::default();let mut listas=Vec::new();let mut cerrado=false;let mut oculto=0usize;
    let volcar=|actual:&mut Bloque,bloques:&mut Vec<Bloque>,forzar:bool|{if forzar || !actual.texto().is_empty(){bloques.push(std::mem::replace(actual,Bloque::vacio()));}};
    let mut cursor=0;
    while cursor<html.len() {
        if html.as_bytes()[cursor]!=b'<' {
            let fin=html[cursor..].find('<').map_or(html.len(),|i|cursor+i);
            let texto=entidades(&html[cursor..fin]);
            if oculto==0 && !(cerrado && texto.trim().is_empty()) && let Bloque::Texto{texto:contenido,tramos,..}=&mut actual {let a=contenido.len();contenido.push_str(&texto);if a<contenido.len(){tramos.push(Tramo{rango:a..contenido.len(),formato:formato.clone()});} cerrado=false;}
            cursor=fin;continue;
        }
        if html[cursor..].starts_with("<!--") {cursor=html[cursor+4..].find("-->").map_or(html.len(),|i|cursor+4+i+3);continue;}
        let mut comilla=None;let mut fin=cursor+1;
        for (i,c) in html[cursor+1..].char_indices(){if let Some(q)=comilla {if c==q{comilla=None;}}else if c=='\''||c=='"'{comilla=Some(c);}else if c=='>'{fin=cursor+1+i;break;}}
        if fin<=cursor+1 {break;}
        let tag=html[cursor+1..fin].trim();cursor=fin+1;
        let cierre=tag.starts_with('/');let tag=tag.trim_start_matches('/');let nombre=tag.split_whitespace().next().unwrap_or("").trim_end_matches('/').to_lowercase();
        if nombre=="script"||nombre=="style" {if cierre{oculto=oculto.saturating_sub(1);}else{oculto+=1;}continue;}
        if oculto>0 {continue;}
        let attrs=atributos(tag);
        if cierre {
            match nombre.as_str(){
                "div"|"p"|"li"|"h1"|"h2"|"h3"|"blockquote"=>{volcar(&mut actual,&mut bloques,!cerrado);cerrado=true;}
                "ul"|"ol"=>{listas.pop();}
                _=>{}
            }
            if let Some(i)=pila.iter().rposition(|(n,_)|n==&nombre){formato=pila[i].1.clone();pila.truncate(i);}
            continue;
        }
        match nombre.as_str(){
            "div"|"p"|"li"|"h1"|"h2"|"h3"|"blockquote"=>{
                volcar(&mut actual,&mut bloques,false);cerrado=false;
                if let Bloque::Texto{lista,alineacion:a,..}=&mut actual {
                    *a=alineacion(&attrs);
                    if nombre=="li" {*lista=match listas.last(){Some(Lista::Tarea(_))=>Lista::Tarea(atributo(&attrs,"class").is_some_and(|s|s.split_whitespace().any(|v|v=="hecha"))),Some(l)=>*l,None=>Lista::Vineta};}
                }
            }
            "ul"|"ol"=>{volcar(&mut actual,&mut bloques,false);listas.push(if nombre=="ol"{Lista::Numero}else if atributo(&attrs,"class").is_some_and(|s|s.split_whitespace().any(|v|v=="tareas")){Lista::Tarea(false)}else{Lista::Vineta});}
            "br"=>{if actual.texto().is_empty(){cerrado=false;}else{volcar(&mut actual,&mut bloques,true);cerrado=false;}}
            "img"=>{
                volcar(&mut actual,&mut bloques,false);
                if let Some(src)=atributo(&attrs,"src") {
                    let ancho=estilo(&attrs,"width").or_else(||atributo(&attrs,"width").map(str::to_owned)).and_then(|s|s.trim_end_matches("px").trim().parse().ok());
                    let centro=estilo(&attrs,"margin-left").as_deref()==Some("auto");let derecha=estilo(&attrs,"margin-right").as_deref()!=Some("auto");
                    bloques.push(Bloque::Imagen{fuente:Arc::from(src),ancho,alineacion:if centro {if derecha{Alineacion::Derecha}else{Alineacion::Centro}}else{Alineacion::Izquierda}});
                } cerrado=true;
            }
            _=>{}
        }
        if matches!(nombre.as_str(),"b"|"strong"|"i"|"em"|"u"|"s"|"strike"|"del"|"span"|"font") {
            pila.push((nombre.clone(),formato.clone()));
            match nombre.as_str(){"b"|"strong"=>formato.negrita=true,"i"|"em"=>formato.cursiva=true,"u"=>formato.subrayado=true,"s"|"strike"|"del"=>formato.tachado=true,_=>{}}
            if let Some(c)=estilo(&attrs,"color").or_else(||atributo(&attrs,"color").map(str::to_owned)){formato.color=Some(c);}
            if let Some(c)=estilo(&attrs,"background-color"){formato.fondo=Some(c);}
            if estilo(&attrs,"font-weight").is_some_and(|v|v=="bold"||v=="700"){formato.negrita=true;}
            if estilo(&attrs,"font-style").as_deref()==Some("italic"){formato.cursiva=true;}
            if let Some(s)=estilo(&attrs,"text-decoration"){formato.subrayado|=s.contains("underline");formato.tachado|=s.contains("line-through");}
        }
    }
    volcar(&mut actual,&mut bloques,!cerrado);
    if bloques.is_empty(){bloques.push(Bloque::vacio());}
    Nota{bloques}
}
pub fn escribir(nota:&Nota)->String {
    let mut salida=String::new();
    for bloque in &nota.bloques {
        let alinear=|a:&Alineacion|match a {Alineacion::Izquierda=>"left",Alineacion::Centro=>"center",Alineacion::Derecha=>"right"};
        match bloque {
            Bloque::Imagen{fuente,ancho,alineacion}=>{
                let margen=match alineacion {Alineacion::Izquierda=>"",Alineacion::Centro=>"margin-left:auto;margin-right:auto;",Alineacion::Derecha=>"margin-left:auto;"};
                let ancho=ancho.map_or(String::new(),|a|format!("width:{a}px;"));
                salida.push_str(&format!("<img src=\"{}\" style=\"{ancho}{margen}\">",escapar(fuente)));
            }
            Bloque::Texto{texto,tramos,lista,alineacion}=>{
                let (abrir,cerrar)=match lista {Lista::Ninguna=>(format!("<div style=\"text-align:{}\">",alinear(alineacion)),"</div>"),Lista::Vineta=>("<ul><li>".into(),"</li></ul>"),Lista::Numero=>("<ol><li>".into(),"</li></ol>"),Lista::Tarea(hecha)=>(format!("<ul class=\"tareas\"><li class=\"{}\">",if *hecha{"hecha"}else{""}),"</li></ul>")};
                salida.push_str(&abrir);let mut cursor=0;
                for t in tramos {
                    if t.rango.start>cursor{salida.push_str(&escapar(&texto[cursor..t.rango.start]));}
                    let f=&t.formato;let mut estilos=String::new();
                    if f.negrita{estilos.push_str("font-weight:bold;");}
                    if f.cursiva{estilos.push_str("font-style:italic;");}
                    if f.subrayado||f.tachado{estilos.push_str("text-decoration:");if f.subrayado{estilos.push_str("underline ");}
                        if f.tachado{estilos.push_str("line-through");}estilos.push(';');}
                    if let Some(c)=&f.color{estilos.push_str(&format!("color:{};",escapar(c)));}
                    if let Some(c)=&f.fondo{estilos.push_str(&format!("background-color:{};",escapar(c)));}
                    salida.push_str(&format!("<span style=\"{estilos}\">{}</span>",escapar(&texto[t.rango.clone()])));cursor=t.rango.end;
                }
                salida.push_str(&escapar(&texto[cursor..]));if texto.is_empty(){salida.push_str("<br>");}salida.push_str(cerrar);
            }
        }
    } salida
}
#[cfg(test)]
mod pruebas {
    use super::*;
    #[test]
    fn webkit_formato_tareas_imagenes_entidades_y_desconocidas() {
        let html=r##"<div>Título &amp; &#233; &#xE9;</div><div><b>negrita</b><em> cursiva</em><u> subrayada</u><strike> tachada</strike><font color="#ff0000"> roja</font><span style="background-color:#00ff00"> fondo</span></div><ul class="tareas"><li class="hecha">hecha</li><li>pendiente</li></ul><div style="text-align:center">centro</div><img src="data:image/png;base64,YQ==" style="width:600px;margin-left:auto;margin-right:auto"><desconocido>conservado</desconocido>"##;
        let nota=leer(html);assert!(nota.plano().contains("Título & é é"));assert!(nota.plano().contains("conservado"));
        assert!(nota.bloques.iter().any(|b|matches!(b,Bloque::Texto{lista:Lista::Tarea(true),..})));
        assert_eq!(leer(&escribir(&nota)),nota);
    }
    #[test]
    fn lineas_vacias_no_se_duplican() {
        let nota=leer("<div>Primera</div><div><br></div><div>Última</div>");assert_eq!(nota.plano(),"Primera\n\nÚltima");assert_eq!(leer(&escribir(&nota)),nota);
    }
    #[test]
    fn editar_conserva_formato_fuera_de_la_seleccion() {
        let mut nota=leer("<div>Título</div><div><b>hola</b> mundo</div>");nota.reemplazar(9..10,"X",&Formato::default());
        assert_eq!(nota.plano(),"Título\nhXla mundo");assert_eq!(leer(&escribir(&nota)),nota);
    }
}
