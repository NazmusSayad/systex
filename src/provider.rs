use crate::error::{Result, SystexError};
use crate::model::ContextSnapshot;
use crate::permissions::{self, Permission, PermissionStatus};

pub struct SystemProvider;

impl SystemProvider {
    pub fn new() -> Self {
        Self
    }

    pub fn is_available(&self) -> bool {
        permissions::status(Permission::Accessibility) == PermissionStatus::Granted
    }

    #[cfg(target_os = "macos")]
    pub fn capture(&self) -> Result<ContextSnapshot> {
        if !self.is_available() {
            return Err(SystexError::PermissionDenied);
        }

        crate::macos::capture()
    }

    #[cfg(not(target_os = "macos"))]
    pub fn capture(&self) -> Result<ContextSnapshot> {
        if cfg!(target_os = "windows") {
            return Err(SystexError::Unimplemented("windows ui automation capture"));
        }
        if cfg!(target_os = "linux") {
            return Err(SystexError::Unimplemented("linux at-spi capture"));
        }

        Err(SystexError::UnsupportedPlatform(std::env::consts::OS))
    }
}

impl Default for SystemProvider {
    fn default() -> Self {
        Self::new()
    }
}
