use crate::model::ContextSnapshot;

/// Installs the callback the engine hands every snapshot to. Answers `false` when a callback is
/// already installed, or when the platform has no engine.
#[cfg(target_os = "macos")]
pub fn listen(callback: impl Fn(ContextSnapshot) + Send + Sync + 'static) -> bool {
    crate::macos::listen(callback)
}

#[cfg(not(target_os = "macos"))]
pub fn listen(_callback: impl Fn(ContextSnapshot) + Send + Sync + 'static) -> bool {
    false
}

/// Drives the engine forward: re-targets it at the frontmost application and publishes a snapshot.
/// Between ticks the engine also publishes on its own, whenever the focused element's value,
/// selection or window changes.
///
/// On macOS this must be called on the thread owning the main run loop, because that is where the
/// accessibility observer's notifications are delivered.
#[cfg(target_os = "macos")]
pub fn tick() {
    crate::macos::tick();
}

#[cfg(not(target_os = "macos"))]
pub fn tick() {}

/// Detaches the engine from the application it is watching. Called on the same thread as `tick`.
#[cfg(target_os = "macos")]
pub fn stop() {
    crate::macos::stop();
}

#[cfg(not(target_os = "macos"))]
pub fn stop() {}
