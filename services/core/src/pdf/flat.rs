use pdfium_render::prelude::*;

use crate::{AppError, AppResult};

use super::{ExtractedField, TextSegment, extract::infer_kind};

const MIN_PLACEHOLDER_CHARS: usize = 4;
const MAX_LABEL_LENGTH: usize = 180;

#[derive(Debug, Clone, Copy)]
struct Glyph {
    value: char,
    left: f32,
    bottom: f32,
    right: f32,
    top: f32,
}

#[derive(Debug, Clone)]
struct TextLine {
    text: String,
    glyphs: Vec<Glyph>,
}

#[derive(Debug, Clone, Copy)]
struct Bounds {
    left: f32,
    bottom: f32,
    right: f32,
    top: f32,
}

pub(super) fn extract_flat_page(
    page_text: &PdfPageText<'_>,
    page_number: u16,
    page_width: f32,
) -> AppResult<(Vec<TextSegment>, Vec<ExtractedField>)> {
    let lines = text_lines(page_text)?;
    let segments = lines
        .iter()
        .filter_map(|line| {
            line_bounds(&line.glyphs).map(|bounds| TextSegment {
                text: line.text.clone(),
                page_number,
                x: bounds.left,
                y: bounds.bottom,
                width: bounds.right - bounds.left,
                height: bounds.top - bounds.bottom,
            })
        })
        .collect();
    let fields = infer_flat_fields(page_number, page_width, &lines);
    Ok((segments, fields))
}

fn text_lines(page_text: &PdfPageText<'_>) -> AppResult<Vec<TextLine>> {
    let mut lines = Vec::new();
    let mut glyphs = Vec::new();

    for character in page_text.chars().iter() {
        let Some(value) = character.unicode_char() else {
            continue;
        };
        if value == '\r' {
            continue;
        }
        if value == '\n' {
            push_line(&mut lines, &mut glyphs);
            continue;
        }
        let bounds = character.loose_bounds().map_err(AppError::internal)?;
        glyphs.push(Glyph {
            value,
            left: bounds.left().value,
            bottom: bounds.bottom().value,
            right: bounds.right().value,
            top: bounds.top().value,
        });
    }
    push_line(&mut lines, &mut glyphs);
    Ok(lines)
}

fn push_line(lines: &mut Vec<TextLine>, glyphs: &mut Vec<Glyph>) {
    let text = glyphs
        .iter()
        .map(|glyph| glyph.value)
        .collect::<String>()
        .trim()
        .to_owned();
    if !text.is_empty() {
        lines.push(TextLine {
            text,
            glyphs: std::mem::take(glyphs),
        });
    } else {
        glyphs.clear();
    }
}

fn infer_flat_fields(page_number: u16, page_width: f32, lines: &[TextLine]) -> Vec<ExtractedField> {
    let mut fields = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| {
            let (start, end) = placeholder_run(&line.glyphs)?;
            let placeholder = line_bounds(&line.glyphs[start..end])?;
            let before = line.glyphs[..start]
                .iter()
                .map(|glyph| glyph.value)
                .collect::<String>();
            let after = line.glyphs[end..]
                .iter()
                .map(|glyph| glyph.value)
                .collect::<String>();
            let label = label_for(lines, index, &before, &after)?;
            let x = placeholder.left.max(24.0);
            let y = (placeholder.bottom - 2.0).max(24.0);
            let width = (placeholder.right - x).min(page_width - x - 24.0);
            let height = (placeholder.top - placeholder.bottom + 6.0).max(18.0);

            (width > 40.0).then(|| ExtractedField {
                key: format!("flat-{page_number}-{index}"),
                kind: infer_kind(&label).to_owned(),
                label,
                page_number,
                x,
                y,
                width,
                height,
            })
        })
        .collect::<Vec<_>>();
    qualify_guarantor_groups(&mut fields);
    fields
}

fn qualify_guarantor_groups(fields: &mut [ExtractedField]) {
    let pattern = ["name", "address", "occupation", "telephone no"];
    let mut index = 0;
    let mut group = 1;
    while index + pattern.len() <= fields.len() {
        let matches = fields[index..index + pattern.len()]
            .iter()
            .map(|field| field.label.to_ascii_lowercase())
            .eq(pattern.map(str::to_owned));
        if matches {
            for field in &mut fields[index..index + pattern.len()] {
                field.label = format!("Guarantor {group} {}", field.label);
            }
            group += 1;
            index += pattern.len();
        } else {
            index += 1;
        }
    }
}

fn placeholder_run(glyphs: &[Glyph]) -> Option<(usize, usize)> {
    let mut best = None;
    let mut start = None;

    for (index, glyph) in glyphs.iter().enumerate() {
        if is_placeholder(glyph.value) {
            start.get_or_insert(index);
        } else if let Some(run_start) = start.take() {
            update_best_run(&mut best, run_start, index);
        }
    }
    if let Some(run_start) = start {
        update_best_run(&mut best, run_start, glyphs.len());
    }
    best.filter(|(run_start, run_end)| run_end - run_start >= MIN_PLACEHOLDER_CHARS)
}

fn update_best_run(best: &mut Option<(usize, usize)>, start: usize, end: usize) {
    if best.is_none_or(|(best_start, best_end)| end - start > best_end - best_start) {
        *best = Some((start, end));
    }
}

