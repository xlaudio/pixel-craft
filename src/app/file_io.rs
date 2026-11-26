//! # file_io — Importazione ed esportazione
//!
//! Gestisce il caricamento e il salvataggio delle immagini e le operazioni
//! di copia/incolla tramite file. Le modifiche al documento passano per
//! lo stato condiviso di [`PaintApp`].

use egui::Color32;
use printpdf::{Image, ImageTransform, Mm, PdfDocument};
use quick_xml::{
    events::{BytesStart, Event},
    Reader,
};
use std::io::{Cursor, Read, Seek, Write};
use zip::write::FileOptions;

use super::document::{composite_pixel, valid_canvas_dimensions};
use super::export::{ExportManager, ExportOptions, RenderedImage};
use crate::model::{ClipboardData, Layer, PaintApp};

const MAX_OPENRASTER_MEMBER_SIZE: u64 = 512 * 1024 * 1024;

struct OpenRasterLayer {
    name: String,
    source: String,
    opacity: f32,
    visible: bool,
    x: i64,
    y: i64,
}

fn xml_attributes(
    element: &BytesStart<'_>,
    decoder: quick_xml::encoding::Decoder,
) -> Result<std::collections::HashMap<String, String>, String> {
    element
        .attributes()
        .map(|attribute| {
            let attribute = attribute.map_err(|error| error.to_string())?;
            let name = std::str::from_utf8(attribute.key.as_ref())
                .map_err(|error| error.to_string())?
                .to_owned();
            let value = attribute
                .decoded_and_normalized_value(quick_xml::XmlVersion::Implicit1_0, decoder)
                .map_err(|error| error.to_string())?
                .into_owned();
            Ok((name, value))
        })
        .collect()
}

fn parse_openraster_stack(xml: &str) -> Result<(usize, usize, Vec<OpenRasterLayer>), String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut dimensions = None;
    let mut layers = Vec::new();
    let mut stack_open = false;
    let mut saw_stack = false;

    loop {
        match reader.read_event().map_err(|error| error.to_string())? {
            Event::Start(element) => match element.name().as_ref() {
                b"image" => {
                    let attributes = xml_attributes(&element, reader.decoder())?;
                    let width = attributes
                        .get("w")
                        .ok_or_else(|| "Larghezza mancante in stack.xml.".to_string())?
                        .parse::<usize>()
                        .map_err(|error| format!("Larghezza OpenRaster non valida: {error}"))?;
                    let height = attributes
                        .get("h")
                        .ok_or_else(|| "Altezza mancante in stack.xml.".to_string())?
                        .parse::<usize>()
                        .map_err(|error| format!("Altezza OpenRaster non valida: {error}"))?;
                    dimensions = Some((width, height));
                }
                b"stack" => {
                    if stack_open {
                        return Err(
                            "I gruppi di livelli OpenRaster non sono supportati.".to_string()
                        );
                    }
                    stack_open = true;
                    saw_stack = true;
                }
                b"layer" if stack_open => {
                    let attributes = xml_attributes(&element, reader.decoder())?;
                    layers.push(parse_openraster_layer(&attributes)?);
                }
                _ => {}
            },
            Event::Empty(element) if element.name().as_ref() == b"layer" && stack_open => {
                let attributes = xml_attributes(&element, reader.decoder())?;
                layers.push(parse_openraster_layer(&attributes)?);
            }
            Event::Empty(element) if element.name().as_ref() == b"stack" => {
                if stack_open {
                    return Err("I gruppi di livelli OpenRaster non sono supportati.".to_string());
                }
                saw_stack = true;
            }
            Event::End(element) if element.name().as_ref() == b"stack" => stack_open = false,
            Event::Eof => break,
            _ => {}
        }
    }

    let (width, height) =
        dimensions.ok_or_else(|| "Dimensioni immagine mancanti in stack.xml.".to_string())?;
    if !valid_canvas_dimensions(width, height) {
        return Err("Dimensioni OpenRaster non valide o superiori ai limiti.".to_string());
    }
    if !saw_stack || stack_open {
        return Err("Stack dei livelli OpenRaster mancante o non valido.".to_string());
    }
    Ok((width, height, layers))
}

