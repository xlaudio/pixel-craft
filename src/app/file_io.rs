//! # file_io — Importazione ed esportazione
//!
//! Gestisce il caricamento e il salvataggio delle immagini e le operazioni
//! di copia/incolla tramite file. Le modifiche al documento passano per
//! lo stato condiviso di [`PaintApp`].

use egui::Color32;
use printpdf::{Image, ImageTransform, Mm, PdfDocument};

use crate::model::{ClipboardData, Layer, PaintApp};

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
        let img_data = self.composite();
        if let Err(e) = image::save_buffer(
            path,
            img_data.as_raw(),
            self.width as u32,
            self.height as u32,
            image::ColorType::Rgba8,
        ) {
            eprintln!("Errore durante il salvataggio: {}", e);
            notices.push(format!("Errore salvataggio: {}", e));
        } else {
            self.current_filepath = Some(path.to_string());
            notices.push(format!("Salvato in {}", path));
            println!("Immagine salvata in: {}", path);
        }
    }

    pub fn copy_to_file(&mut self, path: &str, notices: &mut Vec<String>) {
        if let Some((x0, y0, x1, y1)) = self.get_selection_bounds() {
            let w = (x1 - x0) as u32;
            let h = (y1 - y0) as u32;
            let mut buffer: Vec<u8> = Vec::with_capacity((w * h * 4) as usize);
            if let Some(layer) = self.layers.get(self.active_layer_idx) {
                for y in y0..y1 {
                    for x in x0..x1 {
                        let p = layer.pixels[y * self.width + x];
                        buffer.extend_from_slice(&[p.r(), p.g(), p.b(), p.a()]);
                    }
                }
                match image::save_buffer(path, &buffer, w, h, image::ColorType::Rgba8) {
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
    use super::encode_pdf;

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
}
