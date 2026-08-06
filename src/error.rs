use thiserror::Error;

pub type Result<T> = std::result::Result<T, SystexError>;

#[derive(Debug, Error)]
pub enum SystexError {
    #[error("accessibility permission is not granted")]
    PermissionDenied,

    #[error("nothing is focused right now")]
    NothingFocused,

    #[error("platform `{0}` is not supported")]
    UnsupportedPlatform(&'static str),

    #[error("`{0}` is not implemented yet")]
    Unimplemented(&'static str),

    #[error("capture failed: {0}")]
    Capture(String),
}
