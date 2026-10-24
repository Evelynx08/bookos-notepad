use regex::{Regex, RegexBuilder};
use std::ops::Range;
#[derive(Default)]
pub struct Busqueda {
    pub expresion: Option<Regex>,
    pub coincidencias: Vec<Range<usize>>,
    pub invalida: bool,
}
impl Busqueda {
    pub fn nueva(texto: &str, termino: &str, mayusculas: bool, regex: bool) -> Self {
        if termino.is_empty() {
            return Self::default();
        }
        let patron = if regex {
            termino.into()
        } else {
            regex::escape(termino)
        };
        match RegexBuilder::new(&patron)
            .case_insensitive(!mayusculas)
            .multi_line(true)
            .build()
        {
            Ok(expresion) => Self {
                coincidencias: expresion.find_iter(texto).map(|m| m.range()).collect(),
                expresion: Some(expresion),
                invalida: false,
            },
            Err(_) => Self {
                invalida: true,
                ..Self::default()
            },
        }
    }
    pub fn sustituir(
        &self,
        texto: &str,
        rango: Range<usize>,
        reemplazo: &str,
        grupos: bool,
    ) -> Option<String> {
        let re = self.expresion.as_ref()?;
        let coincidencia = re.captures_at(texto, rango.start)?;
        if coincidencia.get(0)?.range() != rango {
            return None;
        }
        let mut nuevo = String::new();
        if grupos {
            coincidencia.expand(reemplazo, &mut nuevo);
        } else {
            nuevo.push_str(reemplazo);
        }
        Some(nuevo)
    }
}
#[cfg(test)]
mod pruebas {
    use super::*;
    #[test]
    fn literal_regex_grupos_y_vacios() {
        assert_eq!(
            Busqueda::nueva("Hola hola", "hola", false, false)
                .coincidencias
                .len(),
            2
        );
        assert_eq!(
            Busqueda::nueva("Hola hola", "hola", true, false)
                .coincidencias
                .len(),
            1
        );
        assert!(Busqueda::nueva("x", "[", false, true).invalida);
        let b = Busqueda::nueva("abc12", "([a-z]+)([0-9]+)", true, true);
        assert_eq!(
            b.sustituir("abc12", 0..5, "$2-$1", true).as_deref(),
            Some("12-abc")
        );
        assert_eq!(Busqueda::nueva("á", "", true, true).coincidencias.len(), 0);
        assert_eq!(
            Busqueda::nueva("ab", "^|$", true, true).coincidencias.len(),
            2
        );
    }
}
