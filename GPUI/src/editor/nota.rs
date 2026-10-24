use std::{ops::Range, sync::Arc};
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Formato { pub negrita: bool, pub cursiva: bool, pub subrayado: bool, pub tachado: bool, pub color: Option<String>, pub fondo: Option<String> }
#[derive(Clone, Debug, PartialEq)]
pub struct Tramo { pub rango: Range<usize>, pub formato: Formato }
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Alineacion { #[default] Izquierda, Centro, Derecha }
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Lista { #[default] Ninguna, Vineta, Numero, Tarea(bool) }
#[derive(Clone, Debug, PartialEq)]
pub enum Bloque {
    Texto { texto: String, tramos: Vec<Tramo>, lista: Lista, alineacion: Alineacion },
    Imagen { fuente: Arc<str>, ancho: Option<f32>, alineacion: Alineacion },
}
impl Bloque {
    pub fn vacio() -> Self { Self::Texto {texto:String::new(),tramos:Vec::new(),lista:Lista::Ninguna,alineacion:Alineacion::Izquierda} }
    pub fn texto(&self) -> &str { match self {Self::Texto {texto,..}=>texto,Self::Imagen {..}=>"\u{fffc}"} }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Nota { pub bloques: Vec<Bloque> }
impl Default for Nota { fn default()->Self {Self {bloques:vec![Bloque::vacio()]} } }
impl Nota {
    pub fn plano(&self)->String { self.bloques.iter().map(Bloque::texto).collect::<Vec<_>>().join("\n") }
    pub fn vacia(&self)->bool {self.bloques.iter().all(|b|matches!(b,Bloque::Texto {texto,..} if texto.trim().is_empty()))}
    pub fn posicion(&self, byte:usize)->(usize,usize) {
        let mut inicio=0;
        for (i,b) in self.bloques.iter().enumerate() { if byte<=inicio+b.texto().len() {return(i,byte-inicio);} inicio+=b.texto().len()+1; }
        let i=self.bloques.len()-1;(i,self.bloques[i].texto().len())
    }
    pub fn reemplazar(&mut self,rango:Range<usize>,nuevo:&str,formato:&Formato) {
        let (primero,desde)=self.posicion(rango.start); let (ultimo,hasta)=self.posicion(rango.end);
        let inicio=self.bloques[primero].clone(); let fin=self.bloques[ultimo].clone();
        let (lista,alineacion)=match &inicio {Bloque::Texto{lista,alineacion,..}=>(*lista,*alineacion),Bloque::Imagen{alineacion,..}=>(Lista::Ninguna,*alineacion)};
        let prefijo=if matches!(inicio,Bloque::Texto{..}) { &inicio.texto()[..desde] } else { "" };
        let sufijo=if matches!(fin,Bloque::Texto{..}) { &fin.texto()[hasta..] } else { "" };
        let combinado=format!("{prefijo}{nuevo}{sufijo}");
        let mut corridas=Vec::new();
        if let Bloque::Texto{tramos,..}=&inicio { for t in tramos { let extremo=t.rango.end.min(prefijo.len()); if t.rango.start<extremo { corridas.push(Tramo{rango:t.rango.start..extremo,formato:t.formato.clone()}); } } }
        if !nuevo.is_empty() {corridas.push(Tramo {rango:prefijo.len()..prefijo.len()+nuevo.len(),formato:formato.clone()});}
        if let Bloque::Texto{tramos,..}=&fin { for t in tramos {let comienzo=t.rango.start.max(hasta); if comienzo<t.rango.end {corridas.push(Tramo{rango:prefijo.len()+nuevo.len()+comienzo-hasta..prefijo.len()+nuevo.len()+t.rango.end-hasta,formato:t.formato.clone()});}}}
        let mut offset=0;
        let bloques=combinado.split('\n').enumerate().map(|(i,texto)|{
            let tramos=corridas.iter().filter_map(|t|{let a=t.rango.start.max(offset);let b=t.rango.end.min(offset+texto.len());(a<b).then(||Tramo {rango:a-offset..b-offset,formato:t.formato.clone()})}).collect();
            offset+=texto.len()+1;
            Bloque::Texto{texto:texto.into(),tramos,lista:if i>0 && matches!(lista,Lista::Tarea(_)){Lista::Tarea(false)}else{lista},alineacion}
        }).collect::<Vec<_>>();
        self.bloques.splice(primero..=ultimo,bloques);
    }
    pub fn aplicar(&mut self,rango:Range<usize>,cambiar:impl Fn(&mut Formato)) {
        let mut inicio=0;
        for (indice,b) in self.bloques.iter_mut().enumerate() {
            let longitud=b.texto().len();
            if let Bloque::Texto{texto,tramos,..}=b {
                let a=rango.start.saturating_sub(inicio).min(longitud);let z=rango.end.saturating_sub(inicio).min(longitud);
                if a<z {
                    let mut bordes=vec![0,a,z,texto.len()];for t in tramos.iter(){bordes.extend([t.rango.start,t.rango.end]);} bordes.sort_unstable();bordes.dedup();
                    *tramos=bordes.windows(2).filter(|v|v[0]<v[1]).map(|v|{
                        let mut formato=tramos.iter().find(|t|t.rango.contains(&v[0])).map(|t|t.formato.clone()).unwrap_or_default();
                        if (a..z).contains(&v[0]) {cambiar(&mut formato); if indice==0 {formato.negrita=false;} }
                        Tramo{rango:v[0]..v[1],formato}
                    }).collect();
                }
            }
            inicio+=longitud+1;
        }
    }
}
