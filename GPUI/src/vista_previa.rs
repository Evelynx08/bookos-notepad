use crate::{estado::Ajustes, markdown, textos::Textos};
use bookos_ui::{
    Tema,
    controles::{self, EstiloBoton},
    entrada::{Confirmado, Entrada},
    ventana,
};
use gpui::{prelude::*, *};
use std::{path::PathBuf, sync::Arc};
pub fn normalizar(raw: &str) -> String {
    let raw = raw.trim();
    if raw.starts_with("ttps://") || raw.starts_with("ttp://") {
        return format!("h{raw}");
    }
    if raw.starts_with("http://") || raw.starts_with("https://") {
        return raw.into();
    }
    let host = raw
        .split('/')
        .next()
        .unwrap_or("")
        .split(':')
        .next()
        .unwrap_or("");
    if host == "localhost" || host.parse::<std::net::Ipv4Addr>().is_ok() {
        format!("http://{raw}")
    } else if host.contains('.') {
        format!("https://{raw}")
    } else {
        raw.into()
    }
}
pub struct VistaPrevia {
    tema: Tema,
    textos: Textos,
    pub ajustes: Ajustes,
    pub documento: String,
    pub extension: String,
    bloques: Vec<markdown::Bloque>,
    svg: Option<Arc<Image>>,
    url: Entity<Entrada>,
    foco: FocusHandle,
    padre: WeakEntity<crate::app::Vista>,
    error: Option<String>,
    temporal: Option<PathBuf>,
    _subs: Vec<Subscription>,
}
impl VistaPrevia {
    pub fn nueva(
        documento: String,
        extension: String,
        ajustes: Ajustes,
        padre: WeakEntity<crate::app::Vista>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let url = cx.new(|cx| Entrada::new(ajustes.preview_url.clone(), "https://…", cx));
        let sub = cx.subscribe_in(&url, window, |this, _, _: &Confirmado, _, cx| {
            this.abrir(cx)
        });
        let mut this = Self {
            tema: Tema::para_ventana("auto", window),
            textos: Textos::cargar(&ajustes.ui_lang),
            ajustes,
            documento: String::new(),
            extension: String::new(),
            bloques: Vec::new(),
            svg: None,
            url,
            foco: cx.focus_handle(),
            padre,
            error: None,
            temporal: None,
            _subs: vec![sub],
        };
        this.actualizar(documento, extension, cx);
        window.focus(&this.foco, cx);
        this
    }
    pub fn actualizar(&mut self, documento: String, extension: String, cx: &mut Context<Self>) {
        self.documento = documento;
        self.extension = extension;
        self.bloques = markdown::parsear(&self.documento);
        self.svg = (self.extension == "svg").then(|| {
            Arc::new(Image::from_bytes(
                ImageFormat::Svg,
                self.documento.as_bytes().to_vec(),
            ))
        });
        cx.notify();
    }
    fn modo(&self) -> &str {
        if self.ajustes.preview_mode == "auto" {
            if ["md", "markdown", "mdx"].contains(&self.extension.as_str()) {
                "md"
            } else {
                "html"
            }
        } else {
            &self.ajustes.preview_mode
        }
    }
    fn guardar_ajustes(&self, cx: &mut Context<Self>) {
        let ajustes = self.ajustes.clone();
        if let Err(e) = self.padre.update(cx, |p, cx| {
            p.estado.settings.preview_mode = ajustes.preview_mode;
            p.estado.settings.preview_url = ajustes.preview_url;
            p.persistir(cx);
        }) {
            eprintln!("bookos-notepad: no se pudieron aplicar los ajustes de vista previa: {e}");
        }
    }
    fn error(&mut self, mensaje: String, cx: &mut Context<Self>) {
        eprintln!("bookos-notepad: {mensaje}");
        self.error = Some(mensaje);
        cx.notify();
    }
    fn abrir(&mut self, cx: &mut Context<Self>) {
        self.error = None;
        self.ajustes.preview_url = self.url.read(cx).texto().into();
        self.guardar_ajustes(cx);
        let destino = if self.modo() == "url" {
            let normal = normalizar(&self.ajustes.preview_url);
            if !normal.starts_with("https://") && !normal.starts_with("http://") {
                self.error(
                    self.textos.elegir(
                        "Introduce una URL HTTP o HTTPS",
                        "Enter an HTTP or HTTPS URL",
                    ),
                    cx,
                );
                return;
            }
            normal
        } else {
            let dir = std::env::var_os("XDG_RUNTIME_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(std::env::temp_dir);
            let ruta = dir.join(format!("bookos-notepad-vista-{}.html", std::process::id()));
            let contenido = if self.extension == "css" {
                format!(
                    "<!doctype html><meta charset=utf-8><style>{}</style><main><h1>Vista previa CSS</h1><p>Texto de ejemplo</p><button>Botón</button><input placeholder='Entrada'><ul><li>Elemento</li></ul></main>",
                    self.documento
                )
            } else {
                self.documento.clone()
            };
            match crate::archivos::escribir_bytes(&ruta, contenido.as_bytes()) {
                Ok(_) => {}
                Err(e) => {
                    self.error(e, cx);
                    return;
                }
            }
            self.temporal = Some(ruta.clone());
            ruta.to_string_lossy().into_owned()
        };
        let tarea = cx.background_executor().spawn(async move {
            let estado = std::process::Command::new("xdg-open")
                .arg(&destino)
                .status()
                .map_err(|e| e.to_string())?;
            if !estado.success() {
                return Err("El navegador no pudo abrir la vista previa".into());
            }
            Ok::<_, String>(())
        });
        cx.spawn(async move |this, cx| {
            let resultado = tarea.await;
            if let Err(e) = this.update(cx, |this, cx| {
                if let Err(e) = resultado {
                    this.error(e, cx);
                }
                cx.notify();
            }) {
                eprintln!("bookos-notepad: vista previa cerrada: {e}");
            }
        })
        .detach();
    }
}
impl Drop for VistaPrevia {
    fn drop(&mut self) {
        if let Some(ruta) = &self.temporal
            && let Err(e) = std::fs::remove_file(ruta)
        {
            eprintln!("bookos-notepad: no se pudo retirar la vista temporal: {e}");
        }
    }
}
impl Render for VistaPrevia {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let preferencia = self
            .padre
            .upgrade()
            .map(|p| p.read(cx).estado.theme.clone())
            .unwrap_or_else(|| "auto".into());
        self.tema = Tema::para_ventana(&preferencia, window);
        self.textos = Textos::cargar(&self.ajustes.ui_lang);
        let tm = self.tema;
        let textos = self.textos;
        let barra = div()
            .flex()
            .items_center()
            .gap(px(6.))
            .h(px(46.))
            .px(px(10.))
            .children(
                [
                    ("auto", "Auto"),
                    ("html", "HTML"),
                    ("md", "MD"),
                    ("url", "URL"),
                ]
                .into_iter()
                .map(|(modo, etiqueta)| {
                    controles::boton(
                        &tm,
                        modo,
                        etiqueta,
                        if self.ajustes.preview_mode == modo {
                            EstiloBoton::Primario
                        } else {
                            EstiloBoton::Secundario
                        },
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.ajustes.preview_mode = modo.into();
                        this.guardar_ajustes(cx);
                        cx.notify();
                    }))
                }),
            )
            .child(div().flex_1())
            .child(
                controles::boton(&tm, "recargar", "↻", EstiloBoton::Secundario).on_click(
                    cx.listener(|this, _, _, cx| {
                        if this.modo() == "url" {
                            this.abrir(cx);
                        } else {
                            if let Err(e) = this.padre.update(cx, |p, cx| p.actualizar_previa(cx)) {
                                this.error(e.to_string(), cx);
                            }
                        }
                    }),
                ),
            );
        let mut contenido = div().size_full().flex().flex_col().child(barra);
        if self.modo() == "url" {
            contenido = contenido.child(
                div()
                    .mx(px(10.))
                    .mb(px(8.))
                    .h(px(32.))
                    .px(px(10.))
                    .bg(tm.card)
                    .rounded(px(10.))
                    .flex()
                    .items_center()
                    .child(self.url.clone()),
            );
        }
        let cuerpo = if self.modo() == "md" {
            div()
                .id("markdown")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .p(px(26.))
                .children(
                    markdown::Estilo {
                        tema: tm,
                        escala: 1.,
                        fuente: bookos_ui::FUENTE_UI,
                        mono: bookos_ui::FUENTE_MONO,
                    }
                    .pintar(&self.bloques),
                )
                .into_any_element()
        } else if self.extension == "svg" && self.modo() != "url" {
            div()
                .flex_1()
                .p(px(22.))
                .children(
                    self.svg
                        .clone()
                        .map(|i| img(i).size_full().object_fit(ObjectFit::Contain)),
                )
                .into_any_element()
        } else {
            div()
                .flex_1()
                .flex()
                .flex_col()
                .justify_center()
                .items_center()
                .gap(px(14.))
                .p(px(24.))
                .child(textos.elegir(
                    "La vista web se abre en el navegador",
                    "Web previews open in your browser",
                ))
                .child(
                    controles::boton(
                        &tm,
                        "navegador",
                        textos.elegir("Abrir en el navegador", "Open in browser"),
                        EstiloBoton::Primario,
                    )
                    .on_click(cx.listener(|this, _, _, cx| this.abrir(cx))),
                )
                .into_any_element()
        };
        contenido = contenido.child(cuerpo).children(
            self.error
                .as_ref()
                .map(|e| div().p(px(12.)).text_color(tm.red).child(e.clone())),
        );
        ventana::marco(
            &tm,
            window,
            Some(textos.elegir("Vista previa", "Preview").into()),
            None,
            contenido,
            [],
        )
        .track_focus(&self.foco)
        .on_key_down(cx.listener(|_, ev: &KeyDownEvent, w, cx| {
            if ev.keystroke.key == "escape" {
                w.remove_window();
                cx.stop_propagation();
            }
        }))
    }
}
#[cfg(test)]
mod pruebas {
    use super::normalizar;
    #[test]
    fn normaliza_urls() {
        for (a, b) in [
            ("ttps://a.es", "https://a.es"),
            ("localhost:3000", "http://localhost:3000"),
            ("127.0.0.1/a", "http://127.0.0.1/a"),
            ("ejemplo.es", "https://ejemplo.es"),
        ] {
            assert_eq!(normalizar(a), b);
        }
    }
}
