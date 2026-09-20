use pdfium_render::prelude::*;

use crate::{AppError, AppResult};

use super::{ExtractedField, TextSegment, extract::infer_kind};

const MIN_PLACEHOLDER_CHARS: usize = 4;
const GRID_INSET: f32 = 5.0;
const CELL_INSET: f32 = 2.0;
const LABEL_GAP: f32 = 6.0;
const SINGLE_LINE_CELL: f32 = 26.0;
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

#[derive(Debug, Clone, Copy)]
pub struct GridWall {
    pub x: f32,
    pub bottom: f32,
    pub top: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct GridRail {
    pub y: f32,
    pub left: f32,
    pub right: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Cell {
    left: f32,
    bottom: f32,
    right: f32,
    top: f32,
}

impl Cell {
    fn height(self) -> f32 {
        self.top - self.bottom
    }

    fn key(self) -> (i32, i32, i32, i32) {
        (
            self.left.round() as i32,
            self.bottom.round() as i32,
            self.right.round() as i32,
            self.top.round() as i32,
        )
    }
}

pub fn snap_fields_to_grid(fields: &mut [ExtractedField], walls: &[GridWall]) {
    for field in fields {
        if matches!(field.kind.as_str(), "checkbox" | "radio" | "signature") {
            continue;
        }
        let probe = field.y + 3.0;
        let Some(right) = walls
            .iter()
            .filter(|wall| {
                wall.top - wall.bottom >= 8.0
                    && wall.x > field.x + 16.0
                    && wall.bottom <= probe
                    && wall.top >= probe
            })
            .map(|wall| wall.x)
            .min_by(|left, right| left.total_cmp(right))
        else {
            continue;
        };
        let width = right - GRID_INSET - field.x;
        if width >= 24.0 {
            field.width = field.width.min(width);
        }
    }
}

/// Clip each write-in field to the printed cell it sits in. A lone field in a
/// short row takes the whole remaining cell so the answer sits in the box, not
/// on the label; fields that share a tall cell keep their own writing line.
pub fn snap_fields_to_cells(fields: &mut [ExtractedField], walls: &[GridWall], rails: &[GridRail]) {
    let cells = fields
        .iter()
        .map(|field| enclosing_cell(field, walls, rails))
        .collect::<Vec<_>>();
    let mut occupancy = std::collections::HashMap::<_, usize>::new();
    for (field, cell) in fields.iter().zip(&cells) {
        if matches!(field.kind.as_str(), "checkbox" | "radio" | "signature") {
            continue;
        }
        if let Some(cell) = cell {
            *occupancy.entry(cell.key()).or_default() += 1;
        }
    }

    for (field, cell) in fields.iter_mut().zip(cells) {
        if matches!(field.kind.as_str(), "checkbox" | "radio" | "signature") {
            continue;
        }
        let Some(cell) = cell else {
            continue;
        };
        let right = cell.right - GRID_INSET;
        if right - field.x >= 24.0 {
            field.width = field.width.min(right - field.x);
        }
        let shared_tall = occupancy.get(&cell.key()).copied().unwrap_or(0) > 1
            && cell.height() > SINGLE_LINE_CELL;
        if !shared_tall && cell.height() >= 10.0 {
            field.y = cell.bottom + CELL_INSET;
            field.height = (cell.top - CELL_INSET - field.y).max(9.0);
        } else {
            field.y = field
                .y
                .clamp(cell.bottom + CELL_INSET, (cell.top - 10.0).max(cell.bottom));
            if field.y + field.height > cell.top - CELL_INSET {
                field.height = (cell.top - CELL_INSET - field.y).max(9.0);
            }
        }
    }
}

fn enclosing_cell(field: &ExtractedField, walls: &[GridWall], rails: &[GridRail]) -> Option<Cell> {
    let probe_x = field.x + field.width.min(12.0);
    let probe_y = field.y + field.height.min(6.0);
    let right = walls
        .iter()
        .filter(|wall| wall.x > field.x + 16.0 && wall.bottom <= probe_y && wall.top >= probe_y)
        .map(|wall| wall.x)
        .min_by(f32::total_cmp)?;
    let left = walls
        .iter()
        .filter(|wall| wall.x < field.x + 4.0 && wall.bottom <= probe_y && wall.top >= probe_y)
        .map(|wall| wall.x)
        .max_by(f32::total_cmp)
        .unwrap_or(0.0);
    let top = rails
        .iter()
        .filter(|rail| rail.y > probe_y + 2.0 && rail.left <= probe_x && rail.right >= probe_x)
        .map(|rail| rail.y)
        .min_by(f32::total_cmp)?;
    let bottom = rails
        .iter()
        .filter(|rail| rail.y < probe_y + 4.0 && rail.left <= probe_x && rail.right >= probe_x)
        .map(|rail| rail.y)
        .max_by(f32::total_cmp)?;
    (top - bottom >= 8.0).then_some(Cell {
        left,
        bottom,
        right,
        top,
    })
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
    let mut fields = infer_flat_fields(page_number, page_width, &lines);
    if fields.is_empty() {
        fields = infer_labelled_fields(page_number, page_width, &lines);
    }
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

const MIN_LABELLED_WIDTH: f32 = 40.0;
const RIGHT_MARGIN: f32 = 24.0;

fn infer_labelled_fields(
    page_number: u16,
    page_width: f32,
    lines: &[TextLine],
) -> Vec<ExtractedField> {
    let mut fields = Vec::new();
    for (row_index, row) in cluster_rows(lines).into_iter().enumerate() {
        if is_instruction_row(&row_text(&row)) {
            continue;
        }
        let labels = colon_labels(&row);
        for (label_index, label) in labels.iter().enumerate() {
            if !is_write_in_label(&label.text) {
                continue;
            }
            let next_left = labels
                .get(label_index + 1)
                .map(|next| next.left)
                .or_else(|| header_stop_left(&row, label.field_left))
                .unwrap_or(page_width - RIGHT_MARGIN);
            let x = writable_start(&row, label.field_left, next_left);
            let width = next_left - x - 4.0;
            if width < MIN_LABELLED_WIDTH {
                continue;
            }
            let kind = infer_kind(&label.text).to_owned();
            fields.push(ExtractedField {
                key: format!("label-{page_number}-{row_index}-{label_index}"),
                kind,
                label: label.text.clone(),
                page_number,
                x,
                y: (label.bottom - 1.0).max(24.0),
                width,
                height: 12.0,
            });
        }

        for (box_index, mark) in checkbox_marks(&row).into_iter().enumerate() {
            fields.push(ExtractedField {
                key: format!("check-{page_number}-{row_index}-{box_index}"),
                kind: "checkbox".to_owned(),
                label: mark.label,
                page_number,
                x: mark.x,
                y: mark.bottom.max(24.0),
                width: mark.width,
                height: mark.height,
            });
        }

        if let Some(certification) = certification_blank(page_number, &row) {
            fields.push(certification);
        }
    }
    fields
}

struct ColonLabel {
    text: String,
    left: f32,
    field_left: f32,
    bottom: f32,
}

struct CheckboxMark {
    label: String,
    x: f32,
    bottom: f32,
    width: f32,
    height: f32,
}

fn cluster_rows(lines: &[TextLine]) -> Vec<Vec<&TextLine>> {
    let mut ordered: Vec<&TextLine> = lines.iter().collect();
    ordered.sort_by(|left, right| {
        let left_y = line_bounds(&left.glyphs)
            .map(|bounds| bounds.bottom)
            .unwrap_or(0.0);
        let right_y = line_bounds(&right.glyphs)
            .map(|bounds| bounds.bottom)
            .unwrap_or(0.0);
        right_y.total_cmp(&left_y)
    });
    let mut rows: Vec<Vec<&TextLine>> = Vec::new();
    for line in ordered {
        let bottom = line_bounds(&line.glyphs)
            .map(|bounds| bounds.bottom)
            .unwrap_or(0.0);
        if let Some(row) = rows.last_mut().filter(|row| {
            row.iter().any(|existing| {
                line_bounds(&existing.glyphs)
                    .is_some_and(|bounds| (bounds.bottom - bottom).abs() <= 6.0)
            })
        }) {
            row.push(line);
        } else {
            rows.push(vec![line]);
        }
    }
    for row in &mut rows {
        row.sort_by(|left, right| {
            let left_x = line_bounds(&left.glyphs)
                .map(|bounds| bounds.left)
                .unwrap_or(0.0);
            let right_x = line_bounds(&right.glyphs)
                .map(|bounds| bounds.left)
                .unwrap_or(0.0);
            left_x.total_cmp(&right_x)
        });
    }
    rows
}

fn row_text(row: &[&TextLine]) -> String {
    row.iter()
        .map(|line| line.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_instruction_row(text: &str) -> bool {
    let normalized = text.trim().to_ascii_lowercase();
    normalized.starts_with("valid id for")
        || normalized.starts_with("please ")
        || normalized.starts_with("ensure ")
        || normalized.starts_with("completing ")
        || normalized.contains("refer to the note")
        || normalized.starts_with("section ")
        || normalized.starts_with("instructions")
}

fn is_write_in_label(label: &str) -> bool {
    let normalized = label.trim().to_ascii_lowercase();
    !matches!(
        normalized.as_str(),
        "sex"
            | "valid id"
            | "for individual only"
            | "for enterprise/sole proprietorship only"
            | "requests"
            | "documents required"
    ) && !normalized.starts_with("for ")
        && !normalized.starts_with("section")
        && !normalized.contains("update")
        && !normalized.contains("activation")
}

fn colon_labels(row: &[&TextLine]) -> Vec<ColonLabel> {
    let mut labels = Vec::new();
    for line in row {
        let mut start = 0;
        for (index, glyph) in line.glyphs.iter().enumerate() {
            if glyph.value != ':' {
                continue;
            }
            let token_start = label_token_start(&line.glyphs, start, index);
            let raw: String = line.glyphs[token_start..=index]
                .iter()
                .map(|item| item.value)
                .collect();
            let text = colon_field_label(&raw);
            if text
                .chars()
                .filter(|character| character.is_alphabetic())
                .count()
                < 2
            {
                start = index + 1;
                continue;
            }
            let left = line.glyphs[token_start].left;
            labels.push(ColonLabel {
                text,
                left,
                field_left: field_left_after(&line.glyphs, index),
                bottom: glyph.bottom,
            });
            start = index + 1;
            while start < line.glyphs.len() && line.glyphs[start].value.is_whitespace() {
                start += 1;
            }
        }
    }
    labels.sort_by(|left, right| left.left.total_cmp(&right.left));
    labels
}

fn label_token_start(glyphs: &[Glyph], floor: usize, colon_index: usize) -> usize {
    let mut start = colon_index;
    while start > floor {
        let previous = start - 1;
        let value = glyphs[previous].value;
        if matches!(value, ':' | '☐' | '□') {
            break;
        }
        if start < colon_index && glyphs[start].left - glyphs[previous].right > 15.0 {
            break;
        }
        start = previous;
    }
    while start < colon_index
        && (glyphs[start].value.is_whitespace() || matches!(glyphs[start].value, '☐' | '□'))
    {
        start += 1;
    }
    start
}

fn header_stop_left(row: &[&TextLine], after: f32) -> Option<f32> {
    for line in row {
        let text: String = line.glyphs.iter().map(|glyph| glyph.value).collect();
        let normalized = text.replace(' ', "").to_ascii_lowercase();
        let Some(index) = normalized.find("validid") else {
            continue;
        };
        let mut seen = 0usize;
        for glyph in &line.glyphs {
            if glyph.value.is_whitespace() {
                continue;
            }
            if seen == index && glyph.left > after {
                return Some(glyph.left);
            }
            seen += 1;
        }
    }
    None
}

fn writable_start(row: &[&TextLine], after_colon: f32, until: f32) -> f32 {
    let mut left = after_colon;
    for _ in 0..24 {
        let Some(right) = row
            .iter()
            .flat_map(|line| line.glyphs.iter())
            .filter(|glyph| {
                !glyph.value.is_whitespace()
                    && glyph.left >= left - 1.0
                    && glyph.left <= left + 12.0
                    && glyph.left < until - 8.0
            })
            .map(|glyph| glyph.right)
            .max_by(|left, right| left.total_cmp(right))
        else {
            break;
        };
        left = right + 3.0;
    }
    if left + LABEL_GAP + MIN_LABELLED_WIDTH < until {
        left + LABEL_GAP
    } else {
        left
    }
}

fn field_left_after(glyphs: &[Glyph], colon_index: usize) -> f32 {
    let mut index = colon_index + 1;
    while index < glyphs.len() && glyphs[index].value.is_whitespace() {
        index += 1;
    }
    if index < glyphs.len() && glyphs[index].value == '(' {
        while index < glyphs.len() && glyphs[index].value != ')' {
            index += 1;
        }
        if index < glyphs.len() {
            index += 1;
        }
        while index < glyphs.len() && glyphs[index].value.is_whitespace() {
            index += 1;
        }
        return glyphs
            .get(index)
            .map(|glyph| glyph.left)
            .or_else(|| {
                glyphs
                    .get(index.saturating_sub(1))
                    .map(|glyph| glyph.right + 4.0)
            })
            .unwrap_or(glyphs[colon_index].right + 4.0);
    }
    glyphs[colon_index].right + 4.0
}

fn checkbox_marks(row: &[&TextLine]) -> Vec<CheckboxMark> {
    let mut marks = Vec::new();
    for line in row {
        for (index, glyph) in line.glyphs.iter().enumerate() {
            if glyph.value != '☐' && glyph.value != '□' {
                continue;
            }
            let mut following = String::new();
            let mut previous_right = glyph.right;
            for item in line.glyphs[index + 1..].iter() {
                if matches!(item.value, ':' | '☐' | '□') || item.left - previous_right > 15.0 {
                    break;
                }
                following.push(item.value);
                previous_right = item.right;
            }
            let text = checkbox_label(following.trim());
            if text.is_empty() {
                continue;
            }
            marks.push(CheckboxMark {
                label: text,
                x: glyph.left,
                bottom: glyph.bottom,
                width: (glyph.right - glyph.left).clamp(8.0, 14.0),
                height: (glyph.top - glyph.bottom).clamp(8.0, 14.0),
            });
        }
    }
    marks
}

fn certification_blank(page_number: u16, row: &[&TextLine]) -> Option<ExtractedField> {
    let we = row
        .iter()
        .find(|line| line.text.trim().eq_ignore_ascii_case("i/we"))?;
    let certify = row
        .iter()
        .find(|line| line.text.trim().to_ascii_lowercase().starts_with("certify"))?;
    let start = line_bounds(&we.glyphs)?;
    let end = line_bounds(&certify.glyphs)?;
    let x = start.right + 4.0;
    let width = end.left - x - 4.0;
    (width >= MIN_LABELLED_WIDTH).then(|| ExtractedField {
        key: "certification-name".to_owned(),
        kind: "text".to_owned(),
        label: "Certification Name".to_owned(),
        page_number,
        x,
        y: (start.bottom - 4.0).max(24.0),
        width,
        height: 16.0,
    })
}

fn colon_field_label(value: &str) -> String {
    let text = clean_label(&tidy_label(value));
    let lower = text.to_ascii_lowercase();
    if lower.ends_with("date of birth") {
        return "Date of Birth".to_owned();
    }
    text
}

fn checkbox_label(value: &str) -> String {
    let text = tidy_label(value);
    let lower = text.to_ascii_lowercase();
    if lower == "valid id" || lower == "documents required" {
        return String::new();
    }
    for stop in [" date of birth", " valid id", " old phone", " new phone"] {
        if let Some(index) = lower.find(stop) {
            return text[..index].trim().to_owned();
        }
    }
    text
}

fn tidy_label(value: &str) -> String {
    humanize_label(
        &value
            .replace(['☐', '□'], " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" "),
    )
}

fn humanize_label(value: &str) -> String {
    let mut expanded = String::new();
    let characters: Vec<char> = value.chars().collect();
    for (index, character) in characters.iter().enumerate() {
        if index > 0 && character.is_uppercase() {
            let previous = characters[index - 1];
            let next_is_lower = characters
                .get(index + 1)
                .is_some_and(|next| next.is_lowercase());
            if previous.is_lowercase() || (previous.is_uppercase() && next_is_lower) {
                expanded.push(' ');
            }
        }
        expanded.push(*character);
    }
    expanded.split_whitespace().collect::<Vec<_>>().join(" ")
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

    fn line_at(text: &str, start_x: f32, bottom: f32) -> TextLine {
        TextLine {
            text: text.to_owned(),
            glyphs: text
                .chars()
                .enumerate()
                .map(|(index, value)| Glyph {
                    value,
                    left: start_x + index as f32 * 5.0,
                    bottom,
                    right: start_x + index as f32 * 5.0 + 5.0,
                    top: bottom + 12.0,
                })
                .collect(),
        }
    }

    #[test]
    fn a_lone_field_fills_its_printed_cell() {
        let mut fields = vec![ExtractedField {
            key: "account".to_owned(),
            label: "Account Number".to_owned(),
            kind: "number".to_owned(),
            page_number: 1,
            x: 105.7,
            y: 509.6,
            width: 185.4,
            height: 12.0,
        }];
        snap_fields_to_cells(
            &mut fields,
            &[
                GridWall {
                    x: 26.9,
                    bottom: 506.3,
                    top: 521.9,
                },
                GridWall {
                    x: 296.0,
                    bottom: 506.3,
                    top: 521.9,
                },
            ],
            &[
                GridRail {
                    y: 506.3,
                    left: 27.4,
                    right: 296.0,
                },
                GridRail {
                    y: 521.9,
                    left: 27.4,
                    right: 296.0,
                },
            ],
        );
        assert!((fields[0].y - 508.3).abs() < 0.2);
        assert!(fields[0].y + fields[0].height <= 520.0);
        assert!(fields[0].x + fields[0].width < 292.0);
    }

    #[test]
    fn shared_tall_cells_keep_each_writing_line() {
        let mut fields = vec![
            ExtractedField {
                key: "first".to_owned(),
                label: "First Name".to_owned(),
                kind: "text".to_owned(),
                page_number: 1,
                x: 81.0,
                y: 558.0,
                width: 55.0,
                height: 12.0,
            },
            ExtractedField {
                key: "dob".to_owned(),
                label: "Date of Birth".to_owned(),
                kind: "date".to_owned(),
                page_number: 1,
                x: 239.0,
                y: 535.0,
                width: 80.0,
                height: 12.0,
            },
        ];
        let walls = [
            GridWall {
                x: 26.9,
                bottom: 534.4,
                top: 570.9,
            },
            GridWall {
                x: 565.4,
                bottom: 534.4,
                top: 570.9,
            },
        ];
        let rails = [
            GridRail {
                y: 534.4,
                left: 27.4,
                right: 565.4,
            },
            GridRail {
                y: 570.9,
                left: 27.4,
                right: 565.4,
            },
        ];
        snap_fields_to_cells(&mut fields, &walls, &rails);
        assert!(fields[0].y > 550.0);
        assert!(fields[1].y < 545.0);
        assert!(fields[0].height <= 16.0);
    }

    #[test]
    fn grid_walls_clip_fields_before_the_next_cell() {
        let mut fields = vec![ExtractedField {
            key: "new-phone".to_owned(),
            label: "New Phone No".to_owned(),
            kind: "phone".to_owned(),
            page_number: 1,
            x: 394.0,
            y: 345.0,
            width: 172.0,
            height: 13.0,
        }];
        snap_fields_to_grid(
            &mut fields,
            &[GridWall {
                x: 455.5,
                bottom: 345.0,
                top: 359.0,
            }],
        );
        assert!(fields[0].x + fields[0].width < 456.0);
        assert!(fields[0].width < 70.0);
    }

    #[test]
    fn title_skips_the_printed_honorific_hint() {
        let fields = infer_labelled_fields(
            1,
            600.0,
            &[line_at("Title: (MR/MRS/MISS/DR/CHIEF/PROF)", 30.0, 569.0)],
        );
        let title = fields
            .iter()
            .find(|field| field.label == "Title")
            .expect("title");
        assert!(title.x > 180.0);
        assert!(title.x + title.width <= 580.0);
    }

    #[test]
    fn labelled_fields_use_gaps_after_colons() {
        let fields = infer_labelled_fields(
            1,
            600.0,
            &[
                line_at("First Name:          Middle Name:", 30.0, 560.0),
                line_at("Surname:", 380.0, 560.0),
            ],
        );
        let labels: Vec<_> = fields.iter().map(|field| field.label.as_str()).collect();
        assert!(labels.contains(&"First Name"));
        assert!(labels.contains(&"Middle Name"));
        assert!(labels.contains(&"Surname"));
        let first = fields
            .iter()
            .find(|field| field.label == "First Name")
            .expect("first name");
        let middle = fields
            .iter()
            .find(|field| field.label == "Middle Name")
            .expect("middle name");
        assert!(first.x + first.width <= middle.x + 1.0);
        assert!(first.width >= MIN_LABELLED_WIDTH);
    }

    #[test]
    fn request_ticks_drop_the_valid_id_column() {
        let fields = infer_labelled_fields(
            1,
            600.0,
            &[line_at("☐ Soft Token {Re}activation Valid ID", 30.0, 330.0)],
        );
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].kind, "checkbox");
        assert_eq!(fields[0].label, "Soft Token {Re}activation");
    }

    #[test]
    fn splits_acronym_from_following_word() {
        assert_eq!(humanize_label("USSDOpt-in"), "USSD Opt-in");
    }

    #[test]
    fn skips_instruction_colons() {
        let fields = infer_labelled_fields(
            1,
            600.0,
            &[line_at(
                "Valid ID for Nigeria Residents: Driver's License, Voter Card",
                26.0,
                667.0,
            )],
        );
        assert!(fields.is_empty());
    }

    #[test]
    fn splits_sex_boxes_from_date_of_birth() {
        let fields = infer_labelled_fields(
            1,
            600.0,
            &[line_at("Sex: ☐ Male ☐ Female Date of Birth:", 30.0, 530.0)],
        );
        let labels: Vec<_> = fields.iter().map(|field| field.label.as_str()).collect();
        assert!(labels.contains(&"Male"));
        assert!(labels.contains(&"Female"));
        assert!(labels.contains(&"Date of Birth"));
        assert!(!labels.iter().any(|label| label.contains('☐')));
        assert_eq!(
            fields
                .iter()
                .filter(|field| field.kind == "checkbox")
                .count(),
            2
        );
    }

    #[test]
    fn finds_certification_name_between_i_we_and_certify() {
        let fields = infer_labelled_fields(
            1,
            600.0,
            &[
                line_at("I/We", 26.0, 208.0),
                line_at(
                    "certify that the information provided is true",
                    187.0,
                    208.0,
                ),
            ],
        );
        assert_eq!(fields[0].label, "Certification Name");
        assert!(fields[0].x > 26.0);
        assert!(fields[0].x + fields[0].width < 200.0);
    }
}