fn parse_openraster_layer(
    attributes: &std::collections::HashMap<String, String>,
) -> Result<OpenRasterLayer, String> {
    let parse_integer = |name: &str| -> Result<i64, String> {
        attributes.get(name).map_or(Ok(0), |value| {
            value
                .parse::<i64>()
                .map_err(|error| format!("Posizione OpenRaster non valida: {error}"))
        })
    };
    let opacity = attributes.get("opacity").map_or(Ok(1.0), |value| {
        value
            .parse::<f32>()
            .map_err(|error| format!("Opacità OpenRaster non valida: {error}"))
    })?;
    if !opacity.is_finite() || !(0.0..=1.0).contains(&opacity) {
        return Err("Opacità OpenRaster fuori intervallo.".to_string());
    }

    Ok(OpenRasterLayer {
        name: attributes
            .get("name")
            .cloned()
            .unwrap_or_else(|| "Livello".to_string()),
        source: attributes
            .get("src")
            .cloned()
            .ok_or_else(|| "Percorso immagine mancante per un livello OpenRaster.".to_string())?,
        opacity,
        visible: attributes
            .get("visibility")
            .is_none_or(|visibility| visibility == "visible"),
        x: parse_integer("x")?,
        y: parse_integer("y")?,
    })
}

fn decode_openraster<R: Read + Seek>(source: R) -> Result<(usize, usize, Vec<Layer>), String> {
    let mut archive = zip::ZipArchive::new(source).map_err(|error| error.to_string())?;
    let mimetype_file = archive
        .by_name("mimetype")
        .map_err(|error| format!("File OpenRaster senza mimetype: {error}"))?;
    if mimetype_file.size() > 64 {
        return Err("Il file mimetype OpenRaster non è valido.".to_string());
    }
    let mut mimetype = String::new();
    mimetype_file
        .take(65)
        .read_to_string(&mut mimetype)
        .map_err(|error| error.to_string())?;
    if mimetype != "image/openraster" {
        return Err("Il file non è un documento OpenRaster valido.".to_string());
    }

    let mut stack_xml = String::new();
    let stack_file = archive
        .by_name("stack.xml")
        .map_err(|error| format!("File OpenRaster senza stack.xml: {error}"))?;
    if stack_file.size() > 16 * 1024 * 1024 {
        return Err("Il file stack.xml supera la dimensione consentita.".to_string());
    }
    stack_file
        .take(16 * 1024 * 1024 + 1)
        .read_to_string(&mut stack_xml)
        .map_err(|error| error.to_string())?;
    if stack_xml.len() > 16 * 1024 * 1024 {
        return Err("Il file stack.xml supera la dimensione consentita.".to_string());
    }

    let (width, height, mut descriptors) = parse_openraster_stack(&stack_xml)?;
    let mut layers = Vec::with_capacity(descriptors.len());
    for descriptor in descriptors.drain(..).rev() {
        let source_file = archive
            .by_name(&descriptor.source)
            .map_err(|error| format!("Immagine livello OpenRaster non trovata: {error}"))?;
        if source_file.size() > MAX_OPENRASTER_MEMBER_SIZE {
            return Err("Un'immagine livello OpenRaster è troppo grande.".to_string());
        }
        let mut source_bytes = Vec::with_capacity(source_file.size() as usize);
        source_file
            .take(MAX_OPENRASTER_MEMBER_SIZE + 1)
            .read_to_end(&mut source_bytes)
            .map_err(|error| error.to_string())?;
        if source_bytes.len() as u64 > MAX_OPENRASTER_MEMBER_SIZE {
            return Err("Un'immagine livello OpenRaster è troppo grande.".to_string());
        }

        let source_dimensions = image::io::Reader::with_format(
            Cursor::new(source_bytes.as_slice()),
            image::ImageFormat::Png,
        )
        .into_dimensions()
        .map_err(|error| format!("Impossibile leggere le dimensioni di un livello: {error}"))?;
        if !valid_canvas_dimensions(source_dimensions.0 as usize, source_dimensions.1 as usize) {
            return Err("Dimensioni di un livello OpenRaster non valide.".to_string());
        }
        let source_image =
            image::load_from_memory_with_format(&source_bytes, image::ImageFormat::Png)
                .map_err(|error| {
                    format!("Impossibile decodificare un livello OpenRaster: {error}")
                })?
                .to_rgba8();
        let (source_width, source_height) = source_image.dimensions();
        if !valid_canvas_dimensions(source_width as usize, source_height as usize) {
            return Err("Dimensioni di un livello OpenRaster non valide.".to_string());
        }

        let mut layer = Layer::new(&descriptor.name, width, height);
        layer.opacity = descriptor.opacity;
        layer.visible = descriptor.visible;
        for (source_y, row) in source_image.rows().enumerate() {
            let Some(destination_y) = (source_y as i64).checked_add(descriptor.y) else {
                continue;
            };
            if !(0..height as i64).contains(&destination_y) {
                continue;
            }
            for (source_x, pixel) in row.enumerate() {
                let Some(destination_x) = (source_x as i64).checked_add(descriptor.x) else {
                    continue;
                };
                if (0..width as i64).contains(&destination_x) {
                    layer.pixels[destination_y as usize * width + destination_x as usize] =
                        Color32::from_rgba_unmultiplied(pixel[0], pixel[1], pixel[2], pixel[3]);
                }
            }
        }
        layers.push(layer);
    }

    if layers.is_empty() {
        layers.push(Layer::new("Livello 1", width, height));
    }
    Ok((width, height, layers))
}

