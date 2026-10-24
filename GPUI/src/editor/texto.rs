use std::ops::Range;

#[derive(Default)]
pub struct Texto { pub cadena: String, pub inicios: Vec<usize> }
impl Texto {
    pub fn nuevo(cadena: String) -> Self {
        let mut inicios=vec![0]; inicios.extend(cadena.match_indices('\n').map(|(i,_)| i+1));
        Self { cadena, inicios }
    }
    pub fn linea_de(&self, byte: usize) -> usize { self.inicios.partition_point(|&i| i<=byte).saturating_sub(1) }
    pub fn rango_linea(&self, linea: usize) -> Range<usize> {
        let inicio=self.inicios[linea]; let fin=self.inicios.get(linea+1).map_or(self.cadena.len(), |i| i-1);
        inicio..fin
    }
    pub fn reemplazar(&mut self, rango: Range<usize>, nuevo: &str) {
        let desde=self.inicios.partition_point(|&i| i<=rango.start);
        let hasta=self.inicios.partition_point(|&i| i<=rango.end);
        let diferencia=nuevo.len() as isize-(rango.end-rango.start) as isize;
        for i in &mut self.inicios[hasta..] { *i=i.checked_add_signed(diferencia).expect("índices posteriores al reemplazo"); }
        self.inicios.splice(desde..hasta,nuevo.match_indices('\n').map(|(i,_)| rango.start+i+1));
        self.cadena.replace_range(rango,nuevo);
    }
    pub fn anterior(&self, pos: usize) -> usize { self.cadena[..pos].char_indices().next_back().map_or(0,|(i,_)|i) }
    pub fn siguiente(&self, pos: usize) -> usize { self.cadena[pos..].chars().next().map_or(pos,|c|pos+c.len_utf8()) }
}
#[cfg(test)]
mod pruebas {
    use super::*;
    #[test]
    fn indice_tras_insertar_y_borrar_unicode() {
        let mut texto=Texto::nuevo("á\nb\n".into()); texto.reemplazar(2..3,"\n𝄞\n");
        assert_eq!(texto.inicios,Texto::nuevo(texto.cadena.clone()).inicios);
        texto.reemplazar(0..texto.cadena.len(),"x"); assert_eq!(texto.inicios,vec![0]);
    }
}
