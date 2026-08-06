use crate::error::{Result, SystexError};
use crate::model::{
    AppInfo, CaretContext, ContextSnapshot, ElementInfo, Point, PointerContext, Rect, WindowInfo,
    now_ms,
};

pub trait ContextProvider: Send + Sync {
    fn name(&self) -> &'static str;

    fn is_available(&self) -> bool;

    fn capture(&self) -> Result<ContextSnapshot>;
}

pub struct SystemProvider;

impl SystemProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SystemProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextProvider for SystemProvider {
    fn name(&self) -> &'static str {
        "system"
    }

    fn is_available(&self) -> bool {
        false
    }

    fn capture(&self) -> Result<ContextSnapshot> {
        if cfg!(target_os = "macos") {
            return Err(SystexError::Unimplemented("macos accessibility capture"));
        }
        if cfg!(target_os = "windows") {
            return Err(SystexError::Unimplemented("windows ui automation capture"));
        }
        if cfg!(target_os = "linux") {
            return Err(SystexError::Unimplemented("linux at-spi capture"));
        }
        Err(SystexError::UnsupportedPlatform(std::env::consts::OS))
    }
}

pub struct MockProvider;

impl MockProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MockProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextProvider for MockProvider {
    fn name(&self) -> &'static str {
        "mock"
    }

    fn is_available(&self) -> bool {
        true
    }

    fn capture(&self) -> Result<ContextSnapshot> {
        let elapsed = now_ms() % 10_000;

        Ok(ContextSnapshot {
            captured_at_ms: now_ms(),
            provider: self.name().to_string(),
            focused_app: Some(AppInfo {
                name: "TextEdit".to_string(),
                bundle_id: Some("com.apple.TextEdit".to_string()),
                pid: 4242,
            }),
            focused_window: Some(WindowInfo {
                title: Some("notes.txt".to_string()),
                bounds: Some(Rect {
                    x: 120.0,
                    y: 80.0,
                    width: 980.0,
                    height: 720.0,
                }),
            }),
            caret: Some(CaretContext {
                element: Some(ElementInfo {
                    role: "AXTextArea".to_string(),
                    label: Some("Document body".to_string()),
                    value: None,
                    bounds: Some(Rect {
                        x: 140.0,
                        y: 140.0,
                        width: 940.0,
                        height: 640.0,
                    }),
                    editable: true,
                }),
                text_before: "the quick brown fox jumps over the ".to_string(),
                text_after: " dog and keeps running".to_string(),
                selected_text: Some("lazy".to_string()),
                line: Some(12),
                column: Some(35),
                bounds: Some(Rect {
                    x: 402.0,
                    y: 318.0,
                    width: 2.0,
                    height: 18.0,
                }),
            }),
            pointer: Some(PointerContext {
                position: Point {
                    x: 400.0 + (elapsed as f64 / 40.0),
                    y: 300.0 + (elapsed as f64 / 80.0),
                },
                element: Some(ElementInfo {
                    role: "AXButton".to_string(),
                    label: Some("Save".to_string()),
                    value: None,
                    bounds: Some(Rect {
                        x: 980.0,
                        y: 96.0,
                        width: 72.0,
                        height: 28.0,
                    }),
                    editable: false,
                }),
                app: Some(AppInfo {
                    name: "TextEdit".to_string(),
                    bundle_id: Some("com.apple.TextEdit".to_string()),
                    pid: 4242,
                }),
                window: Some(WindowInfo {
                    title: Some("notes.txt".to_string()),
                    bounds: Some(Rect {
                        x: 120.0,
                        y: 80.0,
                        width: 980.0,
                        height: 720.0,
                    }),
                }),
            }),
        })
    }
}
