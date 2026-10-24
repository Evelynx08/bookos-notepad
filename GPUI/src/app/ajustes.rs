use super::*;
use bookos_ui::controles::{self,EstiloBoton};
impl Vista {
    pub(super) fn pagina_ajustes(&self,cx:&mut Context<Self>)->AnyElement {
        let tm=self.tema;let t=self.textos;let s=&self.estado.settings;
        let fila=|id,titulo,control|controles::fila(&tm,id,t.t(titulo),None,Some(control));
        let indice=|opciones:&[&str],valor:&str|opciones.iter().position(|v|*v==valor).unwrap_or(0);
        let temas=if t.ingles{["Auto","Light","Dark"]}else{["Auto","Claro","Oscuro"]};
        let fuentes=["Sans","Serif","Mono"];
        let idioma=["Auto","Español","English"];
        let apariencia=vec![
            fila("tema","theme",controles::segmentado(&tm,"tema",&temas,indice(&["auto","light","dark"],&self.estado.theme),cx,|this,i,cx|{this.estado.theme=["auto","light","dark"][i].into();this.tema=Tema::cargar(i==2||i==0&&this.tema.oscuro);this.aplicar_ajustes(cx);this.persistir(cx);}).into_any_element()),
            fila("fuente","font",controles::segmentado(&tm,"fuente",&fuentes,indice(&["sans","serif","mono"],&s.font),cx,|this,i,cx|{this.estado.settings.font=["sans","serif","mono"][i].into();this.aplicar_ajustes(cx);this.persistir(cx);}).into_any_element()),
            fila("idioma","ui.lang",controles::segmentado(&tm,"idioma",&idioma,indice(&["auto","es","en"],&s.ui_lang),cx,|this,i,cx|{this.estado.settings.ui_lang=["auto","es","en"][i].into();this.aplicar_ajustes(cx);this.persistir(cx);}).into_any_element()),
        ];
        let mut editor=Vec::new();
        for (id,clave,valor) in [("wrap","editor.wrap",s.wrap),("status","editor.status",s.show_status),("accent","editor.accent",s.file_accent),("restore","editor.restore",s.restore_session),("autosave","editor.autosave",s.auto_save)] {
            editor.push(fila(id,clave,controles::interruptor(&tm,id,valor,cx.listener(move|this,_,_,cx|{
                let s=&mut this.estado.settings;match id{"wrap"=>s.wrap= !s.wrap,"status"=>s.show_status= !s.show_status,"accent"=>s.file_accent= !s.file_accent,"restore"=>s.restore_session= !s.restore_session,"autosave"=>s.auto_save= !s.auto_save,_=>unreachable!("opciones de interruptor cerradas")}
                this.aplicar_ajustes(cx);this.persistir(cx);
            })).into_any_element()));
        }
        editor.push(controles::fila(&tm,"demora",t.elegir("Espera del autoguardado","Autosave delay"),None,Some(controles::segmentado(&tm,"demora",&["1 s","2 s","5 s"],match s.auto_save_delay{0..=1000=>0,1001..=2000=>1,_=>2},cx,|this,i,cx|{this.estado.settings.auto_save_delay=[1000,2000,5000][i];this.persistir(cx);}).into_any_element())));
        div().id("ajustes").size_full().overflow_y_scroll().p(px(24.)).flex().flex_col()
            .child(div().flex().items_center().justify_between().child(div().text_size(px(26.)).font_weight(FontWeight::BOLD).child(t.t("settings")))
                .child(super::interfaz::boton(&tm,"volver",t.elegir("Volver","Back")).on_click(cx.listener(|this,_,w,cx|{this.ajustes=false;w.focus(&this.pestanas[this.activa].editor.focus_handle(cx),cx);cx.notify();}))))
            .child(controles::seccion(&tm,t.t("sec.appearance"))).child(controles::tarjeta(&tm,apariencia))
            .child(controles::seccion(&tm,t.t("sec.editor"))).child(controles::tarjeta(&tm,editor))
            .child(controles::seccion(&tm,t.t("sec.preview")))
            .child(controles::tarjeta(&tm,vec![controles::fila(&tm,"modo",t.elegir("Modo por defecto","Default mode"),None,Some(controles::segmentado(&tm,"modo",&["Auto","HTML","MD","URL"],indice(&["auto","html","md","url"],&s.preview_mode),cx,|this,i,cx|{this.estado.settings.preview_mode=["auto","html","md","url"][i].into();this.aplicar_ajustes(cx);this.persistir(cx);}).into_any_element()))]))
            .child(controles::seccion(&tm,t.t("sec.shortcuts")))
            .child(controles::tarjeta(&tm,vec![
                controles::fila_navegable(&tm,"personalizar",t.t("sc.customize"),None).on_click(cx.listener(|this,_,w,cx|{this.paleta=true;w.focus(&this.foco,cx);cx.notify();})),
                controles::fila_navegable(&tm,"restablecer",t.t("sc.reset"),None).on_click(cx.listener(|this,_,_,cx|{this.estado.settings.shortcuts.clear();this.persistir(cx);})),
            ]))
            .child(controles::seccion(&tm,t.t("sec.updates")))
            .child(controles::boton(&tm,"actualizar",t.t("upd.bookosSettings"),EstiloBoton::Secundario).on_click(cx.listener(|this,_,_,cx|{
                let resultado=std::fs::write("/tmp/bookos-start-page","actualizacion").map_err(|e|e.to_string())
                    .and_then(|_|std::process::Command::new("bookos-settings").args(["--page","actualizacion"]).spawn().map(|mut proceso|{std::thread::spawn(move||if let Err(e)=proceso.wait(){eprintln!("bookos-notepad: actualizaciones: {e}");});}).map_err(|e|e.to_string()));
                if let Err(e)=resultado{this.avisar(e,true,cx);}
            })))
            .child(controles::seccion(&tm,t.t("sec.about")))
            .child(div().text_size(px(14.)).child(format!("BookOS Notepad · {} {}",t.t("about.versionPre"),env!("CARGO_PKG_VERSION"))))
            .into_any_element()
    }
    pub(super) fn panel_atajos(&self,cx:&mut Context<Self>)->AnyElement {
        let tm=self.tema;let t=self.textos;
        let mut filas=Vec::new();let mut seccion="";
        for a in atajos::ACCIONES {
            if a.seccion!=seccion{seccion=a.seccion;filas.push(div().mt(px(12.)).text_color(tm.tx2).text_size(px(12.)).child(seccion).into_any_element());}
            let id=a.id;
            let tecla=if self.captura==Some(id){t.elegir("Pulsa una combinación… (Esc cancela)","Press a shortcut… (Esc cancels)")}else{atajos::tecla(a,&self.estado.settings.shortcuts).into()};
            filas.push(div().id(id).flex().items_center().gap(px(8.)).py(px(4.)).on_mouse_down(MouseButton::Right,cx.listener(move|this,_,w,cx|{this.captura=Some(id);w.focus(&this.foco,cx);cx.notify();}))
                .child(div().flex_1().text_size(px(13.)).child(if t.ingles{nombre_accion(id)}else{a.etiqueta}))
                .child(super::interfaz::boton(&tm,SharedString::from(format!("editar-{id}")),tecla).on_click(cx.listener(move|this,_,w,cx|{this.captura=Some(id);w.focus(&this.foco,cx);cx.notify();})))
                .child(super::interfaz::boton(&tm,SharedString::from(format!("restablecer-{id}")),"↺").on_click(cx.listener(move|this,_,_,cx|{this.estado.settings.shortcuts.remove(id);this.persistir(cx);})))
                .into_any_element());
        }
        div().absolute().inset_0().bg(tm.scrim).flex().items_center().justify_center().on_mouse_down(MouseButton::Left,|_,_,cx|cx.stop_propagation())
            .child(div().id("paleta").w(px(600.)).h(relative(0.85)).rounded(px(20.)).bg(tm.card).p(px(20.)).shadow(bookos_ui::tema::sombra_modal()).flex().flex_col()
                .child(div().flex().justify_between().items_center().child(t.t("sec.shortcuts"))
                    .child(super::interfaz::boton(&tm,"cerrar-atajos","×").on_click(cx.listener(|this,_,_,cx|{this.paleta=false;this.captura=None;cx.notify();}))))
                .child(div().id("lista-atajos").flex_1().min_h_0().overflow_y_scroll().children(filas))
                .child(super::interfaz::boton(&tm,"reset-atajos",t.t("sc.reset")).on_click(cx.listener(|this,_,_,cx|{this.estado.settings.shortcuts.clear();this.persistir(cx);})))).into_any_element()
    }
}
fn nombre_accion(id:&str)->&str {
    match id {"new-tab"=>"New note","new-tab-alt"=>"New file tab","open-file"=>"Open file","save"=>"Save","save-as"=>"Save as","close-tab"=>"Close tab","next-tab-mru"=>"Last used tab","prev-tab"=>"Previous tab","preview"=>"Preview","settings"=>"Settings","fullscreen"=>"Fullscreen","shortcuts"=>"Keyboard shortcuts","bold"=>"Bold","italic"=>"Italic","underline"=>"Underline","undo"=>"Undo","redo"|"redo-alt"=>"Redo","select-all"=>"Select all","find"=>"Find","replace"=>"Find and replace","find-next"=>"Find next","move-line-up"=>"Move line up","move-line-dn"=>"Move line down","dup-line"=>"Duplicate line","zoom-in"=>"Zoom in","zoom-out"=>"Zoom out","zoom-reset"=>"Reset zoom",_=>id}
}
