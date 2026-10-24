use super::*;
use crate::editor::buscar::Busqueda;
impl Vista {
    pub(super) fn actualizar_busqueda(&mut self, cx: &mut Context<Self>) {
        if self.pestanas.is_empty() {
            return;
        }
        let termino = self.buscar.read(cx).texto();
        let e = &self.pestanas[self.activa].editor;
        let busqueda = if self.busqueda_visible {
            Busqueda::nueva(e.read(cx).texto(), termino, self.mayusculas, self.regex)
        } else {
            Busqueda::default()
        };
        self.coincidencia = None;
        e.update(cx, |e, cx| {
            e.busqueda = busqueda;
            cx.notify();
        });
        cx.notify();
    }
    pub(super) fn siguiente(&mut self, anterior: bool, w: &mut Window, cx: &mut Context<Self>) {
        let e = &self.pestanas[self.activa].editor;
        let editor = e.read(cx);
        let cantidad = editor.busqueda.coincidencias.len();
        if cantidad == 0 {
            return;
        }
        let i = match self.coincidencia {
            Some(i) => {
                if anterior {
                    (i + cantidad - 1) % cantidad
                } else {
                    (i + 1) % cantidad
                }
            }
            None => {
                if anterior {
                    editor
                        .busqueda
                        .coincidencias
                        .iter()
                        .rposition(|r| r.start < editor.cursor())
                        .unwrap_or(cantidad - 1)
                } else {
                    editor
                        .busqueda
                        .coincidencias
                        .iter()
                        .position(|r| r.start >= editor.cursor())
                        .unwrap_or(0)
                }
            }
        };
        let rango = editor.busqueda.coincidencias[i].clone();
        self.coincidencia = Some(i);
        e.update(cx, |e, cx| {
            e.mover(rango.start, false, cx);
            e.mover(rango.end, true, cx);
            e.lista
                .scroll_to_reveal_item(e.documento.texto().linea_de(rango.start));
        });
        w.focus(&e.focus_handle(cx), cx);
        cx.notify();
    }
    pub(super) fn sustituir(&mut self, todos: bool, w: &mut Window, cx: &mut Context<Self>) {
        let e = &self.pestanas[self.activa].editor;
        let editor = e.read(cx);
        let reemplazo = self.reemplazo.read(cx).texto();
        let cambios = editor
            .busqueda
            .coincidencias
            .iter()
            .enumerate()
            .filter(|(i, _)| todos || self.coincidencia == Some(*i))
            .filter_map(|(_, r)| {
                editor
                    .busqueda
                    .sustituir(editor.texto(), r.clone(), reemplazo, self.regex)
                    .map(|s| (r.clone(), s))
            })
            .collect::<Vec<_>>();
        if cambios.is_empty() && !todos {
            self.siguiente(false, w, cx);
            return;
        }
        if todos {
            e.update(cx, |e, cx| e.reemplazar_varios(cambios, cx));
        } else if let Some((r, s)) = cambios.into_iter().next() {
            e.update(cx, |e, cx| e.reemplazar(r, &s, false, cx));
        }
        self.actualizar_busqueda(cx);
        cx.notify();
    }
    pub(super) fn barra_busqueda(&self, cx: &mut Context<Self>) -> AnyElement {
        let tm = self.tema;
        let b = &self.pestanas[self.activa].editor.read(cx).busqueda;
        let contador = if b.invalida {
            self.textos
                .elegir("Expresión inválida", "Invalid expression")
        } else {
            format!(
                "{}/{}",
                self.coincidencia.map_or(0, |i| i + 1),
                b.coincidencias.len()
            )
        };
        let campo = |entrada: Entity<Entrada>| {
            div()
                .min_w(px(100.))
                .flex_1()
                .h(px(34.))
                .px(px(10.))
                .bg(tm.card)
                .rounded(px(10.))
                .flex()
                .items_center()
                .child(entrada)
        };
        div()
            .flex()
            .flex_col()
            .gap(px(6.))
            .p(px(8.))
            .border_b_1()
            .border_color(tm.brd)
            .child(
                div()
                    .flex()
                    .gap(px(5.))
                    .items_center()
                    .child(campo(self.buscar.clone()))
                    .child(
                        super::interfaz::boton_estado(&tm, "mayusculas", "Aa", self.mayusculas)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.mayusculas = !this.mayusculas;
                                this.actualizar_busqueda(cx);
                            })),
                    )
                    .child(
                        super::interfaz::boton_estado(&tm, "regex", ".*", self.regex).on_click(
                            cx.listener(|this, _, _, cx| {
                                this.regex = !this.regex;
                                this.actualizar_busqueda(cx);
                            }),
                        ),
                    )
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(if b.invalida { tm.red } else { tm.tx2 })
                            .child(contador),
                    )
                    .child(
                        super::interfaz::boton(&tm, "anterior", "↑")
                            .on_click(cx.listener(|this, _, w, cx| this.siguiente(true, w, cx))),
                    )
                    .child(
                        super::interfaz::boton(&tm, "siguiente", "↓")
                            .on_click(cx.listener(|this, _, w, cx| this.siguiente(false, w, cx))),
                    )
                    .child(
                        super::interfaz::boton(&tm, "cerrar-busqueda", "×").on_click(cx.listener(
                            |this, _, _, cx| {
                                this.busqueda_visible = false;
                                this.actualizar_busqueda(cx);
                            },
                        )),
                    ),
            )
            .when(self.reemplazo_visible, |d| {
                d.child(
                    div()
                        .flex()
                        .gap(px(6.))
                        .items_center()
                        .child(campo(self.reemplazo.clone()))
                        .child(
                            super::interfaz::boton(
                                &tm,
                                "reemplazar",
                                self.textos.elegir("Reemplazar", "Replace"),
                            )
                            .on_click(cx.listener(|this, _, w, cx| this.sustituir(false, w, cx))),
                        )
                        .child(
                            super::interfaz::boton(
                                &tm,
                                "reemplazar-todos",
                                self.textos.elegir("Todos", "All"),
                            )
                            .on_click(cx.listener(|this, _, w, cx| this.sustituir(true, w, cx))),
                        ),
                )
            })
            .into_any_element()
    }
}
