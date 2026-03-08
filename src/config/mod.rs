mod keybindings;
mod profile;

pub use keybindings::Keybindings;
pub use profile::Profile;

use directories::ProjectDirs;
use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub appearance: Appearance,
    #[serde(default)]
    pub crt: CrtConfig,
    #[serde(default)]
    pub colors: Colors,
    #[serde(default)]
    pub keybindings: Keybindings,
    #[serde(default)]
    pub profiles: HashMap<String, Profile>,
    #[serde(default = "default_profile_name")]
    pub active_profile: String,
}

fn default_profile_name() -> String {
    "default".to_string()
}

impl Default for Config {
    fn default() -> Self {
        let mut profiles = HashMap::new();
        profiles.insert("default".to_string(), Profile::default());

        Self {
            appearance: Appearance::default(),
            crt: CrtConfig::default(),
            colors: Colors::default(),
            keybindings: Keybindings::default(),
            profiles,
            active_profile: "default".to_string(),
        }
    }
}

impl Config {
    pub fn load_or_default() -> Self {
        if let Some(path) = Self::config_path() {
            if path.exists() {
                match fs::read_to_string(&path) {
                    Ok(content) => match toml::from_str(&content) {
                        Ok(config) => {
                            info!("Loaded config from {:?}", path);
                            return config;
                        }
                        Err(e) => {
                            warn!("Failed to parse config: {}", e);
                        }
                    },
                    Err(e) => {
                        warn!("Failed to read config: {}", e);
                    }
                }
            }
        }
        info!("Using default config");
        Self::default()
    }

    pub fn config_path() -> Option<PathBuf> {
        ProjectDirs::from("com", "80sterminal", "80sterminal")
            .map(|dirs| dirs.config_dir().join("config.toml"))
    }

    pub fn get_profile(&self, name: &str) -> &Profile {
        self.profiles
            .get(name)
            .unwrap_or_else(|| self.profiles.get("default").unwrap())
    }

    pub fn active_profile_config(&self) -> Option<&Profile> {
        self.profiles.get(&self.active_profile)
    }

    pub fn save(&self) -> anyhow::Result<()> {
        if let Some(path) = Self::config_path() {
            // Create parent directory if it doesn't exist
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let content = toml::to_string_pretty(self)?;
            fs::write(&path, content)?;
            info!("Saved config to {:?}", path);
        }
        Ok(())
    }

    pub fn save_window_size(&mut self, width: f64, height: f64) {
        if let Some(profile) = self.profiles.get_mut(&self.active_profile) {
            profile.window_width = width;
            profile.window_height = height;
        }
    }

    pub fn window_size(&self) -> (f64, f64) {
        if let Some(profile) = self.profiles.get(&self.active_profile) {
            (profile.window_width, profile.window_height)
        } else {
            (1024.0, 768.0)
        }
    }

    pub fn save_as_profile(&mut self, name: &str) {
        let mut profile = if let Some(existing) = self.profiles.get(name) {
            existing.clone()
        } else {
            Profile::default()
        };
        profile.name = name.to_string();
        profile.font_size = self.appearance.font_size;
        self.profiles.insert(name.to_string(), profile);
        self.active_profile = name.to_string();
    }

    pub fn load_profile(&mut self, name: &str) {
        if let Some(profile) = self.profiles.get(name) {
            self.appearance.font_size = profile.font_size;
            self.active_profile = name.to_string();
            info!("Loaded profile: {}", name);
        }
    }

