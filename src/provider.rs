use crate::error::{Result, SystexError};
use crate::model::{CaptureOptions, ContextSnapshot, WordBox};
use crate::permissions::{self, Permission, PermissionStatus};

pub struct SystemProvider;

impl SystemProvider {
    pub fn new() -> Self {
        Self
    }

    pub fn is_available(&self) -> bool {
        permissions::status(Permission::Accessibility) == PermissionStatus::Granted
    }

    pub fn capture(&self) -> Result<ContextSnapshot> {
        self.capture_with(CaptureOptions::fast())
    }

    #[cfg(target_os = "macos")]
    pub fn capture_with(&self, options: CaptureOptions) -> Result<ContextSnapshot> {
        if !self.is_available() {
            return Err(SystexError::PermissionDenied);
        }

        crate::macos::capture(options)
    }

    #[cfg(not(target_os = "macos"))]
    pub fn capture_with(&self, _options: CaptureOptions) -> Result<ContextSnapshot> {
        Err(unsupported())
    }

    /// Every word of the focused text field with the rectangle it is drawn in, for overlays that
    /// need to point at individual words rather than at the caret.
    #[cfg(target_os = "macos")]
    pub fn word_boxes(&self) -> Result<Vec<WordBox>> {
        if !self.is_available() {
            return Err(SystexError::PermissionDenied);
        }

        crate::macos::word_boxes()
    }

    #[cfg(not(target_os = "macos"))]
    pub fn word_boxes(&self) -> Result<Vec<WordBox>> {
        Err(unsupported())
    }

    /// Reports exactly what the frontmost app exposes, so an app that reports nothing can be told
    /// apart from one being read wrongly.
    #[cfg(target_os = "macos")]
    pub fn debug_dump(&self) -> Result<String> {
        if !self.is_available() {
            return Err(SystexError::PermissionDenied);
        }

        crate::macos::debug_dump()
    }

    #[cfg(not(target_os = "macos"))]
    pub fn debug_dump(&self) -> Result<String> {
        Err(unsupported())
    }
}

impl Default for SystemProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(not(target_os = "macos"))]
fn unsupported() -> SystexError {
    if cfg!(target_os = "windows") {
        return SystexError::Unimplemented("windows ui automation capture");
    }
    if cfg!(target_os = "linux") {
        return SystexError::Unimplemented("linux at-spi capture");
    }

    SystexError::UnsupportedPlatform(std::env::consts::OS)
}
