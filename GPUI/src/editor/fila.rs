use super::*;

pub struct Fila {
    pub editor: Entity<Editor>,
    pub indice: usize,
}
pub struct Preparada {
    linea: Arc<WrappedLine>,
    origen: Point<Pixels>,
    alto: Pixels,
    inicio: usize,
}
impl IntoElement for Fila {
    type Element = Self;
    fn into_element(self) -> Self {
        self
    }
}
impl Element for Fila {
    type RequestLayoutState = Preparada;
    type PrepaintState = ();
    fn id(&self) -> Option<ElementId> {
        None
    }
    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }
    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Preparada) {
        let e = self.editor.read(cx);
        let t = e.documento.texto();
        let rango = t.rango_linea(self.indice);
        let texto: SharedString = t.cadena[rango.clone()].to_owned().into();
        let mut tamano = e.tamano;
        let mut corridas = Vec::new();
        let mut sangria = 0.;
        let mut base = gpui::font(e.fuente);
        let mut alineacion = Alineacion::Izquierda;
        if let Some(nota) = e.documento.nota() {
            if self.indice == 0 {
                tamano *= 22. / 15.;
                base.weight = FontWeight::BOLD;
            }
            if let Some(Bloque::Texto {
                tramos,
                lista,
                alineacion: a,
                ..
            }) = nota.bloques.get(self.indice)
            {
                alineacion = *a;
                if *lista != Lista::Ninguna {
                    sangria = 30.;
                }
                let mut pos = 0;
                for tramo in tramos {
                    let crear = |len, f: &Formato| {
                        let mut font = base.clone();
                        if f.negrita {
                            font.weight = FontWeight::BOLD;
                        }
                        if f.cursiva {
                            font.style = FontStyle::Italic;
                        }
                        TextRun {
                            len,
                            font,
                            color: f.color.as_deref().and_then(color).unwrap_or(e.tema.tx),
                            background_color: f.fondo.as_deref().and_then(color),
                            underline: f.subrayado.then_some(UnderlineStyle {
                                color: None,
                                thickness: px(1.),
                                wavy: false,
                            }),
                            strikethrough: (f.tachado || *lista == Lista::Tarea(true)).then_some(
                                StrikethroughStyle {
                                    color: None,
                                    thickness: px(1.),
                                },
                            ),
                        }
                    };
                    if tramo.rango.start > pos {
                        corridas.push(crear(tramo.rango.start - pos, &Formato::default()));
                    }
                    corridas.push(crear(tramo.rango.len(), &tramo.formato));
                    pos = tramo.rango.end;
                }
                if pos < texto.len() {
                    corridas.push(TextRun {
                        len: texto.len() - pos,
                        font: base.clone(),
                        color: e.tema.tx,
                        background_color: None,
                        underline: None,
                        strikethrough: (*lista == Lista::Tarea(true)).then_some(
                            StrikethroughStyle {
                                color: None,
                                thickness: px(1.),
                            },
                        ),
                    });
                }
            }
        }
        if corridas.is_empty() {
            corridas.push(TextRun {
                len: texto.len(),
                font: base,
                color: e.tema.tx,
                background_color: None,
                underline: None,
                strikethrough: None,
            });
        }
        let ancho = px((e.ancho - 52. - sangria).max(40.));
        let vacia = texto.is_empty();
        let texto = if vacia { " ".into() } else { texto };
        if vacia {
            corridas[0].len = 1;
        }
        let linea = Arc::new(
            window
                .text_system()
                .shape_text(
                    texto,
                    px(tamano),
                    &corridas,
                    e.ajuste.then_some(ancho),
                    None,
                )
                .expect("texto UTF-8 con corridas completas")
                .remove(0),
        );
        let titulo = e.documento.nota().is_some() && self.indice == 0;
        let alto = px(tamano * if titulo { 1.3 } else { 1.6 });
        let altura = linea.size(alto).height;
        let x = 26.
            + sangria
            + match alineacion {
                Alineacion::Izquierda => 0.,
                Alineacion::Centro => f32::from((ancho - linea.width()).max(px(0.))) / 2.,
                Alineacion::Derecha => f32::from((ancho - linea.width()).max(px(0.))),
            };
        let mut estilo = Style::default();
        estilo.size.width = relative(1.).into();
        estilo.size.height = (altura + if titulo { px(4.) } else { px(0.) }).into();
        let origen = point(px(x - e.desplazamiento_x), px(0.));
        (
            window.request_layout(estilo, [], cx),
            Preparada {
                linea,
                origen,
                alto,
                inicio: rango.start,
            },
        )
    }
    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        caja: Bounds<Pixels>,
        preparada: &mut Preparada,
        _: &mut Window,
        cx: &mut App,
    ) {
        let origen = caja.origin + preparada.origen;
        self.editor.update(cx, |e, _| {
            e.visibles.insert(
                self.indice,
                (
                    Bounds::new(origen, preparada.linea.size(preparada.alto)),
                    preparada.linea.clone(),
                ),
            );
        });
    }
    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        caja: Bounds<Pixels>,
        p: &mut Preparada,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        let e = self.editor.read(cx);
        let origen = caja.origin + p.origen;
        let fin = p.inicio + e.documento.texto().rango_linea(self.indice).len();
        let pintar = |r: Range<usize>, color: Hsla, window: &mut Window| {
            let a = r.start.max(p.inicio);
            let z = r.end.min(fin);
            if a > z || r.end < p.inicio || r.start > fin {
                return;
            }
            let inicio = p
                .linea
                .position_for_index(a - p.inicio, p.alto)
                .unwrap_or_default();
            let final_ = p
                .linea
                .position_for_index(z - p.inicio, p.alto)
                .unwrap_or_default();
            let mut y = inicio.y;
            while y <= final_.y {
                let izquierda = if y == inicio.y { inicio.x } else { px(0.) };
                let derecha = if y == final_.y {
                    final_.x
                } else {
                    caja.size.width - p.origen.x
                };
                if derecha > izquierda {
                    window.paint_quad(fill(
                        Bounds::new(
                            origen + point(izquierda, y),
                            size(derecha - izquierda, p.alto),
                        ),
                        color,
                    ));
                }
                y += p.alto;
            }
        };
        let desde = e
            .busqueda
            .coincidencias
            .partition_point(|r| r.end < p.inicio);
        for r in e.busqueda.coincidencias[desde..]
            .iter()
            .take_while(|r| r.start <= fin)
        {
            pintar(r.clone(), e.tema.yellow.opacity(0.35), window);
        }
        if !e.seleccion.is_empty() {
            pintar(e.seleccion.clone(), e.tema.blue.opacity(0.25), window);
        }

        if e.seleccion.is_empty()
            && e.foco.is_focused(window)
            && e.cursor() >= p.inicio
            && e.cursor() <= fin
            && let Some(pos) = p.linea.position_for_index(e.cursor() - p.inicio, p.alto)
        {
            window.paint_quad(fill(
                Bounds::new(origen + pos, size(px(1.5), p.alto)),
                e.tema.blue,
            ));
        }
        let foco = e.foco.clone();
        if let Err(e) = p
            .linea
            .paint_background(origen, p.alto, TextAlign::Left, None, window, cx)
        {
            eprintln!("bookos-notepad: no se pudo pintar el fondo del texto: {e}");
        }
        if let Err(e) = p
            .linea
            .paint(origen, p.alto, TextAlign::Left, None, window, cx)
        {
            eprintln!("bookos-notepad: no se pudo pintar el texto: {e}");
        }
        window.handle_input(
            &foco,
            ElementInputHandler::new(caja, self.editor.clone()),
            cx,
        );
    }
}
pub fn color(s: &str) -> Option<Hsla> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix('#') {
        let hex = if hex.len() == 3 {
            hex.chars().flat_map(|c| [c, c]).collect()
        } else {
            hex.to_owned()
        };
        u32::from_str_radix(&hex, 16).ok().map(|c| rgb(c).into())
    } else if s.starts_with("rgb(") && s.ends_with(')') {
        let valores = s[4..s.len() - 1]
            .split(',')
            .map(|s| s.trim().parse::<u8>())
            .collect::<Result<Vec<_>, _>>()
            .ok()?;
        if valores.len() == 3 {
            Some(
                rgb((valores[0] as u32) << 16 | (valores[1] as u32) << 8 | valores[2] as u32)
                    .into(),
            )
        } else {
            None
        }
    } else {
        match s {
            "red" => Some(rgb(0xff0000).into()),
            "black" => Some(black()),
            "white" => Some(white()),
            "blue" => Some(rgb(0x0000ff).into()),
            _ => None,
        }
    }
}
pub fn pintar(editor: Entity<Editor>, i: usize, window: &mut Window, cx: &mut App) -> AnyElement {
    let bloque = editor
        .read(cx)
        .documento
        .nota()
        .and_then(|n| n.bloques.get(i))
        .cloned();
    if let Some(Bloque::Imagen {
        fuente,
        ancho,
        alineacion,
    }) = bloque
    {
        let e = editor.read(cx);
        let tm = e.tema;
        let disponible = (e.ancho - 52.).max(40.);
        let seleccionado = e.imagen_seleccionada == Some(i);
        let dimensiones =
            crate::imagenes::dimensiones(&fuente, &cx.svg_renderer()).unwrap_or((400, 200));
        let ancho = ancho.unwrap_or(dimensiones.0 as f32).clamp(40., disponible);
        let alto = ancho * dimensiones.1 as f32 / dimensiones.0 as f32;
        let pixeles = (ancho * window.scale_factor()).round() as u32;
        let requiere = e.imagenes.get(&i).is_none_or(|(a, _)| *a != pixeles);
        if requiere {
            let resultado = crate::imagenes::rasterizar(&fuente, pixeles, &cx.svg_renderer());
            editor.update(cx, |e, _| match resultado {
                Ok(imagen) => {
                    e.imagenes.insert(i, (pixeles, imagen));
                }
                Err(error) => {
                    eprintln!("bookos-notepad: no se pudo mostrar una imagen: {error}");
                    e.error_imagen = Some(error)
                }
            });
        }
        let imagen = editor
            .read(cx)
            .imagenes
            .get(&i)
            .map(|(_, imagen)| imagen.clone());
        let entidad = editor.clone();
        let mut contenido = div()
            .id(("imagen", i))
            .relative()
            .w(px(ancho))
            .h(px(alto))
            .rounded(px(14.))
            .cursor_pointer()
            .on_mouse_down(MouseButton::Left, move |_, w, cx| {
                cx.stop_propagation();
                entidad.update(cx, |e, cx| {
                    w.focus(&e.foco, cx);
                    e.imagen_seleccionada = Some(i);
                    cx.notify();
                });
            })
            .children(imagen.map(|imagen| img(imagen).size_full().rounded(px(14.))))
            .when(seleccionado, |d| {
                d.child(
                    div()
                        .absolute()
                        .inset_0()
                        .rounded(px(14.))
                        .border_2()
                        .border_color(tm.blue),
                )
            });
        if seleccionado {
            for (id, esquina, izquierda, arriba) in [
                ("si", EsquinaImagen::SuperiorIzquierda, true, true),
                ("sd", EsquinaImagen::SuperiorDerecha, false, true),
                ("ii", EsquinaImagen::InferiorIzquierda, true, false),
                ("id", EsquinaImagen::InferiorDerecha, false, false),
            ] {
                let entidad = editor.clone();
                contenido = contenido.child(
                    div()
                        .id((id, i))
                        .absolute()
                        .when(izquierda, |d| d.left(px(-7.)))
                        .when(!izquierda, |d| d.right(px(-7.)))
                        .when(arriba, |d| d.top(px(-7.)))
                        .when(!arriba, |d| d.bottom(px(-7.)))
                        .size(px(14.))
                        .rounded_full()
                        .bg(tm.blue)
                        .border_2()
                        .border_color(tm.card)
                        .cursor(if izquierda == arriba {
                            CursorStyle::ResizeUpLeftDownRight
                        } else {
                            CursorStyle::ResizeUpRightDownLeft
                        })
                        .on_mouse_down(MouseButton::Left, move |ev, w, cx| {
                            cx.stop_propagation();
                            entidad.update(cx, |e, cx| {
                                if let Some(nota) = e.documento.nota() {
                                    e.arrastre_imagen = Some(ArrastreImagen {
                                        indice: i,
                                        ancho,
                                        proporcion: ancho / alto,
                                        inicio: ev.position,
                                        esquina,
                                        antes: nota.clone(),
                                    });
                                    w.focus(&e.foco, cx);
                                    cx.notify();
                                }
                            });
                        }),
                );
            }
        }
        div()
            .px(px(26.))
            .py(px(8.))
            .flex()
            .when(alineacion == Alineacion::Centro, |d| d.justify_center())
            .when(alineacion == Alineacion::Derecha, |d| d.justify_end())
            .child(contenido)
            .into_any_element()
    } else {
        let lista = match bloque {
            Some(Bloque::Texto { lista, .. }) => lista,
            _ => Lista::Ninguna,
        };
        let mut fila = div().relative().child(Fila {
            editor: editor.clone(),
            indice: i,
        });
        if lista != Lista::Ninguna {
            let tm = editor.read(cx).tema;
            let entidad = editor.clone();
            fila = fila.child(
                div()
                    .id(("marcador", i))
                    .absolute()
                    .left(px(26.))
                    .top(px(2.))
                    .w(px(24.))
                    .text_color(tm.tx2)
                    .child(match lista {
                        Lista::Vineta => "•".into(),
                        Lista::Numero => format!("{}.", i),
                        Lista::Tarea(true) => "●".into(),
                        Lista::Tarea(false) => "○".into(),
                        Lista::Ninguna => String::new(),
                    })
                    .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                        cx.stop_propagation();
                        entidad.update(cx, |e, cx| {
                            e.modificar_nota(
                                |n| {
                                    if let Bloque::Texto {
                                        lista: Lista::Tarea(hecha),
                                        ..
                                    } = &mut n.bloques[i]
                                    {
                                        *hecha = !*hecha;
                                    }
                                },
                                cx,
                            )
                        });
                    }),
            );
        }
        fila.into_any_element()
    }
}
