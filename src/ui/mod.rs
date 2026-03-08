mod search;
pub mod settings_window;
mod splits;
mod tabs;

use crate::config::Config;
use crate::terminal::Terminal;
use egui::Context;

pub use search::SearchOverlay;
pub use settings_window::SettingsWindow;
pub use splits::SplitPane;
pub use tabs::{SettingsAction, TabBar};

pub struct Ui {
    tab_bar: TabBar,
    search_overlay: SearchOverlay,
    show_search: bool,
}

impl Ui {
    pub fn new() -> Self {
        Self {
            tab_bar: TabBar::new(),
            search_overlay: SearchOverlay::new(),
            show_search: false,
        }
    }

    pub fn render(&mut self, ctx: &Context, terminal: &Terminal, config: &Config) {
        // Tab bar style - dark chrome, no CRT green here
        let mut style = (*ctx.style()).clone();
        style.visuals.panel_fill = egui::Color32::from_rgb(22, 22, 22);
        style.visuals.override_text_color = None;
        style.spacing.item_spacing = egui::vec2(0.0, 0.0);
        ctx.set_style(style);

        // Render tab bar
        self.tab_bar.render(ctx, config);

        // Render search overlay if active
        if self.show_search {
            // Restore CRT style for search
            let mut style = (*ctx.style()).clone();
            style.visuals.window_fill = egui::Color32::from_rgba_unmultiplied(10, 10, 10, 240);
            let green = egui::Color32::from_rgb(51, 255, 51);
            style.visuals.override_text_color = Some(green);
            style.visuals.window_rounding = egui::Rounding::same(2.0);
            ctx.set_style(style);
            self.search_overlay.render(ctx, terminal);
        }
    }

    pub fn toggle_search(&mut self) {
        self.show_search = !self.show_search;
    }

    pub fn is_search_active(&self) -> bool {
        self.show_search
    }

    pub fn tab_bar(&self) -> &TabBar {
        &self.tab_bar
    }

    pub fn tab_bar_mut(&mut self) -> &mut TabBar {
        &mut self.tab_bar
    }

    pub fn add_tab(&mut self, tab_id: usize) {
        self.tab_bar.add_tab_with_id(tab_id);
    }

    pub fn remove_tab(&mut self, tab_id: usize) {
        self.tab_bar.remove_tab_by_id(tab_id);
    }

    pub fn set_active_tab(&mut self, tab_id: usize) {
        self.tab_bar.set_active_tab_by_id(tab_id);
    }

    pub fn take_pending_new_tab(&mut self) -> bool {
        self.tab_bar.take_pending_new_tab()
    }

    pub fn take_pending_close_tab(&mut self) -> Option<usize> {
        self.tab_bar.take_pending_close_tab()
    }

    pub fn take_pending_switch_tab(&mut self) -> Option<usize> {
        self.tab_bar.take_pending_switch_tab()
    }
}

impl Default for Ui {
    fn default() -> Self {
        Self::new()
    }
}
