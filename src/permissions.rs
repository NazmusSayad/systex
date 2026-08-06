use serde::{Deserialize, Serialize};

use crate::error::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    Accessibility,
    InputMonitoring,
    ScreenRecording,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionStatus {
    Granted,
    Denied,
    Unknown,
}

pub const ALL: [Permission; 3] = [
    Permission::Accessibility,
    Permission::InputMonitoring,
    Permission::ScreenRecording,
];

impl Permission {
    pub fn key(self) -> &'static str {
        if self == Permission::Accessibility {
            return "accessibility";
        }
        if self == Permission::InputMonitoring {
            return "input_monitoring";
        }
        if self == Permission::ScreenRecording {
            return "screen_recording";
        }
        unreachable!("every permission has a key")
    }

    pub fn label(self) -> &'static str {
        if self == Permission::Accessibility {
            return "Accessibility";
        }
        if self == Permission::InputMonitoring {
            return "Input Monitoring";
        }
        if self == Permission::ScreenRecording {
            return "Screen Recording";
        }
        unreachable!("every permission has a label")
    }

    pub fn from_key(key: &str) -> Option<Self> {
        if key == "accessibility" {
            return Some(Permission::Accessibility);
        }
        if key == "input_monitoring" {
            return Some(Permission::InputMonitoring);
        }
        if key == "screen_recording" {
            return Some(Permission::ScreenRecording);
        }
        None
    }
}

impl PermissionStatus {
    pub fn label(self) -> &'static str {
        if self == PermissionStatus::Granted {
            return "granted";
        }
        if self == PermissionStatus::Denied {
            return "not granted";
        }
        if self == PermissionStatus::Unknown {
            return "unknown";
        }
        unreachable!("every status has a label")
    }
}

#[cfg(target_os = "macos")]
pub fn status(permission: Permission) -> PermissionStatus {
    macos::status(permission)
}

#[cfg(not(target_os = "macos"))]
pub fn status(_permission: Permission) -> PermissionStatus {
    PermissionStatus::Unknown
}

#[cfg(target_os = "macos")]
pub fn request(permission: Permission) -> Result<PermissionStatus> {
    macos::request(permission)
}

#[cfg(not(target_os = "macos"))]
pub fn request(_permission: Permission) -> Result<PermissionStatus> {
    Err(crate::error::SystexError::UnsupportedPlatform(
        std::env::consts::OS,
    ))
}

#[cfg(target_os = "macos")]
pub fn open_settings(permission: Permission) -> Result<()> {
    macos::open_settings(permission)
}

#[cfg(not(target_os = "macos"))]
pub fn open_settings(_permission: Permission) -> Result<()> {
    Err(crate::error::SystexError::UnsupportedPlatform(
        std::env::consts::OS,
    ))
}

#[cfg(target_os = "macos")]
mod macos {
    use core_foundation::base::TCFType;
    use core_foundation::boolean::CFBoolean;
    use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
    use core_foundation::string::{CFString, CFStringRef};

    use super::{Permission, PermissionStatus};
    use crate::error::{Result, SystexError};

    #[link(name = "ApplicationServices", kind = "framework")]
    unsafe extern "C" {
        static kAXTrustedCheckOptionPrompt: CFStringRef;

        fn AXIsProcessTrusted() -> u8;

        fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> u8;
    }

    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGPreflightListenEventAccess() -> bool;

        fn CGRequestListenEventAccess() -> bool;

        fn CGPreflightScreenCaptureAccess() -> bool;

        fn CGRequestScreenCaptureAccess() -> bool;
    }

    pub fn status(permission: Permission) -> PermissionStatus {
        let granted = if permission == Permission::Accessibility {
            unsafe { AXIsProcessTrusted() != 0 }
        } else if permission == Permission::InputMonitoring {
            unsafe { CGPreflightListenEventAccess() }
        } else if permission == Permission::ScreenRecording {
            unsafe { CGPreflightScreenCaptureAccess() }
        } else {
            unreachable!("every permission is checked")
        };

        if granted {
            return PermissionStatus::Granted;
        }

        PermissionStatus::Denied
    }

    pub fn request(permission: Permission) -> Result<PermissionStatus> {
        if permission == Permission::Accessibility {
            let options = unsafe {
                CFDictionary::from_CFType_pairs(&[(
                    CFString::wrap_under_get_rule(kAXTrustedCheckOptionPrompt),
                    CFBoolean::true_value(),
                )])
            };

            unsafe { AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef()) };

            return Ok(status(permission));
        }

        if permission == Permission::InputMonitoring {
            unsafe { CGRequestListenEventAccess() };

            return Ok(status(permission));
        }

        if permission == Permission::ScreenRecording {
            unsafe { CGRequestScreenCaptureAccess() };

            return Ok(status(permission));
        }

        unreachable!("every permission can be requested")
    }

    pub fn open_settings(permission: Permission) -> Result<()> {
        let pane = if permission == Permission::Accessibility {
            "Privacy_Accessibility"
        } else if permission == Permission::InputMonitoring {
            "Privacy_ListenEvent"
        } else if permission == Permission::ScreenRecording {
            "Privacy_ScreenCapture"
        } else {
            unreachable!("every permission has a settings pane")
        };

        std::process::Command::new("open")
            .arg(format!(
                "x-apple.systempreferences:com.apple.preference.security?{pane}"
            ))
            .status()
            .map_err(|error| SystexError::Capture(error.to_string()))?;

        Ok(())
    }
}
