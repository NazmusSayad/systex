# systex

Capture the context surrounding the cursor, and inspect it in a window.

- `src/` — the `systex` library: snapshot model, error type, and the `ContextProvider` trait.
  `MockProvider` returns sample data; `SystemProvider` is the real platform capture and is not
  implemented yet.
- `view/` — a Tauri v2 + React app that polls the library and renders the snapshot.

## Run

```sh
make install
make dev
```

## Library

```rust
use systex::{ContextProvider, MockProvider};

let snapshot = MockProvider::new().capture()?;
println!("{:?}", snapshot.caret);
```

## Implementing the real capture

Fill in `SystemProvider::capture` in `src/provider.rs`:

- macOS — Accessibility (`AXUIElement`), requires the accessibility permission.
- Windows — UI Automation.
- Linux — AT-SPI.

Return a `ContextSnapshot` with `focused_app`, `focused_window`, `caret` and `pointer` filled in,
and flip `is_available` to reflect whether the permission is actually granted.

## Check

```sh
make check
```
