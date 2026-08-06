pub mod error;
pub mod model;
pub mod provider;

pub use error::{Result, SystexError};
pub use model::{
    AppInfo, CaretContext, ContextSnapshot, ElementInfo, Point, PointerContext, Rect, WindowInfo,
    now_ms,
};
pub use provider::{ContextProvider, MockProvider, SystemProvider};

pub fn provider_by_name(name: &str) -> Result<Box<dyn ContextProvider>> {
    if name == "system" {
        return Ok(Box::new(SystemProvider::new()));
    }
    if name == "mock" {
        return Ok(Box::new(MockProvider::new()));
    }
    Err(SystexError::Capture(format!("unknown provider `{name}`")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_provider_captures_a_full_snapshot() {
        let snapshot = MockProvider::new().capture().expect("mock capture works");

        assert_eq!(snapshot.provider, "mock");
        assert!(snapshot.focused_app.is_some());
        assert!(snapshot.focused_window.is_some());
        assert!(snapshot.caret.is_some());
        assert!(snapshot.pointer.is_some());
    }

    #[test]
    fn system_provider_is_not_implemented_yet() {
        let error = SystemProvider::new()
            .capture()
            .expect_err("no system capture yet");

        assert!(matches!(
            error,
            SystexError::Unimplemented(_) | SystexError::UnsupportedPlatform(_)
        ));
    }

    #[test]
    fn unknown_provider_name_is_rejected() {
        assert!(provider_by_name("mock").is_ok());
        assert!(provider_by_name("system").is_ok());
        assert!(provider_by_name("nope").is_err());
    }
}
