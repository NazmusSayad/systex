use crate::model::RelatedContent;

const SURROUNDING_CHARS: usize = 400;

/// Everything around the caret that is worth knowing without asking the system for more: the word it
/// sits in, the line, the sentence, the paragraph, and a bounded window of raw text on either side.
pub fn from_caret(text_before: &str, text_after: &str) -> RelatedContent {
    let line_before = text_before.rsplit('\n').next().unwrap_or_default();
    let line_after = text_after.split('\n').next().unwrap_or_default();

    let word_before: String = line_before
        .chars()
        .rev()
        .take_while(|character| is_word(*character))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    let word_after: String = line_after.chars().take_while(|c| is_word(*c)).collect();
    let word = format!("{word_before}{word_after}");

    RelatedContent {
        word: if word.is_empty() { None } else { Some(word) },
        line: format!("{line_before}{line_after}"),
        sentence: sentence(text_before, text_after),
        paragraph: paragraph(text_before, text_after),
        before: bounded_tail(text_before),
        after: bounded_head(text_after),
    }
}

fn is_word(character: char) -> bool {
    character.is_alphanumeric() || character == '\'' || character == '-' || character == '_'
}

fn sentence(text_before: &str, text_after: &str) -> Option<String> {
    let start = text_before
        .char_indices()
        .rfind(|(_, character)| matches!(character, '.' | '!' | '?' | '\n'))
        .map(|(index, character)| index + character.len_utf8())
        .unwrap_or(0);
    let end = text_after
        .char_indices()
        .find(|(_, character)| matches!(character, '.' | '!' | '?' | '\n'))
        .map(|(index, character)| index + character.len_utf8())
        .unwrap_or(text_after.len());

    let sentence = format!("{}{}", &text_before[start..], &text_after[..end]);
    let sentence = sentence.trim();

    if sentence.is_empty() {
        return None;
    }

    Some(sentence.to_string())
}

fn paragraph(text_before: &str, text_after: &str) -> Option<String> {
    let start = text_before
        .rfind("\n\n")
        .map(|index| index + 2)
        .unwrap_or(0);
    let end = text_after.find("\n\n").unwrap_or(text_after.len());

    let paragraph = format!("{}{}", &text_before[start..], &text_after[..end]);
    let paragraph = paragraph.trim();

    if paragraph.is_empty() {
        return None;
    }

    Some(paragraph.to_string())
}

fn bounded_tail(text: &str) -> String {
    let skip = text.chars().count().saturating_sub(SURROUNDING_CHARS);

    text.chars().skip(skip).collect()
}

fn bounded_head(text: &str) -> String {
    text.chars().take(SURROUNDING_CHARS).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_caret_sits_inside_the_word_it_splits() {
        let related = from_caret("the quick bro", "wn fox");

        assert_eq!(related.word.as_deref(), Some("brown"));
        assert_eq!(related.line, "the quick brown fox");
    }

    #[test]
    fn a_caret_between_words_belongs_to_none_of_them() {
        assert_eq!(from_caret("hello ", "world").word.as_deref(), Some("world"));
        assert_eq!(from_caret("hello", " world").word.as_deref(), Some("hello"));
        assert_eq!(from_caret("hello ", " world").word, None);
    }

    #[test]
    fn the_line_stops_at_the_surrounding_newlines() {
        let related = from_caret("first\nsec", "ond\nthird");

        assert_eq!(related.line, "second");
    }

    #[test]
    fn the_sentence_runs_between_its_terminators() {
        let related = from_caret("One. Two is he", "re. Three.");

        assert_eq!(related.sentence.as_deref(), Some("Two is here."));
    }

    #[test]
    fn the_paragraph_runs_between_blank_lines() {
        let related = from_caret("intro\n\nbody sta", "rts here\n\noutro");

        assert_eq!(related.paragraph.as_deref(), Some("body starts here"));
    }

    #[test]
    fn the_surrounding_window_is_bounded() {
        let related = from_caret(&"a".repeat(900), &"b".repeat(900));

        assert_eq!(related.before.chars().count(), SURROUNDING_CHARS);
        assert_eq!(related.after.chars().count(), SURROUNDING_CHARS);
    }

    #[test]
    fn an_empty_field_relates_to_nothing() {
        let related = from_caret("", "");

        assert_eq!(related.word, None);
        assert_eq!(related.line, "");
        assert_eq!(related.sentence, None);
        assert_eq!(related.paragraph, None);
    }
}
