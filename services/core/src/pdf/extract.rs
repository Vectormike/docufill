use std::path::{Path, PathBuf};

use pdfium_render::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{AppError, AppResult};

#[derive(Clone)]
pub struct PdfEngine {
    library_path: Option<PathBuf>,
    unicode_font_path: Option<PathBuf>,
    max_bytes: usize,
    max_pages: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedDocument {
    pub page_count: u16,
    pub fields: Vec<ExtractedField>,
    pub text_segments: Vec<TextSegment>,
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedField {
    pub key: String,
    pub label: String,
    pub kind: String,
    pub page_number: u16,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextSegment {
    pub text: String,
    pub page_number: u16,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl PdfEngine {
    pub fn new(
        library_path: Option<PathBuf>,
        unicode_font_path: Option<PathBuf>,
        max_bytes: usize,
        max_pages: u16,
    ) -> Self {
        Self {
            library_path,
            unicode_font_path,
            max_bytes,
            max_pages,
        }
    }

    pub fn extract(&self, bytes: Vec<u8>) -> AppResult<ExtractedDocument> {
        if bytes.len() > self.max_bytes {
            return Err(AppError::Validation(
                "PDF exceeds the 25 MB limit".to_owned(),
            ));
        }
        if !bytes.starts_with(b"%PDF-") {
            return Err(AppError::Validation(
                "Only valid digital PDF files are supported".to_owned(),
            ));
        }

        let pdfium = self.bind()?;
        let document = pdfium
            .load_pdf_from_byte_vec(bytes, None)
            .map_err(|_| AppError::Validation("PDF is encrypted or malformed".to_owned()))?;
        let page_count = document.pages().len();
        if page_count == 0 || page_count > i32::from(self.max_pages) {
            return Err(AppError::Validation(format!(
                "PDF must contain between 1 and {} pages",
                self.max_pages
            )));
        }

        let mut fields = Vec::new();
        let mut text_segments = Vec::new();
        for (page_index, page) in document.pages().iter().enumerate() {
            let page_number = page_index as u16 + 1;
            let page_fields_before = fields.len();
            for annotation in page.annotations().iter() {
                let Some(field) = annotation.as_form_field() else {
                    continue;
                };
                let bounds = annotation.bounds().map_err(AppError::internal)?;
                let label = field
                    .name()
                    .filter(|name| !name.trim().is_empty())
                    .unwrap_or_else(|| format!("Field {}", fields.len() + 1));
                fields.push(ExtractedField {
                    key: label.clone(),
                    label,
                    kind: field_kind(field.field_type()).to_owned(),
                    page_number,
                    x: bounds.left().value,
                    y: bounds.bottom().value,
                    width: bounds.width().value,
                    height: bounds.height().value,
                });
            }

            let walls = grid_walls(&page);
            let rails = grid_rails(&page);
            let page_text = page.text().map_err(AppError::internal)?;
            let (page_segments, mut inferred_fields) =
                super::flat::extract_flat_page(&page_text, page_number, page.width().value)?;
            super::flat::snap_fields_to_grid(&mut inferred_fields, &walls);
            super::flat::snap_fields_to_cells(&mut inferred_fields, &walls, &rails);
            if fields.len() == page_fields_before {
                fields.extend(inferred_fields);
            }
            if !fields
                .iter()
                .any(|field| field.page_number == page_number && field.kind == "signature")
            {
                fields.extend(super::signing::signing_fields(
                    &page,
                    page_number,
                    &page_segments,
                ));
            }
            text_segments.extend(page_segments);
        }

        Ok(ExtractedDocument {
            page_count: page_count as u16,
            fields,
            text_segments,
            title: None,
        })
    }

    pub(super) fn bind(&self) -> AppResult<Pdfium> {
        let bindings = match &self.library_path {
            Some(path) => Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(
                Path::new(path),
            )),
            None => Pdfium::bind_to_system_library(),
        };
        match bindings {
            Ok(bindings) => Ok(Pdfium::new(bindings)),
            // PDFium loads its bindings once per process, so later operations reuse them.
            Err(PdfiumError::PdfiumLibraryBindingsAlreadyInitialized) => Ok(Pdfium::default()),
            Err(error) => Err(AppError::configuration(format!(
                "PDFium is unavailable; set PDFIUM_LIB_PATH ({error})"
            ))),
        }
    }

    pub(super) fn unicode_font(&self) -> AppResult<Option<Vec<u8>>> {
        self.unicode_font_path
            .as_ref()
            .map(|path| {
                std::fs::read(path).map_err(|error| {
                    AppError::configuration(format!(
                        "UNICODE_FONT_PATH could not be read ({error})"
                    ))
                })
            })
            .transpose()
    }
}

fn grid_walls(page: &PdfPage<'_>) -> Vec<super::flat::GridWall> {
    let mut walls = Vec::new();
    for object in page.objects().iter() {
        if object.object_type() != PdfPageObjectType::Path {
            continue;
        }
        let Ok(bounds) = object.bounds() else {
            continue;
        };
        let width = bounds.width().value;
        let height = bounds.height().value;
        if width < 2.0 && height >= 8.0 {
            walls.push(super::flat::GridWall {
                x: bounds.left().value,
                bottom: bounds.bottom().value,
                top: bounds.bottom().value + height,
            });
        }
    }
    walls
}

fn grid_rails(page: &PdfPage<'_>) -> Vec<super::flat::GridRail> {
    let mut rails = Vec::new();
    for object in page.objects().iter() {
        if object.object_type() != PdfPageObjectType::Path {
            continue;
        }
        let Ok(bounds) = object.bounds() else {
            continue;
        };
        let width = bounds.width().value;
        let height = bounds.height().value;
        if height < 2.0 && width >= 24.0 {
            rails.push(super::flat::GridRail {
                y: bounds.bottom().value,
                left: bounds.left().value,
                right: bounds.left().value + width,
            });
        }
    }
    rails
}

fn field_kind(kind: PdfFormFieldType) -> &'static str {
    match kind {
        PdfFormFieldType::Text => "text",
        PdfFormFieldType::Checkbox => "checkbox",
        PdfFormFieldType::RadioButton => "radio",
        PdfFormFieldType::ComboBox | PdfFormFieldType::ListBox => "choice",
        PdfFormFieldType::Signature => "signature",
        _ => "text",
    }
}

pub fn infer_kind(label: &str) -> &'static str {
    let normalized = label.to_ascii_lowercase();
    // Checked ahead of the date rule because signing lines usually ask for both.
    if normalized.contains("signature")
        || normalized.contains("signatory")
        || (normalized.contains("stamp") && !normalized.contains("required"))
    {
        "signature"
    } else if normalized.contains("date") || normalized.contains("born") {
        "date"
    } else if normalized.contains("email") {
        "email"
    } else if normalized.contains("phone")
        || normalized.contains("mobile")
        || normalized.contains("whatsapp")
        || normalized.contains("whats app")
    {
        "phone"
    } else if normalized.contains("address") {
        "address"
    } else if normalized.contains("tick")
        || normalized.contains("check")
        || normalized.contains("required?")
        || matches!(
            normalized.as_str(),
            "male" | "female" | "sex male" | "sex female"
        )
    {
        "checkbox"
    } else if normalized.contains("account number") || normalized.contains("account no") {
        "number"
    } else {
        "text"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infers_bank_field_kinds() {
        assert_eq!(infer_kind("Account Number"), "number");
        assert_eq!(infer_kind("Mobile / Phone No"), "phone");
        assert_eq!(infer_kind("WhatsApp Number"), "phone");
        assert_eq!(infer_kind("Male"), "checkbox");
        assert_eq!(infer_kind("Specimen Signature"), "signature");
        assert_eq!(infer_kind("Company stamp required?"), "checkbox");
    }

    #[test]
    fn signing_lines_beat_the_date_rule() {
        assert_eq!(infer_kind("Authorized Signatory and Date"), "signature");
        assert_eq!(infer_kind("Signature & Date"), "signature");
        assert_eq!(infer_kind("Date of Birth"), "date");
    }

    #[test]
    fn binding_twice_reuses_the_loaded_library() {
        let Ok(library_path) = std::env::var("PDFIUM_LIB_PATH") else {
            return;
        };
        let engine = PdfEngine::new(Some(PathBuf::from(library_path)), None, 1024, 1);

        assert!(engine.bind().is_ok());
        assert!(engine.bind().is_ok());
    }

    #[test]
    fn labelled_ebanking_form_places_fields_beside_labels() {
        let Ok(library_path) = std::env::var("PDFIUM_LIB_PATH") else {
            return;
        };
        let Ok(bytes) = std::fs::read("/tmp/ebanking-original.pdf") else {
            return;
        };
        let engine = PdfEngine::new(Some(PathBuf::from(library_path)), None, 25_000_000, 10);
        let extracted = engine.extract(bytes).expect("extract");
        let labels: Vec<_> = extracted
            .fields
            .iter()
            .map(|field| field.label.as_str())
            .collect();
        assert!(labels.contains(&"First Name"));
        assert!(labels.contains(&"Business Name"));
        assert!(labels.contains(&"Certification Name"));
        assert!(labels.contains(&"Male"));
        assert!(labels.contains(&"Female"));
        assert!(labels.contains(&"Date of Birth"));
        assert!(labels.contains(&"Phone Number Update"));
        assert!(labels.contains(&"USSD Opt-in"));
        assert!(labels.iter().all(|label| !label.contains("Valid ID")));
        assert!(labels.iter().all(|label| !label.contains('☐')));
        let email = extracted
            .fields
            .iter()
            .find(|field| field.label == "Email Address")
            .expect("email");
        assert_eq!(email.kind, "email");
        assert!(email.y >= 474.5);
        assert!(email.y + email.height <= 491.0);
        let new_phone = extracted
            .fields
            .iter()
            .find(|field| field.label == "New Phone No")
            .expect("new phone");
        assert!(new_phone.width < 70.0);
        assert!(new_phone.x + new_phone.width < 456.0);
        assert!(new_phone.y >= 344.5);
        assert!(new_phone.y + new_phone.height <= 359.0);
        let address = extracted
            .fields
            .iter()
            .find(|field| field.label == "Home/Office Address")
            .expect("address");
        assert!(address.width > 300.0);
        assert!(address.y >= 458.5);
        assert!(address.y + address.height <= 475.5);
        let title = extracted
            .fields
            .iter()
            .find(|field| field.label == "Title")
            .expect("title");
        assert!(title.x > 150.0);
        assert!(title.width < 420.0);
        assert!(title.y >= 570.5);
        assert!(title.y + title.height <= 595.5);
        let first = extracted
            .fields
            .iter()
            .find(|field| field.label == "First Name")
            .expect("first");
        assert!(first.y < 570.0);
        assert!(first.width < 80.0);
        assert!(first.x > 79.6 + 4.0);
    }

    #[test]
    fn ebanking_form_offers_both_drawn_signatory_blocks() {
        let Ok(library_path) = std::env::var("PDFIUM_LIB_PATH") else {
            return;
        };
        let Ok(bytes) = std::fs::read("/tmp/ebanking-original.pdf") else {
            return;
        };
        let engine = PdfEngine::new(Some(PathBuf::from(library_path)), None, 25_000_000, 10);
        let extracted = engine.extract(bytes).expect("extract");
        let signatures = extracted
            .fields
            .iter()
            .filter(|field| field.kind == "signature")
            .collect::<Vec<_>>();

        assert_eq!(
            signatures
                .iter()
                .map(|field| field.label.as_str())
                .collect::<Vec<_>>(),
            ["Signatory 1", "Signatory 2"]
        );
        // Both blocks rest on their printed rule inside Section C, clear of its captions.
        for field in &signatures {
            assert!((140.0..150.0).contains(&field.y), "y was {}", field.y);
            assert!((30.0..45.0).contains(&field.height));
            assert!(field.width > 100.0);
        }
        assert!(signatures[0].x < 200.0);
        assert!(signatures[1].x > 300.0);
    }

    #[test]
    fn renders_ebanking_answers_inside_detected_boxes() {
        let Ok(library_path) = std::env::var("PDFIUM_LIB_PATH") else {
            return;
        };
        let Ok(bytes) = std::fs::read("/tmp/ebanking-original.pdf") else {
            return;
        };
        let engine = PdfEngine::new(Some(PathBuf::from(library_path)), None, 25_000_000, 10);
        let extracted = engine.extract(bytes.clone()).expect("extract");
        let answers = [
            ("Title", "Mr"),
            ("First Name", "Victor"),
            ("Middle Name", "Victor"),
            ("Surname", "Jonah"),
            ("Date of Birth", "1997-05-05"),
            ("Account Number", "8086721"),
            ("Phone Number", "+2348086249721"),
            ("Bank Verification Number", "234324332432"),
            ("Current Country of Residence", "Nigeria"),
            ("Email Address", "victorjonah199@gmail.com"),
            ("Whats App Number", "+2348086249721"),
            ("Home/Office Address", "Dawaki"),
            ("BVN", "324324324243242"),
            ("TIN", "2343234324324343"),
            ("New Phone No", "+2348086249721"),
            ("Certification Name", "Victor Jonah"),
            ("Male", "yes"),
            ("Phone Number Update", "yes"),
        ];
        let placements = extracted
            .fields
            .iter()
            .filter_map(|field| {
                let (_, value) = answers
                    .iter()
                    .find(|(label, _)| *label == field.label.as_str())?;
                Some(crate::pdf::FieldPlacement {
                    key: field.key.clone(),
                    label: field.label.clone(),
                    kind: field.kind.clone(),
                    page_number: field.page_number,
                    x: field.x,
                    y: field.y,
                    width: field.width,
                    height: field.height,
                    font_size: None,
                    alignment: "left".to_owned(),
                    value: (*value).to_owned(),
                })
            })
            .collect::<Vec<_>>();
        assert_eq!(placements.len(), answers.len());
        let signatures = std::fs::read("/tmp/test-signature.png")
            .map(|png_bytes| {
                extracted
                    .fields
                    .iter()
                    .filter(|field| field.kind == "signature")
                    .map(|field| crate::pdf::SignaturePlacement {
                        page_number: field.page_number,
                        x: field.x,
                        y: field.y,
                        width: field.width,
                        height: field.height,
                        png_bytes: png_bytes.clone(),
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let before = glyph_anchors(&engine, bytes.clone());
        let filled =
            crate::pdf::render_answers(&engine, bytes, &placements, &signatures).expect("render");
        std::fs::write("/tmp/ebanking-filled.pdf", &filled).expect("write");

        let printed = before.into_iter().collect::<std::collections::HashSet<_>>();
        let mut written = 0;
        for (character, x, y) in glyph_anchors(&engine, filled) {
            if printed.contains(&(character, x, y)) {
                continue;
            }
            written += 1;
            let box_for_glyph = placements.iter().find(|placement| {
                (placement.x - 1.0..=placement.x + placement.width + 1.0).contains(&(x as f32))
                    && (placement.y - 1.0..=placement.y + placement.height + 1.0)
                        .contains(&(y as f32))
            });
            assert!(
                box_for_glyph.is_some(),
                "answer glyph {character:?} landed at ({x}, {y}), outside every field box"
            );
        }
        assert!(
            written > 40,
            "expected the answers to be written, saw {written} new glyphs"
        );
    }

    #[test]
    fn zenith_answers_land_in_the_character_boxes() {
        let Ok(library_path) = std::env::var("PDFIUM_LIB_PATH") else {
            return;
        };
        let Ok(bytes) = std::fs::read("/tmp/zenith-original.pdf") else {
            return;
        };
        let engine = PdfEngine::new(Some(PathBuf::from(library_path)), None, 25_000_000, 10);
        let pages = engine.render_pages(bytes.clone()).expect("render");
        let mut fields = vec![
            ExtractedField {
                key: "name".to_owned(),
                label: "Name of Account".to_owned(),
                kind: "text".to_owned(),
                page_number: 1,
                x: 40.6,
                y: 2326.3,
                width: 1950.7,
                height: 143.6,
            },
            ExtractedField {
                key: "account".to_owned(),
                label: "Account Number".to_owned(),
                kind: "number".to_owned(),
                page_number: 1,
                x: 40.6,
                y: 2096.6,
                width: 711.2,
                height: 114.9,
            },
        ];
        crate::pdf::snap_scanned_fields(&mut fields, &pages[0]);
        let placements = [("name", "1, Uyo, Nigeria"), ("account", "3243243243433")]
            .into_iter()
            .filter_map(|(key, value)| {
                let field = fields.iter().find(|field| field.key == key)?;
                Some(crate::pdf::FieldPlacement {
                    key: field.key.clone(),
                    label: field.label.clone(),
                    kind: field.kind.clone(),
                    page_number: field.page_number,
                    x: field.x,
                    y: field.y,
                    width: field.width,
                    height: field.height,
                    font_size: None,
                    alignment: "left".to_owned(),
                    value: value.to_owned(),
                })
            })
            .collect::<Vec<_>>();
        let filled = crate::pdf::render_answers(&engine, bytes, &placements, &[]).expect("fill");
        std::fs::write("/tmp/zenith-aligned.pdf", filled).expect("write");
        assert!(placements[1].x > 300.0);
    }

    #[test]
    fn answers_on_an_oversized_scan_are_written_large_enough_to_read() {
        let Ok(library_path) = std::env::var("PDFIUM_LIB_PATH") else {
            return;
        };
        let Ok(bytes) = std::fs::read("/tmp/zenith-original.pdf") else {
            return;
        };
        let engine = PdfEngine::new(Some(PathBuf::from(library_path)), None, 25_000_000, 10);
        // The vision detector sizes these boxes against a 2032pt wide media box.
        let placements = vec![crate::pdf::FieldPlacement {
            key: "name-of-signatory-1".to_owned(),
            label: "Name of Signatory 1".to_owned(),
            kind: "text".to_owned(),
            page_number: 1,
            x: 40.6,
            y: 1953.0,
            width: 914.4,
            height: 114.9,
            font_size: None,
            alignment: "left".to_owned(),
            value: "Victor Jonah".to_owned(),
        }];
        let filled = crate::pdf::render_answers(&engine, bytes, &placements, &[]).expect("render");
        std::fs::write("/tmp/zenith-filled.pdf", &filled).expect("write");

        let written = glyph_heights(&engine, filled, 'V');
        let tallest = written.into_iter().fold(0.0_f32, f32::max);
        // Fixed 9.5pt type put a 7pt capital on a page 2032pt across, far too small
        // to read. Sized against the page it lands near 23pt instead.
        assert!(tallest > 20.0, "capital letter was only {tallest}pt tall");
    }

    fn glyph_heights(engine: &PdfEngine, bytes: Vec<u8>, wanted: char) -> Vec<f32> {
        let pdfium = engine.bind().expect("bind");
        let document = pdfium
            .load_pdf_from_byte_vec(bytes, None)
            .expect("load rendered pdf");
        let page = document.pages().first().expect("page");
        let text = page.text().expect("page text");
        text.chars()
            .iter()
            .filter(|character| character.unicode_char() == Some(wanted))
            .filter_map(|character| character.tight_bounds().ok())
            .map(|bounds| bounds.height().value)
            .collect()
    }

    /// Rounded anchors let us tell the answers we wrote apart from the printed form.
    fn glyph_anchors(engine: &PdfEngine, bytes: Vec<u8>) -> Vec<(char, i32, i32)> {
        let pdfium = engine.bind().expect("bind");
        let document = pdfium
            .load_pdf_from_byte_vec(bytes, None)
            .expect("load rendered pdf");
        let mut anchors = Vec::new();
        for page in document.pages().iter() {
            let text = page.text().expect("page text");
            for character in text.chars().iter() {
                let Some(value) = character
                    .unicode_char()
                    .filter(|value| !value.is_whitespace())
                else {
                    continue;
                };
                let bounds = character.loose_bounds().expect("glyph bounds");
                anchors.push((
                    value,
                    bounds.left().value.round() as i32,
                    bounds.bottom().value.round() as i32,
                ));
            }
        }
        anchors
    }
}