fn openraster_merged_pixels(width: usize, height: usize, layers: &[Layer]) -> Vec<Color32> {
    let mut merged = vec![Color32::TRANSPARENT; width * height];
    for layer in layers
        .iter()
        .filter(|layer| layer.visible && layer.opacity > 0.0)
    {
        for (destination, &source) in merged.iter_mut().zip(&layer.pixels) {
            *destination = composite_pixel(*destination, source, layer.opacity);
        }
    }
    merged
}

fn escape_xml_attribute(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn encode_png(width: usize, height: usize, pixels: &[Color32]) -> Result<Vec<u8>, String> {
    ExportManager::encode_png(width, height, pixels)
}

fn encode_openraster(
    width: usize,
    height: usize,
    layers: &[Layer],
    merged_image: &[u8],
) -> Result<Vec<u8>, String> {
    let mut output = Cursor::new(Vec::new());
    {
        let mut archive = zip::ZipWriter::new(&mut output);
        let mimetype_options =
            FileOptions::default().compression_method(zip::CompressionMethod::Stored);
        archive
            .start_file("mimetype", mimetype_options)
            .map_err(|error| error.to_string())?;
        archive
            .write_all(b"image/openraster")
            .map_err(|error| error.to_string())?;

        let mut stack_xml = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<image version=\"0.0.1\" w=\"{width}\" h=\"{height}\"><stack>"
        );
        for (index, layer) in layers.iter().enumerate().rev() {
            let source = encode_png(width, height, &layer.pixels)?;
            let path = format!("data/layer{index}.png");
            archive
                .start_file(&path, FileOptions::default())
                .map_err(|error| error.to_string())?;
            archive
                .write_all(&source)
                .map_err(|error| error.to_string())?;

            stack_xml.push_str(&format!(
                "<layer name=\"{}\" src=\"{}\" opacity=\"{}\" visibility=\"{}\" composite-op=\"svg:src-over\" x=\"0\" y=\"0\"/>",
                escape_xml_attribute(&layer.name),
                path,
                layer.opacity.clamp(0.0, 1.0),
                if layer.visible { "visible" } else { "hidden" }
            ));
        }
        stack_xml.push_str("</stack></image>");
        archive
            .start_file("stack.xml", FileOptions::default())
            .map_err(|error| error.to_string())?;
        archive
            .write_all(stack_xml.as_bytes())
            .map_err(|error| error.to_string())?;
        archive
            .start_file("mergedimage.png", FileOptions::default())
            .map_err(|error| error.to_string())?;
        archive
            .write_all(merged_image)
            .map_err(|error| error.to_string())?;
        archive.finish().map_err(|error| error.to_string())?;
    }
    Ok(output.into_inner())
}

fn encode_pdf(width: usize, height: usize, pixels: Vec<u8>) -> Result<Vec<u8>, String> {
    let rgba_image = image::RgbaImage::from_raw(width as u32, height as u32, pixels)
        .ok_or_else(|| "Impossibile preparare l'immagine per il PDF.".to_string())?;
    let (page_width, page_height) = if width >= height {
        (297.0, 210.0)
    } else {
        (210.0, 297.0)
    };
    let margin_mm = 10.0;
    let available_width_pt = (page_width - margin_mm * 2.0) * 72.0 / 25.4;
    let available_height_pt = (page_height - margin_mm * 2.0) * 72.0 / 25.4;
    let scale = (available_width_pt / width as f32).min(available_height_pt / height as f32);
    let image_width_pt = width as f32 * scale;
    let image_height_pt = height as f32 * scale;
    let translate_x_mm = (page_width - image_width_pt * 25.4 / 72.0) / 2.0;
    let translate_y_mm = (page_height - image_height_pt * 25.4 / 72.0) / 2.0;

    let (document, page, layer) =
        PdfDocument::new("PixelCraft", Mm(page_width), Mm(page_height), "Immagine");
    Image::from_dynamic_image(&image::DynamicImage::ImageRgba8(rgba_image)).add_to_layer(
        document.get_page(page).get_layer(layer),
        ImageTransform {
            translate_x: Some(Mm(translate_x_mm)),
            translate_y: Some(Mm(translate_y_mm)),
            scale_x: Some(scale),
            scale_y: Some(scale),
            dpi: Some(72.0),
            ..Default::default()
        },
    );
    document.save_to_bytes().map_err(|error| error.to_string())
}

