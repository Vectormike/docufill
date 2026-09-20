use std::collections::{HashMap, HashSet};

use pdfium_render::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{AppError, AppResult};

use super::PdfEngine;

const MAX_FONT_SIZE: f32 = 9.5;
const MIN_FONT_SIZE: f32 = 7.0;
const CHAR_WIDTH: f32 = 0.66;
const BOX_INSET: f32 = 2.5;
const REFERENCE_PAGE_WIDTH: f32 = 595.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldPlacement {
    pub key: String,
    pub label: String,
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
        let scale = page_scale(page_width);
        let inset = BOX_INSET * scale;
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
                if is_ticked(&placement.label, &placement.value) {
                    let stroke = PdfColor::new(28, 28, 28, 255);
                    let weight = PdfPoints::new(
                        (placement.width.min(placement.height) * 0.18).clamp(1.15, 1.85),
                    );
                    for (x1, y1, x2, y2) in
                        tick_strokes(placement.x, placement.y, placement.width, placement.height)
                    {
                        page.objects_mut()
                            .create_path_object_line(
                                PdfPoints::new(x1),
                                PdfPoints::new(y1),
                                PdfPoints::new(x2),
                                PdfPoints::new(y2),
                                stroke,
                                weight,
                            )
                            .map_err(AppError::internal)?;
                    }
                }
                continue;
            }

            let fitted = fit_text_with_max(
                &placement.value,
                placement.width,
                placement.height,
                placement.font_size.unwrap_or(MAX_FONT_SIZE) * scale,
                scale,
            )
            .ok_or_else(|| {
                AppError::Validation(format!(
                    "Answer for '{}' does not fit cleanly",
                    placement.key
                ))
            })?;
            let line_height = fitted.font_size * 1.12;
            let block_height = fitted.lines.len() as f32 * line_height;
            let first_baseline = placement.y
                + (placement.height - block_height).max(0.0) / 2.0
                + fitted.font_size * 0.18;
            let max_width = (placement.width - inset * 2.0).max(4.0 * scale);
            for (line_index, line) in fitted.lines.iter().enumerate() {
                let text_width = estimated_width(line, fitted.font_size).min(max_width);
                let x = match placement.alignment.as_str() {
                    "center" => placement.x + (placement.width - text_width) / 2.0,
                    "right" => placement.x + placement.width - text_width - inset,
                    _ => placement.x + inset,
                }
                .clamp(placement.x, placement.x + placement.width - inset);
                let baseline = first_baseline - line_index as f32 * line_height;
                let mut object = page
                    .objects_mut()
                    .create_text_object(
                        PdfPoints::new(x),
                        PdfPoints::new(baseline),
                        line,
                        font,
                        PdfPoints::new(fitted.font_size),
                    )
                    .map_err(AppError::internal)?;
                if let Ok(drawn) = object.width()
                    && drawn.value > max_width
                    && drawn.value > 0.0
                {
                    let factor = max_width / drawn.value;
                    let (shift_x, shift_y) = recentre_after_scale(x, baseline, factor);
                    object.scale(factor, factor).map_err(AppError::internal)?;
                    object
                        .translate(PdfPoints::new(shift_x), PdfPoints::new(shift_y))
                        .map_err(AppError::internal)?;
                }
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

/// The point sizes here are tuned against A4. Scanned pages routinely carry a media
/// box several times that size, and on those a fixed 9.5pt answer comes out too small
/// to read, so every measurement grows with the page.
fn page_scale(page_width: f32) -> f32 {
    (page_width / REFERENCE_PAGE_WIDTH).max(1.0)
}

pub fn fit_text(value: &str, width: f32, height: f32) -> Option<FittedText> {
    fit_text_with_max(value, width, height, MAX_FONT_SIZE, 1.0)
}

fn fit_text_with_max(
    value: &str,
    width: f32,
    height: f32,
    maximum_font_size: f32,
    scale: f32,
) -> Option<FittedText> {
    if value.trim().is_empty() || width <= 0.0 || height <= 0.0 {
        return None;
    }

    let smallest = MIN_FONT_SIZE * scale;
    let padding = 4.0 * scale;
    let mut font_size = maximum_font_size
        .clamp(smallest, MAX_FONT_SIZE * scale)
        .min((height - padding).max(smallest));
    while font_size >= smallest {
        let max_characters = ((width - padding) / (font_size * CHAR_WIDTH)).floor() as usize;
        let max_lines = ((height - padding) / (font_size * 1.2)).floor() as usize;
        if max_characters > 0 && max_lines > 0 {
            let lines = wrap(value, max_characters);
            if lines.len() <= max_lines {
                return Some(FittedText { font_size, lines });
            }
        }
        font_size -= 0.5 * scale;
    }
    if !value.contains(char::is_whitespace) {
        return Some(FittedText {
            font_size: smallest,
            lines: vec![value.trim().to_owned()],
        });
    }
    None
}

/// Pdfium scales a page object around the page origin rather than its own anchor,
/// so shrinking overlong text also drags it towards the bottom-left corner. These
/// deltas put the baseline anchor back where the caller asked for it.
fn recentre_after_scale(anchor_x: f32, baseline: f32, factor: f32) -> (f32, f32) {
    (anchor_x * (1.0 - factor), baseline * (1.0 - factor))
}

fn estimated_width(value: &str, font_size: f32) -> f32 {
    value.chars().count() as f32 * font_size * CHAR_WIDTH
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
        "true" | "yes" | "1" | "checked" | "on" | "selected" | "tick" | "ticked"
    )
}

