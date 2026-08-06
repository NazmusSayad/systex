mod overlay;
mod tray;

use std::sync::Mutex;

use serde::Serialize;
use systex::{ContextSnapshot, SystemProvider};
use tauri::{AppHandle, Emitter, Manager};

#[derive(Debug, Clone, Serialize)]
pub struct Settings {
    pub interval_ms: u64,
    pub opacity: u8,
    pub overlay_visible: bool,
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
}

fn push_settings(app: &AppHandle) {
    app.emit("settings", app.state::<State>().settings())
        .expect("the settings can be emitted to the overlay");
}

#[tauri::command]
fn settings(state: tauri::State<'_, State>) -> Settings {
    state.settings()
}

#[tauri::command]
fn capture() -> Result<ContextSnapshot, String> {
    SystemProvider::new()
        .capture()
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
                    interval_ms: 500,
                    opacity: 80,
                    overlay_visible: true,
                }),
            });

            overlay::create(app)?;
            tray::create(app)?;
            overlay::set_visible(app.handle(), true)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![settings, capture])
        .run(tauri::generate_context!())
        .expect("the overlay failed to start");
}
