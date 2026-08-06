use std::sync::Mutex;

use serde::Serialize;
use systex::{ContextProvider, ContextSnapshot, MockProvider, provider_by_name};
use tauri::{Manager, State};

struct View {
    provider: Mutex<Box<dyn ContextProvider>>,
}

#[derive(Serialize)]
struct ProviderInfo {
    name: String,
    available: bool,
}

#[tauri::command]
fn provider_info(view: State<'_, View>) -> ProviderInfo {
    let provider = view.provider.lock().expect("provider lock is poisoned");

    ProviderInfo {
        name: provider.name().to_string(),
        available: provider.is_available(),
    }
}

#[tauri::command]
fn set_provider(view: State<'_, View>, name: String) -> Result<ProviderInfo, String> {
    let next = provider_by_name(&name).map_err(|error| error.to_string())?;
    let info = ProviderInfo {
        name: next.name().to_string(),
        available: next.is_available(),
    };

    *view.provider.lock().expect("provider lock is poisoned") = next;

    Ok(info)
}

#[tauri::command]
fn capture(view: State<'_, View>) -> Result<ContextSnapshot, String> {
    view.provider
        .lock()
        .expect("provider lock is poisoned")
        .capture()
        .map_err(|error| error.to_string())
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            app.manage(View {
                provider: Mutex::new(Box::new(MockProvider::new())),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            provider_info,
            set_provider,
            capture
        ])
        .run(tauri::generate_context!())
        .expect("the view window failed to start");
}
