use crate::config::Config;
use crate::render::Renderer;
use crate::terminal::{Point, Selection, Terminal};
use crate::ui::{SettingsAction, SettingsWindow, Ui};
use copypasta::{ClipboardContext, ClipboardProvider};
use log::info;
use std::collections::HashMap;
use std::sync::Arc;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::EventLoopWindowTarget;
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{Window, WindowBuilder, WindowId};

pub struct App {
    window: Arc<Window>,
    renderer: Renderer,
    terminals: HashMap<usize, Terminal>,
    active_tab: usize,
    next_tab_id: usize,
    ui: Ui,
    config: Config,
    should_close: bool,
    pending_new_window: bool,
    pending_settings: bool,
    settings_window: Option<SettingsWindow>,
    selection: Selection,
    clipboard: Option<ClipboardContext>,
    mouse_pressed: bool,
}

impl App {
    pub async fn new(event_loop: &EventLoopWindowTarget<()>) -> anyhow::Result<Self> {
        let config = Config::load_or_default();
        let ui = Ui::new();

        // Create window using winit 0.29 API
        let (win_w, win_h) = config.window_size();
        let window = Arc::new(
            WindowBuilder::new()
                .with_title("80sTerminal")
                .with_inner_size(LogicalSize::new(win_w, win_h))
                .with_min_inner_size(LogicalSize::new(400.0, 300.0))
                .with_transparent(true)
                .build(event_loop)?,
        );

        info!("Window created successfully");

        // Initialize renderer
        let renderer = Renderer::new(Arc::clone(&window), &config).await?;

        // Initialize first terminal
        let size = window.inner_size();
        let cell_size = renderer.cell_size();
        let cols = (size.width as f32 / cell_size.0) as u16;
        let rows = (size.height as f32 / cell_size.1) as u16;
        let terminal = Terminal::new(cols.max(80), rows.max(24), &config)?;

        let mut terminals = HashMap::new();
        terminals.insert(0, terminal);

        let clipboard = ClipboardContext::new().ok();

        Ok(Self {
            window,
            renderer,
            terminals,
            active_tab: 0,
            next_tab_id: 1,
            ui,
            config,
            should_close: false,
            pending_new_window: false,
            pending_settings: false,
            settings_window: None,
            selection: Selection::new(),
            clipboard,
            mouse_pressed: false,
        })
    }

    pub fn window_id(&self) -> WindowId {
        self.window.id()
    }

    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    pub fn should_close(&self) -> bool {
        self.should_close
    }

    pub fn take_pending_new_window(&mut self) -> bool {
        let val = self.pending_new_window;
        self.pending_new_window = false;
        val
    }

    pub fn handle_event(&mut self, event: &WindowEvent) -> bool {
        self.renderer.handle_event(event)
    }

    fn active_terminal(&self) -> Option<&Terminal> {
        self.terminals.get(&self.active_tab)
    }

    fn active_terminal_mut(&mut self) -> Option<&mut Terminal> {
        self.terminals.get_mut(&self.active_tab)
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.renderer.resize(width, height);
        // Recalculate terminal size based on current font size
        let cell_size = self.renderer.cell_size();
        let cols = (width as f32 / cell_size.0) as u16;
        let rows = (height as f32 / cell_size.1) as u16;

        // Resize all terminals
        for terminal in self.terminals.values_mut() {
            terminal.resize(cols.max(80), rows.max(24));
        }

        // Save window size to active profile
        let scale = self.window.scale_factor();
        let logical_w = width as f64 / scale;
        let logical_h = height as f64 / scale;
        self.config.save_window_size(logical_w, logical_h);
        let _ = self.config.save();
    }

    pub fn update(&mut self) {
        // Process all terminals (so background tabs still receive data)
        for terminal in self.terminals.values_mut() {
            terminal.process();
        }
    }

