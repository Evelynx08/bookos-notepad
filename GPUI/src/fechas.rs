use std::time::{SystemTime, UNIX_EPOCH};
fn local(fecha: SystemTime) -> libc::tm {
    let segundos = fecha.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs()) as libc::time_t;
    let mut salida = std::mem::MaybeUninit::uninit();
    // Ambos punteros permanecen válidos durante la llamada; localtime_r no usa memoria global.
    unsafe {
        let resultado = libc::localtime_r(&segundos, salida.as_mut_ptr());
        if resultado.is_null() {
            return std::mem::zeroed();
        }
        salida.assume_init()
    }
}
pub fn grupo(fecha: SystemTime, ahora: SystemTime, ingles: bool) -> String {
    let f = local(fecha);
    let a = local(ahora);
    let dias = (a.tm_year - f.tm_year) * 365 + a.tm_yday - f.tm_yday;
    match dias {
        0 => if ingles { "Today" } else { "Hoy" }.into(),
        1 => if ingles { "Yesterday" } else { "Ayer" }.into(),
        2..=7 => if ingles {
            "Previous 7 days"
        } else {
            "7 días anteriores"
        }
        .into(),
        8..=30 => if ingles {
            "Previous 30 days"
        } else {
            "30 días anteriores"
        }
        .into(),
        _ => format!("{}", f.tm_year + 1900),
    }
}
pub fn corta(fecha: SystemTime) -> String {
    let f = local(fecha);
    format!(
        "{:02}/{:02}/{}",
        f.tm_mday,
        f.tm_mon + 1,
        (f.tm_year + 1900) % 100
    )
}
pub fn larga(fecha: SystemTime, ingles: bool) -> String {
    let f = local(fecha);
    let meses = if ingles {
        [
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
            "November",
            "December",
        ]
    } else {
        [
            "enero",
            "febrero",
            "marzo",
            "abril",
            "mayo",
            "junio",
            "julio",
            "agosto",
            "septiembre",
            "octubre",
            "noviembre",
            "diciembre",
        ]
    };
    let separador = if ingles { " " } else { " de " };
    format!(
        "{}{separador}{}{separador}{}, {:02}:{:02}",
        f.tm_mday,
        meses[f.tm_mon as usize],
        f.tm_year + 1900,
        f.tm_hour,
        f.tm_min
    )
}
