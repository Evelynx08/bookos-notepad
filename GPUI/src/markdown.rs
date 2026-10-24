//! Markdown para la vista rápida.
//!
//! La versión Tauri lo pasa por `marked` (con `gfm` y `breaks`); aquí no hay
//! parser en las dependencias, así que esto cubre lo que usan los documentos
//! de BookOS: encabezados, párrafos, citas, listas, bloques de código,
//! separadores, tablas y, dentro del texto, negrita, cursiva, tachado, código
//! y enlaces. El HTML crudo se queda como texto.

use std::ops::Range;

use gpui::{
    AnyElement, FontStyle, FontWeight, Hsla, SharedString, StrikethroughStyle, StyledText, TextRun,
    div, prelude::*, px,
};

use bookos_ui::Tema;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Marca {
    Negrita,
    Cursiva,
    Tachado,
    Codigo,
    Enlace,
}

/// Texto ya sin marcas de Markdown y los tramos que llevan estilo.
#[derive(Debug, PartialEq)]
pub struct Linea {
    pub texto: String,
    pub marcas: Vec<(Range<usize>, Marca)>,
}

#[derive(Debug, PartialEq)]
pub enum Bloque {
    Titulo(u8, Linea),
    Parrafo(Linea),
    Cita(Vec<Bloque>),
    /// Cada elemento: nivel de sangría, marcador («•» o «3.») y texto.
    Lista(Vec<(usize, String, Linea)>),
    Codigo(String),
    Regla,
    Tabla {
        cabecera: Vec<Linea>,
        filas: Vec<Vec<Linea>>,
    },
}

pub fn parsear(texto: &str) -> Vec<Bloque> {
    let lineas: Vec<&str> = texto.lines().collect();
    let mut bloques = Vec::new();
    let mut i = 0;
    while i < lineas.len() {
        let l = lineas[i];
        let t = l.trim();
        if t.is_empty() {
            i += 1;
        } else if t.starts_with("```") || t.starts_with("~~~") {
            let cierre = &t[..3];
            let mut codigo = Vec::new();
            i += 1;
            while i < lineas.len() && !lineas[i].trim_start().starts_with(cierre) {
                codigo.push(lineas[i]);
                i += 1;
            }
            i += 1;
            bloques.push(Bloque::Codigo(codigo.join("\n")));
        } else if let Some((nivel, resto)) = titulo(t) {
            bloques.push(Bloque::Titulo(nivel, en_linea(resto)));
            i += 1;
        } else if es_regla(t) {
            bloques.push(Bloque::Regla);
            i += 1;
        } else if t.starts_with('>') {
            let mut dentro = Vec::new();
            while i < lineas.len() && lineas[i].trim_start().starts_with('>') {
                let s = lineas[i].trim_start()[1..].strip_prefix(' ').unwrap_or(&lineas[i].trim_start()[1..]);
                dentro.push(s);
                i += 1;
            }
            bloques.push(Bloque::Cita(parsear(&dentro.join("\n"))));
        } else if t.starts_with('|')
            && lineas.get(i + 1).is_some_and(|s| es_separador_tabla(s.trim()))
        {
            let cabecera = celdas(t);
            i += 2;
            let mut filas = Vec::new();
            while i < lineas.len() && lineas[i].trim().starts_with('|') {
                filas.push(celdas(lineas[i].trim()));
                i += 1;
            }
            bloques.push(Bloque::Tabla { cabecera, filas });
        } else if elemento_lista(l).is_some() {
            let mut items: Vec<(usize, String, Linea)> = Vec::new();
            let mut texto_item = String::new();
            let mut actual: Option<(usize, String)> = None;
            // Pasar de números a viñetas (o al revés) en el primer nivel abre
            // otra lista, como en CommonMark.
            let numerada = |m: &str| m != "•";
            let tipo = elemento_lista(l).map(|(_, m, _)| numerada(&m));
            let misma_lista = |s: &str| {
                elemento_lista(s).is_some_and(|(n, m, _)| n > 0 || Some(numerada(&m)) == tipo)
            };
            while i < lineas.len() {
                let s = lineas[i];
                if s.trim().is_empty() {
                    // Una línea en blanco solo sigue la lista si lo siguiente
                    // es otro elemento de ella.
                    if lineas.get(i + 1).is_some_and(|n| misma_lista(n)) {
                        i += 1;
                        continue;
                    }
                    break;
                }
                if elemento_lista(s).is_some() && !misma_lista(s) {
                    break;
                }
                if let Some((nivel, marcador, resto)) = elemento_lista(s) {
                    if let Some((n, m)) = actual.take() {
                        items.push((n, m, en_linea(&texto_item)));
                    }
                    actual = Some((nivel, marcador));
                    texto_item = resto.to_string();
                } else if s.starts_with(' ') || s.starts_with('\t') {
                    texto_item.push('\n');
                    texto_item.push_str(s.trim());
                } else {
                    break;
                }
                i += 1;
            }
            if let Some((n, m)) = actual {
                items.push((n, m, en_linea(&texto_item)));
            }
            bloques.push(Bloque::Lista(items));
        } else {
            // Con `breaks` de marked, cada salto de línea del párrafo cuenta.
            let mut parrafo = Vec::new();
            while i < lineas.len() {
                let s = lineas[i].trim();
                if s.is_empty()
                    || titulo(s).is_some()
                    || es_regla(s)
                    || s.starts_with('>')
                    || s.starts_with("```")
                    || (!parrafo.is_empty() && elemento_lista(lineas[i]).is_some())
                {
                    break;
                }
                parrafo.push(s);
                i += 1;
            }
            bloques.push(Bloque::Parrafo(en_linea(&parrafo.join("\n"))));
        }
    }
    bloques
}

