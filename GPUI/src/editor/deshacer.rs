use super::nota::Nota;
use std::{
    ops::Range,
    time::{Duration, Instant},
};
pub enum Paso {
    Texto {
        rango: Range<usize>,
        quitado: String,
        puesto: String,
        antes: Range<usize>,
        despues: Range<usize>,
        instante: Instant,
    },
    Nota {
        antes: Nota,
        despues: Nota,
        seleccion_antes: Range<usize>,
        seleccion_despues: Range<usize>,
    },
}
#[derive(Default)]
pub struct Historial {
    pub atras: Vec<Paso>,
    pub delante: Vec<Paso>,
}
impl Historial {
    pub fn agregar(&mut self, paso: Paso, agrupar: bool) {
        self.delante.clear();
        if agrupar
            && let (
                Some(Paso::Texto {
                    rango,
                    quitado,
                    puesto,
                    despues,
                    instante,
                    ..
                }),
                Paso::Texto {
                    rango: nuevo,
                    quitado: q,
                    puesto: p,
                    despues: d,
                    instante: t,
                    ..
                },
            ) = (self.atras.last_mut(), &paso)
            && t.duration_since(*instante) < Duration::from_millis(500)
            && quitado.is_empty()
            && q.is_empty()
            && nuevo.start == rango.start + puesto.len()
            && !p.contains('\n')
        {
            puesto.push_str(p);
            *despues = d.clone();
            *instante = *t;
            return;
        }
        self.atras.push(paso);
        if self.atras.len() > 200 {
            self.atras.remove(0);
        }
    }
}
