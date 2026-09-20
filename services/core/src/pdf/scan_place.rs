use super::{ExtractedField, RenderedPage};

const MIN_BOXES: usize = 8;
const MIN_PITCH: f32 = 12.0;
const MAX_PITCH: f32 = 90.0;

#[derive(Debug, Clone, Copy)]
pub struct WritingBand {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Scanned forms print the label on top of, or to the left of, the writable
/// boxes. The vision pass often reports that whole row. Snap each field onto
/// the nearest row of regularly spaced character boxes so the answer lands
/// in the blank, not on the words.
pub fn snap_scanned_fields(fields: &mut [ExtractedField], page: &RenderedPage) {
    let Ok(bands) = writing_bands(page) else {
        return;
    };
    if bands.is_empty() {
        return;
    }
    snap_to_bands(fields, &bands);
}

fn writing_bands(page: &RenderedPage) -> Result<Vec<WritingBand>, ()> {
    let image = image::load_from_memory(&page.jpeg_bytes).map_err(|_| ())?;
    let gray = image.to_luma8();
    let (width, height) = gray.dimensions();
    if width < 80 || height < 80 {
        return Ok(Vec::new());
    }
    let sx = page.page_width / width as f32;
    let sy = page.page_height / height as f32;

    let mut raw = Vec::new();
    let mut y = 4u32;
    while y + 12 < height {
        let window_h = 14.min(height - y);
        if let Some((left, right, count, _)) = regular_walls(&gray, y, window_h) {
            if count >= MIN_BOXES {
                raw.push((y, y + window_h, left, right));
            }
            y += 8;
            continue;
        }
        y += 2;
    }
    Ok(merge_bands(&raw, sx, sy, page.page_height))
}

fn regular_walls(gray: &image::GrayImage, y: u32, window_h: u32) -> Option<(u32, u32, usize, f32)> {
    let (width, _) = gray.dimensions();
    let mut walls = Vec::new();
    let mut x = 0u32;
    while x < width {
        if col_is_wall(gray, x, y, window_h) {
            let start = x;
            while x < width && col_is_wall(gray, x, y, window_h) {
                x += 1;
            }
            walls.push((start + x) / 2);
        } else {
            x += 1;
        }
    }
    if walls.len() < MIN_BOXES {
        return None;
    }
    let gaps = walls
        .windows(2)
        .map(|pair| pair[1] as f32 - pair[0] as f32)
        .collect::<Vec<_>>();
    let mut sorted = gaps.clone();
    sorted.sort_by(f32::total_cmp);
    let pitch = sorted[sorted.len() / 2];
    if !(MIN_PITCH..=MAX_PITCH).contains(&pitch) {
        return None;
    }
    let regular = gaps
        .iter()
        .filter(|gap| (*gap - pitch).abs() <= pitch * 0.45)
        .count();
    if regular * 100 >= gaps.len() * 65 {
        let left = *walls.first()?;
        let right = *walls.last()?;
        return Some((left, right, walls.len(), pitch));
    }
    let mut best: Option<(u32, u32, usize)> = None;
    let mut start = 0;
    for (index, gap) in gaps
        .iter()
        .copied()
        .chain(std::iter::once(999.0))
        .enumerate()
    {
        if (gap - pitch).abs() > pitch * 0.45 {
            let run = &walls[start..=index.min(walls.len() - 1)];
            if run.len() >= MIN_BOXES {
                let candidate = (run[0], *run.last().unwrap(), run.len());
                if best.is_none_or(|current| candidate.2 > current.2) {
                    best = Some(candidate);
                }
            }
            start = index + 1;
        }
    }
    best.map(|(left, right, count)| (left, right, count, pitch))
}

fn col_is_wall(gray: &image::GrayImage, x: u32, y: u32, window_h: u32) -> bool {
    let mut dark = 0u32;
    for row in y..y + window_h {
        if gray.get_pixel(x, row)[0] < 90 {
            dark += 1;
        }
    }
    dark * 100 >= window_h * 35
}

fn merge_bands(
    raw: &[(u32, u32, u32, u32)],
    sx: f32,
    sy: f32,
    page_height: f32,
) -> Vec<WritingBand> {
    let mut merged: Vec<(u32, u32, u32, u32)> = Vec::new();
    for &(y0, y1, left, right) in raw {
        if let Some(last) = merged.last_mut() {
            let close = y0 <= last.1 + 14;
            let last_w = last.3.saturating_sub(last.2).max(1);
            let this_w = right.saturating_sub(left).max(1);
            let similar_width = last_w.max(this_w) as f32 / last_w.min(this_w) as f32 <= 1.35;
            let overlap_x = left <= last.3 + 40 && right + 40 >= last.2;
            if close && similar_width && overlap_x {
                last.1 = last.1.max(y1);
                last.2 = last.2.min(left);
                last.3 = last.3.max(right);
                continue;
            }
        }
        merged.push((y0, y1, left, right));
    }
    merged
        .into_iter()
        .map(|(y0, y1, left, right)| {
            let top_from_top = y0 as f32 * sy;
            let bottom_from_top = y1 as f32 * sy;
            WritingBand {
                x: left as f32 * sx,
                y: page_height - bottom_from_top,
                width: (right.saturating_sub(left)) as f32 * sx,
                height: (bottom_from_top - top_from_top).max(10.0),
            }
        })
        .filter(|band| band.width >= 80.0 && band.height >= 8.0)
        .collect()
}

fn snap_to_bands(fields: &mut [ExtractedField], bands: &[WritingBand]) {
    let mut used = vec![false; bands.len()];
    let mut order = (0..fields.len()).collect::<Vec<_>>();
    order.sort_by(|&left, &right| {
        fields[right]
            .y
            .total_cmp(&fields[left].y)
            .then(fields[left].x.total_cmp(&fields[right].x))
    });

    for index in order {
        if matches!(
            fields[index].kind.as_str(),
            "checkbox" | "radio" | "signature"
        ) {
            continue;
        }
        let Some(band_index) = best_band(&fields[index], bands, &used) else {
            continue;
        };
        used[band_index] = true;
        let band = bands[band_index];
        fields[index].x = band.x + 2.0;
        fields[index].y = band.y + 2.0;
        fields[index].width = (band.width - 4.0).max(24.0);
        fields[index].height = (band.height - 4.0).max(12.0);
    }
}

fn best_band(field: &ExtractedField, bands: &[WritingBand], used: &[bool]) -> Option<usize> {
    let field_top = field.y + field.height;
    bands
        .iter()
        .enumerate()
        .filter(|(index, band)| {
            !used[*index]
                && band.y + band.height <= field_top + 30.0
                && field.y <= band.y + band.height + 180.0
                && width_compatible(field, band)
        })
        .min_by(|(_, left), (_, right)| {
            let left_score = band_score(field, left);
            let right_score = band_score(field, right);
            left_score.total_cmp(&right_score)
        })
        .map(|(index, _)| index)
}

fn width_compatible(field: &ExtractedField, band: &WritingBand) -> bool {
    let ratio = band.width / field.width.max(1.0);
    if field.width > 900.0 {
        return ratio >= 0.7;
    }
    (0.45..=2.6).contains(&ratio)
}

fn band_score(field: &ExtractedField, band: &WritingBand) -> f32 {
    let dy = (field.y - band.y).abs();
    let dw = (field.width - band.width).abs() * 0.15;
    dy + dw
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(label: &str, x: f32, y: f32, width: f32, height: f32) -> ExtractedField {
        ExtractedField {
            key: label.to_owned(),
            label: label.to_owned(),
            kind: "text".to_owned(),
            page_number: 1,
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn a_full_width_name_snaps_onto_the_box_row_below_the_label() {
        let mut fields = [field("Name of Account", 40.0, 2326.0, 1950.0, 144.0)];
        snap_to_bands(
            &mut fields,
            &[WritingBand {
                x: 57.0,
                y: 2149.0,
                width: 1855.0,
                height: 152.0,
            }],
        );
        assert!((fields[0].x - 59.0).abs() < 0.1);
        assert!(fields[0].y < 2200.0);
        assert!(fields[0].width > 1800.0);
    }

    #[test]
    fn account_number_takes_the_shorter_box_row_after_the_label() {
        let mut fields = [
            field("Name of Account", 40.0, 2326.0, 1950.0, 144.0),
            field("Account Number", 40.0, 2096.0, 711.0, 115.0),
        ];
        snap_to_bands(
            &mut fields,
            &[
                WritingBand {
                    x: 57.0,
                    y: 2149.0,
                    width: 1855.0,
                    height: 152.0,
                },
                WritingBand {
                    x: 424.0,
                    y: 2042.0,
                    width: 695.0,
                    height: 68.0,
                },
            ],
        );
        assert!(fields[0].width > 1800.0);
        assert!(fields[1].x > 400.0);
        assert!(fields[1].width < 800.0);
        assert!(fields[1].x + fields[1].width < 1140.0);
    }

    #[test]
    fn zenith_scan_puts_account_number_in_the_boxes_after_the_label() {
        let Ok(library_path) = std::env::var("PDFIUM_LIB_PATH") else {
            return;
        };
        let Ok(bytes) = std::fs::read("/tmp/zenith-original.pdf") else {
            return;
        };
        let engine = crate::pdf::PdfEngine::new(
            Some(std::path::PathBuf::from(library_path)),
            None,
            25_000_000,
            10,
        );
        let pages = engine.render_pages(bytes).expect("render");
        let mut fields = [
            field("Name of Account", 40.6, 2326.3, 1950.7, 143.6),
            field("Account Number", 40.6, 2096.6, 711.2, 114.9),
        ];
        snap_scanned_fields(&mut fields, &pages[0]);
        assert!(
            fields[1].x > 300.0,
            "account number still starts on the label at x={}",
            fields[1].x
        );
        assert!(
            fields[0].y < 2290.0,
            "account name still sits on the heading at y={}",
            fields[0].y
        );
        assert!(fields[0].width > 1400.0);
    }

    #[test]
    fn two_fields_do_not_claim_the_same_band() {
        let mut fields = [
            field("One", 40.0, 2000.0, 800.0, 80.0),
            field("Two", 40.0, 1900.0, 800.0, 80.0),
        ];
        snap_to_bands(
            &mut fields,
            &[
                WritingBand {
                    x: 100.0,
                    y: 1980.0,
                    width: 700.0,
                    height: 40.0,
                },
                WritingBand {
                    x: 100.0,
                    y: 1880.0,
                    width: 700.0,
                    height: 40.0,
                },
            ],
        );
        assert!((fields[0].y - fields[1].y).abs() > 20.0);
    }
}