fn titulo(t: &str) -> Option<(u8, &str)> {
    let n = t.bytes().take_while(|&b| b == b'#').count();
    if (1..=6).contains(&n) && t[n..].starts_with(' ') {
        Some((n as u8, t[n..].trim().trim_end_matches('#').trim_end()))
    } else {
        None
    }
}

fn es_regla(t: &str) -> bool {
    let sin: String = t.chars().filter(|c| !c.is_whitespace()).collect();
    sin.len() >= 3
        && ['-', '*', '_']
            .iter()
            .any(|&c| sin.chars().all(|x| x == c))
}

fn es_separador_tabla(t: &str) -> bool {
    t.starts_with('|')
        && t.contains('-')
        && t.chars().all(|c| matches!(c, '|' | '-' | ':' | ' '))
}

fn celdas(fila: &str) -> Vec<Linea> {
    let interior = fila.trim().trim_start_matches('|').trim_end_matches('|');
    interior.split('|').map(|c| en_linea(c.trim())).collect()
}

/// `(nivel, marcador, resto)` si la línea abre un elemento de lista.
fn elemento_lista(l: &str) -> Option<(usize, String, &str)> {
    let sangria = l.len() - l.trim_start().len();
    let t = l.trim_start();
    let nivel = sangria / 2;
    for viñeta in ["- ", "* ", "+ "] {
        if let Some(resto) = t.strip_prefix(viñeta) {
            return Some((nivel, "•".into(), resto));
        }
    }
    let cifras = t.bytes().take_while(u8::is_ascii_digit).count();
    if cifras > 0 && cifras < 10 {
        let tras = &t[cifras..];
        if let Some(resto) = tras.strip_prefix(". ").or_else(|| tras.strip_prefix(") ")) {
            return Some((nivel, format!("{}.", &t[..cifras]), resto));
        }
    }
    None
}

/// Quita las marcas de énfasis, código y enlaces y apunta dónde iban.
pub fn en_linea(fuente: &str) -> Linea {
    let mut texto = String::with_capacity(fuente.len());
    let mut marcas = Vec::new();
    let mut abiertas: Vec<(Marca, usize)> = Vec::new();
    let mut resto = fuente;

    let mut alternar = |marca: Marca, texto: &String, marcas: &mut Vec<(Range<usize>, Marca)>| {
        if let Some(pos) = abiertas.iter().rposition(|(m, _)| *m == marca) {
            let (_, inicio) = abiertas.remove(pos);
            if inicio < texto.len() {
                marcas.push((inicio..texto.len(), marca));
            }
        } else {
            abiertas.push((marca, texto.len()));
        }
    };

    while let Some(c) = resto.chars().next() {
        if c == '\\' && resto.len() > 1 {
            let siguiente = resto[1..].chars().next().expect("hay algo tras la barra");
            if siguiente.is_ascii_punctuation() {
                texto.push(siguiente);
                resto = &resto[1 + siguiente.len_utf8()..];
                continue;
            }
        }
        if c == '`' {
            let ticks = resto.bytes().take_while(|&b| b == b'`').count();
            let valla = &resto[..ticks];
            if let Some(fin) = resto[ticks..].find(valla) {
                let codigo = resto[ticks..ticks + fin].trim();
                let inicio = texto.len();
                texto.push_str(codigo);
                marcas.push((inicio..texto.len(), Marca::Codigo));
                resto = &resto[ticks + fin + ticks..];
                continue;
            }
        }
        if let Some(r) = resto.strip_prefix("**").or_else(|| resto.strip_prefix("__")) {
            alternar(Marca::Negrita, &texto, &mut marcas);
            resto = r;
            continue;
        }
        if let Some(r) = resto.strip_prefix("~~") {
            alternar(Marca::Tachado, &texto, &mut marcas);
            resto = r;
            continue;
        }
        // Solo `*` para la cursiva: un `_` suelto es casi siempre parte de un
        // nombre (`snake_case`) y no un énfasis.
        if c == '*' {
            alternar(Marca::Cursiva, &texto, &mut marcas);
            resto = &resto[1..];
            continue;
        }
        let es_imagen = resto.starts_with("![");
        if (c == '[' || es_imagen)
            && let Some((etiqueta, tras)) = enlace(&resto[usize::from(es_imagen)..])
        {
            let inicio = texto.len();
            let interior = en_linea(etiqueta);
            for (r, m) in interior.marcas {
                marcas.push((r.start + inicio..r.end + inicio, m));
            }
            texto.push_str(&interior.texto);
            if !es_imagen {
                marcas.push((inicio..texto.len(), Marca::Enlace));
            }
            resto = tras;
            continue;
        }
        texto.push(c);
        resto = &resto[c.len_utf8()..];
    }
    marcas.sort_by_key(|(r, _)| r.start);
    Linea { texto, marcas }
}

