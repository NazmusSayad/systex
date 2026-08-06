use tauri::{App, AppHandle, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

pub const LABEL: &str = "overlay";

pub fn create(app: &App) -> tauri::Result<WebviewWindow> {
    let monitor = app
        .primary_monitor()?
        .expect("there is at least one monitor attached");
    let scale = monitor.scale_factor();
    let size = monitor.size().to_logical::<f64>(scale);
    let position = monitor.position().to_logical::<f64>(scale);

    let window = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::default())
        .title("Systex Overlay")
        .inner_size(size.width, size.height)
        .position(position.x, position.y)
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .closable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible_on_all_workspaces(true)
        .focused(false)
        .visible(false)
        .build()?;

    window.set_ignore_cursor_events(true)?;

    #[cfg(target_os = "macos")]
    to_panel(&window);

    Ok(window)
}

#[cfg(target_os = "macos")]
#[allow(deprecated)]
fn to_panel(window: &WebviewWindow) {
    use tauri_nspanel::WebviewWindowExt;
    use tauri_nspanel::cocoa::appkit::NSWindowCollectionBehavior;

    const NS_WINDOW_STYLE_MASK_NON_ACTIVATING_PANEL: i32 = 1 << 7;
    const NS_SCREEN_SAVER_WINDOW_LEVEL: i32 = 1000;

    let panel = window
        .to_panel()
        .expect("the overlay converts to an NSPanel");

    panel.set_level(NS_SCREEN_SAVER_WINDOW_LEVEL);
    panel.set_style_mask(NS_WINDOW_STYLE_MASK_NON_ACTIVATING_PANEL);
    panel.set_collection_behaviour(
        NSWindowCollectionBehavior::NSWindowCollectionBehaviorCanJoinAllSpaces
            | NSWindowCollectionBehavior::NSWindowCollectionBehaviorStationary
            | NSWindowCollectionBehavior::NSWindowCollectionBehaviorFullScreenAuxiliary
            | NSWindowCollectionBehavior::NSWindowCollectionBehaviorIgnoresCycle,
    );
    panel.set_floating_panel(true);
    panel.set_becomes_key_only_if_needed(true);
    panel.set_hides_on_deactivate(false);
    panel.set_ignore_mouse_events(true);
    panel.set_accepts_mouse_moved_events(false);
    panel.set_opaque(false);
    panel.set_has_shadow(false);
}

#[cfg(target_os = "macos")]
pub fn set_visible(app: &AppHandle, visible: bool) -> tauri::Result<()> {
    use tauri_nspanel::ManagerExt;

    let panel = app
        .get_webview_panel(LABEL)
        .expect("the overlay panel is registered");

    if visible {
        panel.order_front_regardless();
        return Ok(());
    }

    panel.order_out(None);
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn set_visible(app: &AppHandle, visible: bool) -> tauri::Result<()> {
    use tauri::Manager;

    let window = app
        .get_webview_window(LABEL)
        .expect("the overlay window exists");

    if visible {
        window.show()?;
        window.set_always_on_top(true)?;
        return Ok(());
    }

    window.hide()?;
    Ok(())
}