impl PaintApp {
    pub fn render_for_export(&self) -> Result<RenderedImage, String> {
        let image = self.composite();
        RenderedImage::new(self.width, self.height, image.as_raw().to_vec())
    }

    fn save_rendered(&mut self, path: &str, options: ExportOptions, notices: &mut Vec<String>) {
        let rendered = match self.render_for_export() {
            Ok(rendered) => rendered,
            Err(error) => {
                eprintln!("Errore durante la preparazione dell'immagine: {}", error);
                notices.push(format!("Errore salvataggio: {}", error));
                return;
            }
        };
        if let Err(error) = ExportManager::new().export_rendered(path, &rendered, options) {
            eprintln!("Errore durante il salvataggio: {}", error);
            notices.push(format!("Errore salvataggio: {}", error));
        } else {
            self.current_filepath = Some(path.to_string());
            notices.push(format!("Salvato in {}", path));
            println!("Immagine salvata in: {}", path);
        }
    }

    pub fn save_to_openraster(&mut self, path: &str, notices: &mut Vec<String>) {
        let mut output_path = std::path::PathBuf::from(path);
        if !output_path
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .is_some_and(|extension| extension.eq_ignore_ascii_case("ora"))
        {
            output_path.set_extension("ora");
        }

        let merged_pixels = openraster_merged_pixels(self.width, self.height, &self.layers);
        let merged_image = match encode_png(self.width, self.height, &merged_pixels) {
            Ok(image) => image,
            Err(error) => {
                let message = format!("Errore durante la preparazione OpenRaster: {error}");
                eprintln!("{message}");
                notices.push(message);
                return;
            }
        };
        let archive = match encode_openraster(self.width, self.height, &self.layers, &merged_image)
        {
            Ok(archive) => archive,
            Err(error) => {
                let message = format!("Errore durante la creazione OpenRaster: {error}");
                eprintln!("{message}");
                notices.push(message);
                return;
            }
        };

        match std::fs::write(&output_path, archive) {
            Ok(()) => {
                self.current_filepath = Some(output_path.to_string_lossy().into_owned());
                notices.push(format!(
                    "Documento OpenRaster salvato in {}",
                    output_path.display()
                ));
            }
            Err(error) => {
                let message = format!("Impossibile salvare il documento OpenRaster: {error}");
                eprintln!("{message}");
                notices.push(message);
            }
        }
    }

    pub fn save_to_pdf(&mut self, path: &str, notices: &mut Vec<String>) {
        let mut output_path = std::path::PathBuf::from(path);
        if !output_path
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
        {
            output_path.set_extension("pdf");
        }
        let path = output_path.to_string_lossy();
        let composite = self.composite();
        let pdf_bytes = match encode_pdf(self.width, self.height, composite.as_raw().to_vec()) {
            Ok(bytes) => bytes,
            Err(error) => {
                let message = format!("Errore durante la preparazione del PDF: {error}");
                eprintln!("{message}");
                notices.push(message);
                return;
            }
        };

        match std::fs::write(&output_path, pdf_bytes) {
            Ok(()) => notices.push(format!("PDF salvato in {path}")),
            Err(error) => {
                let message = format!("Impossibile creare il PDF: {error}");
                eprintln!("{message}");
                notices.push(message);
            }
        }
    }

    pub fn load_from_openraster(&mut self, path: &str, notices: &mut Vec<String>) {
        let result = std::fs::File::open(path)
            .map_err(|error| error.to_string())
            .and_then(decode_openraster);
        match result {
            Ok((width, height, layers)) => {
                self.push_undo_snapshot();
                self.width = width;
                self.height = height;
                self.active_layer_idx = layers.len() - 1;
                self.layers = layers;
                self.current_filepath = Some(path.to_string());
                self.selection = None;
                self.moving_selection = None;
                self.mark_document_changed();
                notices.push(format!(
                    "Documento OpenRaster aperto: {} ({}x{} px)",
                    path, width, height
                ));
            }
            Err(error) => {
                let message = format!("Errore durante l'apertura OpenRaster: {error}");
                eprintln!("{message}");
                notices.push(message);
            }
        }
    }

