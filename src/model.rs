use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppInfo {
    pub name: String,
    pub bundle_id: Option<String>,
    pub path: Option<String>,
    pub pid: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowInfo {
    pub title: Option<String>,
    pub document: Option<String>,
    pub bounds: Option<Rect>,
    pub minimized: bool,
    pub main: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElementInfo {
    pub role: String,
    pub subrole: Option<String>,
    pub role_description: Option<String>,
    pub label: Option<String>,
    pub help: Option<String>,
    pub placeholder: Option<String>,
    pub identifier: Option<String>,
    pub value: Option<String>,
    pub bounds: Option<Rect>,
    pub editable: bool,
    pub enabled: bool,
    pub character_count: Option<usize>,
    pub attributes: Vec<String>,
    pub parameterized_attributes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaretContext {
    pub element: Option<ElementInfo>,
    pub text_before: String,
    pub text_after: String,
    pub selected_text: Option<String>,
    pub selection_start: usize,
    pub selection_length: usize,
    pub line: Option<u32>,
    pub column: Option<u32>,
    pub bounds: Option<Rect>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PointerContext {
    pub position: Point,
    pub element: Option<ElementInfo>,
    pub app: Option<AppInfo>,
    pub window: Option<WindowInfo>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelatedContent {
    pub word: Option<String>,
    pub line: String,
    pub sentence: Option<String>,
    pub paragraph: Option<String>,
    pub before: String,
    pub after: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WordBox {
    pub text: String,
    pub start: Option<usize>,
    pub length: Option<usize>,
    pub rect: Rect,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextSnapshot {
    pub captured_at_ms: u128,
    pub provider: String,
    pub focused_app: Option<AppInfo>,
    pub focused_window: Option<WindowInfo>,
    pub caret: Option<CaretContext>,
    pub pointer: Option<PointerContext>,
    pub related: Option<RelatedContent>,
    pub window_text: Option<String>,
    pub words: Vec<WordBox>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureOptions {
    pub pointer: bool,
    pub attribute_names: bool,
    pub window_text: bool,
    pub words: bool,
}

impl CaptureOptions {
    pub fn fast() -> Self {
        Self {
            pointer: true,
            attribute_names: false,
            window_text: false,
            words: false,
        }
    }

    pub fn everything() -> Self {
        Self {
            pointer: true,
            attribute_names: true,
            window_text: true,
            words: true,
        }
    }
}

impl Default for CaptureOptions {
    fn default() -> Self {
        Self::fast()
    }
}

pub fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock is before the unix epoch")
        .as_millis()
}
