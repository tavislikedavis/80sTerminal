use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub shell: String,
    pub working_directory: String,
    pub environment: Vec<(String, String)>,
    pub scrollback_lines: usize,
    #[serde(default)]
    pub unlimited_scrollback: bool,
    #[serde(default = "default_font_size")]
    pub font_size: f32,
    #[serde(default = "default_window_width")]
    pub window_width: f64,
    #[serde(default = "default_window_height")]
    pub window_height: f64,
}

fn default_font_size() -> f32 {
    18.0
}

fn default_window_width() -> f64 {
    1024.0
}

fn default_window_height() -> f64 {
    768.0
}

impl Default for Profile {
    fn default() -> Self {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());

        Self {
            name: "Default".to_string(),
            shell,
            working_directory: "~".to_string(),
            environment: Vec::new(),
            scrollback_lines: 10000,
            unlimited_scrollback: false,
            font_size: 18.0,
            window_width: 1024.0,
            window_height: 768.0,
        }
    }
}
