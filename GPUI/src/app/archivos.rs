use super::*;
use crate::{archivos,estado};
impl Vista {
    pub(super) fn elegir(&mut self,guardar:bool,imagen:bool,w:&mut Window,cx:&mut Context<Self>) {
        let id=self.pestanas[self.activa].id;let continuacion=self.continuacion.take();
        let nombre=self.pestanas[self.activa].origen.ruta().map(std::path::Path::to_path_buf);
        let tarea=cx.background_executor().spawn(async move {archivos::elegir(if guardar{"save"}else{"open"},nombre.as_deref(),imagen)});
        cx.spawn_in(w,async move|this,cx| {
            let resultado=tarea.await;
            if let Err(e)=this.update_in(cx,|this,w,cx|match resultado {
                Ok(rutas)=>{let mut guardado=false;for ruta in rutas {
                    if guardar {if let Some(i)=this.indice(id){guardado=this.guardar_como(i,ruta,cx);}}
                    else if imagen {match std::fs::read(&ruta){Ok(bytes)=>this.insertar_imagen(id,&bytes,cx),Err(e)=>this.avisar(e.to_string(),true,cx)}}
                    else{this.abrir_ruta(ruta,None,w,cx);}
                }
                    if guardado&&let Some(continuacion)=continuacion {match continuacion {
                        Continuacion::Cerrar(id)=>if let Some(i)=this.indice(id){this.quitar(i,w,cx);},
                        Continuacion::Salir=>{this.dialogo=Some(Dialogo::Salir);this.confirmar(true,w,cx);},
                    }}
                },
                Err(e)=>this.avisar(e,true,cx),
            }) {eprintln!("bookos-notepad: selector cerrado con la ventana: {e}");}
        }).detach();
    }
    pub(super) fn abrir_ruta(&mut self,ruta:PathBuf,codificacion:Option<String>,w:&mut Window,cx:&mut Context<Self>) {
        let ruta=std::fs::canonicalize(&ruta).unwrap_or(ruta);
        if let Some(i)=self.pestanas.iter().position(|p|p.origen.ruta()==Some(ruta.as_path())) {self.activar(i,w,cx);return;}
        let tarea=cx.background_executor().spawn(async move {let leido=archivos::leer(&ruta,codificacion.as_deref());(ruta,leido)});
        cx.spawn_in(w,async move|this,cx| {
            let (ruta,resultado)=tarea.await;
            if let Err(e)=this.update_in(cx,|this,w,cx|match resultado {
                Ok(a)=>{
                    if let Some(i)=this.pestanas.iter().position(|p|p.origen.ruta()==Some(ruta.as_path())) {this.activar(i,w,cx);return;}
                    this.agregar(Documento::Texto(Texto::nuevo(a.texto)),Origen::Archivo{ruta:Some(ruta),codificacion:a.codificacion,eol:a.eol,fecha:Some(a.fecha)},w,cx);
                },
                Err(e)=>this.avisar(e,true,cx),
            }) {eprintln!("bookos-notepad: apertura cancelada al cerrar: {e}");}
        }).detach();
    }
    pub(super) fn cargar_notas(&mut self,abrir_reciente:bool,w:&mut Window,cx:&mut Context<Self>) {
        self.cargando_notas=true;self.error_notas=None;
        let tarea=cx.background_executor().spawn(async {notas::listar()});
        cx.spawn_in(w,async move|this,cx| {
            let resultado=tarea.await;
            if let Err(e)=this.update_in(cx,|this,w,cx| {
                this.cargando_notas=false;
                match resultado {
                    Ok(notas)=>{
                        this.notas=notas;
                        if abrir_reciente&&this.pestanas.len()==1&&!this.pestanas[0].sucia&&this.pestanas[0].editor.read(cx).documento.nota().is_some_and(Nota::vacia)&&!this.notas.is_empty(){this.abrir_nota(0,w,cx);}
                    },
                    Err(e)=>{this.error_notas=Some(e.clone());this.avisar(format!("No se pudieron leer las notas: {e}"),true,cx);},
                }
                cx.notify();
            }) {eprintln!("bookos-notepad: carga de notas cancelada: {e}");}
        }).detach();
    }
    pub(super) fn abrir_nota(&mut self,i:usize,w:&mut Window,cx:&mut Context<Self>) {
        let n=&self.notas[i];
        if let Some(i)=self.pestanas.iter().position(|p|p.origen.ruta()==Some(n.ruta.as_path())){self.activar(i,w,cx);return;}
        let texto=Texto::nuevo(n.nota.plano());
        self.agregar(Documento::Nota{nota:n.nota.clone(),texto},Origen::Nota(n.ruta.clone()),w,cx);
    }
    pub(super) fn guardar(&mut self,como:bool,w:&mut Window,cx:&mut Context<Self>) {
        if como||self.pestanas[self.activa].origen.ruta().is_none(){self.elegir(true,false,w,cx);}else{self.guardar_indice(self.activa,false,cx);}
    }
    pub(super) fn guardar_como(&mut self,i:usize,ruta:PathBuf,cx:&mut Context<Self>)->bool {
        if self.pestanas.iter().enumerate().any(|(j,p)|j!=i&&p.origen.ruta()==Some(ruta.as_path())){self.avisar("El destino ya está abierto en otra pestaña",true,cx);return false;}
        let e=self.pestanas[i].editor.read(cx);
        let resultado=if let Some(nota)=e.documento.nota(){archivos::escribir_bytes(&ruta,crate::html_nota::escribir(nota).as_bytes())}
            else{let Origen::Archivo{codificacion,eol,..}=&self.pestanas[i].origen else{unreachable!("un texto pertenece a un archivo")};archivos::escribir(&ruta,e.texto(),codificacion,*eol)};
        match resultado {
            Ok(fecha)=>{
                if let Origen::Archivo{ruta:actual,fecha:anterior,..}=&mut self.pestanas[i].origen {
                    *actual=Some(ruta.clone());*anterior=Some(fecha);
                    self.pestanas[i].editor.update(cx,|e,cx|{e.extension=ruta.extension().map(|s|s.to_string_lossy().to_lowercase()).unwrap_or_default();cx.notify();});
                    self.pestanas[i].sucia=false;self.pestanas[i].guardado=Guardado::Listo;self.sesion(cx);
                }
                self.avisar(self.textos.elegir("Guardado","Saved"),false,cx);true
            },
            Err(e)=>{self.avisar(e,true,cx);false},
        }
    }
    pub(super) fn guardar_indice(&mut self,i:usize,forzar:bool,cx:&mut Context<Self>)->bool {
        let p=&self.pestanas[i];let id=p.id;
        let resultado=match &p.origen {
            Origen::Nota(ruta)=>{
                let nota=p.editor.read(cx).documento.nota().expect("las notas contienen un documento enriquecido");
                if nota.vacia()&&!ruta.exists(){self.pestanas[i].sucia=false;self.pestanas[i].guardado=Guardado::Nuevo;return true;}
                std::fs::create_dir_all(estado::carpeta_notas()).map_err(|e|e.to_string()).and_then(|_|archivos::escribir_bytes(ruta,crate::html_nota::escribir(nota).as_bytes()))
            },
            Origen::Archivo{ruta:Some(ruta),codificacion,eol,fecha}=>{
                let actual=std::fs::metadata(ruta).and_then(|m|m.modified());
                if !forzar&&actual.as_ref().ok()!=fecha.as_ref() {
                    self.avisar("El archivo cambió o desapareció del disco. Revisa los cambios antes de sobrescribirlo.",true,cx);
                    self.dialogo=Some(Dialogo::Sobrescribir(id));return false;
                }
                archivos::escribir(ruta,p.editor.read(cx).texto(),codificacion,*eol)
            },
            Origen::Archivo{ruta:None,..}=>return false,
        };
        self.pestanas[i].guardado=Guardado::Guardando;
        match resultado {
            Ok(fecha)=>{
                let p=&mut self.pestanas[i];p.sucia=false;p.guardado=Guardado::Listo;
                if let Origen::Archivo{fecha:anterior,..}=&mut p.origen{*anterior=Some(fecha);}
                if let Origen::Nota(ruta)=&p.origen {
                    let nota=p.editor.read(cx).documento.nota().expect("origen de nota").clone();
                    if let Some(n)=self.notas.iter_mut().find(|n|n.ruta==*ruta){n.nota=nota;n.fecha=fecha;}
                    else{self.notas.push(NotaDisco{ruta:ruta.clone(),nota,fecha});}
                    self.notas.sort_by_key(|n|std::cmp::Reverse(n.fecha));
                }
                cx.notify();true
            },
            Err(e)=>{self.pestanas[i].guardado=Guardado::Error(e.clone());self.avisar(e,true,cx);false},
        }
    }
    pub(super) fn pedir_cerrar(&mut self,i:usize,w:&mut Window,cx:&mut Context<Self>) {
        if self.pestanas[i].sucia {
            if matches!(self.pestanas[i].origen,Origen::Nota(_))&&self.guardar_indice(i,false,cx){self.quitar(i,w,cx);}
            else{self.dialogo=Some(Dialogo::Cerrar(self.pestanas[i].id));}
        }else{self.quitar(i,w,cx);}
        cx.notify();
    }
    pub(super) fn quitar(&mut self,i:usize,w:&mut Window,cx:&mut Context<Self>) {
        let id=self.pestanas.remove(i).id;self.mru.retain(|p|*p!=id);
        self.activa=self.activa.saturating_sub(usize::from(i<self.activa)).min(self.pestanas.len().saturating_sub(1));
        if self.pestanas.is_empty(){self.nueva_nota(w,cx);}else{self.activar(self.activa,w,cx);}
        self.sesion(cx);self.actualizar_previa(cx);
    }
    pub(super) fn pedir_salir(&mut self,_w:&mut Window,cx:&mut Context<Self>)->bool {
        for i in 0..self.pestanas.len(){if matches!(self.pestanas[i].origen,Origen::Nota(_))&&self.pestanas[i].sucia{self.guardar_indice(i,false,cx);}}
        if self.pestanas.iter().any(|p|p.sucia){self.dialogo=Some(Dialogo::Salir);cx.notify();false}else{true}
    }
    pub(super) fn comprobar_disco(&mut self,cx:&mut Context<Self>) {
        if self.dialogo.is_some()||self.pestanas.is_empty(){return;}
        let p=&mut self.pestanas[self.activa];
        if let Origen::Archivo{ruta:Some(ruta),fecha,..}=&mut p.origen {
            match std::fs::metadata(&*ruta).and_then(|m|m.modified()) {
                Ok(actual) if Some(actual)!=*fecha=>{
                    if p.sucia{self.avisar("El archivo cambió en disco y tienes cambios pendientes",true,cx);}
                    else{self.dialogo=Some(Dialogo::Recargar(p.id,actual));cx.notify();}
                },
                Err(e)=>self.avisar(format!("No se pudo comprobar el archivo: {e}"),true,cx),_=>{},
            }
        }
    }
    pub(super) fn recargar(&mut self,i:usize,codificacion:Option<&str>,cx:&mut Context<Self>) {
        let Origen::Archivo{ruta:Some(ruta),..}=&self.pestanas[i].origen else{return;};
        match archivos::leer(ruta,codificacion) {
            Ok(a)=>{
                self.pestanas[i].editor.update(cx,|e,cx|e.cargar(Documento::Texto(Texto::nuevo(a.texto)),cx));
                if let Origen::Archivo{codificacion,eol,fecha,..}=&mut self.pestanas[i].origen{*codificacion=a.codificacion;*eol=a.eol;*fecha=Some(a.fecha);}
                self.pestanas[i].sucia=false;self.pestanas[i].guardado=Guardado::Listo;self.sesion(cx);self.actualizar_previa(cx);
            },Err(e)=>self.avisar(e,true,cx),
        }
    }
    pub(super) fn confirmar(&mut self,guardar:bool,w:&mut Window,cx:&mut Context<Self>) {
        let Some(dialogo)=self.dialogo.take() else{return;};
        match dialogo {
            Dialogo::Borrar(ruta)=>{
                match if ruta.exists(){notas::borrar_en(&estado::carpeta_notas(),&ruta)}else{Ok(())} {
                    Ok(())=>{self.notas.retain(|n|n.ruta!=ruta);if let Some(i)=self.pestanas.iter().position(|p|p.origen.ruta()==Some(ruta.as_path())){self.quitar(i,w,cx);}},
                    Err(e)=>self.avisar(e,true,cx),
                }
            },
            Dialogo::Cerrar(id)=>if let Some(i)=self.indice(id){
                if guardar&&self.pestanas[i].origen.ruta().is_none(){self.activa=i;self.continuacion=Some(Continuacion::Cerrar(id));self.elegir(true,false,w,cx);}
                else if !guardar||self.guardar_indice(i,false,cx){self.quitar(i,w,cx);}
            },
            Dialogo::Salir=>{
                if guardar {
                    for i in 0..self.pestanas.len(){if self.pestanas[i].sucia {
                        if self.pestanas[i].origen.ruta().is_none(){self.activa=i;self.continuacion=Some(Continuacion::Salir);self.elegir(true,false,w,cx);return;}
                        if !self.guardar_indice(i,false,cx){return;}
                    }}
                }
                if let Some(h)=self.previa.take()&&let Err(e)=h.update(cx,|_,w,_|w.remove_window()){eprintln!("bookos-notepad: vista previa ya cerrada: {e}");}
                w.remove_window();
            },
            Dialogo::Recargar(id,fecha)=>if let Some(i)=self.indice(id){if guardar{self.recargar(i,None,cx);}else if let Origen::Archivo{fecha:f,..}=&mut self.pestanas[i].origen{*f=Some(fecha);}},
            Dialogo::Sobrescribir(id)=>if guardar&&let Some(i)=self.indice(id){self.guardar_indice(i,true,cx);},
        }
        if self.pestanas.is_empty(){self.nueva_nota(w,cx);}
        cx.notify();
    }
}
