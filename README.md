# 80sTerminal

A GPU-accelerated terminal emulator built in Rust that recreates authentic CRT monitor experiences from the 1980s. Each CRT style features historically accurate bezel colors, phosphor characteristics, and visual effects rendered in real-time via custom WGSL shaders.

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=flat&logo=rust&logoColor=white)
![macOS](https://img.shields.io/badge/macOS-000000?style=flat&logo=apple&logoColor=white)
![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)

## Screenshots

| IBM 3278 (Green Phosphor) | DEC VT220 (Amber Phosphor) | Commodore 64 |
|:---:|:---:|:---:|
| ![IBM 3278](assets/screenshots/1.png) | ![DEC VT220](assets/screenshots/2.png) | ![Commodore 64](assets/screenshots/3.png) |

## Features

### CRT Monitor Styles
11 historically accurate terminal styles with per-monitor bezel colors and effects:

| Style | Phosphor | Bezel |
|-------|----------|-------|
| IBM 3278 | Green P39 | Dark charcoal |
| DEC VT220 | Amber P3 | Off-white cream |
| Apple //e | Green | Classic Apple beige |
| Commodore 64 | Blue | Dark brown (1702 monitor) |
| IBM PC/XT (CGA) | White | Dark gray (5153 monitor) |
| Osborne 1 | Green | Cream, large bezel |
| Zenith Z-19 | White | Warm off-white |
| Kaypro II | Green | Metallic dark gray |
| Televideo 950 | Amber | Warm beige |
| Tektronix 4010 | Green vector | Dark olive |
| NeXT MegaPixel | White | Matte black |

### GPU Shader Effects
All effects are rendered in real-time via wgpu/WGSL:
- Barrel distortion (screen curvature)
- Scanlines with configurable intensity and count
- Phosphor bloom and glow
- Chromatic aberration
- Screen vignette
- 60Hz flicker simulation
- Static noise
- RGB phosphor mask pattern
- Photorealistic 3D bezels with lighting and shadows

### Terminal Features
- **Tabs** - iTerm2-style tab bar (Cmd+T, Cmd+W, Cmd+1-9)
- **Multiple windows** - Cmd+N
- **Scrollback buffer** - Configurable or unlimited
- **Copy/Paste** - Cmd+C/V with mouse text selection
- **Search** - In-terminal search overlay
- **Font sizing** - Cmd+/- or Cmd+0 to reset
- **Window transparency** - Adjustable per-window, bezel stays opaque
- **Cursor blink** - Configurable
- **Profiles** - Save and load terminal configurations
- **Native macOS menus** - Full menu bar with keyboard shortcuts

### Preferences
Separate native macOS preferences window (Cmd+,) with:
- Font size controls
- Cursor blink toggle
- Window transparency slider (screen only, bezel stays solid)
- Scrollback buffer size (or unlimited)
- Profile management
- Config file save/open

## Building

### Prerequisites
- Rust 1.70+ (install via [rustup](https://rustup.rs))
- macOS 12+ (primary target)

### Build & Run
```bash
cargo build --release
./target/release/80sterminal
```

Or for development:
```bash
cargo run
```

### macOS App Bundle
To build a standalone `.app` bundle with an icon:
```bash
# Generate the app icon (requires Python 3 + Pillow)
pip3 install Pillow
python3 scripts/make_icon.py

# Build the .app bundle
./scripts/bundle.sh
```

The bundle is created at `target/release/80sTerminal.app`. To install:
```bash
cp -r target/release/80sTerminal.app /Applications/
```

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| Cmd+T | New tab |
| Cmd+W | Close tab (or window if last tab) |
| Cmd+N | New window |
| Cmd+, | Preferences |
| Cmd+C | Copy selection (or Ctrl+C if no selection) |
| Cmd+V | Paste |
| Cmd++ | Increase font size |
| Cmd+- | Decrease font size |
| Cmd+0 | Reset font size |
| Cmd+1-9 | Switch to tab by number |
| Cmd+Shift+] | Next tab |
| Cmd+Shift+[ | Previous tab |

## Configuration

Config file is stored at:
```
~/Library/Application Support/com.80sterminal.80sterminal/config.toml
```

Access it via **Settings > Open Config File** in the menu bar.

### Example config.toml
```toml
[appearance]
font_family = "JetBrains Mono"
font_size = 18.0
cursor_blink = true
opacity = 1.0

[crt]
enabled = true
curvature = 0.04
scanline_intensity = 0.45
bezel_size = 0.06
bezel_color = [0.08, 0.08, 0.08]

[colors]
foreground = "#33FF33"
background = "#0A0A0A"
```

## Architecture

```
src/
├── main.rs              # Event loop, native menu bar
├── app.rs               # Application state, input handling
├── config/
│   ├── mod.rs           # Config, CRT styles, color schemes
│   ├── profile.rs       # Terminal profiles
│   └── keybindings.rs   # Keybinding config
├── terminal/
│   ├── mod.rs           # PTY management
│   ├── grid.rs          # Terminal grid + scrollback buffer
│   ├── parser.rs        # VTE escape sequence parsing
│   └── selection.rs     # Mouse text selection
├── render/
│   ├── mod.rs           # Render orchestration
│   ├── context.rs       # wgpu setup + macOS transparency
│   ├── text.rs          # Glyph atlas text renderer
│   └── crt/
│       └── pipeline.rs  # CRT post-processing pipeline
├── shaders/
│   ├── text.wgsl        # Text rendering shader
│   └── crt.wgsl         # CRT effects + bezel shader
└── ui/
    ├── mod.rs            # UI orchestration
    ├── tabs.rs           # iTerm2-style tab bar
    ├── settings_window.rs # Native preferences window
    ├── search.rs         # Search overlay
    └── splits.rs         # Split pane logic
```

### Rendering Pipeline
1. **Text Renderer** - Rasterizes terminal grid to an offscreen texture using a glyph atlas (fontdue)
2. **CRT Shader** - Post-processes the text texture with barrel distortion, scanlines, bloom, chromatic aberration, noise, and renders the photorealistic bezel
3. **egui Overlay** - Renders the tab bar and search UI on top via egui-wgpu

## Dependencies

| Crate | Purpose |
|-------|---------|
| wgpu | GPU rendering |
| winit | Window management |
| egui / egui-wgpu / egui-winit | UI components |
| vte | ANSI escape sequence parsing |
| portable-pty | PTY management |
| fontdue | Font rasterization |
| muda | Native macOS menu bar |
| cocoa / objc | macOS window transparency |
| copypasta | Clipboard |
| serde / toml | Configuration |

## License

MIT