/// `[etiqueta](destino)` al principio de `s`: la etiqueta y lo que sigue.
fn enlace(s: &str) -> Option<(&str, &str)> {
    let cierre = s.find("](")?;
    let fin = s[cierre + 2..].find(')')?;
    Some((&s[1..cierre], &s[cierre + 2 + fin + 1..]))
}

/// Colores y tipos de letra con que se pinta: los del `.ql-md` de Tauri.
pub struct Estilo {
    pub tema: Tema,
    /// El zoom de la vista rápida: multiplica todas las medidas.
    pub escala: f32,
    pub fuente: &'static str,
    pub mono: &'static str,
}

impl Estilo {
    fn texto(&self, linea: &Linea, peso: FontWeight, color: Hsla) -> StyledText {
        let tm = self.tema;
        let base = gpui::font(self.fuente);
        // Un corte en cada borde de marca; cada tramo lleva las que lo cubren.
        let mut cortes: Vec<usize> = vec![0, linea.texto.len()];
        for (r, _) in &linea.marcas {
            cortes.push(r.start);
            cortes.push(r.end);
        }
        cortes.sort_unstable();
        cortes.dedup();
        let runs = cortes
            .windows(2)
            .filter(|w| w[1] > w[0])
            .map(|w| {
                let tiene = |m: Marca| {
                    linea
                        .marcas
                        .iter()
                        .any(|(r, x)| *x == m && r.start <= w[0] && w[1] <= r.end)
                };
                let mut font = if tiene(Marca::Codigo) {
                    gpui::font(self.mono)
                } else {
                    base.clone()
                };
                font.weight = if tiene(Marca::Negrita) {
                    FontWeight::BOLD
                } else {
                    peso
                };
                if tiene(Marca::Cursiva) {
                    font.style = FontStyle::Italic;
                }
                TextRun {
                    len: w[1] - w[0],
                    font,
                    color: if tiene(Marca::Enlace) { tm.blue } else { color },
                    background_color: tiene(Marca::Codigo).then_some(tm.sbg),
                    underline: None,
                    strikethrough: tiene(Marca::Tachado).then_some(StrikethroughStyle {
                        thickness: px(1.),
                        color: None,
                    }),
                }
            })
            .collect();
        StyledText::new(SharedString::from(linea.texto.clone())).with_runs(runs)
    }

    fn p(&self, v: f32) -> gpui::Pixels {
        px(v * self.escala)
    }

    pub fn pintar(&self, bloques: &[Bloque]) -> Vec<AnyElement> {
        bloques
            .iter()
            .enumerate()
            .map(|(i, b)| self.bloque(b, i == 0))
            .collect()
    }

