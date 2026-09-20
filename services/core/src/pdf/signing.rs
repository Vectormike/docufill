use pdfium_render::prelude::*;

use super::{ExtractedField, TextSegment};

const MIN_BOX_WIDTH: f32 = 54.0;
const MIN_BOX_HEIGHT: f32 = 28.0;
const MIN_RULE_WIDTH: f32 = 40.0;
const MAX_RULE_THICKNESS: f32 = 1.5;
const MIN_SIGNING_HEIGHT: f32 = 18.0;
const CAPTION_GAP: f32 = 2.0;

#[derive(Debug, Clone, Copy)]
pub(super) struct Rect {
    left: f32,
    bottom: f32,
    right: f32,
    top: f32,
}

impl Rect {
    fn width(&self) -> f32 {
        self.right - self.left
    }

    fn height(&self) -> f32 {
        self.top - self.bottom
    }

    fn contains(&self, other: &Rect) -> bool {
        self.left <= other.left + 1.0
            && self.right >= other.right - 1.0
            && self.bottom <= other.bottom + 1.0
            && self.top >= other.top - 1.0
    }
}

/// Signature blocks are routinely drawn rather than written: an empty framed box with a
/// rule to sign on and a printed caption beneath it. Nothing about that reaches the text
/// layer, so the only way to offer somewhere to sign is to read the page geometry.
pub(super) fn signing_fields(
    page: &PdfPage<'_>,
    page_number: u16,
    segments: &[TextSegment],
) -> Vec<ExtractedField> {
    let shapes = page_shapes(page);
    let mut spaces = shapes
        .iter()
        .filter(|shape| shape.width() >= MIN_BOX_WIDTH && shape.height() >= MIN_BOX_HEIGHT)
        .filter(|frame| !holds_text(frame, segments))
        .filter_map(|frame| signing_space(frame, &shapes))
        .collect::<Vec<_>>();
    spaces.sort_by(|left, right| {
        right
            .top
            .total_cmp(&left.top)
            .then(left.left.total_cmp(&right.left))
    });

    let total = spaces.len();
    spaces
        .into_iter()
        .enumerate()
        .map(|(index, space)| ExtractedField {
            key: format!("signing-{page_number}-{index}"),
            label: signing_label(index, total),
            kind: "signature".to_owned(),
            page_number,
            x: space.left,
            y: space.bottom,
            width: space.width(),
            height: space.height(),
        })
        .collect()
}

fn page_shapes(page: &PdfPage<'_>) -> Vec<Rect> {
    page.objects()
        .iter()
        .filter(|object| object.object_type() == PdfPageObjectType::Path)
        .filter_map(|object| object.bounds().ok())
        .map(|bounds| Rect {
            left: bounds.left().value,
            bottom: bounds.bottom().value,
            right: bounds.right().value,
            top: bounds.top().value,
        })
        .collect()
}

fn holds_text(frame: &Rect, segments: &[TextSegment]) -> bool {
    segments.iter().any(|segment| {
        segment.x < frame.right
            && frame.left < segment.x + segment.width
            && segment.y < frame.top
            && frame.bottom < segment.y + segment.height
    })
}

/// The writable space is whatever sits between the signing rule and the caption above it.
fn signing_space(frame: &Rect, shapes: &[Rect]) -> Option<Rect> {
    let inner = shapes
        .iter()
        .filter(|shape| frame.contains(shape) && shape.width() < frame.width() - 1.0)
        .collect::<Vec<_>>();
    let rule = inner
        .iter()
        .filter(|shape| shape.height() <= MAX_RULE_THICKNESS && shape.width() >= MIN_RULE_WIDTH)
        .min_by(|left, right| left.top.total_cmp(&right.top))?;
    let ceiling = inner
        .iter()
        .filter(|shape| shape.bottom > rule.top + CAPTION_GAP)
        .map(|shape| shape.bottom)
        .min_by(f32::total_cmp)
        .unwrap_or(frame.top)
        - CAPTION_GAP;

    (ceiling - rule.top >= MIN_SIGNING_HEIGHT).then_some(Rect {
        left: rule.left,
        bottom: rule.top,
        right: rule.right,
        top: ceiling,
    })
}

fn signing_label(index: usize, total: usize) -> String {
    if total <= 1 {
        "Signature".to_owned()
    } else {
        format!("Signatory {}", index + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(left: f32, bottom: f32, right: f32, top: f32) -> Rect {
        Rect {
            left,
            bottom,
            right,
            top,
        }
    }

    fn segment(x: f32, y: f32) -> TextSegment {
        TextSegment {
            text: "I/We".to_owned(),
            page_number: 1,
            x,
            y,
            width: 20.0,
            height: 10.0,
        }
    }

    #[test]
    fn signing_space_sits_above_the_rule_and_below_the_caption() {
        let frame = rect(60.7, 117.8, 278.6, 196.4);
        let shapes = vec![
            frame,
            rect(97.1, 145.0, 215.6, 145.0),
            rect(129.5, 185.8, 161.7, 194.3),
            rect(104.4, 136.0, 140.6, 142.6),
        ];

        let space = signing_space(&frame, &shapes).expect("signing space");

        assert!((space.left - 97.1).abs() < 0.01);
        assert!((space.bottom - 145.0).abs() < 0.01);
        assert!((space.width() - 118.5).abs() < 0.01);
        assert!((space.top - 183.8).abs() < 0.01);
    }

    #[test]
    fn a_frame_without_a_rule_is_not_somewhere_to_sign() {
        let frame = rect(60.0, 120.0, 280.0, 200.0);

        assert!(signing_space(&frame, &[frame]).is_none());
    }

    #[test]
    fn a_rule_pressed_against_the_caption_is_too_thin_to_sign_on() {
        let frame = rect(60.0, 120.0, 280.0, 200.0);
        let shapes = vec![
            frame,
            rect(80.0, 150.0, 200.0, 150.0),
            rect(80.0, 160.0, 200.0, 168.0),
        ];

        assert!(signing_space(&frame, &shapes).is_none());
    }

    #[test]
    fn frames_holding_printed_words_are_left_alone() {
        let frame = rect(21.0, 101.0, 575.0, 240.0);

        assert!(holds_text(&frame, &[segment(27.0, 209.0)]));
        assert!(!holds_text(&frame, &[segment(27.0, 260.0)]));
    }

    #[test]
    fn a_single_block_is_just_called_signature() {
        assert_eq!(signing_label(0, 1), "Signature");
        assert_eq!(signing_label(0, 2), "Signatory 1");
        assert_eq!(signing_label(1, 2), "Signatory 2");
    }
}
