use crate::config::Config;
use egui::Context;

pub struct Tab {
    pub id: usize,
    pub title: String,
}

impl Tab {
    pub fn new(id: usize) -> Self {
        Self {
            id,
            title: format!("Terminal {}", id + 1),
        }
    }
}

pub enum SettingsAction {
    None,
    LoadProfile(String),
    SaveProfile(String),
    FontSizeChanged(f32),
    SaveConfig,
    OpenConfigFile,
    ScrollbackChanged(usize),
    UnlimitedScrollbackChanged(bool),
    CursorBlinkChanged(bool),
    TransparencyChanged(f32),
}

pub struct TabBar {
    tabs: Vec<Tab>,
    active_tab_id: usize,
    // Track pending tab actions to communicate with App
    pending_new_tab: bool,
    pending_close_tab: Option<usize>,
    pending_switch_tab: Option<usize>,
}

impl TabBar {
    pub fn new() -> Self {
        let mut tabs = Vec::new();
        tabs.push(Tab::new(0));

        Self {
            tabs,
            active_tab_id: 0,
            pending_new_tab: false,
            pending_close_tab: None,
            pending_switch_tab: None,
        }
    }

    /// Add a new tab with a specific ID (called by App)
    pub fn add_tab_with_id(&mut self, id: usize) {
        self.tabs.push(Tab::new(id));
        self.active_tab_id = id;
    }

    /// Remove a tab by its ID (called by App)
    pub fn remove_tab_by_id(&mut self, id: usize) {
        if let Some(index) = self.tabs.iter().position(|t| t.id == id) {
            self.tabs.remove(index);
        }
    }

    /// Set active tab by ID (called by App)
    pub fn set_active_tab_by_id(&mut self, id: usize) {
        if self.tabs.iter().any(|t| t.id == id) {
            self.active_tab_id = id;
        }
    }

    /// Get active tab ID
    pub fn active_tab_id(&self) -> usize {
        self.active_tab_id
    }

    /// Check and clear pending new tab request
    pub fn take_pending_new_tab(&mut self) -> bool {
        let pending = self.pending_new_tab;
        self.pending_new_tab = false;
        pending
    }

    /// Check and clear pending close tab request
    pub fn take_pending_close_tab(&mut self) -> Option<usize> {
        self.pending_close_tab.take()
    }

    /// Check and clear pending switch tab request
    pub fn take_pending_switch_tab(&mut self) -> Option<usize> {
        self.pending_switch_tab.take()
    }

