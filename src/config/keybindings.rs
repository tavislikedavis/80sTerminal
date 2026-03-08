use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keybindings {
    pub new_tab: String,
    pub close_tab: String,
    pub next_tab: String,
    pub prev_tab: String,
    pub split_horizontal: String,
    pub split_vertical: String,
    pub close_pane: String,
    pub search: String,
    pub copy: String,
    pub paste: String,
    pub toggle_fullscreen: String,
    pub increase_font_size: String,
    pub decrease_font_size: String,
    pub reset_font_size: String,
}

impl Default for Keybindings {
    fn default() -> Self {
        Self {
            new_tab: "Cmd+T".to_string(),
            close_tab: "Cmd+W".to_string(),
            next_tab: "Cmd+Shift+]".to_string(),
            prev_tab: "Cmd+Shift+[".to_string(),
            split_horizontal: "Cmd+D".to_string(),
            split_vertical: "Cmd+Shift+D".to_string(),
            close_pane: "Cmd+Shift+W".to_string(),
            search: "Cmd+F".to_string(),
            copy: "Cmd+C".to_string(),
            paste: "Cmd+V".to_string(),
            toggle_fullscreen: "Cmd+Enter".to_string(),
            increase_font_size: "Cmd+=".to_string(),
            decrease_font_size: "Cmd+-".to_string(),
            reset_font_size: "Cmd+0".to_string(),
        }
    }
}
