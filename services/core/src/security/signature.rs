use std::io::Cursor;

use base64::{Engine, engine::general_purpose::STANDARD};
use image::{DynamicImage, ImageFormat};

use crate::{AppError, AppResult};

const MAX_SIGNATURE_INPUT_BYTES: usize = 2 * 1024 * 1024;
const MAX_SIGNATURE_WIDTH: u32 = 1_200;
const MAX_SIGNATURE_HEIGHT: u32 = 400;

pub fn normalize_signature_data_url(data_url: &str) -> AppResult<Vec<u8>> {
    let (prefix, encoded) = data_url
        .split_once(',')
        .ok_or_else(|| AppError::Validation("Signature image is invalid".to_owned()))?;
    let format = match prefix {
        "data:image/png;base64" => ImageFormat::Png,
        "data:image/jpeg;base64" | "data:image/jpg;base64" => ImageFormat::Jpeg,
        _ => {
            return Err(AppError::Validation(
                "Signature must be a PNG or JPEG image".to_owned(),
            ));
        }
    };
    let bytes = STANDARD
        .decode(encoded)
        .map_err(|_| AppError::Validation("Signature image is invalid".to_owned()))?;
    if bytes.is_empty() || bytes.len() > MAX_SIGNATURE_INPUT_BYTES {
        return Err(AppError::Validation(
            "Signature image must be smaller than 2 MB".to_owned(),
        ));
    }

    let decoded = image::load_from_memory_with_format(&bytes, format)
        .map_err(|_| AppError::Validation("Signature image cannot be decoded".to_owned()))?;
    let normalized: DynamicImage = decoded.thumbnail(MAX_SIGNATURE_WIDTH, MAX_SIGNATURE_HEIGHT);
    let mut output = Cursor::new(Vec::new());
    normalized
        .write_to(&mut output, ImageFormat::Png)
        .map_err(AppError::internal)?;
    Ok(output.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_svg_payloads() {
        let result = normalize_signature_data_url("data:image/svg+xml;base64,PHN2Zz4=");
        assert!(matches!(result, Err(AppError::Validation(_))));
    }
}
