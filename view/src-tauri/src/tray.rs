use systex::permissions::{self, Permission};
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::{App, AppHandle, Manager, Wry};

use crate::{Route, Settings, State, overlay};

pub const ID: &str = "tray";

const INTERVALS: [u64; 4] = [200, 500, 1000, 2000];
const OPACITIES: [u8; 4] = [40, 60, 80, 100];

pub fn create(app: &App) -> tauri::Result<()> {
    let settings = app.state::<State>().settings();
    let menu = build(app.handle(), &settings)?;

    TrayIconBuilder::with_id(ID)
        .icon(
            app.default_window_icon()
                .expect("the bundle ships a default icon")
                .clone(),
        )
        .icon_as_template(true)
        .tooltip("Systex")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| on_menu_event(app, event.id().as_ref()))
        .build(app)?;

    Ok(())
}

pub fn refresh(app: &AppHandle) -> tauri::Result<()> {
    let settings = app.state::<State>().settings();
    let menu = build(app, &settings)?;

    app.tray_by_id(ID)
        .expect("the tray icon is registered")
        .set_menu(Some(menu))?;

    Ok(())
}

fn build(app: &AppHandle, settings: &Settings) -> tauri::Result<Menu<Wry>> {
    let overlay_item = CheckMenuItem::with_id(
        app,
        "overlay",
        "Show overlay",
        true,
        settings.overlay_visible,
        None::<&str>,
    )?;

    let route_menu = Submenu::with_id(app, "route", "View", true)?;
    for route in Route::ALL {
        route_menu.append(&CheckMenuItem::with_id(
            app,
            format!("route:{}", route.key()),
            route.label(),
            true,
            settings.route == route,
            None::<&str>,
        )?)?;
    }

    let interval_menu = Submenu::with_id(app, "interval", "Refresh rate", true)?;
    for ms in INTERVALS {
        interval_menu.append(&CheckMenuItem::with_id(
            app,
            format!("interval:{ms}"),
            format!("every {ms}ms"),
            true,
            settings.interval_ms == ms,
            None::<&str>,
        )?)?;
    }

    let opacity_menu = Submenu::with_id(app, "opacity", "Opacity", true)?;
    for value in OPACITIES {
        opacity_menu.append(&CheckMenuItem::with_id(
            app,
            format!("opacity:{value}"),
            format!("{value}%"),
            true,
            settings.opacity == value,
            None::<&str>,
        )?)?;
    }

    let details_menu = Submenu::with_id(app, "details", "Capture", true)?;
    for (key, label, enabled) in [
        ("pointer", "Element under the pointer", settings.pointer),
        (
            "attribute_names",
            "Accessibility attribute names",
            settings.attribute_names,
        ),
        ("window_text", "Whole window text", settings.window_text),
        ("words", "Word rectangles", settings.words),
    ] {
        details_menu.append(&CheckMenuItem::with_id(
            app,
            format!("detail:{key}"),
            label,
            true,
            enabled,
            None::<&str>,
        )?)?;
    }

    let permissions_menu = Submenu::with_id(app, "permissions", "Permissions", true)?;
    for permission in permissions::ALL {
        let granted = permissions::status(permission) == permissions::PermissionStatus::Granted;

        permissions_menu.append(&MenuItem::with_id(
            app,
            format!("grant:{}", permission.key()),
            if granted {
                format!("{} · granted", permission.label())
            } else {
                format!("Grant {}", permission.label())
            },
            !granted,
            None::<&str>,
        )?)?;
    }

    Menu::with_items(
        app,
        &[
            &overlay_item,
            &PredefinedMenuItem::separator(app)?,
            &route_menu,
            &interval_menu,
            &opacity_menu,
            &details_menu,
            &PredefinedMenuItem::separator(app)?,
            &permissions_menu,
            &MenuItem::with_id(app, "diagnostics", "Print diagnostics", true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, "quit", "Quit Systex", true, None::<&str>)?,
        ],
    )
}

pub fn on_menu_event(app: &AppHandle, id: &str) {
    if id == "quit" {
        app.exit(0);
        return;
    }

    if id == "overlay" {
        let visible = app.state::<State>().toggle_overlay();
        overlay::set_visible(app, visible).expect("the overlay visibility can be changed");
    } else if id == "diagnostics" {
        match systex::SystemProvider::new().debug_dump() {
            Ok(report) => println!("{report}"),
            Err(error) => eprintln!("diagnostics failed: {error}"),
        }
    } else if let Some(key) = id.strip_prefix("route:") {
        app.state::<State>()
            .set_route(Route::from_key(key).expect("the tray only offers known routes"));
    } else if let Some(key) = id.strip_prefix("detail:") {
        app.state::<State>().toggle_detail(key);
    } else if let Some(ms) = id.strip_prefix("interval:") {
        app.state::<State>()
            .set_interval(ms.parse().expect("the tray only offers numeric intervals"));
    } else if let Some(value) = id.strip_prefix("opacity:") {
        app.state::<State>().set_opacity(
            value
                .parse()
                .expect("the tray only offers numeric opacities"),
        );
    } else if let Some(key) = id.strip_prefix("grant:") {
        let permission = Permission::from_key(key).expect("the tray only offers known permissions");

        match permissions::request(permission) {
            Ok(status) => {
                if status != permissions::PermissionStatus::Granted
                    && let Err(error) = permissions::open_settings(permission)
                {
                    eprintln!(
                        "opening settings for {} failed: {error}",
                        permission.label()
                    );
                }
            }
            Err(error) => eprintln!("requesting {} failed: {error}", permission.label()),
        }
    } else {
        eprintln!("unknown tray item `{id}`");
        return;
    }

    crate::push_settings(app);
    refresh(app).expect("the tray menu can be rebuilt");
}
