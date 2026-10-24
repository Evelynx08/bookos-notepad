use super::*;
use bookos_ui::{
    controles::{self, EstiloBoton},
    icono,
};

pub(super) fn boton(
    tm: &Tema,
    id: impl Into<ElementId>,
    texto: impl Into<SharedString>,
) -> Stateful<Div> {
    div()
        .id(id)
        .px(px(9.))
        .py(px(6.))
        .rounded(px(8.))
        .text_size(px(13.))
        .cursor_pointer()
        .hover({
            let hov = tm.hov;
            move |s| s.bg(hov)
        })
        .child(texto.into())
}
pub(super) fn boton_estado(
    tm: &Tema,
    id: impl Into<ElementId>,
    texto: impl Into<SharedString>,
    activo: bool,
) -> Stateful<Div> {
    boton(tm, id, texto).when(activo, |d| d.bg(tm.blue).text_color(tm.on_accent))
}
fn titulo(p: &Pestana, cx: &App) -> String {
    match &p.origen {
        Origen::Nota(_) => {
            notas::resumen(p.editor.read(cx).documento.nota().expect("pestaña de nota")).0
        }
        Origen::Archivo { ruta, .. } => ruta
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Sin título".into()),
    }
}
impl Vista {
    fn barra_ventana(&self, cx: &mut Context<Self>) -> AnyElement {
        let tm = self.tema;
        let mut controles = div()
            .flex()
            .gap(px(4.))
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation());
        for (id, svg) in [
            ("minimizar", "minimize"),
            ("maximizar", "maximize"),
            ("cerrar", "close"),
        ] {
            controles = controles.child(
                div()
                    .id(id)
                    .w(px(30.))
                    .h(px(26.))
                    .rounded(px(6.))
                    .bg(tm.sbg)
                    .opacity(0.7)
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .hover(move |s| {
                        s.opacity(1.)
                            .bg(if id == "cerrar" { tm.red } else { tm.act })
                    })
                    .child(icono(svg, 16., tm.tx))
                    .on_click(cx.listener(move |this, _, w, cx| match id {
                        "minimizar" => w.minimize_window(),
                        "maximizar" => w.zoom_window(),
                        _ => {
                            if this.pedir_salir(w, cx) {
                                if let Some(h) = this.previa.take()
                                    && let Err(e) = h.update(cx, |_, w, _| w.remove_window())
                                {
                                    eprintln!("bookos-notepad: vista previa ya cerrada: {e}");
                                }
                                w.remove_window();
                            }
                        }
                    })),
            );
        }
        div()
            .id("barra-titulo")
            .h(px(36.))
            .flex_none()
            .pr(px(8.))
            .flex()
            .items_center()
            .justify_end()
            .on_mouse_down(MouseButton::Left, |ev, w, _| {
                if ev.click_count == 2 {
                    w.zoom_window();
                } else {
                    w.start_window_move();
                }
            })
            .child(controles)
            .into_any_element()
    }
    fn boton_icono(
        &self,
        id: &'static str,
        svg: &'static str,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let tm = self.tema;
        div()
            .id(id)
            .size(px(38.))
            .flex_none()
            .rounded(px(14.))
            .bg(tm.card)
            .shadow(tm.sombra_reposo())
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .hover(move |s| s.bg(bookos_ui::tema::mezclar(tm.card, tm.tx, 0.88)))
            .child(icono(svg, 18., tm.tx))
            .on_click(cx.listener(move |this, _, w, cx| match id {
                "alternar-lista" => {
                    this.estado.settings.lista_oculta = !this.estado.settings.lista_oculta;
                    this.persistir(cx);
                }
                "insertar-imagen" => this.elegir(false, true, w, cx),
                "tasks" => {
                    let editor = &this.pestanas[this.activa].editor;
                    editor.update(cx, |e, cx| e.bloque_formato("tasks", cx));
                    w.focus(&editor.focus_handle(cx), cx);
                }
                "borrar-nota" => {
                    if let Origen::Nota(ruta) = &this.pestanas[this.activa].origen {
                        this.dialogo = Some(Dialogo::Borrar(ruta.clone()));
                        cx.notify();
                    }
                }
                _ => this.accion(id, w, cx),
            }))
    }
    fn barra(&self, amplia: bool, cx: &mut Context<Self>) -> AnyElement {
        let tm = self.tema;
        let t = self.textos;
        let nota = matches!(self.pestanas[self.activa].origen, Origen::Nota(_));
        let mut acciones = div().flex().items_center().gap(px(8.)).flex_none();
        if !self.ajustes {
            if nota {
                acciones = acciones
                    .child(self.boton_icono("insertar-imagen", "imagen-btn", cx))
                    .child(self.boton_icono("tasks", "tareas-btn", cx));
            } else {
                acciones = acciones
                    .child(self.boton_icono("save", "save-btn", cx))
                    .child(self.boton_icono("preview", "preview-btn", cx));
            }
        }
        acciones = acciones
            .child(self.boton_icono("open-file", "open-btn", cx))
            .child(self.boton_icono("settings", "settings-btn", cx))
            .child(
                div()
                    .id("new-tab")
                    .h(px(38.))
                    .px(px(12.))
                    .rounded(px(14.))
                    .bg(tm.blue)
                    .text_color(tm.on_accent)
                    .text_size(px(13.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .cursor_pointer()
                    .hover(move |s| s.bg(bookos_ui::tema::mezclar(tm.blue, tm.on_accent, 0.88)))
                    .child(icono("new-btn", 18., tm.on_accent))
                    .child(t.t("notas.nueva"))
                    .on_click(cx.listener(|this, _, w, cx| this.nueva_nota(w, cx))),
            );
        div()
            .id("cabecera")
            .h(px(56.))
            .flex_none()
            .flex()
            .items_center()
            .gap(px(10.))
            .when(amplia, |d| {
                d.child(self.boton_icono("alternar-lista", "lista-btn", cx))
            })
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_baseline()
                    .gap(px(10.))
                    .child(
                        div()
                            .truncate()
                            .text_size(px(22.))
                            .font_weight(FontWeight::BOLD)
                            .child(if self.ajustes {
                                t.t("settings")
                            } else if nota {
                                t.t("notas.titulo")
                            } else {
                                titulo(&self.pestanas[self.activa], cx)
                            }),
                    )
                    .when(
                        amplia && nota && !self.ajustes && self.error_notas.is_none(),
                        |d| {
                            d.child(div().text_size(px(12.)).text_color(tm.tx2).child(format!(
                                "{} {}",
                                self.notas.len(),
                                t.elegir(
                                    if self.notas.len() == 1 {
                                        "nota"
                                    } else {
                                        "notas"
                                    },
                                    if self.notas.len() == 1 {
                                        "note"
                                    } else {
                                        "notes"
                                    }
                                )
                            )))
                        },
                    ),
            )
            .child(acciones)
            .into_any_element()
    }
    fn lista_notas(&self, cx: &mut Context<Self>) -> AnyElement {
        let tm = self.tema;
        let t = self.textos;
        let filtro = self.buscador.read(cx).texto().to_lowercase();
        let activa = self.pestanas[self.activa].origen.ruta();
        let mut filas = Vec::new();
        let mut grupo = String::new();
        for (i, n) in self.notas.iter().enumerate() {
            if !n.nota.plano().to_lowercase().contains(&filtro) {
                continue;
            }
            let g = crate::fechas::grupo(n.fecha, SystemTime::now(), t.ingles);
            if g != grupo {
                grupo = g;
                filas.push(
                    div()
                        .mt(px(16.))
                        .mb(px(6.))
                        .px(px(12.))
                        .text_size(px(12.))
                        .text_color(tm.tx2)
                        .child(grupo.clone())
                        .into_any_element(),
                );
            }
            let (titulo, extracto) = notas::resumen(&n.nota);
            let miniatura = self.miniaturas.get(&n.ruta);
            filas.push(
                div()
                    .id(("nota", i))
                    .min_h(px(56.))
                    .mb(px(2.))
                    .px(px(12.))
                    .py(px(8.))
                    .rounded(px(14.))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .cursor_pointer()
                    .when(activa == Some(n.ruta.as_path()), |d| {
                        d.bg(tm.blue.opacity(0.10))
                    })
                    .hover(move |s| s.bg(tm.hov))
                    .on_click(cx.listener(move |this, _, w, cx| this.abrir_nota(i, w, cx)))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .child(
                                div()
                                    .truncate()
                                    .text_size(px(14.))
                                    .font_weight(if activa == Some(n.ruta.as_path()) {
                                        FontWeight::SEMIBOLD
                                    } else {
                                        FontWeight::MEDIUM
                                    })
                                    .child(titulo),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap(px(6.))
                                    .text_size(px(11.5))
                                    .text_color(tm.tx2)
                                    .child(
                                        div()
                                            .flex_none()
                                            .text_color(tm.tx)
                                            .child(crate::fechas::corta(n.fecha)),
                                    )
                                    .child(div().min_w_0().truncate().child(extracto)),
                            ),
                    )
                    .when_some(miniatura, |d, imagen| {
                        d.child(
                            div()
                                .size(px(40.))
                                .flex_none()
                                .rounded(px(8.))
                                .overflow_hidden()
                                .child(img(imagen.clone()).size_full().rounded(px(8.))),
                        )
                    })
                    .into_any_element(),
            );
        }
        let vacia = filas.is_empty();
        div()
            .id("notas")
            .w(px(264.))
            .h_full()
            .flex_none()
            .p(px(8.))
            .pt(px(10.))
            .bg(tm.card)
            .rounded(px(22.))
            .shadow(tm.sombra_reposo())
            .flex()
            .flex_col()
            .child(
                div()
                    .h(px(36.))
                    .mx(px(4.))
                    .px(px(12.))
                    .gap(px(8.))
                    .flex_none()
                    .bg(tm.sbg)
                    .rounded(px(12.))
                    .flex()
                    .items_center()
                    .text_size(px(13.))
                    .child(icono("buscar", 16., tm.tx2))
                    .child(div().flex_1().min_w_0().child(self.buscador.clone())),
            )
            .when(self.cargando_notas, |d| {
                d.child(
                    div()
                        .mt(px(16.))
                        .text_color(tm.tx2)
                        .child(t.elegir("Cargando…", "Loading…")),
                )
            })
            .when_some(self.error_notas.as_ref(), |d, error| {
                d.child(
                    div()
                        .mt(px(12.))
                        .text_size(px(13.))
                        .text_color(tm.red)
                        .child(error.clone())
                        .child(boton(&tm, "reintentar", t.t("notas.reintentar")).on_click(
                            cx.listener(|this, _, w, cx| this.cargar_notas(false, w, cx)),
                        )),
                )
            })
            .when(
                vacia && !self.cargando_notas && self.error_notas.is_none(),
                |d| {
                    d.child(
                        div()
                            .mt(px(18.))
                            .text_color(tm.tx2)
                            .text_size(px(13.))
                            .child(if filtro.is_empty() {
                                t.elegir(
                                    "Todavía no hay notas. Crea una para empezar.",
                                    "No notes yet. Create one to start.",
                                )
                            } else {
                                t.t("notas.sinResultados").replace("{texto}", &filtro)
                            }),
                    )
                },
            )
            .child(
                div()
                    .id("lista-notas")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .pb(px(10.))
                    .children(filas),
            )
            .into_any_element()
    }
    fn pestanas(&self, cx: &mut Context<Self>) -> AnyElement {
        let tm = self.tema;
        div()
            .id("pestanas")
            .flex()
            .flex_shrink_0()
            .h(px(34.))
            .mb(px(10.))
            .p(px(3.))
            .gap(px(2.))
            .bg(tm.card)
            .rounded(px(14.))
            .overflow_x_scroll()
            .children(
                self.pestanas
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| matches!(p.origen, Origen::Archivo { .. }))
                    .map(|(i, p)| {
                        div()
                            .id(("pestana", p.id))
                            .flex_none()
                            .w(px(200.))
                            .h(px(28.))
                            .rounded(px(10.))
                            .px(px(10.))
                            .flex()
                            .gap(px(8.))
                            .items_center()
                            .cursor_pointer()
                            .when(i == self.activa, |d| d.bg(tm.blue).text_color(tm.on_accent))
                            .hover(move |s| s.bg(tm.hov))
                            .on_click(cx.listener(move |this, _, w, cx| this.activar(i, w, cx)))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(px(12.))
                                    .child(format!(
                                        "{}{}",
                                        if p.sucia { "● " } else { "" },
                                        titulo(p, cx)
                                    )),
                            )
                            .child(
                                boton(&tm, ("cerrar-pestana", p.id), "×").on_click(cx.listener(
                                    move |this, _, w, cx| {
                                        cx.stop_propagation();
                                        this.pedir_cerrar(i, w, cx);
                                    },
                                )),
                            )
                    }),
            )
            .into_any_element()
    }
    fn formato(&self, cx: &mut Context<Self>) -> AnyElement {
        let tm = self.tema;
        let p = &self.pestanas[self.activa];
        let e = p.editor.read(cx);
        let mut fila = div()
            .id("formato")
            .flex()
            .flex_shrink_0()
            .items_center()
            .min_h(px(38.))
            .p(px(4.))
            .bg(tm.card)
            .rounded(px(14.))
            .shadow(tm.sombra_reposo())
            .gap(px(2.))
            .overflow_x_scroll();
        for (id, texto, activo) in [
            ("bold", "B", e.formato.negrita),
            ("italic", "I", e.formato.cursiva),
            ("underline", "U", e.formato.subrayado),
            ("strike", "S̶", e.formato.tachado),
            ("list", "", false),
            ("number", "", false),
            ("left", "", false),
            ("center", "", false),
            ("right", "", false),
        ] {
            if ["list", "left"].contains(&id) {
                fila = fila.child(div().w(px(1.)).h(px(18.)).mx(px(6.)).bg(tm.div));
            }
            let alineacion = e
                .documento
                .nota()
                .and_then(|n| n.bloques.get(e.documento.texto().linea_de(e.cursor())))
                .and_then(|b| match b {
                    crate::editor::nota::Bloque::Texto { alineacion, .. } => Some(*alineacion),
                    _ => None,
                });
            let activo = activo
                || matches!(
                    (id, alineacion),
                    ("left", Some(crate::editor::nota::Alineacion::Izquierda))
                        | ("center", Some(crate::editor::nota::Alineacion::Centro))
                        | ("right", Some(crate::editor::nota::Alineacion::Derecha))
                );
            let mut control = boton_estado(&tm, id, texto, activo)
                .h(px(30.))
                .min_w(px(30.))
                .px(px(9.))
                .py_0()
                .rounded(px(10.))
                .flex()
                .items_center()
                .justify_center()
                .when(!activo, |d| d.opacity(0.75));
            if ["list", "number", "left", "center", "right"].contains(&id) {
                control = control.child(icono(id, 14., if activo { tm.on_accent } else { tm.tx }));
            }
            if id == "bold" {
                control = control.font_weight(FontWeight::BOLD);
            }
            if id == "italic" {
                control = control.italic();
            }
            if id == "underline" {
                control = control.underline();
            }
            fila = fila.child(control.on_click(cx.listener(move |this, _, w, cx| {
                let editor = &this.pestanas[this.activa].editor;
                editor.update(cx, |e, cx| {
                    if ["list", "number", "left", "center", "right"].contains(&id) {
                        e.bloque_formato(id, cx);
                    } else {
                        e.dar_formato(id, None, cx);
                    }
                });
                w.focus(&editor.focus_handle(cx), cx);
            })));
        }
        if e.imagen_seleccionada.is_some() {
            for (id, etiqueta) in [
                ("original", self.textos.t("imagen.original")),
                ("fit", self.textos.t("imagen.ajustar")),
                ("left", "⇤".into()),
                ("center", "↔".into()),
                ("right", "⇥".into()),
                ("delete", "×".into()),
            ] {
                fila = fila.child(
                    boton(&tm, SharedString::from(format!("imagen-{id}")), etiqueta).on_click(
                        cx.listener(move |this, _, _, cx| {
                            this.pestanas[this.activa]
                                .editor
                                .update(cx, |e, cx| e.imagen_accion(id, cx))
                        }),
                    ),
                );
            }
        }
        fila = fila.child(div().w(px(1.)).h(px(18.)).mx(px(6.)).bg(tm.div));
        for (id, etiqueta) in [("color", "A"), ("background", "H")] {
            fila = fila.child(
                boton(&tm, id, "")
                    .size(px(30.))
                    .opacity(0.75)
                    .p_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(FontWeight::BOLD)
                            .when(id == "color", |d| d.border_b_2().border_color(tm.blue))
                            .when(id == "background", |d| {
                                d.px(px(4.))
                                    .rounded(px(4.))
                                    .bg(rgb(0xffff00))
                                    .text_color(black())
                            })
                            .child(etiqueta),
                    )
                    .on_click(cx.listener(move |this, _, w, cx| {
                        let editor = &this.pestanas[this.activa].editor;
                        editor.update(cx, |e, cx| {
                            let actual = if id == "color" {
                                e.formato.color.as_deref()
                            } else {
                                e.formato.fondo.as_deref()
                            };
                            let paleta = ["#d32f2f", "#1565c0", "#2e7d32", "#f9a825"];
                            let siguiente = actual
                                .and_then(|c| paleta.iter().position(|p| *p == c))
                                .map_or(Some(paleta[0]), |i| paleta.get(i + 1).copied());
                            e.dar_formato(id, siguiente.map(str::to_owned), cx);
                        });
                        w.focus(&editor.focus_handle(cx), cx);
                    })),
            );
        }
        fila.child(div().w(px(1.)).h(px(18.)).mx(px(6.)).bg(tm.div))
            .child(
                boton(&tm, "clear", "")
                    .size(px(30.))
                    .p_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(icono("clear", 14., tm.tx))
                    .on_click(cx.listener(|this, _, w, cx| {
                        let editor = &this.pestanas[this.activa].editor;
                        editor.update(cx, |e, cx| e.dar_formato("clear", None, cx));
                        w.focus(&editor.focus_handle(cx), cx);
                    })),
            )
            .into_any_element()
    }
    fn cabecera_nota(&self, cx: &mut Context<Self>) -> AnyElement {
        let tm = self.tema;
        let p = &self.pestanas[self.activa];
        let fecha = p
            .origen
            .ruta()
            .and_then(|r| self.notas.iter().find(|n| n.ruta == r))
            .map(|n| n.fecha)
            .unwrap_or_else(SystemTime::now);
        let estado = match &p.guardado {
            Guardado::Nuevo => self.textos.t("notas.guardado.nuevo"),
            Guardado::Pendiente => self.textos.t("notas.guardado.pendiente"),
            Guardado::Guardando => self.textos.t("notas.guardado.guardando"),
            Guardado::Listo => self.textos.t("notas.guardado.guardado"),
            Guardado::Error(e) => format!(
                "{}: {e}",
                self.textos
                    .elegir("Error · pulsa para reintentar", "Error · click to retry")
            ),
        };
        div()
            .flex_none()
            .pt(px(14.))
            .px(px(26.))
            .flex()
            .items_center()
            .justify_center()
            .gap(px(12.))
            .text_size(px(12.))
            .text_color(tm.tx2)
            .child(crate::fechas::larga(fecha, self.textos.ingles))
            .child(
                div()
                    .id("estado-nota")
                    .text_size(px(11.))
                    .py(px(2.))
                    .cursor_pointer()
                    .when(matches!(p.guardado, Guardado::Error(_)), |d| {
                        d.text_color(tm.red)
                    })
                    .child(estado)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.guardar_indice(this.activa, false, cx);
                    })),
            )
            .into_any_element()
    }
    fn estado_editor(&self, cx: &mut Context<Self>) -> AnyElement {
        let tm = self.tema;
        let p = &self.pestanas[self.activa];
        let e = p.editor.read(cx);
        let texto = e.texto();
        if e.documento.nota().is_some() {
            let lineas = e.documento.texto().inicios.len();
            let palabras = texto
                .split_whitespace()
                .filter(|p| *p != "\u{fffc}")
                .count();
            return div()
                .flex_none()
                .pt(px(8.))
                .px(px(8.))
                .flex()
                .items_center()
                .gap(px(12.))
                .text_size(px(11.5))
                .text_color(tm.tx2)
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .child(icono("caracteres", 12., tm.tx2))
                        .child(format!(
                            "{} {}",
                            texto.chars().filter(|c| *c != '\u{fffc}').count(),
                            self.textos.elegir("caracteres", "characters")
                        )),
                )
                .child("·")
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .child(icono("lineas", 12., tm.tx2))
                        .child(format!(
                            "{lineas} {}",
                            self.textos.elegir(
                                if lineas == 1 { "línea" } else { "líneas" },
                                if lineas == 1 { "line" } else { "lines" }
                            )
                        )),
                )
                .child("·")
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .child(icono("palabras", 12., tm.tx2))
                        .child(format!(
                            "{palabras} {}",
                            self.textos.elegir(
                                if palabras == 1 { "palabra" } else { "palabras" },
                                if palabras == 1 { "word" } else { "words" }
                            )
                        )),
                )
                .into_any_element();
        }
        let linea = e.documento.texto().linea_de(e.cursor());
        let columna = texto[e.documento.texto().inicios[linea]..e.cursor()]
            .chars()
            .count()
            + 1;
        let ruta = p
            .origen
            .ruta()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        let mut barra = div()
            .flex()
            .flex_shrink_0()
            .items_center()
            .h(px(30.))
            .px(px(10.))
            .gap(px(8.))
            .border_t_1()
            .border_color(tm.brd)
            .text_size(px(11.))
            .text_color(tm.tx2)
            .child(div().flex_1().min_w_0().truncate().child(ruta))
            .child(format!(
                "{} · {} {} · {} {} · {} {} · {}:{}",
                if e.extension.is_empty() {
                    "txt"
                } else {
                    &e.extension
                },
                texto.chars().count(),
                self.textos.elegir("car.", "chars"),
                e.documento.texto().inicios.len(),
                self.textos.elegir("líneas", "lines"),
                texto.split_whitespace().count(),
                self.textos.elegir("palabras", "words"),
                linea + 1,
                columna
            ));
        if let Origen::Archivo {
            codificacion, eol, ..
        } = &p.origen
        {
            barra = barra
                .child(
                    boton(&tm, "codificacion", codificacion.clone())
                        .text_size(px(11.))
                        .on_click(cx.listener(|this, _, _, cx| {
                            if this.pestanas[this.activa].sucia {
                                this.avisar(
                                    "Guarda los cambios antes de releer con otra codificación",
                                    true,
                                    cx,
                                );
                                return;
                            }
                            let Origen::Archivo { codificacion, .. } =
                                &this.pestanas[this.activa].origen
                            else {
                                return;
                            };
                            let lista = crate::archivos::CODIFICACIONES;
                            let i = lista.iter().position(|c| *c == codificacion).unwrap_or(0);
                            this.recargar(this.activa, Some(lista[(i + 1) % lista.len()]), cx);
                        })),
                )
                .child(
                    boton(&tm, "eol", eol.etiqueta())
                        .text_size(px(11.))
                        .on_click(cx.listener(|this, _, _, cx| {
                            if let Origen::Archivo { eol, .. } =
                                &mut this.pestanas[this.activa].origen
                            {
                                *eol = if *eol == FinLinea::Lf {
                                    FinLinea::Crlf
                                } else {
                                    FinLinea::Lf
                                };
                            }
                            this.cambio(this.pestanas[this.activa].id, cx);
                        })),
                );
        }
        barra
            .child(
                boton(&tm, "zoom-menos", "−")
                    .on_click(cx.listener(|this, _, w, cx| this.accion("zoom-out", w, cx))),
            )
            .child(
                boton(&tm, "zoom-reset", format!("{}%", self.estado.settings.zoom))
                    .on_click(cx.listener(|this, _, w, cx| this.accion("zoom-reset", w, cx))),
            )
            .child(
                boton(&tm, "zoom-mas", "+")
                    .on_click(cx.listener(|this, _, w, cx| this.accion("zoom-in", w, cx))),
            )
            .into_any_element()
    }
    fn pintar_dialogo(&self, cx: &mut Context<Self>) -> AnyElement {
        let tm = self.tema;
        let t = self.textos;
        let d = self.dialogo.as_ref().expect("diálogo visible");
        let (titulo, mensaje, si) = match d {
            Dialogo::Borrar(_) => (
                t.t("notas.borrar.titulo"),
                t.elegir(
                    "La nota se borrará del disco. No se puede deshacer.",
                    "The note will be deleted from disk. This cannot be undone.",
                ),
                t.t("notas.borrar.si"),
            ),
            Dialogo::Cerrar(_) | Dialogo::Salir => (
                t.elegir("Cambios sin guardar", "Unsaved changes"),
                t.elegir(
                    "Guarda los cambios antes de cerrar.",
                    "Save your changes before closing.",
                ),
                t.elegir("Guardar", "Save"),
            ),
            Dialogo::Recargar(_, _) | Dialogo::Sobrescribir(_) => (
                t.elegir("El archivo cambió en disco", "The file changed on disk"),
                t.elegir(
                    "Revisa la versión del disco antes de continuar.",
                    "Review the disk version before continuing.",
                ),
                if matches!(d, Dialogo::Recargar(..)) {
                    t.elegir("Recargar", "Reload")
                } else {
                    t.elegir("Sobrescribir", "Overwrite")
                },
            ),
        };
        div()
            .absolute()
            .inset_0()
            .bg(tm.scrim)
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .child(
                div()
                    .w(px(430.))
                    .p(px(24.))
                    .rounded(px(20.))
                    .bg(tm.card)
                    .shadow(bookos_ui::tema::sombra_modal())
                    .flex()
                    .flex_col()
                    .gap(px(14.))
                    .child(
                        div()
                            .text_size(px(19.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(titulo),
                    )
                    .child(div().text_size(px(14.)).child(mensaje))
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .justify_end()
                            .gap(px(8.))
                            .child(
                                controles::boton(
                                    &tm,
                                    "cancelar-dialogo",
                                    t.elegir("Cancelar", "Cancel"),
                                    EstiloBoton::Secundario,
                                )
                                .on_click(cx.listener(
                                    |this, _, _, cx| {
                                        this.dialogo = None;
                                        cx.notify();
                                    },
                                )),
                            )
                            .when(
                                matches!(
                                    d,
                                    Dialogo::Cerrar(_) | Dialogo::Salir | Dialogo::Recargar(..)
                                ),
                                |el| {
                                    el.child(
                                        controles::boton(
                                            &tm,
                                            "descartar",
                                            if matches!(d, Dialogo::Recargar(..)) {
                                                t.elegir("Conservar", "Keep")
                                            } else {
                                                t.elegir("Descartar", "Discard")
                                            },
                                            EstiloBoton::Secundario,
                                        )
                                        .on_click(
                                            cx.listener(|this, _, w, cx| {
                                                this.confirmar(false, w, cx)
                                            }),
                                        ),
                                    )
                                },
                            )
                            .child(
                                controles::boton(
                                    &tm,
                                    "confirmar-dialogo",
                                    si,
                                    if matches!(d, Dialogo::Borrar(_) | Dialogo::Sobrescribir(_)) {
                                        EstiloBoton::Peligro
                                    } else {
                                        EstiloBoton::Primario
                                    },
                                )
                                .on_click(
                                    cx.listener(|this, _, w, cx| this.confirmar(true, w, cx)),
                                ),
                            ),
                    ),
            )
            .with_animation(
                "dialogo",
                Animation::new(bookos_ui::DURACION_DIALOGO),
                |d, t| d.opacity(t),
            )
            .into_any_element()
    }
}
impl Render for Vista {
    fn render(&mut self, w: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.pestanas.is_empty() {
            self.nueva_nota(w, cx);
        }
        let tema = Tema::para_ventana(&self.estado.theme, w);
        if tema.oscuro != self.tema.oscuro {
            self.tema = tema;
            self.aplicar_ajustes(cx);
        }
        let tm = self.tema;
        let nota = self.pestanas[self.activa]
            .editor
            .read(cx)
            .documento
            .nota()
            .is_some();
        let mut detalle = div()
            .flex_1()
            .min_w_0()
            .h_full()
            .flex()
            .flex_col()
            .child(self.barra(f32::from(w.viewport_size().width) > 900., cx));
        if self
            .pestanas
            .iter()
            .any(|p| matches!(p.origen, Origen::Archivo { .. }))
        {
            detalle = detalle.child(self.pestanas(cx));
        }
        if self.ajustes {
            detalle = detalle.child(div().flex_1().min_h_0().child(self.pagina_ajustes(cx)));
        } else {
            if self.busqueda_visible {
                detalle = detalle.child(self.barra_busqueda(cx));
            }
            if nota {
                detalle = detalle.child(
                    div()
                        .flex_none()
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .pb(px(10.))
                        .child(self.formato(cx))
                        .child(div().flex_1())
                        .child(self.boton_icono("borrar-nota", "borrar-nota", cx)),
                );
            }
            let mut hoja = div()
                .flex_1()
                .min_h_0()
                .flex()
                .flex_col()
                .bg(tm.card)
                .rounded(px(if nota { 22. } else { 18. }))
                .overflow_hidden()
                .shadow(tm.sombra_reposo());
            if nota {
                hoja = hoja.child(self.cabecera_nota(cx));
            }
            detalle = detalle.child(
                hoja.child(
                    div()
                        .flex_1()
                        .min_h_0()
                        .child(self.pestanas[self.activa].editor.clone()),
                ),
            );
            if let Some(error) = &self.pestanas[self.activa].editor.read(cx).error_imagen {
                detalle = detalle.child(
                    div()
                        .p(px(8.))
                        .text_size(px(12.))
                        .text_color(tm.red)
                        .child(error.clone()),
                );
            }
            if self.estado.settings.show_status {
                detalle = detalle.child(self.estado_editor(cx));
            }
        }
        let contenido = div()
            .size_full()
            .flex()
            .gap(px(12.))
            .px(px(12.))
            .pb(px(12.))
            .when(
                !self.estado.settings.lista_oculta && f32::from(w.viewport_size().width) > 900.,
                |d| d.child(self.lista_notas(cx)),
            )
            .child(detalle);
        let mut capas = Vec::new();
        if let Some(t) = &self.toast {
            capas.push(toast::pintar(t, &tm).into_any_element());
        }
        if self.paleta {
            capas.push(self.panel_atajos(cx));
        }
        if self.dialogo.is_some() {
            capas.push(self.pintar_dialogo(cx));
        }
        div()
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .font_family(bookos_ui::FUENTE_UI)
            .bg(tm.win)
            .text_color(tm.tx)
            .when(!w.is_fullscreen(), |d| {
                d.rounded(px(20.))
                    .border_1()
                    .border_color(tm.brd)
                    .overflow_hidden()
            })
            .child(self.barra_ventana(cx))
            .child(div().flex_1().min_h_0().child(contenido))
            .children(capas)
            .children((!w.is_maximized() && !w.is_fullscreen()).then(asas_ventana))
            .id("raiz")
            .track_focus(&self.foco)
            .capture_key_down(cx.listener(Self::teclado))
            .on_key_up(cx.listener(|this, e: &KeyUpEvent, _, _| {
                if !e.keystroke.modifiers.control && !e.keystroke.modifiers.platform {
                    this.mru_ciclo.clear();
                }
            }))
            .on_drop::<ExternalPaths>(cx.listener(|this, rutas: &ExternalPaths, w, cx| {
                let id = this.pestanas[this.activa].id;
                for ruta in rutas.paths() {
                    if crate::imagenes::es_imagen(ruta)
                        && this.pestanas[this.activa]
                            .editor
                            .read(cx)
                            .documento
                            .nota()
                            .is_some()
                    {
                        match std::fs::read(ruta) {
                            Ok(bytes) => this.insertar_imagen(id, &bytes, cx),
                            Err(e) => this.avisar(e.to_string(), true, cx),
                        }
                    } else {
                        this.abrir_ruta(ruta.clone(), None, w, cx);
                    }
                }
            }))
    }
}

