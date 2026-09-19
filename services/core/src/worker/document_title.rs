use crate::pdf::ExtractedDocument;

const TITLE_KEYWORDS: [(&str, u8); 8] = [
    ("form", 100),
    ("mandate", 95),
    ("application", 90),
    ("agreement", 80),
    ("questionnaire", 75),
    ("undertaking", 70),
    ("contract", 65),
    ("certificate", 60),
];

pub(super) fn suggest(document: &ExtractedDocument) -> Option<String> {
    document
        .text_segments
        .iter()
        .filter(|segment| segment.page_number == 1)
        .filter_map(|segment| {
            let text = segment
                .text
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            let lower = text.to_lowercase();
            let keyword_score = TITLE_KEYWORDS
                .iter()
                .find_map(|(keyword, score)| lower.contains(keyword).then_some(*score))?;
            let valid_length = (4..=120).contains(&text.chars().count());
            let looks_like_field = has_placeholder_run(&text);
            (valid_length && !looks_like_field).then_some((
                keyword_score + u8::from(is_uppercase_heading(&text)) * 10,
                readable_title(&text),
            ))
        })
        .max_by_key(|(score, _)| *score)
        .map(|(_, title)| title)
}

fn has_placeholder_run(value: &str) -> bool {
    value
        .chars()
        .fold((false, 0), |(found, run), character| {
            if found {
                (true, run)
            } else if matches!(character, '.' | '…' | '_' | '-' | '–' | '—') {
                (run >= 3, run + 1)
            } else {
                (false, 0)
            }
        })
        .0
}

fn is_uppercase_heading(value: &str) -> bool {
    let mut letters = value.chars().filter(|character| character.is_alphabetic());
    let count = letters.clone().count();
    count >= 4 && letters.all(|character| !character.is_lowercase())
}

fn readable_title(value: &str) -> String {
    if !is_uppercase_heading(value) {
        return value.to_owned();
    }
    value
        .split_whitespace()
        .map(|word| {
            let mut characters = word.chars();
            characters.next().map_or_else(String::new, |first| {
                first
                    .to_uppercase()
                    .chain(characters.flat_map(char::to_lowercase))
                    .collect()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use crate::pdf::TextSegment;

    use super::*;

    #[test]
    fn selects_and_formats_the_document_heading() {
        let document = ExtractedDocument {
            page_count: 1,
            fields: Vec::new(),
            title: None,
            text_segments: vec![
                segment("FEMI MARTINS CONSULT"),
                segment("PLOT 260, ABUJA"),
                segment("PROSPECTIVE TENANT’S ACQUAINTANCE FORM"),
                segment("Full Name: …………………………………………"),
            ],
        };
        assert_eq!(
            suggest(&document).as_deref(),
            Some("Prospective Tenant’s Acquaintance Form")
        );
    }

    fn segment(text: &str) -> TextSegment {
        TextSegment {
            text: text.to_owned(),
            page_number: 1,
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 12.0,
        }
    }
}
