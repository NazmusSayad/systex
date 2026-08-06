mod elem;
mod engine;
mod words;

use std::collections::HashSet;
use std::ptr;

use accessibility_sys::{
    AXUIElementCopyElementAtPosition, AXUIElementCreateApplication, AXUIElementCreateSystemWide,
    AXUIElementGetPid, AXUIElementRef, kAXDocumentAttribute, kAXFocusedUIElementAttribute,
    kAXFocusedWindowAttribute, kAXMainAttribute, kAXMinimizedAttribute, kAXRoleAttribute,
    kAXSelectedTextAttribute, kAXTitleAttribute, kAXValueAttribute, kAXWindowsAttribute,
};
use core_graphics::event::CGEvent;
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use objc2_app_kit::{NSRunningApplication, NSWorkspace};

use crate::error::{Result, SystexError};
use crate::model::{
    AppInfo, CaptureOptions, CaretContext, ContextSnapshot, Point, PointerContext, WindowInfo,
    WordBox, now_ms,
};

pub use engine::{listen, stop, tick};

use elem::{APP_TIMEOUT, ELEMENT_TIMEOUT, Elem};

/// A whole-window text scrape can walk a browser's entire DOM, so it is bounded on every axis.
const MAX_DEPTH: usize = 40;
const MAX_NODES: usize = 1500;
const MAX_CHARS: usize = 20_000;

pub fn capture(options: CaptureOptions) -> Result<ContextSnapshot> {
    let Some(focused_app) = frontmost_app() else {
        return Err(SystexError::NothingFocused);
    };
    let pid = focused_app.pid;

    let Some(app) =
        Elem::from_create_rule(unsafe { AXUIElementCreateApplication(pid) }, APP_TIMEOUT)
    else {
        return Err(SystexError::Capture(format!(
            "no accessibility element for process {pid}"
        )));
    };

    app.enable_accessibility_tree();

    let window = app
        .element_attribute(kAXFocusedWindowAttribute)
        .or_else(|| first_window(&app));
    let focused = focused_element(&app);
    let caret = focused
        .as_ref()
        .map(|element| caret_context(element, options.attribute_names));

    Ok(ContextSnapshot {
        captured_at_ms: now_ms(),
        provider: "macos-accessibility".to_string(),
        focused_app: Some(focused_app),
        focused_window: window.as_ref().map(window_info),
        related: caret
            .as_ref()
            .map(|caret| crate::related::from_caret(&caret.text_before, &caret.text_after)),
        caret,
        pointer: if options.pointer {
            pointer_context(options.attribute_names)
        } else {
            None
        },
        window_text: if options.window_text {
            window_text(&app, window.as_ref())
        } else {
            None
        },
        words: if options.words {
            match focused.as_ref() {
                Some(element) => words::word_boxes(element),
                None => Vec::new(),
            }
        } else {
            Vec::new()
        },
    })
}

pub fn debug_dump() -> Result<String> {
    let Some(focused_app) = frontmost_app() else {
        return Err(SystexError::NothingFocused);
    };
    let pid = focused_app.pid;

    let Some(app) =
        Elem::from_create_rule(unsafe { AXUIElementCreateApplication(pid) }, APP_TIMEOUT)
    else {
        return Err(SystexError::Capture(format!(
            "no accessibility element for process {pid}"
        )));
    };

    let mut out = String::new();

    out.push_str(&format!("app: {focused_app:?}\n"));
    out.push_str(&format!(
        "AXManualAccessibility -> AXError {}\n",
        app.set_flag("AXManualAccessibility")
    ));
    out.push_str(&format!(
        "AXEnhancedUserInterface -> AXError {}\n",
        app.set_flag("AXEnhancedUserInterface")
    ));
    out.push_str(&format!("app attributes: {:?}\n", app.attribute_names()));

    let Some(focused) = focused_element(&app) else {
        out.push_str("no focused element\n");
        return Ok(out);
    };

    out.push_str(&format!(
        "focused role: {:?} subrole: {:?}\n",
        focused.string_attribute(kAXRoleAttribute),
        focused.string_attribute("AXSubrole")
    ));
    out.push_str(&format!(
        "focused attributes: {:?}\n",
        focused.attribute_names()
    ));
    out.push_str(&format!(
        "focused parameterized: {:?}\n",
        focused.parameterized_attribute_names()
    ));
    out.push_str(&format!(
        "AXValue: {:?}\n",
        focused
            .string_attribute(kAXValueAttribute)
            .map(|value| value.chars().take(120).collect::<String>())
    ));
    out.push_str(&format!("AXSelectedTextRange: {:?}\n", focused.selection()));

    if let Some((start, _)) = focused.selection() {
        out.push_str(&format!("caret rect: {:?}\n", focused.caret_rect(start)));
    }

    Ok(out)
}

