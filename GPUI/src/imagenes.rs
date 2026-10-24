use base64::{Engine, engine::general_purpose::STANDARD};
use gpui::RenderImage;
use std::{io::Cursor, sync::Arc};
pub fn insertar(bytes: &[u8], svg: &gpui::SvgRenderer) -> Result<Arc<str>, String> {
    if std::str::from_utf8(bytes).is_ok_and(|s| s.contains("<svg")) {
        svg.render_single_frame(bytes, 0.5)
            .map_err(|e| e.to_string())?;
        return Ok(Arc::from(format!(
            "data:image/svg+xml;base64,{}",
            STANDARD.encode(bytes)
        )));
    }
    let formato = image::guess_format(bytes).map_err(|e| e.to_string())?;
    let imagen = image::load_from_memory(bytes).map_err(|e| e.to_string())?;
    let reducida = imagen.thumbnail(1600, 1600);
    let salida = if formato == image::ImageFormat::Jpeg {
        image::ImageFormat::Jpeg
    } else {
        image::ImageFormat::Png
    };
    let mut datos = Cursor::new(Vec::new());
    reducida
        .write_to(&mut datos, salida)
        .map_err(|e| e.to_string())?;
    Ok(Arc::from(format!(
        "data:image/{};base64,{}",
        if salida == image::ImageFormat::Jpeg {
            "jpeg"
        } else {
            "png"
        },
        STANDARD.encode(datos.into_inner())
    )))
}
pub fn datos(fuente: &str) -> Result<Vec<u8>, String> {
    let (cabecera, contenido) = fuente.split_once(',').ok_or("Imagen sin datos")?;
    if cabecera.ends_with(";base64") {
        STANDARD.decode(contenido).map_err(|e| e.to_string())
    } else {
        let mut bytes = Vec::new();
        let mut iter = contenido.bytes();
        while let Some(b) = iter.next() {
            if b == b'%' {
                let a = iter.next().ok_or("Escape incompleto")?;
                let z = iter.next().ok_or("Escape incompleto")?;
                let par = [a, z];
                let s = std::str::from_utf8(&par).map_err(|e| e.to_string())?;
                bytes.push(u8::from_str_radix(s, 16).map_err(|e| e.to_string())?);
            } else {
                bytes.push(b);
            }
        }
        Ok(bytes)
    }
}
pub fn dimensiones(fuente: &str, svg: &gpui::SvgRenderer) -> Result<(u32, u32), String> {
    if fuente.starts_with("data:image/svg") {
        let imagen = svg
            .render_single_frame(&datos(fuente)?, 0.5)
            .map_err(|e| e.to_string())?;
        let tamano = imagen.size(0);
        return Ok((tamano.width.0 as u32, tamano.height.0 as u32));
    }
    image::ImageReader::new(Cursor::new(datos(fuente)?))
        .with_guessed_format()
        .map_err(|e| e.to_string())?
        .into_dimensions()
        .map_err(|e| e.to_string())
}
pub fn rasterizar(
    fuente: &str,
    ancho: u32,
    svg: &gpui::SvgRenderer,
) -> Result<Arc<RenderImage>, String> {
    if fuente.starts_with("data:image/svg") {
        let (original, _) = dimensiones(fuente, svg)?;
        return svg
            .render_single_frame(&datos(fuente)?, ancho as f32 / original as f32 / 2.)
            .map_err(|e| e.to_string());
    }
    let imagen = image::load_from_memory(&datos(fuente)?).map_err(|e| e.to_string())?;
    let alto = (imagen.height() as f64 * ancho as f64 / imagen.width() as f64)
        .round()
        .max(1.) as u32;
    let mut rgba = imagen
        .resize(ancho.max(1), alto, image::imageops::FilterType::Triangle)
        .into_rgba8();
    for pixel in rgba.pixels_mut() {
        pixel.0.swap(0, 2);
    }
    Ok(Arc::new(RenderImage::new(vec![image::Frame::new(rgba)])))
}
pub fn miniatura(fuente: &str, svg: &gpui::SvgRenderer) -> Result<Arc<RenderImage>, String> {
    let (ancho, alto) = dimensiones(fuente, svg)?;
    let imagen = rasterizar(
        fuente,
        80.max((80. * ancho as f32 / alto as f32).ceil() as u32),
        svg,
    )?;
    let tamano = imagen.size(0);
    let pixeles = image::RgbaImage::from_raw(
        tamano.width.0 as u32,
        tamano.height.0 as u32,
        imagen
            .as_bytes(0)
            .expect("primer fotograma rasterizado")
            .to_vec(),
    )
    .expect("cuatro canales por píxel rasterizado");
    let recorte = image::imageops::crop_imm(
        &pixeles,
        (pixeles.width() - 80) / 2,
        (pixeles.height() - 80) / 2,
        80,
        80,
    )
    .to_image();
    Ok(Arc::new(RenderImage::new(vec![image::Frame::new(recorte)])))
}
pub fn es_imagen(ruta: &std::path::Path) -> bool {
    ruta.extension().is_some_and(|s| {
        ["png", "jpg", "jpeg", "webp", "gif", "bmp", "svg"]
            .iter()
            .any(|v| s.eq_ignore_ascii_case(v))
    })
}

#[cfg(test)]
mod pruebas {
    use super::{datos, dimensiones, insertar, rasterizar};
    #[test]
    fn miniatura_cuadrada_recorta_el_centro_sin_deformar() {
        let svg = gpui::SvgRenderer::new(std::sync::Arc::new(bookos_ui::Recursos(crate::iconos)));
        let original = br#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100"><rect width="200" height="100" fill="red"/><rect x="50" width="100" height="100" fill="blue"/></svg>"#;
        let fuente = insertar(original, &svg).expect("SVG rectangular válido");
        let miniatura = super::miniatura(&fuente, &svg).expect("recorte central válido");
        assert_eq!(miniatura.size(0).width.0, 80);
        assert_eq!(miniatura.size(0).height.0, 80);
        let datos = miniatura.as_bytes(0).expect("miniatura con un fotograma");
        let centro = (40 * 80 + 40) * 4;
        assert_eq!(&datos[centro..centro + 4], &[255, 0, 0, 255]);
    }
    #[test]
    fn svg_conserva_el_original_y_se_rasteriza_a_la_escala_pedida() {
        let svg = gpui::SvgRenderer::new(std::sync::Arc::new(bookos_ui::Recursos(crate::iconos)));
        let original=br#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="20"><rect width="10" height="20" fill="red"/></svg>"#;
        let fuente = insertar(original, &svg).expect("SVG válido");
        assert_eq!(datos(&fuente).expect("base64 válido"), original);
        assert_eq!(
            dimensiones(&fuente, &svg).expect("dimensiones SVG"),
            (10, 20)
        );
        let imagen = rasterizar(&fuente, 30, &svg).expect("SVG rasterizado");
        assert_eq!(imagen.size(0).width.0, 30);
        assert_eq!(imagen.size(0).height.0, 60);
        assert!(insertar(b"<svg roto", &svg).is_err());
    }
}
