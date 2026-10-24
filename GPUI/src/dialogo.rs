use std::{path::PathBuf,time::SystemTime};
// El marco compartido no ofrece confirmaciones y el sandbox solo permite editar esta app.
pub enum Dialogo {
    Borrar(PathBuf),Cerrar(usize),Salir,Recargar(usize,SystemTime),Sobrescribir(usize),
}