/// Chrome reports nothing for the system-wide element's `AXFocusedUIElement` but does answer the
/// same question on its own application element, so ask the app first.
fn focused_element(app: &Elem) -> Option<Elem> {
    app.element_attribute(kAXFocusedUIElementAttribute)
        .or_else(|| {
            Elem::from_create_rule(unsafe { AXUIElementCreateSystemWide() }, ELEMENT_TIMEOUT)?
                .element_attribute(kAXFocusedUIElementAttribute)
        })
        .map(|element| element.descend_to_focused())
}

fn frontmost_app() -> Option<AppInfo> {
    let app = NSWorkspace::sharedWorkspace().frontmostApplication()?;

    Some(running_app_info(&app))
}

fn running_app_info(app: &NSRunningApplication) -> AppInfo {
    AppInfo {
        name: app
            .localizedName()
            .map(|name| name.to_string())
            .unwrap_or_default(),
        bundle_id: app.bundleIdentifier().map(|id| id.to_string()),
        path: app
            .bundleURL()
            .and_then(|url| url.path())
            .map(|path| path.to_string()),
        pid: app.processIdentifier(),
    }
}

fn window_info(window: &Elem) -> WindowInfo {
    WindowInfo {
        title: window.string_attribute(kAXTitleAttribute),
        document: window.string_attribute(kAXDocumentAttribute),
        bounds: window.frame(),
        minimized: window
            .bool_attribute(kAXMinimizedAttribute)
            .unwrap_or(false),
        main: window.bool_attribute(kAXMainAttribute).unwrap_or(false),
    }
}

fn caret_context(element: &Elem, attribute_names: bool) -> CaretContext {
    let info = element.info(attribute_names);
    let value = info.value.clone().unwrap_or_default();
    let units: Vec<u16> = value.encode_utf16().collect();
    let selection = element.selection();

    let Some((start, length)) = selection else {
        return CaretContext {
            element: Some(info),
            text_before: String::new(),
            text_after: value,
            selected_text: element.string_attribute(kAXSelectedTextAttribute),
            selection_start: 0,
            selection_length: 0,
            line: None,
            column: None,
            bounds: None,
        };
    };

    let head = start.min(units.len());
    let tail = start.saturating_add(length).min(units.len());
    let text_before = String::from_utf16_lossy(&units[..head]);
    let selected = String::from_utf16_lossy(&units[head..tail]);

    CaretContext {
        line: Some(text_before.matches('\n').count() as u32 + 1),
        column: Some(
            text_before
                .rsplit('\n')
                .next()
                .expect("rsplit always yields a segment")
                .chars()
                .count() as u32
                + 1,
        ),
        text_after: String::from_utf16_lossy(&units[tail..]),
        selected_text: if selected.is_empty() {
            element.string_attribute(kAXSelectedTextAttribute)
        } else {
            Some(selected)
        },
        bounds: element.caret_rect(start),
        selection_start: start,
        selection_length: length,
        element: Some(info),
        text_before,
    }
}

