use image::{DynamicImage, ImageEncoder, ImageFormat, codecs::jpeg::JpegEncoder};
use pdfium_render::prelude::*;

use crate::{AppError, AppResult};

use super::PdfEngine;

const TARGET_WIDTH: i32 = 1280;
const JPEG_QUALITY: u8 = 70;

#[derive(Debug, Clone)]
pub struct RenderedPage {
    pub page_number: u16,
    pub page_width: f32,
    pub page_height: f32,
    pub jpeg_bytes: Vec<u8>,
}

impl PdfEngine {
    pub fn render_pages(&self, bytes: Vec<u8>) -> AppResult<Vec<RenderedPage>> {
        let pdfium = self.bind()?;
        let document = pdfium
            .load_pdf_from_byte_vec(bytes, None)
            .map_err(|_| AppError::Validation("PDF is encrypted or malformed".to_owned()))?;
        let config = PdfRenderConfig::new()
            .set_target_width(TARGET_WIDTH)
            .set_maximum_width(TARGET_WIDTH);

        let mut pages = Vec::new();
        for (index, page) in document.pages().iter().enumerate() {
            let bitmap = page
                .render_with_config(&config)
                .map_err(AppError::internal)?;
            let image = bitmap.as_image().map_err(AppError::internal)?;
            pages.push(RenderedPage {
                page_number: index as u16 + 1,
                page_width: page.width().value,
                page_height: page.height().value,
                jpeg_bytes: encode_jpeg(image)?,
            });
        }
        Ok(pages)
    }
}

fn encode_jpeg(image: DynamicImage) -> AppResult<Vec<u8>> {
    let rgb = image.to_rgb8();
    let mut bytes = Vec::new();
    JpegEncoder::new_with_quality(&mut bytes, JPEG_QUALITY)
        .write_image(
            rgb.as_raw(),
            rgb.width(),
            rgb.height(),
            image::ExtendedColorType::Rgb8,
        )
        .map_err(AppError::internal)?;
    if bytes.is_empty() {
        let mut fallback = std::io::Cursor::new(Vec::new());
        DynamicImage::ImageRgb8(rgb)
            .write_to(&mut fallback, ImageFormat::Jpeg)
            .map_err(AppError::internal)?;
        return Ok(fallback.into_inner());
    }
    Ok(bytes)
}
