use tauri::menu::{CheckMenuItem, MenuBuilder, SubmenuBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Wry};

use crate::events::topics;
use crate::safety::PermissionLevel;
use crate::state::AppState;

const TRAY_ID: &str = "cursor-buddy-tray";

pub struct TrayHandles {
    pub pause_item: CheckMenuItem<Wry>,
    pub perm_items: Vec<(PermissionLevel, CheckMenuItem<Wry>)>,
}

/// Build the system tray icon + menu and stash item handles for live updates.
pub fn create(app: &AppHandle) -> Result<(), String> {
    let open = tauri::menu::MenuItem::with_id(app, "open", "Open Buddy", true, None::<&str>)
        .map_err(e)?;
    let pause =
        CheckMenuItem::with_id(app, "pause", "Pause AI", true, false, None::<&str>).map_err(e)?;
    let sep1 = tauri::menu::PredefinedMenuItem::separator(app).map_err(e)?;

    let levels = [
        PermissionLevel::Observe,
        PermissionLevel::Guide,
        PermissionLevel::Assist,
        PermissionLevel::Autopilot,
    ];
    let mut perm_items = Vec::new();
    let mut submenu = SubmenuBuilder::new(app, "Permission Level");
    for level in levels {
        let item = CheckMenuItem::with_id(
            app,
            &format!("perm.{}", level.as_str()),
            capitalize(level.as_str()),
            true,
            false,
            None::<&str>,
        )
        .map_err(e)?;
        submenu = submenu.item(&item);
        perm_items.push((level, item));
    }
    let perm_submenu = submenu.build().map_err(e)?;
    let sep2 = tauri::menu::PredefinedMenuItem::separator(app).map_err(e)?;
    let settings = tauri::menu::MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)
        .map_err(e)?;
    let quit = tauri::menu::MenuItem::with_id(app, "quit", "Quit", true, None::<&str>).map_err(e)?;

    let menu = MenuBuilder::new(app)
        .item(&open)
        .item(&pause)
        .item(&sep1)
        .item(&perm_submenu)
        .item(&sep2)
        .item(&settings)
        .separator()
        .item(&quit)
        .build()
        .map_err(e)?;

    let _ = TrayIconBuilder::with_id(TRAY_ID)
        .icon(app.default_window_icon().expect("default icon").clone())
        .tooltip("Cursor Buddy — Active")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                crate::ai::agent::activate_bubble(tray.app_handle());
            }
        })
        .build(app)
        .map_err(e)?;

    app.manage(std::sync::Mutex::new(Some(TrayHandles {
        pause_item: pause,
        perm_items,
    })));

    sync_tray_state(app);
    Ok(())
}

/// Reflect paused state + active permission level in the tray UI.
pub fn sync_tray_state(app: &AppHandle) {
    let state = app.state::<std::sync::Arc<AppState>>();
    let paused = state.runtime.lock().unwrap().paused;
    let level = state.settings.lock().unwrap().permission_level;

    if let Some(handles) = app.try_state::<std::sync::Mutex<Option<TrayHandles>>>() {
        if let Some(h) = handles.lock().unwrap().as_mut() {
            let _ = h.pause_item.set_checked(paused);
            let _ = h.pause_item.set_text(if paused { "Resume AI" } else { "Pause AI" });
            for (lvl, item) in &mut h.perm_items {
                let _ = item.set_checked(*lvl == level);
            }
        }
    }

    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let status = if paused { "Paused" } else { "Active" };
        let _ = tray.set_tooltip(Some(format!("Cursor Buddy — {status}")));
    }

    // Note: pause state changes are broadcast by toggle_pause_internal;
    // this function only mirrors state into the tray UI.
}

/// Route tray/context-menu clicks. Shared by the global menu-event hook.
pub fn handle_menu(app: &AppHandle, id: &str) {
    match id {
        "open" => crate::ai::agent::activate_bubble(app),
        "pause" => {
            let paused = crate::commands::misc::toggle_pause_internal(app);
            let _ = paused;
            sync_tray_state(app);
        }
        "settings" => crate::commands::misc::show_main(app.clone()),
        "quit" => crate::commands::misc::quit_app(app.clone()),
        other => {
            if let Some(level_str) = other.strip_prefix("perm.") {
                if let Some(level) = PermissionLevel::parse(level_str) {
                    let state = app.state::<std::sync::Arc<AppState>>();
                    state.settings.lock().unwrap().permission_level = level;
                    let _ = state.settings.lock().unwrap().save(&state.db);
                    drop(state);
                    sync_tray_state(app);
                    let _ = app.emit(
                        topics::PERMISSION_CHANGED,
                        crate::events::PermissionChangedPayload {
                            level: level.as_str().to_string(),
                        },
                    );
                }
            }
        }
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn e(err: impl std::fmt::Display) -> String {
    err.to_string()
}
