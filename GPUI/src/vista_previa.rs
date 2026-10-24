use std::{path::PathBuf,sync::Arc};
use bookos_ui::{Tema,ventana,entrada::{Entrada,Confirmado},controles::{self,EstiloBoton}};
use gpui::{prelude::*,*};
use crate::{estado::Ajustes,markdown,textos::Textos};
pub fn normalizar(raw:&str)->String{
    let raw=raw.trim();if raw.starts_with("ttps://")||raw.starts_with("ttp://"){return format!("h{raw}");}
    if raw.starts_with("http://")||raw.starts_with("https://"){return raw.into();}
    let host=raw.split('/').next().unwrap_or("").split(':').next().unwrap_or("");
    if host=="localhost"||host.parse::<std::net::Ipv4Addr>().is_ok(){format!("http://{raw}")}else if host.contains('.') {format!("https://{raw}")}else{raw.into()}
}
pub fn youtube(url:&str,nocookie:bool,directo:bool)->Option<String>{
    let sin_esquema=url.split_once("://")?.1;let(host,resto)=sin_esquema.split_once('/').unwrap_or((sin_esquema,""));let host=host.strip_prefix("www.").unwrap_or(host);
    if !["youtube.com","m.youtube.com","youtu.be","youtube-nocookie.com"].contains(&host){return None;}
    if directo{return Some(url.into());}
    let(ruta,query)=resto.split_once('?').unwrap_or((resto,""));let param=|nombre|query.split('&').filter_map(|s|s.split_once('=')).find(|(k,_)|*k==nombre).map(|(_,v)|v);
    let id=if host=="youtu.be"{ruta.split('/').next()}else if ruta.starts_with("shorts/")||ruta.starts_with("embed/"){ruta.split('/').nth(1)}else{param("v")}?;
    if id.is_empty()||!id.chars().all(|c|c.is_ascii_alphanumeric()||c=='_'||c=='-'){return None;}
    let tiempo=param("t").or_else(||param("start")).and_then(|s|s.trim_end_matches('s').parse::<u64>().ok()).unwrap_or(0);
    Some(format!("https://www.{}/embed/{id}?rel=0&modestbranding=1&iv_load_policy=3&fs=1{}",if nocookie{"youtube-nocookie.com"}else{"youtube.com"},if tiempo>0{format!("&start={tiempo}")}else{String::new()}))
}
pub struct VistaPrevia {
    tema:Tema,textos:Textos,pub ajustes:Ajustes,pub documento:String,pub extension:String,
    bloques:Vec<markdown::Bloque>,svg:Option<Arc<Image>>,url:Entity<Entrada>,foco:FocusHandle,
    padre:WeakEntity<crate::app::Vista>,error:Option<String>,bloqueada:Option<String>,contador:usize,
    temporal:Option<PathBuf>,_subs:Vec<Subscription>,
}
impl VistaPrevia{
    pub fn nueva(documento:String,extension:String,ajustes:Ajustes,padre:WeakEntity<crate::app::Vista>,window:&mut Window,cx:&mut Context<Self>)->Self{
        let url=cx.new(|cx|Entrada::new(ajustes.preview_url.clone(),"https://…",cx));let sub=cx.subscribe_in(&url,window,|this,_,_:&Confirmado,_,cx|this.abrir(false,cx));
        let mut this=Self{tema:Tema::para_ventana("auto",window),textos:Textos::cargar(),ajustes,documento:String::new(),extension:String::new(),bloques:Vec::new(),svg:None,url,foco:cx.focus_handle(),padre,error:None,bloqueada:None,contador:0,temporal:None,_subs:vec![sub]};
        this.actualizar(documento,extension,cx);window.focus(&this.foco,cx);this
    }
    pub fn actualizar(&mut self,documento:String,extension:String,cx:&mut Context<Self>){self.documento=documento;self.extension=extension;self.bloques=markdown::parsear(&self.documento);self.svg=(self.extension=="svg").then(||Arc::new(Image::from_bytes(ImageFormat::Svg,self.documento.as_bytes().to_vec())));cx.notify();}
    fn modo(&self)->&str {if self.ajustes.preview_mode=="auto" {if ["md","markdown","mdx"].contains(&self.extension.as_str()){"md"}else{"html"}}else{&self.ajustes.preview_mode}}
    fn guardar_ajustes(&self,cx:&mut Context<Self>){let ajustes=self.ajustes.clone();let _=self.padre.update(cx,|p,cx|{p.estado.settings.preview_mode=ajustes.preview_mode;p.estado.settings.preview_url=ajustes.preview_url;p.estado.settings.adblock_enabled=ajustes.adblock_enabled;p.persistir(cx);});}
    fn abrir(&mut self,permitir:bool,cx:&mut Context<Self>){
        self.error=None;self.ajustes.preview_url=self.url.read(cx).texto().into();self.guardar_ajustes(cx);
        let destino=if self.modo()=="url"{
            let normal=normalizar(&self.ajustes.preview_url);if !normal.starts_with("https://")&&!normal.starts_with("http://"){self.error=Some(self.textos.elegir("Introduce una URL HTTP o HTTPS","Enter an HTTP or HTTPS URL"));cx.notify();return;}
            youtube(&normal,self.ajustes.youtube_no_cookie,self.ajustes.youtube_direct).unwrap_or(normal)
        }else{
            let dir=std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);let ruta=dir.join(format!("bookos-notepad-vista-{}.html",std::process::id()));
            let contenido=if self.extension=="css"{format!("<!doctype html><meta charset=utf-8><style>{}</style><main><h1>Vista previa CSS</h1><p>Texto de ejemplo</p><button>Botón</button><input placeholder='Entrada'><ul><li>Elemento</li></ul></main>",self.documento)}else{self.documento.clone()};
            match crate::archivos::escribir_bytes(&ruta,contenido.as_bytes()){Ok(_)=>{},Err(e)=>{self.error=Some(e);cx.notify();return;}}
            self.temporal=Some(ruta.clone());ruta.to_string_lossy().into_owned()
        };
        let filtrar=self.modo()=="url"&&self.ajustes.adblock_enabled&&!permitir;
        let tarea=cx.background_executor().spawn(async move{
            if filtrar&&let Some(regla)=crate::adblock::comprobar(&destino)?{return Ok(Some(regla));}
            let estado=std::process::Command::new("xdg-open").arg(&destino).status().map_err(|e|e.to_string())?;
            if !estado.success(){return Err("El navegador no pudo abrir la vista previa".into());}Ok::<_,String>(None)
        });
        cx.spawn(async move|this,cx|{let resultado=tarea.await;let _=this.update(cx,|this,cx|{match resultado{Ok(Some(regla))=>{this.contador+=1;this.bloqueada=Some(regla);},Ok(None)=>this.bloqueada=None,Err(e)=>this.error=Some(e)}cx.notify();});}).detach();
    }
}
impl Drop for VistaPrevia{fn drop(&mut self){if let Some(ruta)=&self.temporal{let _=std::fs::remove_file(ruta);}}}
impl Render for VistaPrevia{
    fn render(&mut self,window:&mut Window,cx:&mut Context<Self>)->impl IntoElement{
        self.tema=Tema::para_ventana("auto",window);let tm=self.tema;let textos=self.textos;
        let barra=div().flex().items_center().gap(px(6.)).h(px(46.)).px(px(10.))
            .children([("auto","Auto"),("html","HTML"),("md","MD"),("url","URL")].into_iter().map(|(modo,etiqueta)|{
                controles::boton(&tm,modo,etiqueta,if self.ajustes.preview_mode==modo{EstiloBoton::Primario}else{EstiloBoton::Secundario}).on_click(cx.listener(move|this,_,_,cx|{this.ajustes.preview_mode=modo.into();this.guardar_ajustes(cx);cx.notify();}))
            }))
            .child(div().flex_1())
            .child(controles::boton(&tm,"escudo",format!("{} {}",if self.ajustes.adblock_enabled{"🛡"}else{"○"},self.contador),EstiloBoton::Secundario).on_click(cx.listener(|this,_,_,cx|{this.ajustes.adblock_enabled=!this.ajustes.adblock_enabled;this.guardar_ajustes(cx);cx.notify();})))
            .child(controles::boton(&tm,"recargar","↻",EstiloBoton::Secundario).on_click(cx.listener(|this,_,_,cx|{if this.modo()=="url"{this.abrir(false,cx);}else{let _=this.padre.update(cx,|p,cx|p.actualizar_previa(cx));}})));
        let mut contenido=div().size_full().flex().flex_col().child(barra);
        if self.modo()=="url"{contenido=contenido.child(div().mx(px(10.)).mb(px(8.)).h(px(32.)).px(px(10.)).bg(tm.card).rounded(px(10.)).flex().items_center().child(self.url.clone()));}
        let cuerpo=if self.modo()=="md"{div().id("markdown").flex_1().min_h_0().overflow_y_scroll().p(px(26.)).children(markdown::Estilo{tema:tm,escala:1.,fuente:bookos_ui::FUENTE_UI,mono:bookos_ui::FUENTE_MONO}.pintar(&self.bloques)).into_any_element()}
        else if self.extension=="svg"&&self.modo()!="url"{div().flex_1().p(px(22.)).children(self.svg.clone().map(|i|img(i).size_full().object_fit(ObjectFit::Contain))).into_any_element()}
        else{div().flex_1().flex().flex_col().justify_center().items_center().gap(px(14.)).p(px(24.))
            .child(textos.elegir("La vista web se abre en el navegador","Web previews open in your browser"))
            .children(self.bloqueada.as_ref().map(|regla|div().text_color(tm.red).child(format!("{}: {regla}",textos.elegir("URL bloqueada por filtro","URL blocked by filter")))))
            .child(controles::boton(&tm,"navegador",if self.bloqueada.is_some(){textos.elegir("Cargar igualmente","Load anyway")}else{textos.elegir("Abrir en el navegador","Open in browser")},EstiloBoton::Primario).on_click(cx.listener(|this,_,_,cx|this.abrir(this.bloqueada.is_some(),cx)))).into_any_element()};
        contenido=contenido.child(cuerpo).children(self.error.as_ref().map(|e|div().p(px(12.)).text_color(tm.red).child(e.clone())));
        ventana::marco(&tm,window,Some(textos.elegir("Vista previa","Preview").into()),None,contenido,[]).track_focus(&self.foco).on_key_down(cx.listener(|_,ev:&KeyDownEvent,w,cx|{if ev.keystroke.key=="escape"||ev.keystroke.key=="space"{w.remove_window();cx.stop_propagation();}}))
    }
}
#[cfg(test)]mod pruebas{use super::*;#[test]fn urls_y_youtube_con_tiempos(){for(a,b)in[("ttps://a.es","https://a.es"),("localhost:3000","http://localhost:3000"),("127.0.0.1/a","http://127.0.0.1/a"),("ejemplo.es","https://ejemplo.es")]{assert_eq!(normalizar(a),b);}for url in ["https://youtube.com/watch?v=abc&t=12","https://youtu.be/abc?t=12","https://youtube.com/embed/abc?start=12"]{let salida=youtube(url,true,false).expect("URL de YouTube");assert!(salida.contains("youtube-nocookie.com/embed/abc"));assert!(salida.ends_with("start=12"));assert_eq!(youtube(url,false,true).as_deref(),Some(url));}assert!(youtube("https://youtube.com/shorts/abc",true,false).expect("short").contains("/embed/abc"));}}
