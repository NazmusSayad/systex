pub mod engine;
pub mod error;
#[cfg(target_os = "macos")]
mod macos;
pub mod model;
pub mod permissions;
pub mod provider;
pub mod related;

pub use error::{Result, SystexError};
pub use model::{
    AppInfo, CaptureOptions, CaretContext, ContextSnapshot, ElementInfo, Point, PointerContext,
    Rect, RelatedContent, WindowInfo, WordBox, now_ms,
};
pub use permissions::{Permission, PermissionStatus};
pub use provider::SystemProvider;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_reports_a_snapshot_or_a_clear_error() {
        let provider = SystemProvider::new();

        if !provider.is_available() {
            assert!(matches!(
                provider.capture().expect_err("capture needs permission"),
                SystexError::PermissionDenied | SystexError::UnsupportedPlatform(_)
            ));
            return;
        }

        match provider.capture() {
            Ok(snapshot) => assert!(!snapshot.provider.is_empty()),
            Err(error) => assert!(matches!(
                error,
                SystexError::NothingFocused
                    | SystexError::Capture(_)
                    | SystexError::Unimplemented(_)
            )),
        }
    }

    #[test]
    fn availability_follows_the_accessibility_permission() {
        assert_eq!(
            SystemProvider::new().is_available(),
            permissions::status(Permission::Accessibility) == PermissionStatus::Granted
        );
    }
}
