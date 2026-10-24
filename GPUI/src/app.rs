mod ajustes;
mod archivos;
mod buscar;
mod interfaz;

use crate::dialogo::Dialogo;
use crate::{
    archivos::FinLinea,
    atajos,
    editor::{Cambio, Documento, Editor, ImagenPegada, nota::Nota, texto::Texto},
    estado::{Estado, Persistencia, Sesion},
    instancia::Instancia,
    notas::{self, NotaDisco},
    textos::Textos,
    vista_previa::VistaPrevia,
};
use bookos_ui::{
    Tema, Toast,
    entrada::{Cambio as CambioEntrada, Entrada},
    toast,
};
use futures::{StreamExt, channel::mpsc::UnboundedReceiver};
use gpui::{prelude::*, *};
use std::{
    path::PathBuf,
    time::{Duration, SystemTime},
};

enum Origen {
    Nota(PathBuf),
    Archivo {
        ruta: Option<PathBuf>,
        codificacion: String,
        eol: FinLinea,
        fecha: Option<SystemTime>,
    },
}
impl Origen {
    fn ruta(&self) -> Option<&std::path::Path> {
        match self {
            Self::Nota(p) => Some(p),
            Self::Archivo { ruta, .. } => ruta.as_deref(),
        }
    }
}
enum Guardado {
    Nuevo,
    Pendiente,
    Guardando,
    Listo,
    Error(String),
}
enum Continuacion {
    Cerrar(usize),
    Salir,
}
struct Pestana {
    id: usize,
    editor: Entity<Editor>,
    origen: Origen,
    sucia: bool,
    generacion: u64,
    guardado: Guardado,
    _subs: Vec<Subscription>,
}
pub struct Vista {
    tema: Tema,
    textos: Textos,
    pub estado: Estado,
    persistencia: Persistencia,
    toast: Option<Toast>,
    pestanas: Vec<Pestana>,
    activa: usize,
    mru: Vec<usize>,
    mru_ciclo: Vec<usize>,
    mru_cursor: usize,
    siguiente_id: usize,
    notas: Vec<NotaDisco>,
    miniaturas: std::collections::HashMap<PathBuf, std::sync::Arc<RenderImage>>,
    error_notas: Option<String>,
    cargando_notas: bool,
    buscador: Entity<Entrada>,
    buscar: Entity<Entrada>,
    reemplazo: Entity<Entrada>,
    busqueda_visible: bool,
    reemplazo_visible: bool,
    regex: bool,
    mayusculas: bool,
    coincidencia: Option<usize>,
    ajustes: bool,
    paleta: bool,
    captura: Option<&'static str>,
    dialogo: Option<Dialogo>,
    continuacion: Option<Continuacion>,
    foco: FocusHandle,
    previa: Option<WindowHandle<VistaPrevia>>,
    previa_gen: u64,
    _instancia: Option<Instancia>,
    _subs: Vec<Subscription>,
}
impl Vista {
    pub fn nueva(
        rutas: Vec<PathBuf>,
        instancia: Option<Instancia>,
        receptor: Option<UnboundedReceiver<Vec<PathBuf>>>,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let (persistencia, estado) = Persistencia::cargar();
        let textos = Textos::cargar(&estado.settings.ui_lang);
        let buscador = cx.new(|cx| Entrada::new("", textos.t("notas.buscar"), cx));
        let buscar = cx.new(|cx| Entrada::new("", textos.elegir("Buscar…", "Find…"), cx));
        let reemplazo = cx.new(|cx| Entrada::new("", textos.elegir("Reemplazar…", "Replace…"), cx));
        let subs = vec![
            cx.observe_window_appearance(w, |this, w, cx| this.recargar_tema(w, cx)),
            cx.observe_window_activation(w, |this, w, cx| {
                if w.is_window_active() {
                    this.recargar_tema(w, cx);
                    this.comprobar_disco(cx);
                }
            }),
            cx.subscribe(&buscador, |_, _, _: &CambioEntrada, cx| cx.notify()),
            cx.subscribe(&buscar, |this, _, _: &CambioEntrada, cx| {
                this.actualizar_busqueda(cx)
            }),
        ];
        let mut this = Self {
            tema: Tema::para_ventana(&estado.theme, w),
            textos,
            estado,
            persistencia,
            toast: None,
            pestanas: Vec::new(),
            activa: 0,
            mru: Vec::new(),
            mru_ciclo: Vec::new(),
            mru_cursor: 0,
            siguiente_id: 0,
            notas: Vec::new(),
            miniaturas: Default::default(),
            error_notas: None,
            cargando_notas: false,
            buscador,
            buscar,
            reemplazo,
            busqueda_visible: false,
            reemplazo_visible: false,
            regex: false,
            mayusculas: false,
            coincidencia: None,
            ajustes: false,
            paleta: false,
            captura: None,
            dialogo: None,
            continuacion: None,
            foco: cx.focus_handle(),
            previa: None,
            previa_gen: 0,
            _instancia: instancia,
            _subs: subs,
        };
        let debil = cx.weak_entity();
        w.on_window_should_close(cx, move |w, cx| {
            debil
                .update(cx, |this, cx| this.pedir_salir(w, cx))
                .unwrap_or(true)
        });
        let sesion = this.estado.settings.session.clone();
        this.nueva_nota(w, cx);
        this.recargar_tema(w, cx);
        this.cargar_notas(rutas.is_empty(), w, cx);
        if this.estado.settings.restore_session {
            for entrada in sesion {
                if entrada.path.is_file() {
                    this.abrir_ruta(entrada.path, entrada.encoding, w, cx);
                }
            }
        }
        for ruta in rutas {
            this.abrir_ruta(ruta, None, w, cx);
        }
        if let Some(mut rx) = receptor {
            cx.spawn_in(w, async move |this, cx| {
                while let Some(rutas) = rx.next().await {
                    if let Err(e) = this.update_in(cx, |this, w, cx| {
                        for ruta in rutas {
                            this.abrir_ruta(ruta, None, w, cx);
                        }
                        w.activate_window();
                    }) {
                        eprintln!("bookos-notepad: la ventana receptora ya se cerró: {e}");
                        break;
                    }
                }
            })
            .detach();
        }
        if let Some(e) = this.persistencia.error.clone() {
            this.avisar(format!("Estado protegido: {e}"), true, cx);
        }
        this
    }
    fn avisar(&mut self, mensaje: impl Into<String>, error: bool, cx: &mut Context<Self>) {
        let mensaje = mensaje.into();
        if error {
            eprintln!("bookos-notepad: {mensaje}");
        }
        toast::mostrar(|v: &mut Self| &mut v.toast, self, mensaje, error, cx);
    }
    pub fn persistir(&mut self, cx: &mut Context<Self>) {
        if let Err(e) = self.persistencia.guardar(&self.estado) {
            self.avisar(e, true, cx);
        }
        cx.notify();
    }
    fn sesion(&mut self, cx: &mut Context<Self>) {
        self.estado.settings.session = self
            .pestanas
            .iter()
            .filter_map(|p| match &p.origen {
                Origen::Archivo {
                    ruta: Some(ruta),
                    codificacion,
                    ..
                } => Some(Sesion {
                    path: ruta.clone(),
                    encoding: Some(codificacion.clone()),
                }),
                _ => None,
            })
            .collect();
        self.persistir(cx);
    }
    fn recargar_tema(&mut self, w: &Window, cx: &mut Context<Self>) {
        self.tema = Tema::para_ventana(&self.estado.theme, w);
        self.aplicar_ajustes(cx);
    }
    fn aplicar_ajustes(&mut self, cx: &mut Context<Self>) {
        self.textos = Textos::cargar(&self.estado.settings.ui_lang);
        for entrada in [&self.buscador, &self.buscar, &self.reemplazo] {
            entrada.update(cx, |e, cx| {
                e.color_pista = self.tema.tx2;
                e.color_cursor = self.tema.blue;
                cx.notify();
            });
        }
        for p in &self.pestanas {
            p.editor.update(cx, |e, cx| {
                e.tema = self.tema;
                e.ajuste = self.estado.settings.wrap;
                e.tamano = 15. * self.estado.settings.zoom as f32 / 100.;
                e.fuente = match self.estado.settings.font.as_str() {
                    "mono" => bookos_ui::FUENTE_MONO,
                    "serif" => "Noto Serif",
                    _ => bookos_ui::FUENTE_UI,
                };
                e.lista.remeasure();
                cx.notify();
            });
        }
        self.actualizar_previa(cx);
        cx.notify();
    }
    fn agregar(
        &mut self,
        documento: Documento,
        origen: Origen,
        w: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Una nota inicial vacía no debe crear una pestaña sobrante al abrir archivos.
        if self.pestanas.len() == 1
            && self.pestanas[0]
                .editor
                .read(cx)
                .documento
                .nota()
                .is_some_and(Nota::vacia)
            && !self.pestanas[0].sucia
        {
            self.pestanas.clear();
            self.mru.clear();
        }
        let editor = cx.new(|cx| Editor::nuevo(documento, cx));
        editor.update(cx, |e, _| {
            e.extension = match &origen {
                Origen::Nota(_) => "html".into(),
                Origen::Archivo { ruta, .. } => ruta
                    .as_ref()
                    .and_then(|r| r.extension())
                    .map(|s| s.to_string_lossy().to_lowercase())
                    .unwrap_or_default(),
            }
        });
        let id = self.siguiente_id;
        self.siguiente_id += 1;
        let subs = vec![
            cx.subscribe(&editor, move |this, _, _: &Cambio, cx| this.cambio(id, cx)),
            cx.subscribe(&editor, move |this, _, imagen: &ImagenPegada, cx| {
                this.insertar_imagen(id, &imagen.0, cx)
            }),
            cx.observe(&editor, |_, _, cx| cx.notify()),
        ];
        let guardado = if origen.ruta().is_some_and(std::path::Path::exists) {
            Guardado::Listo
        } else {
            Guardado::Nuevo
        };
        self.pestanas.push(Pestana {
            id,
            editor,
            origen,
            sucia: false,
            generacion: 0,
            guardado,
            _subs: subs,
        });
        self.aplicar_ajustes(cx);
        self.activar(self.pestanas.len() - 1, w, cx);
        self.sesion(cx);
    }
    fn nueva_nota(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.agregar(
            Documento::Nota {
                nota: Nota::default(),
                texto: Texto::nuevo(String::new()),
            },
            Origen::Nota(notas::nombre_nuevo()),
            w,
            cx,
        );
    }
    fn nuevo_archivo(&mut self, w: &mut Window, cx: &mut Context<Self>) {
        self.agregar(
            Documento::Texto(Texto::nuevo(String::new())),
            Origen::Archivo {
                ruta: None,
                codificacion: "utf-8".into(),
                eol: FinLinea::Lf,
                fecha: None,
            },
            w,
            cx,
        );
    }
    fn activar(&mut self, i: usize, w: &mut Window, cx: &mut Context<Self>) {
        self.activa = i;
        self.ajustes = false;
        let id = self.pestanas[i].id;
        self.mru.retain(|p| *p != id);
        self.mru.push(id);
        w.focus(&self.pestanas[i].editor.focus_handle(cx), cx);
        self.actualizar_busqueda(cx);
        self.actualizar_previa(cx);
        cx.notify();
    }
    fn indice(&self, id: usize) -> Option<usize> {
        self.pestanas.iter().position(|p| p.id == id)
    }
    fn cambio(&mut self, id: usize, cx: &mut Context<Self>) {
        let Some(i) = self.indice(id) else {
            return;
        };
        let p = &mut self.pestanas[i];
        p.sucia = true;
        p.generacion += 1;
        p.guardado = Guardado::Pendiente;
        let generacion = p.generacion;
        let demora = match &p.origen {
            Origen::Nota(_) => 600,
            Origen::Archivo { ruta: Some(_), .. } if self.estado.settings.auto_save => {
                self.estado.settings.auto_save_delay
            }
            _ => {
                self.actualizar_busqueda(cx);
                self.actualizar_previa(cx);
                cx.notify();
                return;
            }
        };
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(demora))
                .await;
            if let Err(e) = this.update(cx, |this, cx| {
                if let Some(i) = this.indice(id)
                    && this.pestanas[i].generacion == generacion
                {
                    this.guardar_indice(i, false, cx);
                }
            }) {
                eprintln!("bookos-notepad: autoguardado cancelado al cerrar: {e}");
            }
        })
        .detach();
        self.actualizar_busqueda(cx);
        self.actualizar_previa(cx);
        cx.notify();
    }
    fn insertar_imagen(&mut self, id: usize, bytes: &[u8], cx: &mut Context<Self>) {
        let Some(i) = self.indice(id) else {
            return;
        };
        if self.pestanas[i].editor.read(cx).documento.nota().is_none() {
            self.avisar("Las imágenes se insertan en notas", true, cx);
            return;
        }
        match crate::imagenes::insertar(bytes, &cx.svg_renderer()) {
            Ok(fuente) => self.pestanas[i]
                .editor
                .update(cx, |e, cx| e.insertar_imagen(fuente, cx)),
            Err(e) => self.avisar(e, true, cx),
        }
    }
    pub fn actualizar_previa(&mut self, cx: &mut Context<Self>) {
        if self.previa.is_none() || self.pestanas.is_empty() {
            return;
        }
        self.previa_gen += 1;
        let generacion = self.previa_gen;
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(300))
                .await;
            if let Err(e) = this.update(cx, |this, cx| {
                if this.previa_gen != generacion {
                    return;
                }
                let Some(handle) = this.previa else {
                    return;
                };
                let e = this.pestanas[this.activa].editor.read(cx);
                let texto = e
                    .documento
                    .nota()
                    .map(crate::html_nota::escribir)
                    .unwrap_or_else(|| e.texto().into());
                let ext = e.extension.clone();
                let ajustes = this.estado.settings.clone();
                if handle
                    .update(cx, |p, _, cx| {
                        p.ajustes = ajustes;
                        p.actualizar(texto, ext, cx);
                    })
                    .is_err()
                {
                    this.previa = None;
                }
            }) {
                eprintln!("bookos-notepad: vista previa cerrada: {e}");
            }
        })
        .detach();
    }
    fn alternar_previa(&mut self, cx: &mut Context<Self>) {
        if let Some(handle) = self.previa.take()
            && handle.update(cx, |_, w, _| w.remove_window()).is_ok()
        {
            return;
        }
        let e = self.pestanas[self.activa].editor.read(cx);
        let texto = e
            .documento
            .nota()
            .map(crate::html_nota::escribir)
            .unwrap_or_else(|| e.texto().into());
        let ext = e.extension.clone();
        let ajustes = self.estado.settings.clone();
        let padre = cx.weak_entity();
        let opciones = bookos_ui::ventana::opciones(
            crate::APP_ID,
            size(px(720.), px(640.)),
            size(px(480.), px(360.)),
            cx,
        );
        match cx.open_window(opciones, move |w, cx| {
            cx.new(|cx| VistaPrevia::nueva(texto, ext, ajustes, padre, w, cx))
        }) {
            Ok(h) => self.previa = Some(h),
            Err(e) => self.avisar(format!("No se pudo abrir la vista previa: {e}"), true, cx),
        }
    }
    fn teclado(&mut self, e: &KeyDownEvent, w: &mut Window, cx: &mut Context<Self>) {
        let combo = atajos::combinacion(e);
        if let Some(id) = self.captura {
            if e.keystroke.key == "escape" {
                self.captura = None;
            } else if !["control", "shift", "alt", "super", "meta"]
                .contains(&e.keystroke.key.as_str())
            {
                if let Some(otra) = atajos::conflicto(id, &combo, &self.estado.settings.shortcuts) {
                    self.avisar(format!("Ese atajo ya pertenece a {otra}"), true, cx);
                } else {
                    self.estado.settings.shortcuts.insert(id.into(), combo);
                    self.captura = None;
                    self.persistir(cx);
                }
            }
            cx.stop_propagation();
            w.prevent_default();
            return;
        }
        if self.dialogo.is_some() {
            if e.keystroke.key == "escape" {
                self.dialogo = None;
                cx.notify();
            }
            cx.stop_propagation();
            w.prevent_default();
            return;
        }
        if e.keystroke.key == "escape" {
            self.busqueda_visible = false;
            self.reemplazo_visible = false;
            self.paleta = false;
            self.ajustes = false;
            self.pestanas[self.activa].editor.update(cx, |e, cx| {
                e.busqueda = Default::default();
                cx.notify();
            });
            w.focus(&self.pestanas[self.activa].editor.focus_handle(cx), cx);
            cx.stop_propagation();
            cx.notify();
            return;
        }
        if self.paleta {
            if atajos::ACCIONES.iter().any(|a| {
                a.id == "shortcuts"
                    && atajos::coincide(atajos::tecla(a, &self.estado.settings.shortcuts), &combo)
            }) {
                self.paleta = false;
                cx.notify();
            }
            cx.stop_propagation();
            w.prevent_default();
            return;
        }
        if self.busqueda_visible
            && e.keystroke.key == "enter"
            && (self.buscar.focus_handle(cx).is_focused(w)
                || self.reemplazo.focus_handle(cx).is_focused(w))
        {
            self.siguiente(e.keystroke.modifiers.shift, w, cx);
            cx.stop_propagation();
            w.prevent_default();
            return;
        }
        if e.keystroke.key == "f3" && e.keystroke.modifiers.shift {
            self.siguiente(true, w, cx);
            cx.stop_propagation();
            return;
        }
        if let Some(a) = atajos::ACCIONES
            .iter()
            .find(|a| atajos::coincide(atajos::tecla(a, &self.estado.settings.shortcuts), &combo))
        {
            if [
                "undo",
                "redo",
                "redo-alt",
                "select-all",
                "bold",
                "italic",
                "underline",
                "move-line-up",
                "move-line-dn",
                "dup-line",
            ]
            .contains(&a.id)
                && !self.pestanas[self.activa]
                    .editor
                    .read(cx)
                    .foco
                    .is_focused(w)
            {
                return;
            }
            self.accion(a.id, w, cx);
            cx.stop_propagation();
            w.prevent_default();
        }
    }
    fn accion(&mut self, id: &str, w: &mut Window, cx: &mut Context<Self>) {
        let editor = self.pestanas[self.activa].editor.clone();
        match id {
            "new-tab" => self.nueva_nota(w, cx),
            "new-tab-alt" => self.nuevo_archivo(w, cx),
            "open-file" => self.elegir(false, false, w, cx),
            "save" => self.guardar(false, w, cx),
            "save-as" => self.guardar(true, w, cx),
            "close-tab" => self.pedir_cerrar(self.activa, w, cx),
            "next-tab-mru" => {
                if self.mru_ciclo.is_empty() {
                    self.mru_ciclo = self.mru.iter().rev().copied().collect();
                    self.mru_cursor = 0;
                }
                self.mru_cursor = (self.mru_cursor + 1) % self.mru_ciclo.len();
                if let Some(i) = self.indice(self.mru_ciclo[self.mru_cursor]) {
                    self.activar(i, w, cx);
                }
            }
            "prev-tab" => self.activar(
                if self.activa == 0 {
                    self.pestanas.len() - 1
                } else {
                    self.activa - 1
                },
                w,
                cx,
            ),
            "preview" => self.alternar_previa(cx),
            "settings" => {
                self.ajustes = !self.ajustes;
                w.focus(&self.foco, cx);
                cx.notify();
            }
            "fullscreen" => w.toggle_fullscreen(),
            "shortcuts" => {
                self.paleta = !self.paleta;
                w.focus(&self.foco, cx);
                cx.notify();
            }
            "bold" | "italic" | "underline" => {
                editor.update(cx, |e, cx| e.dar_formato(id, None, cx))
            }
            "undo" => editor.update(cx, |e, cx| e.deshacer(false, cx)),
            "redo" | "redo-alt" => editor.update(cx, |e, cx| e.deshacer(true, cx)),
            "select-all" => editor.update(cx, |e, cx| {
                e.seleccion = 0..e.texto().len();
                cx.notify();
            }),
            "find" | "replace" => {
                self.busqueda_visible = true;
                self.reemplazo_visible = id == "replace";
                w.focus(&self.buscar.focus_handle(cx), cx);
                self.actualizar_busqueda(cx);
            }
            "find-next" => self.siguiente(false, w, cx),
            "move-line-up" => editor.update(cx, |e, cx| e.linea(-1, false, cx)),
            "move-line-dn" => editor.update(cx, |e, cx| e.linea(1, false, cx)),
            "dup-line" => editor.update(cx, |e, cx| e.linea(1, true, cx)),
            "zoom-in" | "zoom-out" | "zoom-reset" => {
                self.estado.settings.zoom = match id {
                    "zoom-in" => (self.estado.settings.zoom + 10).min(250),
                    "zoom-out" => self.estado.settings.zoom.saturating_sub(10).max(50),
                    _ => 100,
                };
                self.aplicar_ajustes(cx);
                self.persistir(cx);
            }
            _ => self.avisar(format!("Acción desconocida: {id}"), true, cx),
        }
    }
}