    pub fn render(&mut self, ctx: &Context, _config: &Config) {
        // iTerm2-style tab bar colors
        let tab_bar_bg = egui::Color32::from_rgb(22, 22, 22);
        let active_tab_bg = egui::Color32::from_rgb(44, 44, 44);
        let inactive_tab_bg = egui::Color32::from_rgb(22, 22, 22);
        let hovered_tab_bg = egui::Color32::from_rgb(36, 36, 36);
        let active_text = egui::Color32::WHITE;
        let inactive_text = egui::Color32::from_rgb(150, 150, 150);
        let separator_color = egui::Color32::from_rgb(60, 60, 60);

        let tab_height = 36.0;

        egui::TopBottomPanel::top("tab_bar")
            .exact_height(tab_height)
            .frame(egui::Frame::none()
                .fill(tab_bar_bg)
                .inner_margin(egui::Margin::ZERO))
            .show(ctx, |ui| {
                ui.set_min_height(tab_height);

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;

                    let mut tab_to_close: Option<usize> = None;
                    let mut tab_to_switch: Option<usize> = None;

                    // Calculate tab width: fill available space evenly
                    let plus_btn_width = 36.0;
                    let available = ui.available_width() - plus_btn_width;
                    let tab_count = self.tabs.len().max(1) as f32;
                    let tab_width = (available / tab_count).min(240.0).max(120.0);

                    for tab in &self.tabs {
                        let is_active = tab.id == self.active_tab_id;
                        let tab_id = tab.id;

                        let (rect, response) = ui.allocate_exact_size(
                            egui::vec2(tab_width, tab_height),
                            egui::Sense::click(),
                        );

                        let is_hovered = response.hovered();

                        // Tab background - active tab is raised/lighter
                        let bg = if is_active {
                            active_tab_bg
                        } else if is_hovered {
                            hovered_tab_bg
                        } else {
                            inactive_tab_bg
                        };

                        ui.painter().rect_filled(rect, egui::Rounding::ZERO, bg);

                        // Active tab: bright bottom accent line (like iTerm2 blue/accent)
                        if is_active {
                            let accent_rect = egui::Rect::from_min_max(
                                egui::pos2(rect.left(), rect.bottom() - 2.5),
                                rect.right_bottom(),
                            );
                            ui.painter().rect_filled(
                                accent_rect,
                                egui::Rounding::ZERO,
                                egui::Color32::from_rgb(50, 120, 230),
                            );
                        }

                        // Vertical separator between inactive tabs
                        if !is_active {
                            let sep_x = rect.right();
                            ui.painter().line_segment(
                                [egui::pos2(sep_x, rect.top() + 8.0), egui::pos2(sep_x, rect.bottom() - 8.0)],
                                egui::Stroke::new(1.0, separator_color),
                            );
                        }

                        // Close button on hover/active (X on left side, iTerm style)
                        let close_center = egui::pos2(rect.left() + 18.0, rect.center().y);
                        let close_radius = 7.0;
                        let close_rect = egui::Rect::from_center_size(
                            close_center,
                            egui::vec2(close_radius * 2.0 + 4.0, close_radius * 2.0 + 4.0),
                        );

                        let mouse_pos = ctx.input(|i| i.pointer.hover_pos().unwrap_or_default());
                        let close_hovered = is_hovered && close_rect.contains(mouse_pos);

                        if is_hovered || is_active {
                            if close_hovered {
                                ui.painter().circle_filled(close_center, close_radius, egui::Color32::from_rgb(70, 70, 70));
                            }

                            let x_color = if close_hovered {
                                egui::Color32::from_rgb(220, 220, 220)
                            } else {
                                egui::Color32::from_rgb(100, 100, 100)
                            };
                            let s = 3.5;
                            ui.painter().line_segment(
                                [close_center + egui::vec2(-s, -s), close_center + egui::vec2(s, s)],
                                egui::Stroke::new(1.5, x_color),
                            );
                            ui.painter().line_segment(
                                [close_center + egui::vec2(s, -s), close_center + egui::vec2(-s, s)],
                                egui::Stroke::new(1.5, x_color),
                            );

                            if close_hovered && response.clicked() {
                                tab_to_close = Some(tab_id);
                            }
                        }

                        // Tab title - centered, larger font
                        let text_color = if is_active { active_text } else { inactive_text };
                        let title = if tab.title.len() > 24 {
                            format!("{}…", &tab.title[..21])
                        } else {
                            tab.title.clone()
                        };

                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            &title,
                            egui::FontId::proportional(13.0),
                            text_color,
                        );

                        if response.clicked() && tab_to_close.is_none() && !is_active {
                            tab_to_switch = Some(tab_id);
                        }

                        if response.secondary_clicked() && self.tabs.len() > 1 {
                            tab_to_close = Some(tab_id);
                        }
                    }

                    if let Some(id) = tab_to_close {
                        self.pending_close_tab = Some(id);
                    }
                    if let Some(id) = tab_to_switch {
                        self.pending_switch_tab = Some(id);
                    }

                    // New tab (+) button
                    let (plus_rect, plus_response) = ui.allocate_exact_size(
                        egui::vec2(plus_btn_width, tab_height),
                        egui::Sense::click(),
                    );

                    if plus_response.hovered() {
                        ui.painter().rect_filled(plus_rect, egui::Rounding::ZERO, hovered_tab_bg);
                    }

                    ui.painter().text(
                        plus_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "+",
                        egui::FontId::proportional(18.0),
                        if plus_response.hovered() { active_text } else { inactive_text },
                    );

                    if plus_response.clicked() {
                        self.pending_new_tab = true;
                    }
                });
            });

    }
}

impl Default for TabBar {
    fn default() -> Self {
        Self::new()
    }
}
