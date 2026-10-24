pub mod texto;
pub mod nota;
pub mod buscar;
mod deshacer;
mod fila;

use std::{collections::BTreeMap,ops::Range,sync::Arc,time::Instant};
use bookos_ui::Tema;
use gpui::{prelude::*,*, ClipboardEntry};
use texto::Texto;
use nota::{Nota,Bloque,Formato,Lista,Alineacion};
use buscar::Busqueda;
use deshacer::{Historial,Paso};

pub enum Documento { Texto(Texto), Nota { nota:Nota, texto:Texto } }
impl Documento {
    pub fn texto(&self)->&Texto {match self {Self::Texto(t)|Self::Nota{texto:t,..}=>t}}
    pub fn nota(&self)->Option<&Nota>{match self{Self::Nota{nota,..}=>Some(nota),Self::Texto(_)=>None}}
}
pub struct Cambio;
pub struct ImagenPegada(pub Vec<u8>);
impl EventEmitter<Cambio> for Editor{}
impl EventEmitter<ImagenPegada> for Editor{}
pub struct Editor {
    pub documento:Documento, pub seleccion:Range<usize>, pub invertida:bool, marcado:Option<Range<usize>>,
    pub foco:FocusHandle,pub lista:ListState,pub tema:Tema,pub tamano:f32,pub fuente:&'static str,pub ajuste:bool,
    pub ancho:f32,pub desplazamiento_x:f32,pub extension:String,
    pub busqueda:Busqueda,pub formato:Formato,historial:Historial,
    visibles:BTreeMap<usize,(Bounds<Pixels>,Arc<WrappedLine>)>,arrastrando:bool,
    pub imagen_seleccionada:Option<usize>,pub imagenes:BTreeMap<usize,(u32,Arc<RenderImage>)>,
    pub error_imagen:Option<String>,arrastre_imagen:Option<(usize,f32,Pixels,Nota)>,
}
impl Editor {
    pub fn nuevo(documento:Documento,cx:&mut Context<Self>)->Self{
        let cantidad=documento.texto().inicios.len();
        Self{documento,seleccion:0..0,invertida:false,marcado:None,foco:cx.focus_handle(),lista:ListState::new(cantidad,ListAlignment::Top,px(300.)),tema:Tema::claro(),tamano:15.,fuente:bookos_ui::FUENTE_UI,ajuste:true,ancho:700.,desplazamiento_x:0.,extension:String::new(),busqueda:Busqueda::default(),formato:Formato::default(),historial:Historial::default(),visibles:BTreeMap::new(),arrastrando:false,imagen_seleccionada:None,imagenes:BTreeMap::new(),error_imagen:None,arrastre_imagen:None}
    }
    pub fn cargar(&mut self,documento:Documento,cx:&mut Context<Self>){
        self.documento=documento;self.seleccion=0..0;self.invertida=false;self.marcado=None;
        self.historial=Historial::default();self.imagen_seleccionada=None;
        self.lista.reset(self.documento.texto().inicios.len());self.visibles.clear();self.imagenes.clear();self.error_imagen=None;self.busqueda=Busqueda::default();cx.notify();
    }
    pub fn reemplazar_varios(&mut self,cambios:Vec<(Range<usize>,String)>,cx:&mut Context<Self>){
        if cambios.is_empty(){return;}
        let antes=self.seleccion.clone();
        let paso=match &mut self.documento {
            Documento::Texto(texto)=>{
                let quitado=texto.cadena.clone();
                for(r,nuevo)in cambios.into_iter().rev(){texto.reemplazar(r,&nuevo);}
                Paso::Texto{rango:0..texto.cadena.len(),quitado,puesto:texto.cadena.clone(),antes,despues:0..0,instante:Instant::now()}
            },
            Documento::Nota{nota,texto}=>{
                let anterior=nota.clone();
                for(r,nuevo)in cambios.into_iter().rev(){nota.reemplazar(r,&nuevo,&self.formato);}
                *texto=Texto::nuevo(nota.plano());
                Paso::Nota{antes:anterior,despues:nota.clone(),seleccion_antes:antes,seleccion_despues:0..0}
            },
        };
        self.historial.agregar(paso,false);self.seleccion=0..0;self.invertida=false;self.refrescar(cx);
    }
    pub fn texto(&self)->&str{&self.documento.texto().cadena}
    pub fn cursor(&self)->usize{if self.invertida{self.seleccion.start}else{self.seleccion.end}}
    pub fn refrescar(&mut self,cx:&mut Context<Self>){
        if let Documento::Nota{nota,texto}=&mut self.documento{*texto=Texto::nuevo(nota.plano());}
        self.lista.reset(self.documento.texto().inicios.len());self.visibles.clear();self.imagenes.clear();self.error_imagen=None;self.busqueda=Busqueda::default();cx.emit(Cambio);cx.notify();
    }
    fn revelar(&self){self.lista.scroll_to_reveal_item(self.documento.texto().linea_de(self.cursor()));}
    pub fn mover(&mut self,pos:usize,extender:bool,cx:&mut Context<Self>){
        let pos=pos.min(self.texto().len());
        if extender {let ancla=if self.invertida{self.seleccion.end}else{self.seleccion.start};self.invertida=pos<ancla;self.seleccion=pos.min(ancla)..pos.max(ancla);}
        else{self.seleccion=pos..pos;self.invertida=false;}
        self.imagen_seleccionada=None;
        if let Some(nota)=self.documento.nota(){let(i,offset)=nota.posicion(self.cursor());if let Bloque::Texto{tramos,..}=&nota.bloques[i]{self.formato=tramos.iter().find(|t|t.rango.contains(&offset.saturating_sub(1))).map(|t|t.formato.clone()).unwrap_or_default();}}
        cx.notify();
    }
    pub fn reemplazar(&mut self,rango:Range<usize>,nuevo:&str,agrupar:bool,cx:&mut Context<Self>){
        let antes=self.seleccion.clone();let despues=rango.start+nuevo.len()..rango.start+nuevo.len();
        let paso=match &mut self.documento {
            Documento::Texto(texto)=>{let quitado=texto.cadena[rango.clone()].to_owned();texto.reemplazar(rango.clone(),nuevo);Paso::Texto{rango:rango.start..rango.start+nuevo.len(),quitado,puesto:nuevo.into(),antes,despues:despues.clone(),instante:Instant::now()}},
            Documento::Nota{nota,texto}=>{let anterior=nota.clone();nota.reemplazar(rango,nuevo,&self.formato);*texto=Texto::nuevo(nota.plano());Paso::Nota{antes:anterior,despues:nota.clone(),seleccion_antes:antes,seleccion_despues:despues.clone()}},
        };
        self.historial.agregar(paso,agrupar);self.seleccion=despues;self.invertida=false;self.marcado=None;
        self.refrescar(cx);self.revelar();
    }
    pub fn insertar(&mut self,nuevo:&str,cx:&mut Context<Self>){self.reemplazar(self.seleccion.clone(),nuevo,true,cx);}
    pub fn deshacer(&mut self,rehacer:bool,cx:&mut Context<Self>){
        let paso=if rehacer{self.historial.delante.pop()}else{self.historial.atras.pop()};
        if let Some(paso)=paso{
            match (&paso,&mut self.documento){
                (Paso::Texto{rango,quitado,puesto,antes,despues,..},Documento::Texto(texto))=>{if rehacer{texto.reemplazar(rango.start..rango.start+quitado.len(),puesto);self.seleccion=despues.clone();}else{texto.reemplazar(rango.clone(),quitado);self.seleccion=antes.clone();}},
                (Paso::Nota{antes,despues,seleccion_antes,seleccion_despues},Documento::Nota{nota,..})=>{*nota=if rehacer{despues.clone()}else{antes.clone()};self.seleccion=if rehacer{seleccion_despues.clone()}else{seleccion_antes.clone()};},
                _=>unreachable!("cada historial pertenece a un tipo de documento"),
            }
            if rehacer{self.historial.atras.push(paso);}else{self.historial.delante.push(paso);}
            self.refrescar(cx);self.revelar();
        }
    }
    pub fn modificar_nota(&mut self,cambiar:impl FnOnce(&mut Nota),cx:&mut Context<Self>){
        if let Documento::Nota{nota,..}=&mut self.documento{let antes=nota.clone();cambiar(nota);if *nota==antes{return;}self.historial.agregar(Paso::Nota{antes,despues:nota.clone(),seleccion_antes:self.seleccion.clone(),seleccion_despues:self.seleccion.clone()},false);self.refrescar(cx);}
    }
    pub fn dar_formato(&mut self,accion:&str,color:Option<String>,cx:&mut Context<Self>){
        let mut f=self.formato.clone();match accion{"bold"=>f.negrita= !f.negrita,"italic"=>f.cursiva= !f.cursiva,"underline"=>f.subrayado= !f.subrayado,"strike"=>f.tachado= !f.tachado,"color"=>f.color=color,"background"=>f.fondo=color,"clear"=>f=Formato::default(),_=>return}
        let rango=self.seleccion.clone();if !rango.is_empty(){self.modificar_nota(|n|n.aplicar(rango,|actual|match accion{"bold"=>actual.negrita=f.negrita,"italic"=>actual.cursiva=f.cursiva,"underline"=>actual.subrayado=f.subrayado,"strike"=>actual.tachado=f.tachado,"color"=>actual.color=f.color.clone(),"background"=>actual.fondo=f.fondo.clone(),"clear"=>*actual=Formato::default(),_=>{}}),cx);}
        self.formato=f;cx.notify();
    }
    pub fn bloque_formato(&mut self,accion:&str,cx:&mut Context<Self>){
        let sel=self.seleccion.clone();let accion=accion.to_owned();
        self.modificar_nota(|n|{let(a,_)=n.posicion(sel.start);let(b,_)=n.posicion(sel.end);for bloque in &mut n.bloques[a..=b]{if let Bloque::Texto{lista,alineacion,..}=bloque{match accion.as_str(){"list"=>*lista=if *lista==Lista::Vineta{Lista::Ninguna}else{Lista::Vineta},"number"=>*lista=if *lista==Lista::Numero{Lista::Ninguna}else{Lista::Numero},"tasks"=>*lista=if matches!(lista,Lista::Tarea(_)){Lista::Vineta}else{Lista::Tarea(false)},"left"=>*alineacion=Alineacion::Izquierda,"center"=>*alineacion=Alineacion::Centro,"right"=>*alineacion=Alineacion::Derecha,_=>{}}}}},cx);
    }
    pub fn imagen_accion(&mut self,accion:&str,cx:&mut Context<Self>){
        let Some(i)=self.imagen_seleccionada else{return;};
        let disponible=(self.ancho-52.).max(40.);
        if accion=="delete"{self.seleccion=0..0;self.invertida=false;}
        self.modificar_nota(|n|{
            if accion=="delete"{n.bloques.remove(i);if n.bloques.is_empty(){n.bloques.push(Bloque::vacio());}}
            else if let Bloque::Imagen{ancho,alineacion,..}=&mut n.bloques[i]{match accion{
                "original"=>*ancho=None,"fit"=>*ancho=Some(disponible),
                "left"=>*alineacion=Alineacion::Izquierda,"center"=>*alineacion=Alineacion::Centro,"right"=>*alineacion=Alineacion::Derecha,_=>{},
            }}
        },cx);
        if accion=="delete"{self.imagen_seleccionada=None;self.mover(0,false,cx);}
    }
    pub fn insertar_imagen(&mut self,fuente:Arc<str>,cx:&mut Context<Self>){let pos=self.cursor();self.modificar_nota(|n|{let(i,_)=n.posicion(pos);n.bloques.insert(i+1,Bloque::Imagen{fuente,ancho:None,alineacion:Alineacion::Izquierda});if i+2==n.bloques.len(){n.bloques.push(Bloque::vacio());}},cx);}
    pub fn linea(&mut self,direccion:i32,duplicar:bool,cx:&mut Context<Self>){
        let t=self.documento.texto();let i=t.linea_de(self.cursor());let r=t.rango_linea(i);let columna=self.cursor()-r.start;
        if duplicar{let texto=format!("\n{}",&t.cadena[r.clone()]);self.reemplazar(r.end..r.end,&texto,false,cx);self.mover(r.end+1+columna,false,cx);}
        else{let otra=if direccion<0{i.checked_sub(1)}else{(i+1<t.inicios.len()).then_some(i+1)};if let Some(j)=otra{let s=t.rango_linea(j);let(a,b)=if i<j{(r.clone(),s.clone())}else{(s.clone(),r.clone())};let nuevo=format!("{}\n{}",&t.cadena[b.clone()],&t.cadena[a.clone()]);let destino=if direccion<0{a.start+columna}else{a.start+(b.end-b.start)+1+columna};self.reemplazar(a.start..b.end,&nuevo,false,cx);self.mover(destino,false,cx);}}
    }
    fn indice_en(&self,p:Point<Pixels>)->usize{
        let Some((&i,(caja,linea)))=self.visibles.iter().min_by(|(_,a),(_,b)|{let distancia=|c:&Bounds<Pixels>|if p.y<c.top(){c.top()-p.y}else if p.y>c.bottom(){p.y-c.bottom()}else{px(0.)};distancia(&a.0).partial_cmp(&distancia(&b.0)).expect("coordenadas finitas")})else{return self.cursor();};
        let posicion=point(p.x-caja.left()+px(self.desplazamiento_x),p.y-caja.top());
        let offset=linea.closest_index_for_position(posicion,px(self.tamano*1.6)).unwrap_or_else(|i|i);
        self.documento.texto().inicios[i]+offset.min(self.documento.texto().rango_linea(i).len())
    }
    fn teclado(&mut self,e:&KeyDownEvent,window:&mut Window,cx:&mut Context<Self>){
        let key=e.keystroke.key.as_str();let ctrl=e.keystroke.modifiers.control||e.keystroke.modifiers.platform;
        let shift=e.keystroke.modifiers.shift;
        if self.imagen_seleccionada.is_some()&&["backspace","delete"].contains(&key){self.imagen_accion("delete",cx);window.prevent_default();cx.stop_propagation();return;}
        if ctrl{match key{
            "c"|"x"=>{if !self.seleccion.is_empty(){cx.write_to_clipboard(ClipboardItem::new_string(self.texto()[self.seleccion.clone()].into()));if key=="x"{self.insertar("",cx);}}},
            "v"=>{if let Some(item)=cx.read_from_clipboard(){if let Some(ClipboardEntry::Image(imagen))=item.entries.iter().find(|e|matches!(e,ClipboardEntry::Image(_))){cx.emit(ImagenPegada(imagen.bytes().to_vec()));}else if let Some(texto)=item.text(){self.insertar(&texto,cx);}}},
            "left"|"right"=>{let mut p=self.cursor();let t=self.documento.texto();if key=="left"{while p>0&&t.cadena[..p].chars().next_back().is_some_and(char::is_whitespace){p=t.anterior(p);}while p>0&&t.cadena[..p].chars().next_back().is_some_and(|c|!c.is_whitespace()){p=t.anterior(p);}}else{while p<t.cadena.len()&&t.cadena[p..].chars().next().is_some_and(|c|!c.is_whitespace()){p=t.siguiente(p);}while p<t.cadena.len()&&t.cadena[p..].chars().next().is_some_and(char::is_whitespace){p=t.siguiente(p);}}self.mover(p,shift,cx);self.revelar();},
            "home"=>self.mover(0,shift,cx),"end"=>self.mover(self.texto().len(),shift,cx),_=>return}
        }else{match key{
            "left"|"right"=>{let t=self.documento.texto();let p=if !shift&&!self.seleccion.is_empty(){if key=="left"{self.seleccion.start}else{self.seleccion.end}}else if key=="left"{t.anterior(self.cursor())}else{t.siguiente(self.cursor())};self.mover(p,shift,cx);self.revelar();},
            "up"|"down"|"pageup"|"pagedown"=>{let t=self.documento.texto();let i=t.linea_de(self.cursor());let col=t.cadena[t.inicios[i]..self.cursor()].chars().count();let salto=if key.starts_with("page"){15}else{1};let j=if key=="up"||key=="pageup"{i.saturating_sub(salto)}else{(i+salto).min(t.inicios.len()-1)};let r=t.rango_linea(j);let p=t.cadena[r.clone()].char_indices().nth(col).map_or(r.end,|(k,_)|r.start+k);self.mover(p,shift,cx);self.revelar();},
            "home"|"end"=>{let t=self.documento.texto();let r=t.rango_linea(t.linea_de(self.cursor()));self.mover(if key=="home"{r.start}else{r.end},shift,cx);},
            "backspace"|"delete"=>{let mut r=self.seleccion.clone();if r.is_empty(){if key=="backspace"{r.start=self.documento.texto().anterior(r.start);}else{r.end=self.documento.texto().siguiente(r.end);}}
                if !r.is_empty(){self.reemplazar(r,"",false,cx);}},
            "enter"=>self.insertar("\n",cx),"tab"=>{let espacios=if es_codigo(&self.extension){"  "}else{"    "};self.insertar(espacios,cx);},
            _=>return,
        }}
        window.prevent_default();cx.stop_propagation();
    }
    fn utf16(&self,pos:usize)->usize{self.texto()[..pos].encode_utf16().count()}
    fn byte(&self,pos:usize)->usize{let mut total=0;for(i,c)in self.texto().char_indices(){if total>=pos{return i;}total+=c.len_utf16();}self.texto().len()}
    fn rango_byte(&self,r:&Range<usize>)->Range<usize>{self.byte(r.start)..self.byte(r.end)}
    fn caja_cursor(&self,pos:usize)->Option<Bounds<Pixels>>{let i=self.documento.texto().linea_de(pos);let(caja,linea)=self.visibles.get(&i)?;let p=linea.position_for_index(pos-self.documento.texto().inicios[i],px(self.tamano*1.6))?;Some(Bounds::new(caja.origin+p-point(px(self.desplazamiento_x),px(0.)),size(px(1.5),px(self.tamano*1.6))))}
}
pub fn es_codigo(extension:&str)->bool{!["txt","log","md","markdown","rst","adoc",""].contains(&extension)}
impl Focusable for Editor{fn focus_handle(&self,_:&App)->FocusHandle{self.foco.clone()}}
impl Render for Editor{
    fn render(&mut self,_window:&mut Window,cx:&mut Context<Self>)->impl IntoElement{
        self.visibles.clear();let entidad=cx.entity();
        div().id("editor").size_full().relative().track_focus(&self.foco).cursor(CursorStyle::IBeam)
            .on_key_down(cx.listener(Self::teclado))
            .on_mouse_down(MouseButton::Left,cx.listener(|this,e:&MouseDownEvent,w,cx|{w.focus(&this.foco,cx);this.arrastrando=true;let p=this.indice_en(e.position);this.mover(p,e.modifiers.shift,cx);if e.click_count>=3{let t=this.documento.texto();this.seleccion=t.rango_linea(t.linea_de(p));}else if e.click_count==2{let inicio=this.texto()[..p].rfind(char::is_whitespace).map_or(0,|i|i+this.texto()[i..].chars().next().expect("carácter encontrado").len_utf8());let fin=this.texto()[p..].find(char::is_whitespace).map_or(this.texto().len(),|i|p+i);this.seleccion=inicio..fin;}cx.notify();}))
            .on_mouse_move(cx.listener(|this,e:&MouseMoveEvent,_,cx|{if let Some((i,ancho,inicio,_))=&this.arrastre_imagen {let indice= *i;let nuevo=(*ancho+f32::from(e.position.x-*inicio)).clamp(40.,(this.ancho-52.).max(40.));if let Documento::Nota{nota,..}=&mut this.documento&&let Bloque::Imagen{ancho,..}=&mut nota.bloques[indice]{*ancho=Some(nuevo);}this.lista.remeasure();cx.notify();}else if this.arrastrando{this.mover(this.indice_en(e.position),true,cx);}}))
            .on_mouse_up(MouseButton::Left,cx.listener(|this,_,_,cx|{this.arrastrando=false;if let Some((_,_,_,antes))=this.arrastre_imagen.take(){if let Some(nota)=this.documento.nota(){this.historial.agregar(Paso::Nota{antes,despues:nota.clone(),seleccion_antes:this.seleccion.clone(),seleccion_despues:this.seleccion.clone()},false);}this.refrescar(cx);}}))
            .on_mouse_up_out(MouseButton::Left,cx.listener(|this,_,_,cx|{this.arrastrando=false;if let Some((_,_,_,antes))=this.arrastre_imagen.take(){if let Some(nota)=this.documento.nota(){this.historial.agregar(Paso::Nota{antes,despues:nota.clone(),seleccion_antes:this.seleccion.clone(),seleccion_despues:this.seleccion.clone()},false);}this.refrescar(cx);}}))
            .on_scroll_wheel(cx.listener(|this,e:&ScrollWheelEvent,_,cx|{if !this.ajuste&&!e.modifiers.control{let delta=e.delta.pixel_delta(px(24.));this.desplazamiento_x=(this.desplazamiento_x-f32::from(delta.x)).max(0.);cx.notify();}}))
            .child(list(self.lista.clone(),move|i,w,cx|fila::pintar(entidad.clone(),i,w,cx)).size_full())
    }
}
impl EntityInputHandler for Editor{
    fn text_for_range(&mut self,r:Range<usize>,ajustado:&mut Option<Range<usize>>,_:&mut Window,_:&mut Context<Self>)->Option<String>{let r=self.rango_byte(&r);*ajustado=Some(self.utf16(r.start)..self.utf16(r.end));Some(self.texto()[r].into())}
    fn selected_text_range(&mut self,_:bool,_:&mut Window,_:&mut Context<Self>)->Option<UTF16Selection>{Some(UTF16Selection{range:self.utf16(self.seleccion.start)..self.utf16(self.seleccion.end),reversed:self.invertida})}
    fn marked_text_range(&self,_:&mut Window,_:&mut Context<Self>)->Option<Range<usize>>{self.marcado.as_ref().map(|r|self.utf16(r.start)..self.utf16(r.end))}
    fn unmark_text(&mut self,_:&mut Window,_:&mut Context<Self>){self.marcado=None;}
    fn replace_text_in_range(&mut self,r:Option<Range<usize>>,nuevo:&str,_:&mut Window,cx:&mut Context<Self>){
        let rango=r.as_ref().map(|r|self.rango_byte(r)).or(self.marcado.clone()).unwrap_or(self.seleccion.clone());
        if matches!(self.documento,Documento::Texto(_))&&es_codigo(&self.extension)&&nuevo.chars().count()==1 {
            if rango.is_empty()&&[")","]","}","\"","'","`"].contains(&nuevo)&&self.texto()[rango.start..].starts_with(nuevo){self.mover(rango.start+nuevo.len(),false,cx);return;}
            let cierre=match nuevo{"("=>Some(")"),"["=>Some("]"),"{"=>Some("}"),"\""=>Some("\""),"'"=>Some("'"),"`"=>Some("`"),_=>None};
            if let Some(cierre)=cierre{let contenido=format!("{nuevo}{}{cierre}",&self.texto()[rango.clone()]);let posicion=rango.start+nuevo.len();self.reemplazar(rango,&contenido,false,cx);self.mover(posicion,false,cx);return;}
            if nuevo==">"&&["html","htm","xhtml","xml","svg","vue","svelte"].contains(&self.extension.as_str()){
                let antes=&self.texto()[..rango.start];if let Some(i)=antes.rfind('<'){let tag=&antes[i+1..];let nombre=tag.split_whitespace().next().unwrap_or("");if !tag.ends_with('/')&&!nombre.starts_with('/')&&!nombre.starts_with('!')&&!nombre.is_empty()&&!['<','>'].iter().any(|c|tag.contains(*c))&&!["area","base","br","col","embed","hr","img","input","link","meta","param","source","track","wbr"].contains(&nombre.to_lowercase().as_str()){let contenido=format!("></{nombre}>");let posicion=rango.start+1;self.reemplazar(rango,&contenido,false,cx);self.mover(posicion,false,cx);return;}}
            }
        }
        self.reemplazar(rango,nuevo,true,cx);
    }
    fn replace_and_mark_text_in_range(&mut self,r:Option<Range<usize>>,nuevo:&str,sel:Option<Range<usize>>,_:&mut Window,cx:&mut Context<Self>){let r=r.as_ref().map(|r|self.rango_byte(r)).or(self.marcado.clone()).unwrap_or(self.seleccion.clone());let inicio=r.start;self.reemplazar(r,nuevo,false,cx);self.marcado=Some(inicio..inicio+nuevo.len());if let Some(sel)=sel{let byte=|pos|{let mut n=0;for(i,c)in nuevo.char_indices(){if n>=pos{return i;}n+=c.len_utf16();}nuevo.len()};self.seleccion=inicio+byte(sel.start)..inicio+byte(sel.end);}}
    fn bounds_for_range(&mut self,r:Range<usize>,_:Bounds<Pixels>,_:&mut Window,_:&mut Context<Self>)->Option<Bounds<Pixels>>{self.caja_cursor(self.byte(r.start))}
    fn character_index_for_point(&mut self,p:Point<Pixels>,_:&mut Window,_:&mut Context<Self>)->Option<usize>{Some(self.utf16(self.indice_en(p)))}
}