fn pointer_context(attribute_names: bool) -> Option<PointerContext> {
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState).ok()?;
    let location = CGEvent::new(source).ok()?.location();
    let position = Point {
        x: location.x,
        y: location.y,
    };

    let Some(system) =
        Elem::from_create_rule(unsafe { AXUIElementCreateSystemWide() }, ELEMENT_TIMEOUT)
    else {
        return Some(PointerContext {
            position,
            element: None,
            app: None,
            window: None,
        });
    };

    let mut raw: AXUIElementRef = ptr::null_mut();
    let err = unsafe {
        AXUIElementCopyElementAtPosition(system.0, location.x as f32, location.y as f32, &mut raw)
    };
    let hit = if err == 0 {
        Elem::from_create_rule(raw, ELEMENT_TIMEOUT)
    } else {
        None
    };

    let Some(hit) = hit else {
        return Some(PointerContext {
            position,
            element: None,
            app: None,
            window: None,
        });
    };

    let mut pid = 0;
    unsafe { AXUIElementGetPid(hit.0, &mut pid) };

    Some(PointerContext {
        position,
        window: enclosing_window(&hit).as_ref().map(window_info),
        app: NSRunningApplication::runningApplicationWithProcessIdentifier(pid)
            .map(|app| running_app_info(&app)),
        element: Some(hit.info(attribute_names)),
    })
}

fn enclosing_window(element: &Elem) -> Option<Elem> {
    let mut current = Elem::from_get_rule(element.0, ELEMENT_TIMEOUT)?;

    for _ in 0..MAX_DEPTH {
        if current.string_attribute(kAXRoleAttribute).as_deref() == Some("AXWindow") {
            return Some(current);
        }

        current = current.element_attribute("AXParent")?;
    }

    None
}

/// The visible text of the whole front window, for the cases where the caret alone is not the
/// context: what the user is reading, not just what they are typing. A web area is preferred when
/// there is one, because in a browser the chrome around the page is noise.
fn window_text(app: &Elem, window: Option<&Elem>) -> Option<String> {
    let mut best = String::new();

    // Chromium finishes building its tree some time after the accessibility flags are set, so an
    // empty first read is retried rather than reported as "this app exposes nothing".
    for attempt in 0..5 {
        let web = focused_element(app).and_then(|focused| ancestor_web_area(&focused));
        let root = match (&web, window) {
            (Some(web), _) => web,
            (None, Some(window)) => window,
            (None, None) => app,
        };

        let mut out = String::new();
        let mut seen = HashSet::new();
        let mut nodes = 0usize;

        collect_text(root, 0, &mut nodes, &mut seen, &mut out);

        if out.len() > best.len() {
            best = out;
        }
        if nodes > 1 {
            break;
        }
        if attempt < 4 {
            std::thread::sleep(std::time::Duration::from_millis(120));
        }
    }

    if best.trim().is_empty() {
        return None;
    }

    Some(best)
}

fn collect_text(
    element: &Elem,
    depth: usize,
    nodes: &mut usize,
    seen: &mut HashSet<String>,
    out: &mut String,
) {
    if depth > MAX_DEPTH || *nodes > MAX_NODES || out.len() > MAX_CHARS {
        return;
    }

    *nodes += 1;

    for name in [kAXTitleAttribute, kAXValueAttribute, "AXDescription"] {
        if let Some(text) = element.string_attribute(name) {
            let text = text.trim();

            if !text.is_empty() && seen.insert(text.to_string()) {
                out.push_str(text);
                out.push('\n');
            }
        }
    }

    for child in element.children() {
        if *nodes > MAX_NODES || out.len() > MAX_CHARS {
            break;
        }

        collect_text(&child, depth + 1, nodes, seen, out);
    }
}

fn ancestor_web_area(element: &Elem) -> Option<Elem> {
    let mut current = Elem::from_get_rule(element.0, ELEMENT_TIMEOUT)?;

    for _ in 0..MAX_DEPTH {
        if current.string_attribute(kAXRoleAttribute).as_deref() == Some("AXWebArea") {
            return Some(current);
        }

        current = current.element_attribute("AXParent")?;
    }

    None
}

fn first_window(app: &Elem) -> Option<Elem> {
    app.children_of(kAXWindowsAttribute).into_iter().next()
}

pub fn word_boxes() -> Result<Vec<WordBox>> {
    let Some(pid) = frontmost_app().map(|app| app.pid) else {
        return Err(SystexError::NothingFocused);
    };
    let Some(app) =
        Elem::from_create_rule(unsafe { AXUIElementCreateApplication(pid) }, APP_TIMEOUT)
    else {
        return Err(SystexError::Capture(format!(
            "no accessibility element for process {pid}"
        )));
    };
    let Some(focused) = focused_element(&app) else {
        return Err(SystexError::NothingFocused);
    };

    Ok(words::word_boxes(&focused))
}