fn label_for(lines: &[TextLine], index: usize, before: &str, after: &str) -> Option<String> {
    let trimmed_before = before.trim().trim_matches([',', ':']);
    let normalized_after = after.to_ascii_lowercase();
    if trimmed_before.eq_ignore_ascii_case("i") {
        let label = if normalized_after.contains("name in full") {
            "Guarantor full name"
        } else {
            "Declarant name"
        };
        return Some(label.to_owned());
    }
    if trimmed_before.eq_ignore_ascii_case("mr/mrs/miss") {
        return Some("Prospective tenant name".to_owned());
    }
    if matches!(trimmed_before, "N" | "₦") && normalized_after.contains("in advance") {
        return Some("Annual rent".to_owned());
    }

    let direct = clean_label(before);
    if has_item_marker(before) {
        return valid_label(direct);
    }

    if let Some(start) = preceding_item_start(lines, index) {
        let mut parts = lines[start..index]
            .iter()
            .map(|line| clean_label(&line.text))
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();
        if !direct.is_empty() {
            parts.push(direct.clone());
        }
        if let Some(label) = valid_label(parts.join(" ")) {
            return Some(label);
        }
    }

    if let Some(label) = valid_label(direct) {
        return Some(label);
    }

    let previous = index.checked_sub(1).and_then(|previous_index| {
        let line = &lines[previous_index];
        placeholder_run(&line.glyphs)
            .is_none()
            .then(|| clean_label(&line.text))
    });
    valid_label(previous.unwrap_or_default())
}

fn preceding_item_start(lines: &[TextLine], index: usize) -> Option<usize> {
    let lower_bound = index.saturating_sub(3);
    for candidate in (lower_bound..index).rev() {
        if placeholder_run(&lines[candidate].glyphs).is_some() {
            return None;
        }
        if has_item_marker(&lines[candidate].text) {
            return Some(candidate);
        }
    }
    None
}

fn clean_label(value: &str) -> String {
    let without_marker = strip_item_marker(value.trim());
    without_marker
        .trim_matches(|character: char| {
            character.is_whitespace()
                || character == ':'
                || character == '?'
                || character == ','
                || is_placeholder(character)
        })
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn valid_label(label: String) -> Option<String> {
    let alphabetic = label
        .chars()
        .filter(|character| character.is_alphabetic())
        .count();
    (alphabetic >= 3 && label.len() <= MAX_LABEL_LENGTH).then_some(label)
}

fn has_item_marker(value: &str) -> bool {
    strip_item_marker(value.trim()).len() < value.trim().len()
}

fn strip_item_marker(value: &str) -> &str {
    let Some(dot) = value.find('.') else {
        return value;
    };
    let marker = &value[..dot];
    let is_numeric =
        !marker.is_empty() && marker.chars().all(|character| character.is_ascii_digit());
    let is_alpha = marker.len() == 1
        && marker
            .chars()
            .all(|character| character.is_ascii_alphabetic());
    if (is_numeric || is_alpha) && value[dot + 1..].starts_with(char::is_whitespace) {
        value[dot + 1..].trim_start()
    } else {
        value
    }
}

fn is_placeholder(value: char) -> bool {
    matches!(value, '.' | '…' | '_' | '-' | '–' | '—')
}

fn line_bounds(glyphs: &[Glyph]) -> Option<Bounds> {
    let first = *glyphs.first()?;
    Some(glyphs.iter().skip(1).fold(
        Bounds {
            left: first.left,
            bottom: first.bottom,
            right: first.right,
            top: first.top,
        },
        |bounds, glyph| Bounds {
            left: bounds.left.min(glyph.left),
            bottom: bounds.bottom.min(glyph.bottom),
            right: bounds.right.max(glyph.right),
            top: bounds.top.max(glyph.top),
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(text: &str) -> TextLine {
        TextLine {
            text: text.to_owned(),
            glyphs: text
                .chars()
                .enumerate()
                .map(|(index, value)| Glyph {
                    value,
                    left: index as f32 * 5.0,
                    bottom: 100.0,
                    right: index as f32 * 5.0 + 5.0,
                    top: 112.0,
                })
                .collect(),
        }
    }

    #[test]
    fn joins_wrapped_question_labels() {
        let fields = infer_flat_fields(
            1,
            600.0,
            &[line("2. Telephone"), line("Number: …………………………………………")],
        );
        assert_eq!(fields[0].label, "Telephone Number");
    }

    #[test]
    fn keeps_complete_inline_label() {
        let fields = infer_flat_fields(1, 600.0, &[line("3. Email Address: …………………………………………")]);
        assert_eq!(fields[0].label, "Email Address");
        assert_eq!(fields[0].kind, "email");
    }

    #[test]
    fn ignores_unlabelled_continuation_lines() {
        let fields = infer_flat_fields(
            1,
            600.0,
            &[
                line("Property address: --------------------------------"),
                line("--------------------------------"),
            ],
        );
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].label, "Property address");
    }

    #[test]
    fn names_inline_guarantor_fields_without_using_surrounding_prose() {
        let fields = infer_flat_fields(
            5,
            600.0,
            &[
                line("questionable character and capable of paying the annual rent of"),
                line("N………………………………in advance"),
                line("MR/MRS/MISS………………………………whose photograph appeared above"),
            ],
        );
        assert_eq!(fields[0].label, "Annual rent");
        assert_eq!(fields[1].label, "Prospective tenant name");
    }

    #[test]
    fn qualifies_repeated_guarantor_questions() {
        let fields = infer_flat_fields(
            2,
            600.0,
            &[
                line("a. Name: …………………………………………"),
                line("Address: …………………………………………"),
                line("Occupation: …………………………………………"),
                line("Telephone No: …………………………………………"),
                line("b. Name: …………………………………………"),
                line("Address: …………………………………………"),
                line("Occupation: …………………………………………"),
                line("Telephone No: …………………………………………"),
            ],
        );
        assert_eq!(fields[0].label, "Guarantor 1 Name");
        assert_eq!(fields[3].label, "Guarantor 1 Telephone No");
        assert_eq!(fields[4].label, "Guarantor 2 Name");
        assert_eq!(fields[7].label, "Guarantor 2 Telephone No");
    }
}