    pub fn profile_names(&self) -> Vec<String> {
        self.profiles.keys().cloned().collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Appearance {
    pub font_family: String,
    pub font_size: f32,
    pub line_height: f32,
    #[serde(default = "default_true")]
    pub cursor_blink: bool,
    #[serde(default = "default_opacity")]
    pub opacity: f32,
}

fn default_true() -> bool {
    true
}

fn default_opacity() -> f32 {
    1.0
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            font_family: "JetBrains Mono".to_string(),
            font_size: 18.0,
            line_height: 1.2,
            cursor_blink: true,
            opacity: 1.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrtConfig {
    pub enabled: bool,
    pub curvature: f32,
    pub scanline_intensity: f32,
    pub scanline_count: f32,
    pub bloom_intensity: f32,
    pub bloom_radius: f32,
    pub chromatic_aberration: f32,
    pub vignette_intensity: f32,
    pub flicker: bool,
    pub flicker_intensity: f32,
    pub noise: f32,
    pub phosphor_persistence: f32,
    pub bezel_size: f32,
    pub screen_brightness: f32,
    #[serde(default = "default_bezel_color")]
    pub bezel_color: [f32; 3],
    #[serde(default = "default_bezel_corner_radius")]
    pub bezel_corner_radius: f32,
}

fn default_bezel_color() -> [f32; 3] {
    [0.78, 0.73, 0.65] // Classic Macintosh beige
}

fn default_bezel_corner_radius() -> f32 {
    0.015
}

impl Default for CrtConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            curvature: 0.04,
            scanline_intensity: 0.45,
            scanline_count: 800.0,
            bloom_intensity: 0.4,
            bloom_radius: 2.5,
            chromatic_aberration: 0.002,
            vignette_intensity: 0.3,
            flicker: true,
            flicker_intensity: 0.02,
            noise: 0.015,
            phosphor_persistence: 0.85,
            bezel_size: 0.06,
            screen_brightness: 1.1,
            bezel_color: [0.78, 0.73, 0.65], // Classic Macintosh beige
            bezel_corner_radius: 0.015,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Colors {
    pub foreground: String,
    pub background: String,
    pub cursor: String,
    pub selection: String,

    // ANSI colors
    pub black: String,
    pub red: String,
    pub green: String,
    pub yellow: String,
    pub blue: String,
    pub magenta: String,
    pub cyan: String,
    pub white: String,

    // Bright variants
    pub bright_black: String,
    pub bright_red: String,
    pub bright_green: String,
    pub bright_yellow: String,
    pub bright_blue: String,
    pub bright_magenta: String,
    pub bright_cyan: String,
    pub bright_white: String,
}

impl Default for Colors {
    fn default() -> Self {
        // Classic green phosphor CRT colors
        Self {
            foreground: "#33FF33".to_string(),
            background: "#0A0A0A".to_string(),
            cursor: "#33FF33".to_string(),
            selection: "#1A4A1A".to_string(),

            // Green-tinted ANSI colors for CRT aesthetic
            black: "#0A0A0A".to_string(),
            red: "#AA3333".to_string(),
            green: "#33FF33".to_string(),
            yellow: "#AAFF33".to_string(),
            blue: "#33AAAA".to_string(),
            magenta: "#AA33AA".to_string(),
            cyan: "#33FFAA".to_string(),
            white: "#AAFFAA".to_string(),

            bright_black: "#1A1A1A".to_string(),
            bright_red: "#FF5555".to_string(),
            bright_green: "#55FF55".to_string(),
            bright_yellow: "#FFFF55".to_string(),
            bright_blue: "#55FFFF".to_string(),
            bright_magenta: "#FF55FF".to_string(),
            bright_cyan: "#55FFFF".to_string(),
            bright_white: "#FFFFFF".to_string(),
        }
    }
}

/// A CRT terminal style preset combining CRT effects and color scheme
pub struct CrtStyle {
    pub name: &'static str,
    pub crt: CrtConfig,
    pub colors: Colors,
}

pub fn crt_styles() -> Vec<CrtStyle> {
    vec![
        // 1. IBM 3278 - Dark charcoal professional terminal
        CrtStyle {
            name: "IBM 3278 (Green Phosphor)",
            crt: CrtConfig {
                enabled: true,
                curvature: 0.06,
                scanline_intensity: 0.55,
                scanline_count: 700.0,
                bloom_intensity: 0.5,
                bloom_radius: 3.0,
                chromatic_aberration: 0.001,
                vignette_intensity: 0.4,
                flicker: true,
                flicker_intensity: 0.03,
                noise: 0.02,
                phosphor_persistence: 0.9,
                bezel_size: 0.06,
                screen_brightness: 1.15,
                bezel_color: [0.20, 0.20, 0.22], // Dark charcoal IBM case
                bezel_corner_radius: 0.012,
            },
            colors: Colors {
                foreground: "#33FF33".into(),
                background: "#0A0A0A".into(),
                cursor: "#33FF33".into(),
                selection: "#1A4A1A".into(),
                black: "#0A0A0A".into(),
                red: "#33AA33".into(),
                green: "#33FF33".into(),
                yellow: "#66FF66".into(),
                blue: "#22BB22".into(),
                magenta: "#44DD44".into(),
                cyan: "#33FF99".into(),
                white: "#AAFFAA".into(),
                bright_black: "#1A3A1A".into(),
                bright_red: "#55CC55".into(),
                bright_green: "#55FF55".into(),
                bright_yellow: "#88FF88".into(),
                bright_blue: "#44DD44".into(),
                bright_magenta: "#66EE66".into(),
                bright_cyan: "#55FFAA".into(),
                bright_white: "#CCFFCC".into(),
            },
        },
        // 2. DEC VT220 - Off-white/cream DEC terminal case
        CrtStyle {
            name: "DEC VT220 (Amber)",
            crt: CrtConfig {
                enabled: true,
                curvature: 0.04,
                scanline_intensity: 0.4,
                scanline_count: 800.0,
                bloom_intensity: 0.35,
                bloom_radius: 2.0,
                chromatic_aberration: 0.001,
                vignette_intensity: 0.3,
                flicker: true,
                flicker_intensity: 0.015,
                noise: 0.01,
                phosphor_persistence: 0.88,
                bezel_size: 0.06,
                screen_brightness: 1.1,
                bezel_color: [0.75, 0.72, 0.65], // DEC off-white/cream
                bezel_corner_radius: 0.010,
            },
            colors: Colors {
                foreground: "#FFB000".into(),
                background: "#0A0800".into(),
                cursor: "#FFB000".into(),
                selection: "#4A3800".into(),
                black: "#0A0800".into(),
                red: "#CC8800".into(),
                green: "#DDAA00".into(),
                yellow: "#FFB000".into(),
                blue: "#AA7700".into(),
                magenta: "#CC9922".into(),
                cyan: "#DDAA44".into(),
                white: "#FFCC66".into(),
                bright_black: "#332200".into(),
                bright_red: "#DDAA22".into(),
                bright_green: "#EEBB33".into(),
                bright_yellow: "#FFCC44".into(),
                bright_blue: "#BB8811".into(),
                bright_magenta: "#DDAA44".into(),
                bright_cyan: "#EEBB55".into(),
                bright_white: "#FFDD88".into(),
            },
        },
        // 3. Apple //e - Classic Apple beige/platinum
        CrtStyle {
            name: "Apple //e",
            crt: CrtConfig {
                enabled: true,
                curvature: 0.05,
                scanline_intensity: 0.3,
                scanline_count: 500.0,
                bloom_intensity: 0.6,
                bloom_radius: 3.5,
                chromatic_aberration: 0.003,
                vignette_intensity: 0.35,
                flicker: true,
                flicker_intensity: 0.02,
                noise: 0.025,
                phosphor_persistence: 0.82,
                bezel_size: 0.06,
                screen_brightness: 1.2,
                bezel_color: [0.78, 0.73, 0.65], // Classic Apple beige
                bezel_corner_radius: 0.015,
            },
            colors: Colors {
                foreground: "#41FF00".into(),
                background: "#000000".into(),
                cursor: "#41FF00".into(),
                selection: "#1A4A00".into(),
                black: "#000000".into(),
                red: "#CC3333".into(),
                green: "#41FF00".into(),
                yellow: "#CCFF00".into(),
                blue: "#00AACC".into(),
                magenta: "#CC44CC".into(),
                cyan: "#00FFAA".into(),
                white: "#AAFFAA".into(),
                bright_black: "#333333".into(),
                bright_red: "#FF5555".into(),
                bright_green: "#66FF33".into(),
                bright_yellow: "#EEFF55".into(),
                bright_blue: "#55CCFF".into(),
                bright_magenta: "#FF66FF".into(),
                bright_cyan: "#55FFCC".into(),
                bright_white: "#FFFFFF".into(),
            },
        },
        // 4. Commodore 64 - Commodore 1702 dark brown monitor
        CrtStyle {
            name: "Commodore 64",
            crt: CrtConfig {
                enabled: true,
                curvature: 0.07,
                scanline_intensity: 0.5,
                scanline_count: 480.0,
                bloom_intensity: 0.45,
                bloom_radius: 3.0,
                chromatic_aberration: 0.004,
                vignette_intensity: 0.4,
                flicker: true,
                flicker_intensity: 0.03,
                noise: 0.03,
                phosphor_persistence: 0.8,
                bezel_size: 0.06,
                screen_brightness: 1.0,
                bezel_color: [0.28, 0.22, 0.16], // Dark brown Commodore 1702
                bezel_corner_radius: 0.020,
            },
            colors: Colors {
                foreground: "#A0A0FF".into(),
                background: "#40318D".into(),
                cursor: "#A0A0FF".into(),
                selection: "#6050AA".into(),
                black: "#000000".into(),
                red: "#883932".into(),
                green: "#67B551".into(),
                yellow: "#C8D077".into(),
                blue: "#40318D".into(),
                magenta: "#8B4F96".into(),
                cyan: "#6FC2DB".into(),
                white: "#C0C0C0".into(),
                bright_black: "#444444".into(),
                bright_red: "#BB6662".into(),
                bright_green: "#97E581".into(),
                bright_yellow: "#F8FFA7".into(),
                bright_blue: "#7061BD".into(),
                bright_magenta: "#BB7FC6".into(),
                bright_cyan: "#9FF2FF".into(),
                bright_white: "#FFFFFF".into(),
            },
        },
        // 5. IBM PC/XT - IBM 5153 dark gray monitor
        CrtStyle {
            name: "IBM PC/XT (CGA)",
            crt: CrtConfig {
                enabled: true,
                curvature: 0.03,
                scanline_intensity: 0.35,
                scanline_count: 900.0,
                bloom_intensity: 0.25,
                bloom_radius: 2.0,
                chromatic_aberration: 0.001,
                vignette_intensity: 0.25,
                flicker: false,
                flicker_intensity: 0.01,
                noise: 0.008,
                phosphor_persistence: 0.85,
                bezel_size: 0.06,
                screen_brightness: 1.05,
                bezel_color: [0.25, 0.25, 0.27], // IBM 5153 dark gray
                bezel_corner_radius: 0.010,
            },
            colors: Colors {
                foreground: "#AAAAAA".into(),
                background: "#000000".into(),
                cursor: "#AAAAAA".into(),
                selection: "#333333".into(),
                black: "#000000".into(),
                red: "#AA0000".into(),
                green: "#00AA00".into(),
                yellow: "#AA5500".into(),
                blue: "#0000AA".into(),
                magenta: "#AA00AA".into(),
                cyan: "#00AAAA".into(),
                white: "#AAAAAA".into(),
                bright_black: "#555555".into(),
                bright_red: "#FF5555".into(),
                bright_green: "#55FF55".into(),
                bright_yellow: "#FFFF55".into(),
                bright_blue: "#5555FF".into(),
                bright_magenta: "#FF55FF".into(),
                bright_cyan: "#55FFFF".into(),
                bright_white: "#FFFFFF".into(),
            },
        },
        // 6. Osborne 1 - Beige/cream luggable case with large bezel
        CrtStyle {
            name: "Osborne 1",
            crt: CrtConfig {
                enabled: true,
                curvature: 0.12,
                scanline_intensity: 0.6,
                scanline_count: 500.0,
                bloom_intensity: 0.55,
                bloom_radius: 3.5,
                chromatic_aberration: 0.005,
                vignette_intensity: 0.5,
                flicker: true,
                flicker_intensity: 0.04,
                noise: 0.035,
                phosphor_persistence: 0.78,
                bezel_size: 0.10,  // Larger bezel - tiny screen in big case
                screen_brightness: 1.2,
                bezel_color: [0.68, 0.64, 0.55], // Warm cream/beige Osborne case
                bezel_corner_radius: 0.025,
            },
            colors: Colors {
                foreground: "#33DD33".into(),
                background: "#050A05".into(),
                cursor: "#33DD33".into(),
                selection: "#1A3A1A".into(),
                black: "#050A05".into(),
                red: "#339933".into(),
                green: "#33DD33".into(),
                yellow: "#55DD55".into(),
                blue: "#228822".into(),
                magenta: "#33BB33".into(),
                cyan: "#33DD88".into(),
                white: "#88DD88".into(),
                bright_black: "#113311".into(),
                bright_red: "#44AA44".into(),
                bright_green: "#55FF55".into(),
                bright_yellow: "#77FF77".into(),
                bright_blue: "#33BB33".into(),
                bright_magenta: "#55CC55".into(),
                bright_cyan: "#55FF99".into(),
                bright_white: "#AAFFAA".into(),
            },
        },
        // 7. Zenith Z-19 - Off-white Zenith terminal
        CrtStyle {
            name: "Zenith Z-19 (White)",
            crt: CrtConfig {
                enabled: true,
                curvature: 0.03,
                scanline_intensity: 0.3,
                scanline_count: 900.0,
                bloom_intensity: 0.3,
                bloom_radius: 2.0,
                chromatic_aberration: 0.002,
                vignette_intensity: 0.2,
                flicker: false,
                flicker_intensity: 0.01,
                noise: 0.01,
                phosphor_persistence: 0.85,
                bezel_size: 0.06,
                screen_brightness: 1.1,
                bezel_color: [0.82, 0.80, 0.74], // Warm off-white Zenith
                bezel_corner_radius: 0.012,
            },
            colors: Colors {
                foreground: "#E0E0E8".into(),
                background: "#0A0A10".into(),
                cursor: "#E0E0E8".into(),
                selection: "#303040".into(),
                black: "#0A0A10".into(),
                red: "#CC5555".into(),
                green: "#55CC55".into(),
                yellow: "#CCCC55".into(),
                blue: "#5555CC".into(),
                magenta: "#CC55CC".into(),
                cyan: "#55CCCC".into(),
                white: "#E0E0E8".into(),
                bright_black: "#404050".into(),
                bright_red: "#FF7777".into(),
                bright_green: "#77FF77".into(),
                bright_yellow: "#FFFF77".into(),
                bright_blue: "#7777FF".into(),
                bright_magenta: "#FF77FF".into(),
                bright_cyan: "#77FFFF".into(),
                bright_white: "#FFFFFF".into(),
            },
        },
        // 8. Kaypro II - Metallic dark gray portable case
        CrtStyle {
            name: "Kaypro II",
            crt: CrtConfig {
                enabled: true,
                curvature: 0.05,
                scanline_intensity: 0.45,
                scanline_count: 600.0,
                bloom_intensity: 0.4,
                bloom_radius: 2.5,
                chromatic_aberration: 0.002,
                vignette_intensity: 0.35,
                flicker: true,
                flicker_intensity: 0.02,
                noise: 0.02,
                phosphor_persistence: 0.86,
                bezel_size: 0.08,  // Larger bezel for portable
                screen_brightness: 1.1,
                bezel_color: [0.32, 0.32, 0.34], // Metallic dark gray Kaypro
                bezel_corner_radius: 0.008,
            },
            colors: Colors {
                foreground: "#44DD44".into(),
                background: "#081208".into(),
                cursor: "#44DD44".into(),
                selection: "#1A3A1A".into(),
                black: "#081208".into(),
                red: "#44AA22".into(),
                green: "#44DD44".into(),
                yellow: "#77EE44".into(),
                blue: "#33AA33".into(),
                magenta: "#55CC33".into(),
                cyan: "#44DD77".into(),
                white: "#99EE99".into(),
                bright_black: "#1A2A1A".into(),
                bright_red: "#55BB33".into(),
                bright_green: "#66FF66".into(),
                bright_yellow: "#99FF66".into(),
                bright_blue: "#44CC44".into(),
                bright_magenta: "#77DD55".into(),
                bright_cyan: "#66FF99".into(),
                bright_white: "#BBFFBB".into(),
            },
        },
        // 9. Televideo 950 - Warm beige/cream terminal
        CrtStyle {
            name: "Televideo 950 (Amber)",
            crt: CrtConfig {
                enabled: true,
                curvature: 0.05,
                scanline_intensity: 0.5,
                scanline_count: 750.0,
                bloom_intensity: 0.55,
                bloom_radius: 3.0,
                chromatic_aberration: 0.002,
                vignette_intensity: 0.35,
                flicker: true,
                flicker_intensity: 0.025,
                noise: 0.015,
                phosphor_persistence: 0.9,
                bezel_size: 0.06,
                screen_brightness: 1.15,
                bezel_color: [0.73, 0.70, 0.62], // Warm beige Televideo case
                bezel_corner_radius: 0.012,
            },
            colors: Colors {
                foreground: "#FFA500".into(),
                background: "#0D0800".into(),
                cursor: "#FFA500".into(),
                selection: "#4A3000".into(),
                black: "#0D0800".into(),
                red: "#CC7700".into(),
                green: "#DD9900".into(),
                yellow: "#FFA500".into(),
                blue: "#AA6600".into(),
                magenta: "#CC8822".into(),
                cyan: "#DD9944".into(),
                white: "#FFBB55".into(),
                bright_black: "#332200".into(),
                bright_red: "#DD9922".into(),
                bright_green: "#EEAA33".into(),
                bright_yellow: "#FFBB44".into(),
                bright_blue: "#BB7711".into(),
                bright_magenta: "#DDAA44".into(),
                bright_cyan: "#EEBB55".into(),
                bright_white: "#FFDD88".into(),
            },
        },
        // 10. Tektronix 4010 - Dark olive/green-gray instrument case
        CrtStyle {
            name: "Tektronix 4010 (Vector)",
            crt: CrtConfig {
                enabled: true,
                curvature: 0.02,
                scanline_intensity: 0.15,
                scanline_count: 1200.0,
                bloom_intensity: 0.7,
                bloom_radius: 4.0,
                chromatic_aberration: 0.0,
                vignette_intensity: 0.2,
                flicker: false,
                flicker_intensity: 0.01,
                noise: 0.005,
                phosphor_persistence: 0.95,
                bezel_size: 0.06,
                screen_brightness: 1.3,
                bezel_color: [0.28, 0.30, 0.24], // Dark olive Tektronix
                bezel_corner_radius: 0.010,
            },
            colors: Colors {
                foreground: "#00FF66".into(),
                background: "#000800".into(),
                cursor: "#00FF66".into(),
                selection: "#004422".into(),
                black: "#000800".into(),
                red: "#00CC44".into(),
                green: "#00FF66".into(),
                yellow: "#33FF88".into(),
                blue: "#00BB55".into(),
                magenta: "#22DD66".into(),
                cyan: "#00FF99".into(),
                white: "#66FFAA".into(),
                bright_black: "#003311".into(),
                bright_red: "#22DD55".into(),
                bright_green: "#33FF88".into(),
                bright_yellow: "#66FFAA".into(),
                bright_blue: "#22DD77".into(),
                bright_magenta: "#44EE88".into(),
                bright_cyan: "#44FFBB".into(),
                bright_white: "#AAFFCC".into(),
            },
        },
        // 11. NeXT MegaPixel Display - Matte black with crisp white phosphor
        CrtStyle {
            name: "NeXT MegaPixel Display",
            crt: CrtConfig {
                enabled: true,
                curvature: 0.01,             // Nearly flat - advanced for its era
                scanline_intensity: 0.2,
                scanline_count: 1120.0,      // High res (1120x832)
                bloom_intensity: 0.2,
                bloom_radius: 1.5,
                chromatic_aberration: 0.0,    // Crisp monochrome display
                vignette_intensity: 0.15,
                flicker: false,
                flicker_intensity: 0.005,
                noise: 0.005,
                phosphor_persistence: 0.92,
                bezel_size: 0.05,            // Slim bezel for its time
                screen_brightness: 1.15,
                bezel_color: [0.08, 0.08, 0.08], // Matte black NeXT cube aesthetic
                bezel_corner_radius: 0.005,      // Sharp corners
            },
            colors: Colors {
                foreground: "#E8E8E8".into(),    // Crisp white phosphor
                background: "#000000".into(),
                cursor: "#E8E8E8".into(),
                selection: "#404040".into(),
                black: "#000000".into(),
                red: "#CC4444".into(),
                green: "#44CC44".into(),
                yellow: "#CCCC44".into(),
                blue: "#4444CC".into(),
                magenta: "#CC44CC".into(),
                cyan: "#44CCCC".into(),
                white: "#E8E8E8".into(),
                bright_black: "#555555".into(),
                bright_red: "#FF6666".into(),
                bright_green: "#66FF66".into(),
                bright_yellow: "#FFFF66".into(),
                bright_blue: "#6666FF".into(),
                bright_magenta: "#FF66FF".into(),
                bright_cyan: "#66FFFF".into(),
                bright_white: "#FFFFFF".into(),
            },
        },
    ]
}

impl Config {
    /// Apply a CRT style preset by name
    pub fn apply_crt_style(&mut self, name: &str) {
        if let Some(style) = crt_styles().into_iter().find(|s| s.name == name) {
            self.crt = style.crt;
            self.colors = style.colors;
            info!("Applied CRT style: {}", name);
        }
    }

    /// Get list of available CRT style names
    pub fn crt_style_names() -> Vec<&'static str> {
        crt_styles().iter().map(|s| s.name).collect()
    }
}

impl Colors {
    pub fn parse_hex(hex: &str) -> [f32; 4] {
        let hex = hex.trim_start_matches('#');
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0) as f32 / 255.0;
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0) as f32 / 255.0;
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0) as f32 / 255.0;
        [r, g, b, 1.0]
    }

    pub fn foreground_rgba(&self) -> [f32; 4] {
        Self::parse_hex(&self.foreground)
    }

    pub fn background_rgba(&self) -> [f32; 4] {
        Self::parse_hex(&self.background)
    }

    pub fn get_ansi_color(&self, index: u8) -> [f32; 4] {
        let hex = match index {
            0 => &self.black,
            1 => &self.red,
            2 => &self.green,
            3 => &self.yellow,
            4 => &self.blue,
            5 => &self.magenta,
            6 => &self.cyan,
            7 => &self.white,
            8 => &self.bright_black,
            9 => &self.bright_red,
            10 => &self.bright_green,
            11 => &self.bright_yellow,
            12 => &self.bright_blue,
            13 => &self.bright_magenta,
            14 => &self.bright_cyan,
            15 => &self.bright_white,
            _ => &self.foreground,
        };
        Self::parse_hex(hex)
    }
}