fn is_ticked(label: &str, value: &str) -> bool {
    if parse_checked(value) {
        return true;
    }
    let value = value
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect::<String>();
    let label = label
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect::<String>();
    !value.is_empty() && value == label
}

fn tick_strokes(x: f32, y: f32, width: f32, height: f32) -> [(f32, f32, f32, f32); 2] {
    let pad = width.min(height) * 0.16;
    let left = x + pad;
    let right = x + width - pad * 0.55;
    let notch_x = left + (right - left) * 0.34;
    let notch_y = y + pad * 0.85;
    let stem_y = y + height * 0.46;
    let top = y + height - pad * 0.45;
    [
        (left, stem_y, notch_x, notch_y),
        (notch_x, notch_y, right, top),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_answers_stay_on_one_line_inside_a_cell() {
        let fitted = fit_text("Victor", 57.0, 13.0).expect("fits");
        assert_eq!(fitted.lines, ["Victor"]);
        assert!(fitted.font_size <= 9.5);
        assert!(estimated_width("Victor", fitted.font_size) <= 54.0);
    }

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
    fn shrinks_a_long_identifier_onto_one_line() {
        let fitted = fit_text("+2348086249721", 56.0, 13.0).expect("fits");
        assert_eq!(fitted.lines.len(), 1);
        assert_eq!(fitted.lines[0], "+2348086249721");
    }

    #[test]
    fn breaks_long_identifiers_to_prevent_overflow() {
        let lines = wrap("ABCDEFGHIJKLMNOPQRSTUVWXYZ", 10);
        assert_eq!(lines.join(""), "ABCDEFGHIJKLMNOPQRSTUVWXYZ");
        assert!(lines.iter().all(|line| line.chars().count() <= 10));
    }

    #[test]
    fn ticks_selected_checkboxes_and_matching_labels() {
        assert!(is_ticked("Male", "yes"));
        assert!(is_ticked("Phone Number Update", "Phone Number Update"));
        assert!(!is_ticked("Male", "no"));
        assert!(!is_ticked("Female", "Male"));
    }

    #[test]
    fn an_oversized_scan_gets_proportionally_larger_type() {
        // A 2032pt wide scan is roughly three and a half A4 pages across.
        let scale = page_scale(2032.0);
        let fitted =
            fit_text_with_max("Victor Jonah", 914.0, 115.0, 9.5 * scale, scale).expect("fits");

        assert!((scale - 3.415).abs() < 0.01);
        assert!(fitted.lines.len() == 1);
        // Same share of the page as 9.5pt on A4, rather than a third of it.
        assert!((fitted.font_size / 2032.0 - 9.5 / 595.0).abs() < 0.001);
    }

    #[test]
    fn ordinary_pages_keep_their_existing_type_size() {
        assert_eq!(page_scale(594.96), 1.0);
        assert_eq!(page_scale(612.0), 612.0 / 595.0);
    }

    #[test]
    fn shrinking_overlong_text_keeps_it_on_its_own_baseline() {
        let (anchor_x, baseline, factor) = (396.8, 348.48, 0.9442);
        let (shift_x, shift_y) = recentre_after_scale(anchor_x, baseline, factor);

        assert!((anchor_x * factor + shift_x - anchor_x).abs() < 0.01);
        assert!((baseline * factor + shift_y - baseline).abs() < 0.01);
    }

    #[test]
    fn text_that_already_fits_is_never_moved() {
        assert_eq!(recentre_after_scale(396.8, 348.48, 1.0), (0.0, 0.0));
    }

    #[test]
    fn tick_stays_inside_the_printed_box() {
        let strokes = tick_strokes(100.0, 200.0, 10.0, 10.0);
        for (x1, y1, x2, y2) in strokes {
            for value in [x1, x2] {
                assert!((100.0..=110.0).contains(&value));
            }
            for value in [y1, y2] {
                assert!((200.0..=210.0).contains(&value));
            }
        }
    }
}
