use std::{io::Cursor,sync::Arc};
use base64::{Engine,engine::general_purpose::STANDARD};
use gpui::RenderImage;
pub fn insertar(bytes:&[u8])->Result<Arc<str>,String>{
    let formato=image::guess_format(bytes).map_err(|e|e.to_string())?;
    let imagen=image::load_from_memory(bytes).map_err(|e|e.to_string())?;
    let reducida=imagen.thumbnail(1600,1600);
    let salida=if formato==image::ImageFormat::Jpeg {image::ImageFormat::Jpeg}else{image::ImageFormat::Png};
    let mut datos=Cursor::new(Vec::new());reducida.write_to(&mut datos,salida).map_err(|e|e.to_string())?;
    Ok(Arc::from(format!("data:image/{};base64,{}",if salida==image::ImageFormat::Jpeg{"jpeg"}else{"png"},STANDARD.encode(datos.into_inner()))))
}
pub fn datos(fuente:&str)->Result<Vec<u8>,String>{
    let (cabecera,contenido)=fuente.split_once(',').ok_or("Imagen sin datos")?;
    if cabecera.ends_with(";base64"){STANDARD.decode(contenido).map_err(|e|e.to_string())}else{
        let mut bytes=Vec::new();let mut iter=contenido.bytes();while let Some(b)=iter.next(){if b==b'%' {let a=iter.next().ok_or("Escape incompleto")?;let z=iter.next().ok_or("Escape incompleto")?;let par=[a,z];let s=std::str::from_utf8(&par).map_err(|e|e.to_string())?;bytes.push(u8::from_str_radix(s,16).map_err(|e|e.to_string())?);}else{bytes.push(b);}}Ok(bytes)
    }
}
pub fn dimensiones(fuente:&str)->Result<(u32,u32),String>{image::ImageReader::new(Cursor::new(datos(fuente)?)).with_guessed_format().map_err(|e|e.to_string())?.into_dimensions().map_err(|e|e.to_string())}
pub fn rasterizar(fuente:&str,ancho:u32)->Result<Arc<RenderImage>,String>{
    let imagen=image::load_from_memory(&datos(fuente)?).map_err(|e|e.to_string())?;
    let alto=(imagen.height()as f64*ancho as f64/imagen.width()as f64).round().max(1.)as u32;
    let mut rgba=imagen.resize(ancho.max(1),alto,image::imageops::FilterType::Triangle).into_rgba8();
    for pixel in rgba.pixels_mut(){pixel.0.swap(0,2);}
    Ok(Arc::new(RenderImage::new(vec![image::Frame::new(rgba)])))
}
pub fn es_imagen(ruta:&std::path::Path)->bool{ruta.extension().is_some_and(|s|["png","jpg","jpeg","webp","gif","bmp","svg"].iter().any(|v|s.eq_ignore_ascii_case(v)))}
