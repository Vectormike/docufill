use std::collections::{HashMap, HashSet};

use pdfium_render::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{AppError, AppResult};

use super::PdfEngine;

const MAX_FONT_SIZE: f32 = 12.0;
const MIN_FONT_SIZE: f32 = 7.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldPlacement {
    pub key: String,
    pub kind: String,
    pub page_number: u16,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub font_size: Option<f32>,
    pub alignment: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct SignaturePlacement {
    pub page_number: u16,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub png_bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FittedText {
    pub font_size: f32,
    pub lines: Vec<String>,
}

pub fn render_answers(
    engine: &PdfEngine,
    bytes: Vec<u8>,
    placements: &[FieldPlacement],
    signatures: &[SignaturePlacement],
) -> AppResult<Vec<u8>> {
    let pdfium = engine.bind()?;
    let mut document = pdfium
        .load_pdf_from_byte_vec(bytes, None)
        .map_err(|_| AppError::Validation("PDF is encrypted or malformed".to_owned()))?;
    let unicode_font = engine.unicode_font()?;
    let font_token = unicode_font
        .as_ref()
        .map(|font_data| {
            document
                .fonts_mut()
                .load_true_type_from_bytes(font_data, true)
                .map_err(AppError::internal)
        })
        .transpose()?;
    let font = font_token.unwrap_or_else(|| document.fonts_mut().helvetica());
    let mut filled_keys = HashSet::new();

    for page_index in 0..document.pages().len() {
        let page_number = (page_index + 1) as u16;
        let page_placements: Vec<_> = placements
            .iter()
            .filter(|placement| placement.page_number == page_number)
            .collect();
        let page_signatures: Vec<_> = signatures
            .iter()
            .filter(|signature| signature.page_number == page_number)
            .collect();
        if page_placements.is_empty() && page_signatures.is_empty() {
            continue;
        }

        let by_key: HashMap<&str, &FieldPlacement> = page_placements
            .iter()
            .map(|placement| (placement.key.as_str(), *placement))
            .collect();
        let mut page = document
            .pages_mut()
            .get(page_index)
            .map_err(AppError::internal)?;
        let page_width = page.width().value;
        let page_height = page.height().value;
        if page_placements.iter().any(|placement| {
            !inside_page(
                placement.x,
                placement.y,
                placement.width,
                placement.height,
                page_width,
                page_height,
            )
        }) || page_signatures.iter().any(|signature| {
            !inside_page(
                signature.x,
                signature.y,
                signature.width,
                signature.height,
                page_width,
                page_height,
            )
        }) {
            return Err(AppError::Validation(format!(
                "A field on page {page_number} falls outside the page"
            )));
        }

        for mut annotation in page.annotations().iter() {
            let Some(field) = annotation.as_form_field_mut() else {
                continue;
            };
            let Some(name) = field.name() else {
                continue;
            };
            let Some(placement) = by_key.get(name.as_str()) else {
                continue;
            };

            match field.field_type() {
                PdfFormFieldType::Text => {
                    if let Some(text) = field.as_text_field_mut() {
                        text.set_value(&placement.value)
                            .map_err(AppError::internal)?;
                        filled_keys.insert(placement.key.clone());
                    }
                }
                PdfFormFieldType::Checkbox => {
                    if let Some(checkbox) = field.as_checkbox_field_mut() {
                        checkbox
                            .set_checked(parse_checked(&placement.value))
                            .map_err(AppError::internal)?;
                        filled_keys.insert(placement.key.clone());
                    }
                }
                _ => {}
            }
        }

        for placement in page_placements {
            if filled_keys.contains(&placement.key) || placement.kind == "signature" {
                continue;
            }
            if placement.kind == "checkbox" || placement.kind == "radio" {
                if parse_checked(&placement.value) {
                    page.objects_mut()
                        .create_text_object(
                            PdfPoints::new(placement.x),
                            PdfPoints::new(placement.y),
                            "X",
                            font,
                            PdfPoints::new(placement.height.min(12.0)),
                        )
                        .map_err(AppError::internal)?;
                }
                continue;
            }

            let fitted = fit_text_with_max(
                &placement.value,
                placement.width,
                placement.height,
                placement.font_size.unwrap_or(MAX_FONT_SIZE),
            )
            .ok_or_else(|| {
                AppError::Validation(format!(
                    "Answer for '{}' does not fit cleanly",
                    placement.key
                ))
            })?;
            let line_height = fitted.font_size * 1.2;
            for (line_index, line) in fitted.lines.iter().enumerate() {
                let text_width = estimated_width(line, fitted.font_size);
                let x = match placement.alignment.as_str() {
                    "center" => placement.x + (placement.width - text_width) / 2.0,
                    "right" => placement.x + placement.width - text_width - 2.0,
                    _ => placement.x + 2.0,
                }
                .max(placement.x);
                page.objects_mut()
                    .create_text_object(
                        PdfPoints::new(x),
                        PdfPoints::new(
                            placement.y + placement.height
                                - fitted.font_size
                                - line_index as f32 * line_height,
                        ),
                        line,
                        font,
                        PdfPoints::new(fitted.font_size),
                    )
                    .map_err(AppError::internal)?;
            }
        }

        for signature in page_signatures {
            let image =
                image::load_from_memory_with_format(&signature.png_bytes, image::ImageFormat::Png)
                    .map_err(|_| AppError::Validation("Saved signature is invalid".to_owned()))?;
            let image_ratio = image.width() as f32 / image.height().max(1) as f32;
            let box_ratio = signature.width / signature.height.max(1.0);
            let (width, height) = if image_ratio > box_ratio {
                (signature.width, signature.width / image_ratio)
            } else {
                (signature.height * image_ratio, signature.height)
            };
            let x = signature.x + (signature.width - width) / 2.0;
            let y = signature.y + (signature.height - height) / 2.0;
            page.objects_mut()
                .create_image_object(
                    PdfPoints::new(x),
                    PdfPoints::new(y),
                    &image,
                    Some(PdfPoints::new(width)),
                    Some(PdfPoints::new(height)),
                )
                .map_err(AppError::internal)?;
        }

        page.regenerate_content().map_err(AppError::internal)?;
    }

    document.save_to_bytes().map_err(AppError::internal)
}

pub fn fit_text(value: &str, width: f32, height: f32) -> Option<FittedText> {
    fit_text_with_max(value, width, height, MAX_FONT_SIZE)
}

fn fit_text_with_max(
    value: &str,
    width: f32,
    height: f32,
    maximum_font_size: f32,
) -> Option<FittedText> {
    if value.trim().is_empty() || width <= 0.0 || height <= 0.0 {
        return None;
    }

    let mut font_size = maximum_font_size
        .clamp(MIN_FONT_SIZE, MAX_FONT_SIZE)
        .min((height - 4.0).max(MIN_FONT_SIZE));
    while font_size >= MIN_FONT_SIZE {
        let max_characters = ((width - 4.0) / (font_size * 0.52)).floor() as usize;
        let max_lines = ((height - 4.0) / (font_size * 1.2)).floor() as usize;
        if max_characters > 0 && max_lines > 0 {
            let lines = wrap(value, max_characters);
            if lines.len() <= max_lines {
                return Some(FittedText { font_size, lines });
            }
        }
        font_size -= 0.5;
    }
    None
}

fn estimated_width(value: &str, font_size: f32) -> f32 {
    value.chars().count() as f32 * font_size * 0.52
}

fn wrap(value: &str, max_characters: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in value.lines() {
        let mut current = String::new();
        for word in paragraph.split_whitespace() {
            let word_length = word.chars().count();
            if word_length > max_characters {
                if !current.is_empty() {
                    lines.push(std::mem::take(&mut current));
                }
                let characters = word.chars().collect::<Vec<_>>();
                for chunk in characters.chunks(max_characters) {
                    let value = chunk.iter().collect::<String>();
                    if chunk.len() == max_characters {
                        lines.push(value);
                    } else {
                        current = value;
                    }
                }
                continue;
            }
            if !current.is_empty() && current.chars().count() + word_length + 1 > max_characters {
                lines.push(std::mem::take(&mut current));
            }
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        }
        if !current.is_empty() {
            lines.push(current);
        }
    }
    lines
}

fn inside_page(x: f32, y: f32, width: f32, height: f32, page_width: f32, page_height: f32) -> bool {
    x >= 0.0
        && y >= 0.0
        && width > 0.0
        && height > 0.0
        && x + width <= page_width
        && y + height <= page_height
}

fn parse_checked(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "true" | "yes" | "1" | "checked" | "on"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_text_without_dropping_words() {
        let fitted = fit_text("Senior Backend Engineer", 90.0, 40.0).expect("fits");
        assert!(fitted.lines.len() > 1);
        assert_eq!(fitted.lines.join(" "), "Senior Backend Engineer");
    }

    #[test]
    fn refuses_unreadably_small_text() {
        assert!(fit_text(&"word ".repeat(100), 30.0, 10.0).is_none());
    }

    #[test]
    fn breaks_long_identifiers_to_prevent_overflow() {
        let lines = wrap("ABCDEFGHIJKLMNOPQRSTUVWXYZ", 10);
        assert_eq!(lines.join(""), "ABCDEFGHIJKLMNOPQRSTUVWXYZ");
        assert!(lines.iter().all(|line| line.chars().count() <= 10));
    }
}