    fn bloque(&self, b: &Bloque, primero: bool) -> AnyElement {
        let tm = self.tema;
        match b {
            Bloque::Titulo(nivel, linea) => {
                let (tam, arriba, abajo, peso) = match nivel {
                    1 => (27., 9., 5., FontWeight::BOLD),
                    2 => (21.75, 10.5, 4.5, FontWeight::BOLD),
                    3 => (18., 10.5, 4.5, FontWeight::SEMIBOLD),
                    _ => (15., 9., 4., FontWeight::SEMIBOLD),
                };
                div()
                    .when(!primero, |d| d.mt(self.p(arriba)))
                    .mb(self.p(abajo))
                    .text_size(self.p(tam))
                    .line_height(self.p(tam * 1.3))
                    .when(*nivel == 2, |d| d.pb(self.p(4.)).border_b_1().border_color(tm.div))
                    .child(self.texto(linea, peso, tm.tx))
                    .into_any_element()
            }
            Bloque::Parrafo(linea) => div()
                .my(self.p(7.5))
                .child(self.texto(linea, FontWeight::NORMAL, tm.tx))
                .into_any_element(),
            Bloque::Cita(dentro) => div()
                .my(self.p(9.))
                .py(self.p(3.))
                .px(self.p(14.))
                .border_l(self.p(3.))
                .border_color(tm.blue)
                .rounded_r(self.p(8.))
                .bg(tm.sbg)
                .text_color(tm.tx2)
                .children(dentro.iter().map(|b| match b {
                    // Dentro de la cita el texto va en `--tx2`, como en el CSS.
                    Bloque::Parrafo(l) => div()
                        .my(self.p(3.))
                        .child(self.texto(l, FontWeight::NORMAL, tm.tx2))
                        .into_any_element(),
                    otro => self.bloque(otro, false),
                }))
                .into_any_element(),
            Bloque::Lista(items) => div()
                .my(self.p(6.))
                .flex()
                .flex_col()
                .children(items.iter().map(|(nivel, marcador, linea)| {
                    div()
                        .my(self.p(2.25))
                        .pl(self.p(24. * *nivel as f32))
                        .flex()
                        .child(
                            div()
                                .w(self.p(24.))
                                .flex_none()
                                .text_color(tm.tx2)
                                .child(marcador.clone()),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w(self.p(0.))
                                .child(self.texto(linea, FontWeight::NORMAL, tm.tx)),
                        )
                }))
                .into_any_element(),
            Bloque::Codigo(codigo) => div()
                .my(self.p(9.))
                .py(self.p(12.))
                .px(self.p(16.))
                .rounded(self.p(10.))
                .bg(tm.sbg)
                .font_family(self.mono)
                .text_size(self.p(13.))
                .line_height(self.p(19.5))
                .child(codigo.clone())
                .into_any_element(),
            Bloque::Regla => div()
                .my(self.p(15.))
                .h(self.p(1.))
                .bg(tm.div)
                .into_any_element(),
            Bloque::Tabla { cabecera, filas } => {
                let columnas = cabecera.len().max(1);
                let fila = |celdas: &[Linea], es_cabecera: bool| {
                    div()
                        .flex()
                        .when(es_cabecera, |d| d.bg(tm.sbg))
                        .children((0..columnas).map(|c| {
                            let celda = div()
                                .flex_1()
                                .min_w(self.p(0.))
                                .py(self.p(6.))
                                .px(self.p(12.))
                                .border_color(tm.div)
                                .border_b_1()
                                .when(c > 0, |d| d.border_l_1());
                            match celdas.get(c) {
                                Some(l) => celda.child(self.texto(
                                    l,
                                    if es_cabecera {
                                        FontWeight::SEMIBOLD
                                    } else {
                                        FontWeight::NORMAL
                                    },
                                    tm.tx,
                                )),
                                None => celda,
                            }
                        }))
                };
                div()
                    .my(self.p(9.))
                    .border_1()
                    .border_b_0()
                    .border_color(tm.div)
                    .text_size(self.p(13.5))
                    .line_height(self.p(20.))
                    .child(fila(cabecera, true))
                    .children(filas.iter().map(|f| fila(f, false)))
                    .into_any_element()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quita_las_marcas_y_apunta_los_estilos() {
        let l = en_linea("un **dato** y `--bg` en [la guía](#guia)");
        assert_eq!(l.texto, "un dato y --bg en la guía");
        assert_eq!(
            l.marcas,
            vec![
                (3..7, Marca::Negrita),
                (10..14, Marca::Codigo),
                (18..26, Marca::Enlace),
            ]
        );
    }

    #[test]
    fn el_guion_bajo_de_un_nombre_no_es_cursiva() {
        let l = en_linea("usa mi_variable_larga");
        assert_eq!(l.texto, "usa mi_variable_larga");
        assert!(l.marcas.is_empty());
    }

    #[test]
    fn reconoce_los_bloques_de_la_guia() {
        let md = "# Título\n\n> cita\n> sigue\n\n---\n\n1. uno\n2. dos\n\n- a\n  - b\n\n\
                  | A | B |\n|---|---|\n| `x` | y |\n\n```\nfn main() {}\n```\npárrafo\nsegunda";
        let b = parsear(md);
        assert!(matches!(&b[0], Bloque::Titulo(1, l) if l.texto == "Título"));
        assert!(matches!(&b[1], Bloque::Cita(d) if d.len() == 1));
        assert_eq!(b[2], Bloque::Regla);
        assert!(matches!(&b[3], Bloque::Lista(i) if i.len() == 2 && i[1].1 == "2."));
        assert!(matches!(&b[4], Bloque::Lista(i) if i[1].0 == 1));
        assert!(matches!(&b[5], Bloque::Tabla { cabecera, filas } if cabecera.len() == 2 && filas.len() == 1));
        assert_eq!(b[6], Bloque::Codigo("fn main() {}".into()));
        assert!(matches!(&b[7], Bloque::Parrafo(l) if l.texto == "párrafo\nsegunda"));
    }
}
