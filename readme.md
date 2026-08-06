# systex

Capture the context surrounding the cursor, and render it as a click-through overlay.

- `src/` — the `systex` library: the snapshot model, the permission checks, and `SystemProvider`,
  whose `capture` is the one piece still unimplemented.
- `view/` — a Tauri v2 + React app. It has no normal window: a full-display, transparent,
  click-through overlay plus a tray icon that holds every config.

## Run

```sh
make install
make dev
```

The app lives in the tray only — there is no dock icon and no focusable window.

## The overlay

Covers the primary display, is always on top, and never takes clicks or focus:

- macOS — the window is converted to a non-activating `NSPanel` (`tauri-nspanel`) at screen-saver
  level, joining all spaces and floating over fullscreen apps.
- other platforms — always-on-top plus `set_ignore_cursor_events`.

It draws the focused window frame, the focused element frame, the caret, a pointer marker, and a
HUD card with the captured values.

## The tray menu

- **Show overlay** — toggle it off without quitting.
- **Refresh rate** — 200ms / 500ms / 1s / 2s.
- **Opacity** — 40% / 60% / 80% / 100%.
- **Permissions** — one submenu per permission showing its live status, with *Grant permission*
  (triggers the real system prompt) and *Open system settings* (jumps to the right pane).
- **Quit Systex**.

## Library

```rust
use systex::SystemProvider;

let provider = SystemProvider::new();

if provider.is_available() {
    let snapshot = provider.capture()?;
    println!("{:?}", snapshot.caret);
}
```

## Implementing the capture

`SystemProvider::capture` in `src/provider.rs` returns `PermissionDenied` without accessibility
access, and `Unimplemented` otherwise. Fill it in with:

- macOS — Accessibility (`AXUIElement`).
- Windows — UI Automation.
- Linux — AT-SPI.

Return a `ContextSnapshot` with `focused_app`, `focused_window`, `caret` and `pointer` filled in.
Until then the overlay shows the error the provider returns.

Permission checks in `src/permissions.rs` are real on macOS (`AXIsProcessTrusted`,
`CGPreflightListenEventAccess`, `CGPreflightScreenCaptureAccess`) and report `Unknown` elsewhere.

## Check

```sh
make check
```
