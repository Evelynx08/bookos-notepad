use super::*;
use bookos_ui::{ventana,lateral,icono,controles::{self,EstiloBoton}};

pub(super) fn boton(tm:&Tema,id:impl Into<ElementId>,texto:impl Into<SharedString>)->Stateful<Div> {
    div().id(id).px(px(9.)).py(px(6.)).rounded(px(8.)).text_size(px(13.)).cursor_pointer().hover({let hov=tm.hov;move|s|s.bg(hov)}).child(texto.into())
}
pub(super) fn boton_estado(tm:&Tema,id:impl Into<ElementId>,texto:impl Into<SharedString>,activo:bool)->Stateful<Div> {
    boton(tm,id,texto).when(activo,|d|d.bg(tm.blue_soft).text_color(tm.blue))
}
fn titulo(p:&Pestana,cx:&App)->String {
    match &p.origen {
        Origen::Nota(_)=>notas::resumen(p.editor.read(cx).documento.nota().expect("pestaña de nota")).0,
        Origen::Archivo{ruta,..}=>ruta.as_ref().and_then(|p|p.file_name()).map(|s|s.to_string_lossy().into_owned()).unwrap_or_else(||"Sin título".into()),
    }
}
impl Vista {
    fn barra(&self,cx:&mut Context<Self>)->AnyElement {
        let tm=self.tema;let t=self.textos;
        let mut barra=div().id("barra-herramientas").overflow_x_scroll().flex().flex_shrink_0().h(px(48.)).items_center().gap(px(4.)).px(px(12.)).border_b_1().border_color(tm.brd);
        for (id,svg,etiqueta) in [("new-tab","iconos/new-btn.svg",t.t("notas.nueva")),("open-file","iconos/open-btn.svg",t.elegir("Abrir","Open")),("save","iconos/save-btn.svg",t.elegir("Guardar","Save")),("preview","iconos/preview-btn.svg",t.elegir("Vista previa","Preview"))] {
            barra=barra.child(boton(&tm,id,etiqueta).flex().gap(px(6.)).items_center().child(icono(svg,16.,tm.tx)).on_click(cx.listener(move|this,_,w,cx|this.accion(id,w,cx))));
        }
        barra.child(div().flex_1())
            .child(boton(&tm,"atajos","⌘").on_click(cx.listener(|this,_,w,cx|this.accion("shortcuts",w,cx))))
            .child(boton(&tm,"ajustes",t.t("settings")).on_click(cx.listener(|this,_,w,cx|this.accion("settings",w,cx))))
            .into_any_element()
    }
    fn lista_notas(&self,cx:&mut Context<Self>)->AnyElement {
        let tm=self.tema;let t=self.textos;let filtro=self.buscador.read(cx).texto().to_lowercase();
        let activa=self.pestanas[self.activa].origen.ruta();
        let mut filas=Vec::new();let mut grupo=String::new();
        for (i,n) in self.notas.iter().enumerate() {
            if !n.nota.plano().to_lowercase().contains(&filtro){continue;}
            let g=crate::fechas::grupo(n.fecha,SystemTime::now(),t.ingles);
            if g!=grupo{grupo=g;filas.push(div().mt(px(16.)).mb(px(6.)).text_size(px(12.)).text_color(tm.tx2).child(grupo.clone()).into_any_element());}
            let (titulo,extracto)=notas::resumen(&n.nota);let ruta=n.ruta.clone();
            filas.push(div().id(("nota",i)).rounded(px(12.)).p(px(12.)).cursor_pointer().when(activa==Some(n.ruta.as_path()),|d|d.bg(tm.blue_soft)).hover(move|s|s.bg(tm.hov))
                .on_click(cx.listener(move|this,_,w,cx|this.abrir_nota(i,w,cx)))
                .child(div().flex().items_center().gap(px(6.)).child(div().flex_1().min_w_0().truncate().font_weight(FontWeight::SEMIBOLD).text_size(px(14.)).child(titulo))
                    .child(boton(&tm,("borrar-nota",i),"×").text_color(tm.red).on_click(cx.listener(move|this,_,_,cx|{cx.stop_propagation();this.dialogo=Some(Dialogo::Borrar(ruta.clone()));cx.notify();}))))
                .child(div().text_size(px(12.)).text_color(tm.tx2).truncate().child(extracto))
                .child(div().mt(px(5.)).text_size(px(11.)).text_color(tm.tx3).child(format!("{} · {}",crate::fechas::corta(n.fecha),crate::fechas::larga(n.fecha,t.ingles))))
                .into_any_element());
        }
        let vacia=filas.is_empty();
        div().id("notas").w(px(384.)).h_full().flex_none().px(px(18.)).pb(px(20.)).overflow_y_scroll()
            .child(div().pt(px(14.)).pb(px(12.)).text_size(px(25.)).font_weight(FontWeight::BOLD).child(t.t("notas.titulo")))
            .child(div().h(px(36.)).px(px(10.)).bg(tm.card).rounded(px(12.)).flex().items_center().child(self.buscador.clone()))
            .when(self.cargando_notas,|d|d.child(div().mt(px(16.)).text_color(tm.tx2).child(t.elegir("Cargando…","Loading…"))))
            .when_some(self.error_notas.as_ref(),|d,error|d.child(div().mt(px(12.)).text_size(px(13.)).text_color(tm.red).child(error.clone())
                .child(boton(&tm,"reintentar",t.t("notas.reintentar")).on_click(cx.listener(|this,_,w,cx|this.cargar_notas(false,w,cx))))))
            .when(vacia&&!self.cargando_notas&&self.error_notas.is_none(),|d|d.child(div().mt(px(18.)).text_color(tm.tx2).text_size(px(13.)).child(if filtro.is_empty(){t.elegir("Todavía no hay notas. Crea una para empezar.","No notes yet. Create one to start.")}else{t.t("notas.sinResultados").replace("{texto}",&filtro)})))
            .children(filas).into_any_element()
    }
    fn pestanas(&self,cx:&mut Context<Self>)->AnyElement {
        let tm=self.tema;
        div().id("pestanas").flex().flex_shrink_0().h(px(38.)).overflow_x_scroll().border_b_1().border_color(tm.brd)
            .children(self.pestanas.iter().enumerate().map(|(i,p)| {
                let e=p.editor.read(cx);let color=if e.documento.nota().is_some(){tm.tx2}else{match e.extension.as_str(){"rs"|"html"|"css"=>tm.orange,"md"|"json"|"js"|"ts"=>tm.blue,_=>tm.green}};
                div().id(("pestana",p.id)).flex_none().max_w(px(240.)).min_w(px(120.)).px(px(10.)).flex().gap(px(8.)).items_center().cursor_pointer()
                    .when(self.estado.settings.file_accent,|d|d.border_t_2().border_color(color))
                    .when(i==self.activa,|d|d.bg(tm.card)).hover(move|s|s.bg(tm.hov))
                    .on_click(cx.listener(move|this,_,w,cx|this.activar(i,w,cx)))
                    .child(div().flex_1().min_w_0().truncate().text_size(px(12.)).child(format!("{}{}",if p.sucia{"● "}else{""},titulo(p,cx))))
                    .child(boton(&tm,("cerrar-pestana",p.id),"×").on_click(cx.listener(move|this,_,w,cx|{cx.stop_propagation();this.pedir_cerrar(i,w,cx);})))
            }))
            .child(boton(&tm,"nuevo-archivo","+").on_click(cx.listener(|this,_,w,cx|this.nuevo_archivo(w,cx))))
            .into_any_element()
    }
    fn formato(&self,cx:&mut Context<Self>)->AnyElement {
        let tm=self.tema;let p=&self.pestanas[self.activa];let e=p.editor.read(cx);
        let estado=match &p.guardado {Guardado::Nuevo=>self.textos.t("notas.guardado.nuevo"),Guardado::Pendiente=>self.textos.t("notas.guardado.pendiente"),Guardado::Guardando=>self.textos.t("notas.guardado.guardando"),Guardado::Listo=>self.textos.t("notas.guardado.guardado"),Guardado::Error(e)=>format!("{}: {e}",self.textos.elegir("Error · pulsa para reintentar","Error · click to retry"))};
        let mut fila=div().id("formato").flex().flex_shrink_0().items_center().h(px(38.)).px(px(8.)).gap(px(2.)).overflow_x_scroll();
        for (id,texto,activo) in [("bold","B",e.formato.negrita),("italic","I",e.formato.cursiva),("underline","U",e.formato.subrayado),("strike","S̶",e.formato.tachado),("list","•",false),("number","1.",false),("tasks","☑",false),("left","⇤",false),("center","↔",false),("right","⇥",false),("clear","Tx",false)] {
            fila=fila.child(boton_estado(&tm,id,texto,activo).on_click(cx.listener(move|this,_,w,cx| {
                let editor=&this.pestanas[this.activa].editor;
                editor.update(cx,|e,cx|if ["list","number","tasks","left","center","right"].contains(&id){e.bloque_formato(id,cx);}else{e.dar_formato(id,None,cx);});
                w.focus(&editor.focus_handle(cx),cx);
            })));
        }
        if e.imagen_seleccionada.is_some(){
            for (id,etiqueta) in [("original",self.textos.t("imagen.original")),("fit",self.textos.t("imagen.ajustar")),("left","⇤".into()),("center","↔".into()),("right","⇥".into()),("delete","×".into())] {
                fila=fila.child(boton(&tm,SharedString::from(format!("imagen-{id}")),etiqueta).on_click(cx.listener(move|this,_,_,cx|this.pestanas[this.activa].editor.update(cx,|e,cx|e.imagen_accion(id,cx)))));
            }
        }
        for (id,etiqueta) in [("color","A"),("background","▨")] {
            fila=fila.child(boton(&tm,id,etiqueta).on_click(cx.listener(move|this,_,w,cx|{
                let editor=&this.pestanas[this.activa].editor;
                editor.update(cx,|e,cx|{
                    let actual=if id=="color"{e.formato.color.as_deref()}else{e.formato.fondo.as_deref()};
                    let paleta=["#d32f2f","#1565c0","#2e7d32","#f9a825"];
                    let siguiente=actual.and_then(|c|paleta.iter().position(|p|*p==c)).map_or(Some(paleta[0]),|i|paleta.get(i+1).copied());
                    e.dar_formato(id,siguiente.map(str::to_owned),cx);
                });w.focus(&editor.focus_handle(cx),cx);
            })));
        }
        fila.child(boton(&tm,"insertar-imagen","▧").child(icono("iconos/imagen-btn.svg",15.,tm.tx)).on_click(cx.listener(|this,_,w,cx|this.elegir(false,true,w,cx))))
            .child(div().flex_1())
            .child(boton(&tm,"estado-nota",estado).text_size(px(11.)).text_color(if matches!(p.guardado,Guardado::Error(_)){tm.red}else{tm.tx2}).on_click(cx.listener(|this,_,_,cx|{this.guardar_indice(this.activa,false,cx);})))
            .into_any_element()
    }
    fn estado_editor(&self,cx:&mut Context<Self>)->AnyElement {
        let tm=self.tema;let p=&self.pestanas[self.activa];let e=p.editor.read(cx);let texto=e.texto();
        let linea=e.documento.texto().linea_de(e.cursor());let columna=texto[e.documento.texto().inicios[linea]..e.cursor()].chars().count()+1;
        let ruta=p.origen.ruta().map(|p|p.display().to_string()).unwrap_or_default();
        let mut barra=div().flex().flex_shrink_0().items_center().h(px(30.)).px(px(10.)).gap(px(8.)).border_t_1().border_color(tm.brd).text_size(px(11.)).text_color(tm.tx2)
            .child(div().flex_1().min_w_0().truncate().child(ruta))
            .child(format!("{} · {} {} · {} {} · {} {} · {}:{}",if e.extension.is_empty(){"txt"}else{&e.extension},texto.chars().count(),self.textos.elegir("car.","chars"),e.documento.texto().inicios.len(),self.textos.elegir("líneas","lines"),texto.split_whitespace().count(),self.textos.elegir("palabras","words"),linea+1,columna));
        if let Origen::Archivo{codificacion,eol,..}=&p.origen {
            barra=barra.child(boton(&tm,"codificacion",codificacion.clone()).text_size(px(11.)).on_click(cx.listener(|this,_,_,cx| {
                if this.pestanas[this.activa].sucia{this.avisar("Guarda los cambios antes de releer con otra codificación",true,cx);return;}
                let Origen::Archivo{codificacion,..}=&this.pestanas[this.activa].origen else{return;};
                let lista=crate::archivos::CODIFICACIONES;let i=lista.iter().position(|c|*c==codificacion).unwrap_or(0);this.recargar(this.activa,Some(lista[(i+1)%lista.len()]),cx);
            }))).child(boton(&tm,"eol",eol.etiqueta()).text_size(px(11.)).on_click(cx.listener(|this,_,_,cx| {
                if let Origen::Archivo{eol,..}=&mut this.pestanas[this.activa].origen{*eol=if *eol==FinLinea::Lf{FinLinea::Crlf}else{FinLinea::Lf};}
                this.cambio(this.pestanas[this.activa].id,cx);
            })));
        }
        barra.child(boton(&tm,"zoom-menos","−").on_click(cx.listener(|this,_,w,cx|this.accion("zoom-out",w,cx))))
            .child(boton(&tm,"zoom-reset",format!("{}%",self.estado.settings.zoom)).on_click(cx.listener(|this,_,w,cx|this.accion("zoom-reset",w,cx))))
            .child(boton(&tm,"zoom-mas","+").on_click(cx.listener(|this,_,w,cx|this.accion("zoom-in",w,cx))))
            .into_any_element()
    }
    fn pintar_dialogo(&self,cx:&mut Context<Self>)->AnyElement {
        let tm=self.tema;let t=self.textos;let d=self.dialogo.as_ref().expect("diálogo visible");
        let (titulo,mensaje,si)=match d {
            Dialogo::Borrar(_)=>(t.t("notas.borrar.titulo"),t.elegir("La nota se borrará del disco. No se puede deshacer.","The note will be deleted from disk. This cannot be undone."),t.t("notas.borrar.si")),
            Dialogo::Cerrar(_)|Dialogo::Salir=>(t.elegir("Cambios sin guardar","Unsaved changes"),t.elegir("Guarda los cambios antes de cerrar.","Save your changes before closing."),t.elegir("Guardar","Save")),
            Dialogo::Recargar(_,_)|Dialogo::Sobrescribir(_)=>(t.elegir("El archivo cambió en disco","The file changed on disk"),t.elegir("Revisa la versión del disco antes de continuar.","Review the disk version before continuing."),if matches!(d,Dialogo::Recargar(..)){t.elegir("Recargar","Reload")}else{t.elegir("Sobrescribir","Overwrite")}),
        };
        div().absolute().inset_0().bg(tm.scrim).flex().items_center().justify_center().on_mouse_down(MouseButton::Left,|_,_,cx|cx.stop_propagation())
            .child(div().w(px(430.)).p(px(24.)).rounded(px(20.)).bg(tm.card).shadow(bookos_ui::tema::sombra_modal()).flex().flex_col().gap(px(14.))
                .child(div().text_size(px(19.)).font_weight(FontWeight::SEMIBOLD).child(titulo))
                .child(div().text_size(px(14.)).child(mensaje))
                .child(div().flex().flex_wrap().justify_end().gap(px(8.))
                    .child(controles::boton(&tm,"cancelar-dialogo",t.elegir("Cancelar","Cancel"),EstiloBoton::Secundario).on_click(cx.listener(|this,_,_,cx|{this.dialogo=None;cx.notify();})))
                    .when(matches!(d,Dialogo::Cerrar(_)|Dialogo::Salir|Dialogo::Recargar(..)),|el|el.child(controles::boton(&tm,"descartar",if matches!(d,Dialogo::Recargar(..)){t.elegir("Conservar","Keep")}else{t.elegir("Descartar","Discard")},EstiloBoton::Secundario).on_click(cx.listener(|this,_,w,cx|this.confirmar(false,w,cx)))))
                    .child(controles::boton(&tm,"confirmar-dialogo",si,if matches!(d,Dialogo::Borrar(_)|Dialogo::Sobrescribir(_)){EstiloBoton::Peligro}else{EstiloBoton::Primario}).on_click(cx.listener(|this,_,w,cx|this.confirmar(true,w,cx))))))
            .with_animation("dialogo",Animation::new(bookos_ui::DURACION_DIALOGO),|d,t|d.opacity(t))
            .into_any_element()
    }
}
impl Render for Vista {
    fn render(&mut self,w:&mut Window,cx:&mut Context<Self>)->impl IntoElement {
        if self.pestanas.is_empty(){self.nueva_nota(w,cx);}
        let tema=Tema::para_ventana(&self.estado.theme,w);
        if tema.oscuro!=self.tema.oscuro{self.tema=tema;self.aplicar_ajustes(cx);}
        let tm=self.tema;
        let mut detalle=div().flex_1().min_w_0().h_full().flex().flex_col().bg(tm.pane).child(self.barra(cx)).child(self.pestanas(cx));
        if self.ajustes{detalle=detalle.child(div().flex_1().min_h_0().child(self.pagina_ajustes(cx)));}
        else {
            if self.busqueda_visible{detalle=detalle.child(self.barra_busqueda(cx));}
            if self.pestanas[self.activa].editor.read(cx).documento.nota().is_some(){detalle=detalle.child(self.formato(cx));}
            detalle=detalle.child(div().flex_1().min_h_0().child(self.pestanas[self.activa].editor.clone()));
            if let Some(error)=&self.pestanas[self.activa].editor.read(cx).error_imagen {
                detalle=detalle.child(div().p(px(8.)).text_size(px(12.)).text_color(tm.red).child(error.clone()));
            }
            if self.estado.settings.show_status{detalle=detalle.child(self.estado_editor(cx));}
        }
        let contenido=div().size_full().flex()
            .child(lateral::plegable(!self.estado.settings.lista_oculta,self.lateral_gen,self.lista_notas(cx)))
            .child(detalle);
        let alternar=ventana::boton_lateral(&tm,cx.listener(|this,_,_,cx|{this.estado.settings.lista_oculta= !this.estado.settings.lista_oculta;this.lateral_gen+=1;this.persistir(cx);}));
        let mut capas=Vec::new();
        // El botón del marco llama directamente a remove_window; esta zona conserva la confirmación de cierre.
        capas.push(div().absolute().top(px(4.)).right(px(12.)).w(px(32.)).h(px(28.)).id("cerrar-con-confirmacion").cursor_pointer()
            .flex().items_center().justify_center().hover(move|s|s.bg(tm.red).rounded(px(6.)))
            .child(icono("ventana-cerrar",15.,tm.tx))
            .on_mouse_down(MouseButton::Left,|_,_,cx|cx.stop_propagation())
            .on_click(cx.listener(|this,_,w,cx|if this.pedir_salir(w,cx){if let Some(h)=this.previa.take()&&let Err(e)=h.update(cx,|_,w,_|w.remove_window()){eprintln!("bookos-notepad: vista previa ya cerrada: {e}");}w.remove_window();})).into_any_element());
        if let Some(t)=&self.toast{capas.push(toast::pintar(t,&tm).into_any_element());}
        if self.paleta{capas.push(self.panel_atajos(cx));}
        if self.dialogo.is_some(){capas.push(self.pintar_dialogo(cx));}
        ventana::marco(&tm,w,Some(self.textos.t("app.title").into()),Some(alternar),contenido,capas)
            .id("raiz").track_focus(&self.foco).capture_key_down(cx.listener(Self::teclado))
            .on_key_up(cx.listener(|this,e:&KeyUpEvent,_,_|if !e.keystroke.modifiers.control&&!e.keystroke.modifiers.platform{this.mru_ciclo.clear();}))
            .on_drop::<ExternalPaths>(cx.listener(|this,rutas:&ExternalPaths,w,cx| {
                let id=this.pestanas[this.activa].id;
                for ruta in rutas.paths(){if crate::imagenes::es_imagen(ruta)&&this.pestanas[this.activa].editor.read(cx).documento.nota().is_some(){match std::fs::read(ruta){Ok(bytes)=>this.insertar_imagen(id,&bytes,cx),Err(e)=>this.avisar(e.to_string(),true,cx)}}else{this.abrir_ruta(ruta.clone(),None,w,cx);}}
            }))
    }
}