#[cfg(test)]
mod pruebas_integracion {
    use super::{Editor,Documento,Texto};
    use gpui::{App,AppContext,Context};
    fn con_editor(documento:Documento,comprobar:impl FnOnce(&mut Editor,&mut Context<Editor>)+'static) {
        gpui_platform::headless().run(move|cx:&mut App|{
            cx.spawn(async move|cx|{
                cx.update(|cx|{
                    let editor=cx.new(|cx|Editor::nuevo(documento,cx));
                    editor.update(cx,comprobar);
                    cx.quit();
                });
            }).detach();
        });
    }
    #[test]
    fn reemplazar_todos_se_deshace_en_un_paso_con_unicode() {
        con_editor(Documento::Texto(Texto::nuevo("á uno\ná dos".into())),|e,cx|{
            e.reemplazar_varios(vec![(0..2,"é".into()),(7..9,"í".into())],cx);
            assert_eq!(e.texto(),"é uno\ní dos");
            e.deshacer(false,cx);assert_eq!(e.texto(),"á uno\ná dos");
            e.deshacer(true,cx);assert_eq!(e.texto(),"é uno\ní dos");
        });
    }
    #[test]
    fn recargar_limpia_historial_y_el_borrado_de_imagen_se_deshace() {
        let nota=crate::html_nota::leer("<div><b>España</b></div><img src='data:image/png;base64,YQ=='>");
        con_editor(Documento::Nota{texto:Texto::nuevo(nota.plano()),nota},|e,cx|{
            let antes=e.documento.nota().expect("nota").clone();
            e.imagen_seleccionada=Some(1);e.imagen_accion("delete",cx);
            assert_eq!(e.documento.nota().expect("nota").bloques.len(),1);
            e.deshacer(false,cx);assert_eq!(e.documento.nota(),Some(&antes));
            e.cargar(Documento::Texto(Texto::nuevo("Recargado".into())),cx);
            e.deshacer(false,cx);assert_eq!(e.texto(),"Recargado");
            assert_eq!(e.seleccion,0..0);
        });
    }
}
