mod app;
mod config;
mod render;
mod terminal;
mod ui;

use app::App;
use config::Config;
use log::info;
use muda::{
    accelerator::{Accelerator, Code, Modifiers},
    AboutMetadata, CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu,
};
use winit::event::{Event, StartCause, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::ModifiersState;

// Menu item IDs
const MENU_INCREASE_FONT: &str = "increase_font";
const MENU_DECREASE_FONT: &str = "decrease_font";
const MENU_RESET_FONT: &str = "reset_font";
const MENU_SAVE_CONFIG: &str = "save_config";
const MENU_OPEN_CONFIG: &str = "open_config";
const MENU_COPY: &str = "copy";
const MENU_PASTE: &str = "paste";
const MENU_SETTINGS: &str = "settings";
const PROFILE_PREFIX: &str = "profile:";
const CRT_STYLE_PREFIX: &str = "crt_style:";

struct AppMenu {
    menu: Menu,
    profiles_submenu: Submenu,
    crt_styles_submenu: Submenu,
}

fn create_menu(config: &Config) -> AppMenu {
    let menu = Menu::new();

    // Application menu (80sTerminal) - on macOS this becomes the app menu
    let app_menu = Submenu::new("80sTerminal", true);

    // About item
    app_menu
        .append(&PredefinedMenuItem::about(
            Some("About 80sTerminal"),
            Some(AboutMetadata {
                name: Some("80sTerminal".to_string()),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
                copyright: Some("© 2026 t@vis.wtf".to_string()),
                credits: Some("A GPU-accelerated CRT terminal emulator\nwith authentic retro monitor styles.\n\nContact: t@vis.wtf\nLicense: MIT".to_string()),
                ..Default::default()
            }),
        ))
        .unwrap();
    app_menu.append(&PredefinedMenuItem::separator()).unwrap();

    // Services submenu (standard macOS)
    app_menu
        .append(&PredefinedMenuItem::services(None))
        .unwrap();
    app_menu.append(&PredefinedMenuItem::separator()).unwrap();

    // Hide/Show
    app_menu.append(&PredefinedMenuItem::hide(None)).unwrap();
    app_menu
        .append(&PredefinedMenuItem::hide_others(None))
        .unwrap();
    app_menu.append(&PredefinedMenuItem::show_all(None)).unwrap();
    app_menu.append(&PredefinedMenuItem::separator()).unwrap();

    // Quit
    app_menu
        .append(&PredefinedMenuItem::quit(Some("Quit 80sTerminal")))
        .unwrap();

    // Edit menu
    let edit_menu = Submenu::new("Edit", true);
    edit_menu.append(&PredefinedMenuItem::undo(None)).unwrap();
    edit_menu.append(&PredefinedMenuItem::redo(None)).unwrap();
    edit_menu.append(&PredefinedMenuItem::separator()).unwrap();
    edit_menu.append(&PredefinedMenuItem::cut(None)).unwrap();
    edit_menu
        .append(&MenuItem::with_id(
            MENU_COPY,
            "Copy",
            true,
            Some(Accelerator::new(Some(Modifiers::META), Code::KeyC)),
        ))
        .unwrap();
    edit_menu
        .append(&MenuItem::with_id(
            MENU_PASTE,
            "Paste",
            true,
            Some(Accelerator::new(Some(Modifiers::META), Code::KeyV)),
        ))
        .unwrap();
    edit_menu
        .append(&PredefinedMenuItem::select_all(None))
        .unwrap();

    // View menu
    let view_menu = Submenu::new("View", true);
    view_menu
        .append(&MenuItem::with_id(
            MENU_INCREASE_FONT,
            "Increase Font Size",
            true,
            Some(Accelerator::new(Some(Modifiers::META), Code::Equal)),
        ))
        .unwrap();
    view_menu
        .append(&MenuItem::with_id(
            MENU_DECREASE_FONT,
            "Decrease Font Size",
            true,
            Some(Accelerator::new(Some(Modifiers::META), Code::Minus)),
        ))
        .unwrap();
    view_menu
        .append(&MenuItem::with_id(
            MENU_RESET_FONT,
            "Reset Font Size",
            true,
            Some(Accelerator::new(Some(Modifiers::META), Code::Digit0)),
        ))
        .unwrap();
    view_menu.append(&PredefinedMenuItem::separator()).unwrap();
    view_menu
        .append(&PredefinedMenuItem::fullscreen(None))
        .unwrap();

    // Settings menu
    let settings_menu = Submenu::new("Settings", true);

    // Preferences panel (Cmd+,)
    settings_menu
        .append(&MenuItem::with_id(
            MENU_SETTINGS,
            "Preferences...",
            true,
            Some(Accelerator::new(Some(Modifiers::META), Code::Comma)),
        ))
        .unwrap();
    settings_menu.append(&PredefinedMenuItem::separator()).unwrap();

    // CRT Styles submenu with checkmarks
    let crt_styles_submenu = Submenu::new("CRT Style", true);
    for (i, name) in Config::crt_style_names().iter().enumerate() {
        let menu_id = format!("{}{}", CRT_STYLE_PREFIX, name);
        crt_styles_submenu
            .append(&CheckMenuItem::with_id(menu_id, *name, true, false, None::<Accelerator>))
            .unwrap();
    }
    settings_menu.append(&crt_styles_submenu).unwrap();

    settings_menu.append(&PredefinedMenuItem::separator()).unwrap();

    // Profiles submenu
    let profiles_submenu = Submenu::new("Profiles", true);
    for name in config.profile_names() {
        let is_active = name == config.active_profile;
        let menu_id = format!("{}{}", PROFILE_PREFIX, name);
        profiles_submenu
            .append(&CheckMenuItem::with_id(menu_id, &name, true, is_active, None::<Accelerator>))
            .unwrap();
    }
    settings_menu.append(&profiles_submenu).unwrap();

    settings_menu.append(&PredefinedMenuItem::separator()).unwrap();

    // Save Config
    settings_menu
        .append(&MenuItem::with_id(
            MENU_SAVE_CONFIG,
            "Save Config",
            true,
            Some(Accelerator::new(Some(Modifiers::META), Code::KeyS)),
        ))
        .unwrap();

    // Open Config File
    settings_menu
        .append(&MenuItem::with_id(
            MENU_OPEN_CONFIG,
            "Open Config File...",
            true,
            None::<Accelerator>,
        ))
        .unwrap();

    // Window menu - no close_window (we handle Cmd+W ourselves for tab management)
    let window_menu = Submenu::new("Window", true);
    window_menu
        .append(&PredefinedMenuItem::minimize(None))
        .unwrap();
    window_menu
        .append(&PredefinedMenuItem::maximize(None))
        .unwrap();

    menu.append(&app_menu).unwrap();
    menu.append(&edit_menu).unwrap();
    menu.append(&view_menu).unwrap();
    menu.append(&settings_menu).unwrap();
    menu.append(&window_menu).unwrap();

    AppMenu {
        menu,
        profiles_submenu,
        crt_styles_submenu,
    }
}

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    info!("Starting 80sTerminal - CRT Terminal Emulator");

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    // Load config to populate profiles menu
    let initial_config = Config::load_or_default();

    // Create menu bar
    let app_menu = create_menu(&initial_config);

    let mut windows: Vec<App> = Vec::new();
    let mut modifiers = ModifiersState::empty();
    let mut cursor_position: (f64, f64) = (0.0, 0.0);
    let menu_channel = MenuEvent::receiver();

    event_loop.run(move |event, elwt| {
        // Handle menu events - apply to the first (focused) window
        if let Ok(menu_event) = menu_channel.try_recv() {
            let menu_id = menu_event.id.0.as_str();

            if let Some(app) = windows.first_mut() {
                if menu_id.starts_with(CRT_STYLE_PREFIX) {
                    let style_name = &menu_id[CRT_STYLE_PREFIX.len()..];
                    app.apply_crt_style(style_name);

                    // Update checkmarks in CRT styles submenu
                    for item in app_menu.crt_styles_submenu.items() {
                        if let muda::MenuItemKind::Check(check_item) = item {
                            let item_id = check_item.id().0.as_str();
                            check_item.set_checked(item_id == menu_id);
                        }
                    }
                } else if menu_id.starts_with(PROFILE_PREFIX) {
                    let profile_name = &menu_id[PROFILE_PREFIX.len()..];
                    app.load_profile(profile_name);

                    for item in app_menu.profiles_submenu.items() {
                        if let muda::MenuItemKind::Check(check_item) = item {
                            let item_id = check_item.id().0.as_str();
                            let is_selected = item_id == menu_id;
                            check_item.set_checked(is_selected);
                        }
                    }
                } else {
                    match menu_id {
                        MENU_SETTINGS => {
                            if app.settings_window_id().is_some() {
                                app.close_settings();
                            } else {
                                app.open_settings(elwt);
                            }
                        }
                        MENU_COPY => {
                            app.copy_to_clipboard();
                        }
                        MENU_PASTE => {
                            app.paste_from_clipboard();
                        }
                        MENU_INCREASE_FONT => {
                            app.change_font_size_public(2.0);
                        }
                        MENU_DECREASE_FONT => {
                            app.change_font_size_public(-2.0);
                        }
                        MENU_RESET_FONT => {
                            app.reset_font_size();
                        }
                        MENU_SAVE_CONFIG => {
                            app.save_config();
                        }
                        MENU_OPEN_CONFIG => {
                            if let Some(path) = Config::config_path() {
                                if let Some(parent) = path.parent() {
                                    let _ = std::fs::create_dir_all(parent);
                                }
                                if !path.exists() {
                                    app.save_config();
                                }
                                #[cfg(target_os = "macos")]
                                {
                                    let _ = std::process::Command::new("open")
                                        .arg("-t")
                                        .arg(&path)
                                        .spawn();
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        match event {
            Event::NewEvents(StartCause::Init) => {
                #[cfg(target_os = "macos")]
                {
                    app_menu.menu.init_for_nsapp();
                }

                match pollster::block_on(App::new(elwt)) {
                    Ok(new_app) => {
                        windows.push(new_app);
                    }
                    Err(e) => {
                        log::error!("Failed to create app: {}", e);
                        elwt.exit();
                    }
                }
            }
            Event::WindowEvent { event, window_id } => {
                // Check if event is for a settings window
                let is_settings = windows.iter().any(|a| a.settings_window_id() == Some(window_id));

                if is_settings {
                    // Find the app that owns this settings window
                    if let Some(app) = windows.iter_mut().find(|a| a.settings_window_id() == Some(window_id)) {
                        app.handle_settings_event(&event);
                        match event {
                            WindowEvent::CloseRequested => {
                                app.close_settings();
                            }
                            WindowEvent::Resized(size) => {
                                app.resize_settings(size.width, size.height);
                            }
                            WindowEvent::RedrawRequested => {
                                // Settings rendering happens in app.render()
                            }
                            _ => {}
                        }
                    }
                    return;
                }

                // Find the main window this event belongs to
                let app_index = windows.iter().position(|a| a.window_id() == window_id);
                if let Some(idx) = app_index {
                    let app = &mut windows[idx];

                    match &event {
                        WindowEvent::Focused(focused) => {
                            if *focused {
                                // Reset modifiers on focus gain to prevent stale
                                // Cmd state from Cmd+Tab app switching
                                modifiers = ModifiersState::empty();
                            }
                        }
                        WindowEvent::ModifiersChanged(new_modifiers) => {
                            modifiers = new_modifiers.state();
                        }
                        WindowEvent::CursorMoved { position, .. } => {
                            cursor_position = (position.x, position.y);
                            app.handle_cursor_moved(cursor_position);
                        }
                        WindowEvent::MouseWheel { delta, .. } => {
                            app.handle_scroll(delta);
                        }
                        WindowEvent::MouseInput { state, button, .. } => {
                            app.handle_mouse_input(*state, *button, cursor_position);
                        }
                        WindowEvent::KeyboardInput { event: key_event, .. } => {
                            app.handle_keyboard(key_event, modifiers);

                            // Check if Cmd+N was pressed (new window)
                            if app.take_pending_new_window() {
                                match pollster::block_on(App::new(elwt)) {
                                    Ok(new_app) => {
                                        info!("Created new window");
                                        windows.push(new_app);
                                    }
                                    Err(e) => {
                                        log::error!("Failed to create new window: {}", e);
                                    }
                                }
                                return;
                            }

                            // Check if settings window should open
                            if app.take_pending_settings() {
                                app.open_settings(elwt);
                                return;
                            }

                            // Check if window should close (Cmd+W on last tab)
                            if app.should_close() {
                                windows.remove(idx);
                                if windows.is_empty() {
                                    elwt.exit();
                                }
                                return;
                            }
                        }
                        _ => {}
                    }

                    // Re-find the app since we may have mutated windows
                    if let Some(app) = windows.iter_mut().find(|a| a.window_id() == window_id) {
                        if app.handle_event(&event) {
                            return;
                        }

                        match event {
                            WindowEvent::CloseRequested => {
                                let wid = app.window_id();
                                windows.retain(|a| a.window_id() != wid);
                                if windows.is_empty() {
                                    elwt.exit();
                                }
                            }
                            WindowEvent::Resized(size) => {
                                app.resize(size.width, size.height);
                            }
                            WindowEvent::RedrawRequested => {
                                app.update();
                                if let Err(e) = app.render() {
                                    log::error!("Render error: {}", e);
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            Event::AboutToWait => {
                for app in &windows {
                    app.request_redraw();
                    app.request_settings_redraw();
                }
            }
            _ => {}
        }
    })?;

    Ok(())
}
