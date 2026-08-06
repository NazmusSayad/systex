mod overlay;
mod tray;

use std::sync::Mutex;

use serde::Serialize;
use systex::{CaptureOptions, ContextSnapshot, SystemProvider, WordBox};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Route {
    Basic,
    WindowContent,
}

impl Route {
    pub const ALL: [Route; 2] = [Route::Basic, Route::WindowContent];

    pub fn key(self) -> &'static str {
        if self == Route::Basic {
            return "basic";
        }
        if self == Route::WindowContent {
            return "window_content";
        }

        unreachable!("every route has a key")
    }

    pub fn label(self) -> &'static str {
        if self == Route::Basic {
            return "Basic";
        }
        if self == Route::WindowContent {
            return "Window content";
        }

        unreachable!("every route has a label")
    }

    pub fn from_key(key: &str) -> Option<Route> {
        Route::ALL.into_iter().find(|route| route.key() == key)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Settings {
    pub route: Route,
    pub columns: u8,
    pub interval_ms: u64,
    pub opacity: u8,
    pub overlay_visible: bool,
    pub pointer: bool,
    pub attribute_names: bool,
    pub window_text: bool,
    pub words: bool,
}

impl Settings {
    fn options(&self) -> CaptureOptions {
        CaptureOptions {
            pointer: self.pointer,
            attribute_names: self.attribute_names,
            window_text: self.window_text || self.route == Route::WindowContent,
            words: self.words,
        }
    }
}

pub struct State {
    settings: Mutex<Settings>,
}

impl State {
    fn settings(&self) -> Settings {
        self.settings
            .lock()
            .expect("the settings lock is poisoned")
            .clone()
    }

    fn set_route(&self, route: Route) {
        self.settings
            .lock()
            .expect("the settings lock is poisoned")
            .route = route;
    }

    fn set_columns(&self, columns: u8) {
        self.settings
            .lock()
            .expect("the settings lock is poisoned")
            .columns = columns;
    }

    fn set_interval(&self, interval_ms: u64) {
        self.settings
            .lock()
            .expect("the settings lock is poisoned")
            .interval_ms = interval_ms;
    }

    fn set_opacity(&self, opacity: u8) {
        self.settings
            .lock()
            .expect("the settings lock is poisoned")
            .opacity = opacity;
    }

    fn toggle_overlay(&self) -> bool {
        let mut settings = self.settings.lock().expect("the settings lock is poisoned");

        settings.overlay_visible = !settings.overlay_visible;

        settings.overlay_visible
    }

    fn toggle_detail(&self, key: &str) {
        let mut settings = self.settings.lock().expect("the settings lock is poisoned");

        if key == "pointer" {
            settings.pointer = !settings.pointer;
        } else if key == "attribute_names" {
            settings.attribute_names = !settings.attribute_names;
        } else if key == "window_text" {
            settings.window_text = !settings.window_text;
        } else if key == "words" {
            settings.words = !settings.words;
        } else {
            unreachable!("the tray only offers known details");
        }
    }
}

fn push_settings(app: &AppHandle) {
    app.emit("settings", app.state::<State>().settings())
        .expect("the settings can be emitted to the overlay");
}

#[tauri::command]
fn settings(state: tauri::State<'_, State>) -> Settings {
    state.settings()
}

/// A one-shot read, using whatever the tray is currently configured to gather. The expensive parts
/// — the whole-window text scrape and the per-word rectangles — only happen here, never on the main
/// thread, because they can take hundreds of milliseconds.
#[tauri::command]
async fn capture(state: tauri::State<'_, State>) -> Result<ContextSnapshot, String> {
    SystemProvider::new()
        .capture_with(state.settings().options())
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn word_boxes() -> Result<Vec<WordBox>, String> {
    SystemProvider::new()
        .word_boxes()
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn debug_dump() -> Result<String, String> {
    SystemProvider::new()
        .debug_dump()
        .map_err(|error| error.to_string())
}

pub fn run() {
    let builder = tauri::Builder::default();

    #[cfg(target_os = "macos")]
    let builder = builder.plugin(tauri_nspanel::init());

    builder
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            app.manage(State {
                settings: Mutex::new(Settings {
                    route: Route::Basic,
                    columns: 2,
                    interval_ms: 500,
                    opacity: 80,
                    overlay_visible: true,
                    pointer: true,
                    attribute_names: false,
                    window_text: true,
                    words: false,
                }),
            });

            overlay::create(app)?;
            tray::create(app)?;
            overlay::set_visible(app.handle(), true)?;

            let handle = app.handle().clone();
            systex::engine::listen(move |snapshot| {
                handle
                    .emit("context", snapshot)
                    .expect("the snapshot can be emitted to the overlay");
            });

            // The engine publishes by itself whenever the focused element changes, but it has to be
            // driven from the thread owning the run loop to notice a new frontmost application, and
            // the pointer moves without any notification at all.
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                loop {
                    let interval = handle.state::<State>().settings().interval_ms;

                    handle
                        .run_on_main_thread(systex::engine::tick)
                        .expect("the engine can be driven from the main thread");
                    std::thread::sleep(std::time::Duration::from_millis(interval));
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            settings, capture, word_boxes, debug_dump
        ])
        .run(tauri::generate_context!())
        .expect("the overlay failed to start");
}
