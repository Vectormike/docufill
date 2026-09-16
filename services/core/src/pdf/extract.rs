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

            let page_text = page.text().map_err(AppError::internal)?;
            let (page_segments, inferred_fields) =
                super::flat::extract_flat_page(&page_text, page_number, page.width().value)?;
            text_segments.extend(page_segments);
            if fields.len() == page_fields_before {
                fields.extend(inferred_fields);
            }
        }

        Ok(ExtractedDocument {
            page_count: page_count as u16,
            fields,
            text_segments,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binding_twice_reuses_the_loaded_library() {
        let Ok(library_path) = std::env::var("PDFIUM_LIB_PATH") else {
            return;
        };
        let engine = PdfEngine::new(Some(PathBuf::from(library_path)), None, 1024, 1);

        assert!(engine.bind().is_ok());
        assert!(engine.bind().is_ok());
    }
}

pub(super) fn infer_kind(label: &str) -> &'static str {
    let normalized = label.to_ascii_lowercase();
    if normalized.contains("date") || normalized.contains("born") {
        "date"
    } else if normalized.contains("email") {
        "email"
    } else if normalized.contains("phone") || normalized.contains("mobile") {
        "phone"
    } else if normalized.contains("address") {
        "address"
    } else if normalized.contains("signature") {
        "signature"
    } else {
        "text"
    }
}