    pub fn load_from_png(&mut self, path: &str, notices: &mut Vec<String>) {
        match image::open(path) {
            Ok(img) => {
                self.push_undo_snapshot();
                let rgba = img.to_rgba8();
                let (w, h) = rgba.dimensions();
                let w = w as usize;
                let h = h as usize;

                self.width = w;
                self.height = h;

                let mut bg_layer = Layer::new("Sfondo", w, h);
                bg_layer.pixels.fill(Color32::WHITE);

                let mut img_layer = Layer::new(&format!("Immagine ({})", path), w, h);
                for (i, pixel) in rgba.pixels().enumerate() {
                    if i < img_layer.pixels.len() {
                        img_layer.pixels[i] =
                            Color32::from_rgba_unmultiplied(pixel[0], pixel[1], pixel[2], pixel[3]);
                    }
                }

                self.layers = vec![bg_layer, img_layer];
                self.active_layer_idx = 1;
                self.current_filepath = Some(path.to_string());
                self.selection = None;
                self.mark_document_changed();
                notices.push(format!("Immagine aperta: {} ({}x{} px)", path, w, h));
                println!("Immagine caricata: {} ({}x{} px)", path, w, h);
            }
            Err(e) => {
                let err_msg = format!("Errore durante il caricamento: {}", e);
                eprintln!("{}", err_msg);
                notices.push(err_msg);
            }
        }
    }

    pub fn save_to_png(&mut self, path: &str, notices: &mut Vec<String>) {
        self.save_rendered(path, ExportOptions::png(), notices);
    }

    pub fn save_to_jpeg(&mut self, path: &str, notices: &mut Vec<String>) {
        self.save_rendered(path, ExportOptions::jpeg(), notices);
    }

    pub fn save_to_bmp(&mut self, path: &str, notices: &mut Vec<String>) {
        self.save_rendered(path, ExportOptions::bmp(), notices);
    }

    pub fn save_to_webp(&mut self, path: &str, notices: &mut Vec<String>) {
        self.save_rendered(path, ExportOptions::webp(), notices);
    }

    pub fn copy_to_file(&mut self, path: &str, notices: &mut Vec<String>) {
        if let Some((x0, y0, x1, y1)) = self.get_selection_bounds() {
            let w = (x1 - x0) as u32;
            let h = (y1 - y0) as u32;
            let mut buffer: Vec<u8> = Vec::with_capacity((w * h * 4) as usize);
            if let Some(layer) = self.layers.get(self.active_layer_idx) {
                for y in y0..y1 {
                    for x in x0..x1 {
                        let p = if self
                            .selection
                            .as_ref()
                            .is_some_and(|selection| selection.contains_pixel(x, y))
                        {
                            layer.pixels[y * self.width + x]
                        } else {
                            Color32::TRANSPARENT
                        };
                        buffer.extend_from_slice(&[p.r(), p.g(), p.b(), p.a()]);
                    }
                }
                match ExportManager::new().export_rgba_to_file(
                    path,
                    w as usize,
                    h as usize,
                    &buffer,
                    ExportOptions::png(),
                ) {
                    Ok(()) => notices.push("Selezione salvata su file".to_string()),
                    Err(error) => {
                        let message = format!("Impossibile salvare la selezione: {error}");
                        eprintln!("{message}");
                        notices.push(message);
                    }
                }
            }
        }
    }

