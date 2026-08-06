use accessibility_sys::{kAXRoleAttribute, kAXValueAttribute};

use crate::macos::elem::{Elem, MAX_CARET_HEIGHT};
use crate::model::{Rect, WordBox};

const MAX_WORD_BOXES: usize = 80;
const MAX_RUN_DEPTH: usize = 14;

/// Every word costs an accessibility round-trip, so this is capped rather than mapping a whole
/// document. Offsets are UTF-16 code units, matching both the AX API and JavaScript string indices.
pub fn word_boxes(element: &Elem) -> Vec<WordBox> {
    // Fast path: the field itself can map ranges to rectangles, which plain inputs and native text
    // views do. Offsets are then real offsets into its own value.
    let value = element
        .string_attribute(kAXValueAttribute)
        .unwrap_or_default();
    let mut boxes: Vec<WordBox> = words_with_offsets(&value)
        .into_iter()
        .take(MAX_WORD_BOXES)
        .filter_map(|(start, length, text)| {
            let rect = element.bounds_for_range(start, length)?;

            if rect.width <= 0.0 || rect.height <= 0.0 || rect.height > MAX_CARET_HEIGHT {
                return None;
            }

            Some(WordBox {
                text,
                start: Some(start),
                length: Some(length),
                rect,
            })
        })
        .collect();

    if !boxes.is_empty() {
        return boxes;
    }

    // Slow path: rich editors — Twitter's composer, anything Draft.js-like — expose no text on the
    // focused element itself; it lives in `AXStaticText` descendants, one per styled run.
    let mut runs = Vec::new();

    collect_text_runs(element, &mut runs, 0);

    // The rects come from the child runs, but the offsets have to be into the focused element's own
    // value. Runs arrive in document order, so each word is located by scanning forward from the
    // previous match.
    let value_units: Vec<u16> = value.encode_utf16().collect();
    let mut search_from = 0usize;

    for (text, frame, run) in runs {
        let total = text.encode_utf16().count().max(1) as f64;

        for (start, length, word) in words_with_offsets(&text) {
            if boxes.len() >= MAX_WORD_BOXES {
                break;
            }

            let word_units: Vec<u16> = word.encode_utf16().collect();
            let offset = find_utf16(&value_units, &word_units, search_from);

            if let Some(offset) = offset {
                search_from = offset + word_units.len();
            }

            // A run may still answer ranges even when its container did not. Failing that, the run
            // is a single line of uniform text, so its width divides evenly across the characters.
            let rect = match run.bounds_for_range(start, length) {
                Some(rect) if rect.width > 0.0 && rect.height > 0.0 => rect,
                _ => Rect {
                    x: frame.x + frame.width * (start as f64 / total),
                    y: frame.y,
                    width: frame.width * (length as f64 / total),
                    height: frame.height,
                },
            };

            boxes.push(WordBox {
                start: offset,
                length: offset.map(|_| word_units.len()),
                text: word,
                rect,
            });
        }
    }

    boxes
}

fn collect_text_runs(element: &Elem, out: &mut Vec<(String, Rect, Elem)>, depth: usize) {
    if out.len() >= MAX_WORD_BOXES || depth > MAX_RUN_DEPTH {
        return;
    }

    let role = element
        .string_attribute(kAXRoleAttribute)
        .unwrap_or_default();

    if role == "AXStaticText" {
        let value = element
            .string_attribute(kAXValueAttribute)
            .unwrap_or_default();

        if let Some(frame) = element.frame()
            && !value.trim().is_empty()
            && frame.width > 0.0
            && frame.height > 0.0
            && let Some(run) = Elem::from_get_rule(element.0, crate::macos::elem::ELEMENT_TIMEOUT)
        {
            out.push((value, frame, run));
        }

        return;
    }

    for child in element.children() {
        collect_text_runs(&child, out, depth + 1);
    }
}

/// Splits `text` into words, yielding each with its UTF-16 offset and length.
fn words_with_offsets(text: &str) -> Vec<(usize, usize, String)> {
    let mut words = Vec::new();
    let mut offset = 0usize;
    let mut start = 0usize;
    let mut current = String::new();

    for character in text.chars().chain(std::iter::once('\n')) {
        if character.is_alphanumeric() || character == '\'' || character == '-' {
            if current.is_empty() {
                start = offset;
            }

            current.push(character);
        } else if !current.is_empty() {
            words.push((start, offset - start, std::mem::take(&mut current)));
        }

        offset += character.len_utf16();
    }

    words
}

fn find_utf16(haystack: &[u16], needle: &[u16], from: usize) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() || from > haystack.len() - needle.len() {
        return None;
    }

    (from..=haystack.len() - needle.len())
        .find(|&index| &haystack[index..index + needle.len()] == needle)
}