    pub fn render(&mut self) -> anyhow::Result<()> {
        if !self.terminals.contains_key(&self.active_tab) {
            return Ok(());
        }

        {
            let terminal = self.terminals.get(&self.active_tab).unwrap();
            self.renderer.render(terminal, &mut self.ui, &self.config, &self.selection)?;
        }

        // Render settings window if open and collect action
        let settings_action = if let Some(ref mut settings) = self.settings_window {
            settings.render(&self.config)
        } else {
            SettingsAction::None
        };

        // Handle settings actions
        match settings_action {
            SettingsAction::None => {}
            SettingsAction::FontSizeChanged(delta) => {
                if delta == 0.0 {
                    // Reset to default
                    self.config.appearance.font_size = 18.0;
                    info!("Font size reset to default: 18.0");
                } else {
                    self.change_font_size(delta);
                }
                self.renderer.set_font_size(&self.config);
                let size = self.window.inner_size();
                let cell_size = self.renderer.cell_size();
                let cols = (size.width as f32 / cell_size.0) as u16;
                let rows = (size.height as f32 / cell_size.1) as u16;
                for terminal in self.terminals.values_mut() {
                    terminal.resize(cols.max(80), rows.max(24));
                }
            }
            SettingsAction::LoadProfile(name) => {
                self.config.load_profile(&name);
                self.renderer.set_font_size(&self.config);
                let size = self.window.inner_size();
                let cell_size = self.renderer.cell_size();
                let cols = (size.width as f32 / cell_size.0) as u16;
                let rows = (size.height as f32 / cell_size.1) as u16;
                for terminal in self.terminals.values_mut() {
                    terminal.resize(cols.max(80), rows.max(24));
                }
            }
            SettingsAction::SaveProfile(name) => {
                self.config.save_as_profile(&name);
                info!("Saved profile: {}", name);
            }
            SettingsAction::SaveConfig => {
                if let Err(e) = self.config.save() {
                    log::error!("Failed to save config: {}", e);
                }
            }
            SettingsAction::OpenConfigFile => {
                if let Some(path) = Config::config_path() {
                    if let Some(parent) = path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    if !path.exists() {
                        let _ = self.config.save();
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
            SettingsAction::ScrollbackChanged(lines) => {
                // Update the profile config
                let profile_name = self.config.active_profile.clone();
                if let Some(profile) = self.config.profiles.get_mut(&profile_name) {
                    profile.scrollback_lines = lines;
                }
                // Apply to all active terminals
                for terminal in self.terminals.values() {
                    terminal.grid().lock().set_scrollback_limit(lines);
                }
                info!("Scrollback buffer set to {} lines", lines);
            }
            SettingsAction::CursorBlinkChanged(blink) => {
                self.config.appearance.cursor_blink = blink;
                info!("Cursor blink: {}", blink);
            }
            SettingsAction::TransparencyChanged(transparency) => {
                self.config.appearance.opacity = 1.0 - transparency;
                self.renderer.set_opacity(1.0 - transparency);
                info!("Transparency: {:.0}%", transparency * 100.0);
            }
            SettingsAction::UnlimitedScrollbackChanged(unlimited) => {
                let profile_name = self.config.active_profile.clone();
                if let Some(profile) = self.config.profiles.get_mut(&profile_name) {
                    profile.unlimited_scrollback = unlimited;
                }
                for terminal in self.terminals.values() {
                    terminal.grid().lock().set_unlimited_scrollback(unlimited);
                }
                info!("Unlimited scrollback: {}", unlimited);
            }
        }

        // Handle pending tab actions from UI
        if self.ui.take_pending_new_tab() {
            self.new_tab();
        }
        if let Some(tab_id) = self.ui.take_pending_close_tab() {
            self.close_tab_by_id(tab_id);
        }
        if let Some(tab_id) = self.ui.take_pending_switch_tab() {
            self.switch_to_tab(tab_id);
        }

        Ok(())
    }

    /// Close a specific tab by ID
    fn close_tab_by_id(&mut self, tab_id: usize) {
        if self.terminals.len() <= 1 {
            self.should_close = true;
            return;
        }

        // Find the next tab to switch to
        let mut tab_ids: Vec<usize> = self.terminals.keys().cloned().collect();
        tab_ids.sort();
        let current_index = tab_ids.iter().position(|&id| id == tab_id).unwrap_or(0);

        let new_active = if current_index > 0 {
            tab_ids[current_index - 1]
        } else if tab_ids.len() > 1 {
            tab_ids[1]
        } else {
            return;
        };

        self.terminals.remove(&tab_id);
        self.ui.remove_tab(tab_id);
        self.active_tab = new_active;
        self.ui.set_active_tab(new_active);
        info!("Closed tab: {}, now active: {}", tab_id, new_active);
    }

    /// Create a new tab with a new terminal
    pub fn new_tab(&mut self) {
        let size = self.window.inner_size();
        let cell_size = self.renderer.cell_size();
        let cols = (size.width as f32 / cell_size.0) as u16;
        let rows = (size.height as f32 / cell_size.1) as u16;

        match Terminal::new(cols.max(80), rows.max(24), &self.config) {
            Ok(terminal) => {
                let tab_id = self.next_tab_id;
                self.next_tab_id += 1;
                self.terminals.insert(tab_id, terminal);
                self.active_tab = tab_id;
                self.ui.add_tab(tab_id);
                info!("Created new tab: {}", tab_id);
            }
            Err(e) => {
                log::error!("Failed to create new terminal: {}", e);
            }
        }
    }

    /// Close the current tab (does nothing if it's the last tab)
    pub fn close_current_tab(&mut self) {
        if self.terminals.len() <= 1 {
            return;
        }

        let tab_to_close = self.active_tab;

        // Find the next tab to switch to
        let tab_ids: Vec<usize> = self.terminals.keys().cloned().collect();
        let current_index = tab_ids.iter().position(|&id| id == tab_to_close).unwrap_or(0);

        // Switch to previous tab, or next if at the beginning
        let new_active = if current_index > 0 {
            tab_ids[current_index - 1]
        } else if tab_ids.len() > 1 {
            tab_ids[1]
        } else {
            return;
        };

        self.terminals.remove(&tab_to_close);
        self.ui.remove_tab(tab_to_close);
        self.active_tab = new_active;
        self.ui.set_active_tab(new_active);
        info!("Closed tab: {}, now active: {}", tab_to_close, new_active);
    }

    /// Switch to next tab
    pub fn next_tab(&mut self) {
        let mut tab_ids: Vec<usize> = self.terminals.keys().cloned().collect();
        tab_ids.sort();

        if let Some(current_index) = tab_ids.iter().position(|&id| id == self.active_tab) {
            let next_index = (current_index + 1) % tab_ids.len();
            self.active_tab = tab_ids[next_index];
            self.ui.set_active_tab(self.active_tab);
            info!("Switched to tab: {}", self.active_tab);
        }
    }

    /// Switch to previous tab
    pub fn prev_tab(&mut self) {
        let mut tab_ids: Vec<usize> = self.terminals.keys().cloned().collect();
        tab_ids.sort();

        if let Some(current_index) = tab_ids.iter().position(|&id| id == self.active_tab) {
            let prev_index = if current_index == 0 {
                tab_ids.len() - 1
            } else {
                current_index - 1
            };
            self.active_tab = tab_ids[prev_index];
            self.ui.set_active_tab(self.active_tab);
            info!("Switched to tab: {}", self.active_tab);
        }
    }

    /// Switch to a specific tab by ID
    pub fn switch_to_tab(&mut self, tab_id: usize) {
        if self.terminals.contains_key(&tab_id) {
            self.active_tab = tab_id;
            self.ui.set_active_tab(tab_id);
        }
    }

    fn change_font_size(&mut self, delta: f32) {
        let new_size = (self.config.appearance.font_size + delta).clamp(8.0, 72.0);
        if new_size != self.config.appearance.font_size {
            self.config.appearance.font_size = new_size;
            info!("Font size changed to: {}", new_size);

            // Update renderer with new font size
            self.renderer.set_font_size(&self.config);

            // Recalculate terminal dimensions
            let size = self.window.inner_size();
            let cell_size = self.renderer.cell_size();
            let cols = (size.width as f32 / cell_size.0) as u16;
            let rows = (size.height as f32 / cell_size.1) as u16;
            for terminal in self.terminals.values_mut() {
                terminal.resize(cols.max(80), rows.max(24));
            }
        }
    }

    /// Public method for menu to change font size
    pub fn change_font_size_public(&mut self, delta: f32) {
        self.change_font_size(delta);
    }

    /// Reset font size to default (18pt)
    pub fn reset_font_size(&mut self) {
        self.config.appearance.font_size = 18.0;
        info!("Font size reset to default: 18.0");
        self.renderer.set_font_size(&self.config);
        let size = self.window.inner_size();
        let cell_size = self.renderer.cell_size();
        let cols = (size.width as f32 / cell_size.0) as u16;
        let rows = (size.height as f32 / cell_size.1) as u16;
        for terminal in self.terminals.values_mut() {
            terminal.resize(cols.max(80), rows.max(24));
        }
    }

    /// Request to open/close settings window
    pub fn toggle_settings(&mut self) {
        if self.settings_window.is_some() {
            self.settings_window = None;
        } else {
            self.pending_settings = true;
        }
    }

    /// Check and clear pending settings window request
    pub fn take_pending_settings(&mut self) -> bool {
        let val = self.pending_settings;
        self.pending_settings = false;
        val
    }

    /// Open the settings window (called from main event loop with access to EventLoop)
    pub fn open_settings(&mut self, event_loop: &EventLoopWindowTarget<()>) {
        if self.settings_window.is_some() {
            return;
        }
        match pollster::block_on(SettingsWindow::new(event_loop, &self.config)) {
            Ok(sw) => {
                self.settings_window = Some(sw);
                info!("Settings window opened");
            }
            Err(e) => {
                log::error!("Failed to open settings window: {}", e);
            }
        }
    }

    /// Get the settings window ID if it exists
    pub fn settings_window_id(&self) -> Option<WindowId> {
        self.settings_window.as_ref().map(|s| s.window_id())
    }

    /// Handle a window event for the settings window, returns true if consumed
    pub fn handle_settings_event(&mut self, event: &WindowEvent) -> bool {
        if let Some(ref mut settings) = self.settings_window {
            settings.handle_event(event)
        } else {
            false
        }
    }

    /// Resize the settings window
    pub fn resize_settings(&mut self, width: u32, height: u32) {
        if let Some(ref mut settings) = self.settings_window {
            settings.resize(width, height);
        }
    }

    /// Close the settings window
    pub fn close_settings(&mut self) {
        self.settings_window = None;
    }

    /// Request redraw for settings window
    pub fn request_settings_redraw(&self) {
        if let Some(ref settings) = self.settings_window {
            settings.request_redraw();
        }
    }

    /// Copy selection to clipboard (or send Ctrl+C if no selection)
    pub fn copy_to_clipboard(&mut self) {
        if self.selection.has_selection() {
            let text = {
                let terminal = self.terminals.get(&self.active_tab);
                terminal.map(|t| {
                    let grid = t.grid().lock();
                    self.selection.get_text(&grid)
                }).unwrap_or_default()
            };
            if !text.is_empty() {
                if let Some(clipboard) = &mut self.clipboard {
                    let _ = clipboard.set_contents(text);
                }
            }
            self.selection.clear();
        } else {
            // No selection — send Ctrl+C (SIGINT)
            if let Some(terminal) = self.active_terminal_mut() {
                terminal.write(&[0x03]);
            }
        }
    }

    /// Paste from clipboard into terminal
    pub fn paste_from_clipboard(&mut self) {
        if let Some(clipboard) = &mut self.clipboard {
            if let Ok(text) = clipboard.get_contents() {
                if let Some(terminal) = self.active_terminal_mut() {
                    terminal.write(text.as_bytes());
                }
            }
        }
    }

    /// Apply a CRT style preset
    pub fn apply_crt_style(&mut self, name: &str) {
        self.config.apply_crt_style(name);
    }

    /// Save config to disk
    pub fn save_config(&mut self) {
        if let Err(e) = self.config.save() {
            log::error!("Failed to save config: {}", e);
        }
    }

    /// Load a profile by name
    pub fn load_profile(&mut self, name: &str) {
        self.config.load_profile(name);
        self.renderer.set_font_size(&self.config);
        let size = self.window.inner_size();
        let cell_size = self.renderer.cell_size();
        let cols = (size.width as f32 / cell_size.0) as u16;
        let rows = (size.height as f32 / cell_size.1) as u16;
        for terminal in self.terminals.values_mut() {
            terminal.resize(cols.max(80), rows.max(24));
        }
    }

    fn pixel_to_cell(&self, x: f64, y: f64) -> (u16, u16) {
        let cell_size = self.renderer.cell_size();
        let scale = self.window.scale_factor() as f32;

        // Subtract tab bar height (36 logical pixels, converted to physical)
        let tab_bar_h = 36.0 * scale;
        let size = self.window.inner_size();
        let content_h = size.height as f32 - tab_bar_h;

        // Subtract CRT bezel top inset (bezel_size * 0.4 matches shader scale)
        let bezel = if self.config.crt.enabled { self.config.crt.bezel_size } else { 0.0 };
        let bezel_top = bezel * 0.4 * content_h;

        let y = (y as f32 - tab_bar_h - bezel_top).max(0.0);
        let x = x as f32;

        let col = (x / cell_size.0) as u16;
        let row = (y / cell_size.1) as u16;
        if let Some(terminal) = self.active_terminal() {
            (
                col.min(terminal.cols().saturating_sub(1)),
                row.min(terminal.rows().saturating_sub(1)),
            )
        } else {
            (col, row)
        }
    }

    pub fn handle_mouse_input(&mut self, state: ElementState, button: MouseButton, position: (f64, f64)) {
        if button != MouseButton::Left {
            return;
        }

        match state {
            ElementState::Pressed => {
                self.mouse_pressed = true;
                let (col, row) = self.pixel_to_cell(position.0, position.1);
                self.selection.start_selection(Point::new(col, row));
            }
            ElementState::Released => {
                self.mouse_pressed = false;
                self.selection.end_selection();
            }
        }
    }

    pub fn handle_cursor_moved(&mut self, position: (f64, f64)) {
        if self.mouse_pressed {
            let (col, row) = self.pixel_to_cell(position.0, position.1);
            self.selection.update_selection(Point::new(col, row));
        }
    }

    pub fn selection(&self) -> &Selection {
        &self.selection
    }

    pub fn handle_scroll(&mut self, delta: &MouseScrollDelta) {
        // winit already applies the OS natural scrolling preference to the delta,
        // so we use the values directly: positive y = scroll content up (show older lines)
        let lines = match delta {
            MouseScrollDelta::LineDelta(_, y) => (*y * 3.0) as i32,
            MouseScrollDelta::PixelDelta(pos) => (pos.y / 10.0) as i32,
        };

        if let Some(terminal) = self.active_terminal_mut() {
            let mut grid = terminal.grid().lock();
            let max_offset = grid.scrollback().len();
            let current = grid.scroll_offset() as i32;
            let new_offset = (current + lines).clamp(0, max_offset as i32) as usize;
            grid.set_scroll_offset(new_offset);
        }
    }

    pub fn handle_keyboard(&mut self, event: &KeyEvent, modifiers: ModifiersState) {
        if event.state != ElementState::Pressed {
            return;
        }

        // Check for modifier keys
        let cmd_pressed = modifiers.super_key();
        let ctrl_pressed = modifiers.control_key();
        let shift_pressed = modifiers.shift_key();

        // Handle Ctrl key combinations - send control characters to terminal
        if ctrl_pressed && !cmd_pressed {
            if let Key::Character(c) = &event.logical_key {
                // Get the first character and convert to control character
                if let Some(ch) = c.chars().next() {
                    let ctrl_char = match ch.to_ascii_lowercase() {
                        'a'..='z' => {
                            // Control characters: Ctrl+A = 0x01, Ctrl+B = 0x02, etc.
                            let code = (ch.to_ascii_lowercase() as u8) - b'a' + 1;
                            Some(code)
                        }
                        '[' => Some(0x1b), // Escape
                        '\\' => Some(0x1c),
                        ']' => Some(0x1d),
                        '^' => Some(0x1e),
                        '_' => Some(0x1f),
                        _ => None,
                    };

                    if let Some(code) = ctrl_char {
                        if let Some(terminal) = self.active_terminal_mut() {
                            terminal.write(&[code]);
                        }
                        return;
                    }
                }
            }
        }

        // Handle Cmd shortcuts
        if cmd_pressed {
            match &event.logical_key {
                // Cmd+,: Open settings
                Key::Character(c) if c == "," => {
                    self.toggle_settings();
                    return;
                }
                // Cmd+C: Copy selection to clipboard (handled via menu accelerator)
                Key::Character(c) if c == "c" || c == "C" => {
                    self.copy_to_clipboard();
                    return;
                }
                // Cmd+V: Paste from clipboard (handled via menu accelerator)
                Key::Character(c) if c == "v" || c == "V" => {
                    self.paste_from_clipboard();
                    return;
                }
                // Cmd+T: New tab
                Key::Character(c) if c == "t" || c == "T" => {
                    if !shift_pressed {
                        self.new_tab();
                        return;
                    }
                }
                // Cmd+W: Close tab (or window if last tab)
                Key::Character(c) if c == "w" || c == "W" => {
                    if self.terminals.len() > 1 {
                        self.close_current_tab();
                    } else {
                        self.should_close = true;
                    }
                    return;
                }
                // Cmd+N: New window (handled by main event loop)
                Key::Character(c) if c == "n" || c == "N" => {
                    self.pending_new_window = true;
                    return;
                }
                // Cmd+Shift+] : Next tab
                Key::Character(c) if c == "}" || c == "]" => {
                    if shift_pressed {
                        self.next_tab();
                        return;
                    }
                }
                // Cmd+Shift+[ : Previous tab
                Key::Character(c) if c == "{" || c == "[" => {
                    if shift_pressed {
                        self.prev_tab();
                        return;
                    }
                }
                Key::Character(c) if c == "+" || c == "=" => {
                    self.change_font_size(2.0);
                    return;
                }
                Key::Character(c) if c == "-" || c == "_" => {
                    self.change_font_size(-2.0);
                    return;
                }
                Key::Character(c) if c == "0" => {
                    // Reset to default font size
                    self.config.appearance.font_size = 18.0;
                    info!("Font size reset to default: 18.0");
                    self.renderer.set_font_size(&self.config);
                    let size = self.window.inner_size();
                    let cell_size = self.renderer.cell_size();
                    let cols = (size.width as f32 / cell_size.0) as u16;
                    let rows = (size.height as f32 / cell_size.1) as u16;
                    for terminal in self.terminals.values_mut() {
                        terminal.resize(cols.max(80), rows.max(24));
                    }
                    return;
                }
                // Cmd+1-9: Switch to tab by number
                Key::Character(c) => {
                    if let Some(digit) = c.chars().next().and_then(|ch| ch.to_digit(10)) {
                        if digit >= 1 && digit <= 9 {
                            let mut tab_ids: Vec<usize> = self.terminals.keys().cloned().collect();
                            tab_ids.sort();
                            let index = (digit as usize) - 1;
                            if index < tab_ids.len() {
                                self.switch_to_tab(tab_ids[index]);
                            }
                            return;
                        }
                    }
                }
                _ => {}
            }
        }

        // Regular key handling - send to terminal
        if let Some(terminal) = self.active_terminal_mut() {
            // Reset scroll offset when user types
            terminal.grid().lock().set_scroll_offset(0);

            match &event.logical_key {
                Key::Character(c) => {
                    // Don't send if Cmd or Ctrl is pressed (handled above)
                    if !cmd_pressed && !ctrl_pressed {
                        terminal.write(c.as_bytes());
                    }
                }
                Key::Named(NamedKey::Enter) => {
                    terminal.write(b"\r");
                }
                Key::Named(NamedKey::Backspace) => {
                    terminal.write(b"\x7f");
                }
                Key::Named(NamedKey::Tab) => {
                    terminal.write(b"\t");
                }
                Key::Named(NamedKey::Escape) => {
                    terminal.write(b"\x1b");
                }
                Key::Named(NamedKey::ArrowUp) => {
                    terminal.write(b"\x1b[A");
                }
                Key::Named(NamedKey::ArrowDown) => {
                    terminal.write(b"\x1b[B");
                }
                Key::Named(NamedKey::ArrowRight) => {
                    terminal.write(b"\x1b[C");
                }
                Key::Named(NamedKey::ArrowLeft) => {
                    terminal.write(b"\x1b[D");
                }
                Key::Named(NamedKey::Home) => {
                    terminal.write(b"\x1b[H");
                }
                Key::Named(NamedKey::End) => {
                    terminal.write(b"\x1b[F");
                }
                Key::Named(NamedKey::PageUp) => {
                    terminal.write(b"\x1b[5~");
                }
                Key::Named(NamedKey::PageDown) => {
                    terminal.write(b"\x1b[6~");
                }
                Key::Named(NamedKey::Delete) => {
                    terminal.write(b"\x1b[3~");
                }
                Key::Named(NamedKey::Space) => {
                    terminal.write(b" ");
                }
                _ => {}
            }
        }
    }
}