    pub fn paste_from_file(&mut self, path: &str, notices: &mut Vec<String>) {
        match image::open(path) {
            Ok(img) => {
                let rgba = img.to_rgba8();
                let (w, h) = rgba.dimensions();
                let pixels: Vec<Color32> = rgba
                    .pixels()
                    .map(|p| Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
                    .collect();
                self.clipboard = Some(ClipboardData {
                    width: w as usize,
                    height: h as usize,
                    pixels,
                });
                self.paste(notices);
            }
            Err(error) => {
                let message = format!("Errore durante l'apertura dell'immagine: {error}");
                eprintln!("{message}");
                notices.push(message);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        decode_openraster, encode_openraster, encode_pdf, encode_png, openraster_merged_pixels,
    };
    use crate::model::Layer;
    use egui::Color32;
    use std::io::{Cursor, Read};

    #[test]
    fn pdf_export_encodes_a_valid_pdf_document() {
        let pixels = [255, 0, 0, 255].repeat(4);
        let pdf = encode_pdf(2, 2, pixels).expect("PDF encoding should succeed");

        assert!(pdf.starts_with(b"%PDF-"));
        assert!(pdf.windows(9).any(|window| window == b"/MediaBox"));
    }

    #[test]
    fn pdf_export_rejects_mismatched_pixel_data() {
        assert!(encode_pdf(2, 2, vec![0; 4]).is_err());
    }

    #[test]
    fn openraster_export_keeps_layers_in_top_to_bottom_order() {
        let mut bottom = Layer::new("Sfondo", 1, 1);
        bottom.pixels[0] = Color32::from_rgb(12, 34, 56);
        let mut top = Layer::new("Livello & \"alto\"", 1, 1);
        top.pixels[0] = Color32::from_rgba_unmultiplied(78, 90, 123, 200);
        top.opacity = 0.5;
        top.visible = false;
        let merged_image = encode_png(1, 1, &[top.pixels[0]]).unwrap();
        let bytes = encode_openraster(1, 1, &[bottom, top], &merged_image).unwrap();
        assert_eq!(&bytes[..4], b"PK\x03\x04");
        assert_eq!(&bytes[30..38], b"mimetype");
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();

        let mut mimetype = String::new();
        archive
            .by_name("mimetype")
            .unwrap()
            .read_to_string(&mut mimetype)
            .unwrap();
        assert_eq!(mimetype, "image/openraster");

        let mut stack = String::new();
        archive
            .by_name("stack.xml")
            .unwrap()
            .read_to_string(&mut stack)
            .unwrap();
        assert!(
            stack.find("Livello &amp; &quot;alto&quot;").unwrap()
                < stack.find("name=\"Sfondo\"").unwrap()
        );
        assert!(stack.contains("opacity=\"0.5\" visibility=\"hidden\""));

        let top_layer = archive.by_name("data/layer1.png").unwrap();
        let decoded = image::load_from_memory(&{
            let mut png = Vec::new();
            let mut top_layer = top_layer;
            top_layer.read_to_end(&mut png).unwrap();
            png
        })
        .unwrap()
        .to_rgba8();
        assert_eq!(decoded.get_pixel(0, 0).0, [78, 90, 123, 200]);
    }

    #[test]
    fn openraster_merged_image_keeps_transparency_and_omits_hidden_layers() {
        let mut visible = Layer::new("Visibile", 1, 1);
        visible.pixels[0] = Color32::from_rgba_unmultiplied(20, 40, 60, 128);
        let mut hidden = Layer::new("Nascosto", 1, 1);
        hidden.pixels[0] = Color32::WHITE;
        hidden.visible = false;
        let expected = visible.pixels[0].to_srgba_unmultiplied();

        let merged = openraster_merged_pixels(1, 1, &[visible, hidden]);

        assert_eq!(merged[0].to_srgba_unmultiplied(), expected);
    }

    #[test]
    fn openraster_import_restores_layer_order_properties_and_pixels() {
        let mut bottom = Layer::new("Sfondo", 2, 1);
        bottom.pixels[0] = Color32::from_rgb(12, 34, 56);
        let mut top = Layer::new("Livello & alto", 2, 1);
        top.pixels[1] = Color32::from_rgba_unmultiplied(78, 90, 123, 200);
        top.opacity = 0.5;
        top.visible = false;
        let merged = encode_png(2, 1, &[Color32::TRANSPARENT; 2]).unwrap();
        let archive = encode_openraster(2, 1, &[bottom, top], &merged).unwrap();

        let (width, height, layers) = decode_openraster(Cursor::new(archive)).unwrap();

        assert_eq!((width, height), (2, 1));
        assert_eq!(layers.len(), 2);
        assert_eq!(layers[0].name, "Sfondo");
        assert_eq!(layers[0].pixels[0], Color32::from_rgb(12, 34, 56));
        assert_eq!(layers[1].name, "Livello & alto");
        assert_eq!(
            layers[1].pixels[1].to_srgba_unmultiplied(),
            [78, 90, 123, 200]
        );
        assert_eq!(layers[1].opacity, 0.5);
        assert!(!layers[1].visible);
    }

    #[test]
    fn openraster_import_rejects_nested_layer_groups() {
        let xml = r#"<image w="1" h="1"><stack><stack><layer name="a" src="a.png"/></stack></stack></image>"#;

        assert!(super::parse_openraster_stack(xml).is_err());
    }
}