fn asas_ventana() -> AnyElement {
    let mut asas = div().absolute().inset_0();
    for (borde, cursor) in [
        (ResizeEdge::Top, CursorStyle::ResizeUpDown),
        (ResizeEdge::Bottom, CursorStyle::ResizeUpDown),
        (ResizeEdge::Left, CursorStyle::ResizeLeftRight),
        (ResizeEdge::Right, CursorStyle::ResizeLeftRight),
        (ResizeEdge::TopLeft, CursorStyle::ResizeUpLeftDownRight),
        (ResizeEdge::BottomRight, CursorStyle::ResizeUpLeftDownRight),
        (ResizeEdge::TopRight, CursorStyle::ResizeUpRightDownLeft),
        (ResizeEdge::BottomLeft, CursorStyle::ResizeUpRightDownLeft),
    ] {
        let asa =
            div()
                .absolute()
                .cursor(cursor)
                .on_mouse_down(MouseButton::Left, move |_, w, cx| {
                    cx.stop_propagation();
                    w.start_window_resize(borde);
                });
        asas = asas.child(match borde {
            ResizeEdge::Top => asa.top_0().left(px(8.)).right(px(8.)).h(px(8.)),
            ResizeEdge::Bottom => asa.bottom_0().left(px(8.)).right(px(8.)).h(px(8.)),
            ResizeEdge::Left => asa.left_0().top(px(8.)).bottom(px(8.)).w(px(8.)),
            ResizeEdge::Right => asa.right_0().top(px(8.)).bottom(px(8.)).w(px(8.)),
            ResizeEdge::TopLeft => asa.top_0().left_0().size(px(8.)),
            ResizeEdge::TopRight => asa.top_0().right_0().size(px(8.)),
            ResizeEdge::BottomLeft => asa.bottom_0().left_0().size(px(8.)),
            ResizeEdge::BottomRight => asa.bottom_0().right_0().size(px(8.)),
        });
    }
    asas.into_any_element()
}
