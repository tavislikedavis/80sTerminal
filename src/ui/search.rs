use crate::terminal::Terminal;
use egui::{Context, Key, TextEdit};

pub struct SearchOverlay {
    query: String,
    matches: Vec<SearchMatch>,
    current_match: usize,
    case_sensitive: bool,
    regex_mode: bool,
}

#[derive(Clone)]
pub struct SearchMatch {
    pub row: usize,
    pub col_start: usize,
    pub col_end: usize,
}

impl SearchOverlay {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            matches: Vec::new(),
            current_match: 0,
            case_sensitive: false,
            regex_mode: false,
        }
    }

    pub fn render(&mut self, ctx: &Context, terminal: &Terminal) {
        egui::Window::new("Search")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::RIGHT_TOP, [-10.0, 40.0])
            .frame(egui::Frame::window(&ctx.style())
                .fill(egui::Color32::from_rgba_unmultiplied(15, 25, 15, 240)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Find:").color(egui::Color32::from_rgb(51, 255, 51)));

                    let response = ui.add(
                        TextEdit::singleline(&mut self.query)
                            .desired_width(200.0)
                            .hint_text("Search...")
                            .text_color(egui::Color32::from_rgb(51, 255, 51))
                    );

                    if response.changed() {
                        self.search(terminal);
                    }

                    // Handle Enter key to go to next match
                    if response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                        self.next_match();
                    }
                });

                ui.horizontal(|ui| {
                    if ui.checkbox(&mut self.case_sensitive, "Case sensitive").changed() {
                        self.search(terminal);
                    }

                    if ui.checkbox(&mut self.regex_mode, "Regex").changed() {
                        self.search(terminal);
                    }
                });

                ui.horizontal(|ui| {
                    if ui.button("Previous").clicked() {
                        self.prev_match();
                    }
                    if ui.button("Next").clicked() {
                        self.next_match();
                    }

                    if !self.matches.is_empty() {
                        ui.label(egui::RichText::new(format!(
                            "{} of {} matches",
                            self.current_match + 1,
                            self.matches.len()
                        )).color(egui::Color32::from_rgb(51, 255, 51)));
                    } else if !self.query.is_empty() {
                        ui.label(egui::RichText::new("No matches").color(egui::Color32::from_rgb(200, 100, 100)));
                    }
                });
            });
    }

    fn search(&mut self, terminal: &Terminal) {
        self.matches.clear();
        self.current_match = 0;

        if self.query.is_empty() {
            return;
        }

        let grid = terminal.grid().lock();
        let cols = grid.cols() as usize;
        let rows = grid.rows() as usize;

        let search_query = if self.case_sensitive {
            self.query.clone()
        } else {
            self.query.to_lowercase()
        };

        for row in 0..rows {
            let mut line = String::new();
            for col in 0..cols {
                if let Some(cell) = grid.get(col as u16, row as u16) {
                    line.push(cell.c);
                }
            }

            let search_line = if self.case_sensitive {
                line.clone()
            } else {
                line.to_lowercase()
            };

            // Simple substring search
            let mut start = 0;
            while let Some(pos) = search_line[start..].find(&search_query) {
                let col_start = start + pos;
                let col_end = col_start + search_query.len();
                self.matches.push(SearchMatch {
                    row,
                    col_start,
                    col_end,
                });
                start = col_start + 1;
            }
        }
    }

    fn next_match(&mut self) {
        if !self.matches.is_empty() {
            self.current_match = (self.current_match + 1) % self.matches.len();
        }
    }

    fn prev_match(&mut self) {
        if !self.matches.is_empty() {
            if self.current_match == 0 {
                self.current_match = self.matches.len() - 1;
            } else {
                self.current_match -= 1;
            }
        }
    }

    pub fn current_match(&self) -> Option<&SearchMatch> {
        self.matches.get(self.current_match)
    }

    pub fn matches(&self) -> &[SearchMatch] {
        &self.matches
    }

    pub fn clear(&mut self) {
        self.query.clear();
        self.matches.clear();
        self.current_match = 0;
    }
}

impl Default for SearchOverlay {
    fn default() -> Self {
        Self::new()
    }
}
