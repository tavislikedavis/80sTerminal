use super::grid::{CellColor, CellStyle, Grid};
use log::debug;
use std::cell::RefCell;
use vte::{Params, Perform};

pub struct Parser {
    vte_parser: vte::Parser,
}

impl Parser {
    pub fn new() -> Self {
        Self {
            vte_parser: vte::Parser::new(),
        }
    }

    pub fn process(&mut self, data: &[u8], grid: &mut Grid) {
        GRID.with(|g| {
            *g.borrow_mut() = Some(grid as *mut Grid);
        });

        let mut performer = Performer;
        for byte in data {
            self.vte_parser.advance(&mut performer, *byte);
        }

        GRID.with(|g| {
            *g.borrow_mut() = None;
        });
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

thread_local! {
    static GRID: RefCell<Option<*mut Grid>> = const { RefCell::new(None) };
}

fn with_grid<F, R>(f: F) -> R
where
    F: FnOnce(&mut Grid) -> R,
{
    GRID.with(|g| {
        let ptr = g.borrow().expect("Grid not set");
        unsafe { f(&mut *ptr) }
    })
}

struct Performer;

impl Perform for Performer {
    fn print(&mut self, c: char) {
        with_grid(|grid| grid.put_char(c));
    }

    fn execute(&mut self, byte: u8) {
        with_grid(|grid| {
            match byte {
                0x07 => {} // Bell
                0x08 => grid.put_char('\x08'), // Backspace
                0x09 => grid.put_char('\t'),   // Tab
                0x0A | 0x0B | 0x0C => grid.linefeed(), // LF, VT, FF
                0x0D => grid.put_char('\r'),   // CR
                0x0E => {} // Shift Out
                0x0F => {} // Shift In
                _ => debug!("Unhandled execute: 0x{:02x}", byte),
            }
        });
    }

    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: char) {}

    fn put(&mut self, _byte: u8) {}

    fn unhook(&mut self) {}

    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        if params.is_empty() {
            return;
        }

        match params[0] {
            b"0" | b"1" | b"2" => {
                // Set window title - we could handle this
            }
            _ => {}
        }
    }

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], _ignore: bool, action: char) {
        with_grid(|grid| {
            let params: Vec<u16> = params.iter().map(|p| p[0]).collect();

            match (action, intermediates) {
                // Cursor movement
                ('A', []) => {
                    let n = params.first().copied().unwrap_or(1).max(1) as i16;
                    grid.move_cursor(0, -n);
                }
                ('B', []) => {
                    let n = params.first().copied().unwrap_or(1).max(1) as i16;
                    grid.move_cursor(0, n);
                }
                ('C', []) => {
                    let n = params.first().copied().unwrap_or(1).max(1) as i16;
                    grid.move_cursor(n, 0);
                }
                ('D', []) => {
                    let n = params.first().copied().unwrap_or(1).max(1) as i16;
                    grid.move_cursor(-n, 0);
                }
                ('E', []) => {
                    let n = params.first().copied().unwrap_or(1).max(1);
                    grid.move_cursor(0, n as i16);
                    let (_, y) = grid.cursor();
                    grid.set_cursor(0, y);
                }
                ('F', []) => {
                    let n = params.first().copied().unwrap_or(1).max(1);
                    grid.move_cursor(0, -(n as i16));
                    let (_, y) = grid.cursor();
                    grid.set_cursor(0, y);
                }
                ('G', []) => {
                    let col = params.first().copied().unwrap_or(1).saturating_sub(1);
                    let (_, y) = grid.cursor();
                    grid.set_cursor(col, y);
                }
                ('H', []) | ('f', []) => {
                    let row = params.first().copied().unwrap_or(1).saturating_sub(1);
                    let col = params.get(1).copied().unwrap_or(1).saturating_sub(1);
                    grid.set_cursor(col, row);
                }

                // Erase
                ('J', []) => {
                    let mode = params.first().copied().unwrap_or(0);
                    grid.clear_screen(mode);
                }
                ('K', []) => {
                    let mode = params.first().copied().unwrap_or(0);
                    grid.clear_line(mode);
                }

                // Insert/Delete
                ('L', []) => {
                    let n = params.first().copied().unwrap_or(1).max(1);
                    grid.insert_lines(n);
                }
                ('M', []) => {
                    let n = params.first().copied().unwrap_or(1).max(1);
                    grid.delete_lines(n);
                }
                ('P', []) => {
                    let n = params.first().copied().unwrap_or(1).max(1);
                    grid.delete_chars(n);
                }
                ('@', []) => {
                    let n = params.first().copied().unwrap_or(1).max(1);
                    grid.insert_chars(n);
                }

                // Scroll
                ('S', []) => {
                    let n = params.first().copied().unwrap_or(1).max(1);
                    grid.scroll_up(n);
                }
                ('T', []) => {
                    let n = params.first().copied().unwrap_or(1).max(1);
                    grid.scroll_down(n);
                }

                // Set scroll region
                ('r', []) => {
                    let top = params.first().copied().unwrap_or(1).saturating_sub(1);
                    let bottom = params
                        .get(1)
                        .copied()
                        .unwrap_or(grid.rows())
                        .saturating_sub(1);
                    grid.set_scroll_region(top, bottom);
                }

                // SGR (Select Graphic Rendition)
                ('m', []) => {
                    if params.is_empty() {
                        grid.set_style(CellStyle::default());
                    } else {
                        handle_sgr(grid, &params);
                    }
                }

                // DEC Private Mode Set (DECSET)
                ('h', [b'?']) => {
                    for p in &params {
                        match p {
                            1 => {} // Application cursor keys
                            7 => grid.auto_wrap = true,
                            25 => grid.set_cursor_visible(true),
                            47 | 1047 => grid.enter_alt_screen(), // Alternate screen buffer
                            1049 => {
                                // Alternate screen buffer with cursor save/restore
                                grid.save_cursor();
                                grid.enter_alt_screen();
                            }
                            _ => debug!("Unhandled DECSET: {}", p),
                        }
                    }
                }
                // DEC Private Mode Reset (DECRST)
                ('l', [b'?']) => {
                    for p in &params {
                        match p {
                            1 => {} // Normal cursor keys
                            7 => grid.auto_wrap = false,
                            25 => grid.set_cursor_visible(false),
                            47 | 1047 => grid.exit_alt_screen(), // Main screen buffer
                            1049 => {
                                // Main screen buffer with cursor restore
                                grid.exit_alt_screen();
                                grid.restore_cursor();
                            }
                            _ => debug!("Unhandled DECRST: {}", p),
                        }
                    }
                }

                // Save/restore cursor
                ('s', []) => grid.save_cursor(),
                ('u', []) => grid.restore_cursor(),

                // Device attributes
                ('c', []) | ('c', [b'>']) => {
                    // We could respond with device attributes here
                }

                _ => {
                    debug!(
                        "Unhandled CSI: action={}, intermediates={:?}, params={:?}",
                        action, intermediates, params
                    );
                }
            }
        });
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, byte: u8) {
        with_grid(|grid| {
            match (byte, intermediates) {
                (b'7', []) => grid.save_cursor(),
                (b'8', []) => grid.restore_cursor(),
                (b'D', []) => grid.linefeed(),
                (b'E', []) => {
                    grid.put_char('\r');
                    grid.linefeed();
                }
                (b'M', []) => grid.reverse_linefeed(),
                (b'c', []) => {
                    // Reset
                    grid.clear_all();
                    grid.set_cursor(0, 0);
                    grid.set_style(CellStyle::default());
                }
                _ => {
                    debug!(
                        "Unhandled ESC: byte=0x{:02x}, intermediates={:?}",
                        byte, intermediates
                    );
                }
            }
        });
    }
}

fn handle_sgr(grid: &mut Grid, params: &[u16]) {
    let mut i = 0;

    while i < params.len() {
        let p = params[i];
        match p {
            0 => {
                grid.set_style(CellStyle::default());
            }
            1 => grid.current_style_mut().bold = true,
            2 => grid.current_style_mut().dim = true,
            3 => grid.current_style_mut().italic = true,
            4 => grid.current_style_mut().underline = true,
            7 => grid.current_style_mut().inverse = true,
            9 => grid.current_style_mut().strikethrough = true,
            21 | 22 => {
                grid.current_style_mut().bold = false;
                grid.current_style_mut().dim = false;
            }
            23 => grid.current_style_mut().italic = false,
            24 => grid.current_style_mut().underline = false,
            27 => grid.current_style_mut().inverse = false,
            29 => grid.current_style_mut().strikethrough = false,

            // Foreground colors
            30..=37 => {
                grid.current_style_mut().fg_color = CellColor::Indexed((p - 30) as u8);
            }
            38 => {
                i += 1;
                if i < params.len() {
                    match params[i] {
                        5 => {
                            // 256 color
                            i += 1;
                            if i < params.len() {
                                grid.current_style_mut().fg_color =
                                    CellColor::Indexed(params[i] as u8);
                            }
                        }
                        2 => {
                            // RGB
                            if i + 3 < params.len() {
                                let r = params[i + 1] as u8;
                                let g = params[i + 2] as u8;
                                let b = params[i + 3] as u8;
                                grid.current_style_mut().fg_color = CellColor::Rgb(r, g, b);
                                i += 3;
                            }
                        }
                        _ => {}
                    }
                }
            }
            39 => grid.current_style_mut().fg_color = CellColor::Default,

            // Background colors
            40..=47 => {
                grid.current_style_mut().bg_color = CellColor::Indexed((p - 40) as u8);
            }
            48 => {
                i += 1;
                if i < params.len() {
                    match params[i] {
                        5 => {
                            // 256 color
                            i += 1;
                            if i < params.len() {
                                grid.current_style_mut().bg_color =
                                    CellColor::Indexed(params[i] as u8);
                            }
                        }
                        2 => {
                            // RGB
                            if i + 3 < params.len() {
                                let r = params[i + 1] as u8;
                                let g = params[i + 2] as u8;
                                let b = params[i + 3] as u8;
                                grid.current_style_mut().bg_color = CellColor::Rgb(r, g, b);
                                i += 3;
                            }
                        }
                        _ => {}
                    }
                }
            }
            49 => grid.current_style_mut().bg_color = CellColor::DefaultBg,

            // Bright foreground colors
            90..=97 => {
                grid.current_style_mut().fg_color = CellColor::Indexed((p - 90 + 8) as u8);
            }

            // Bright background colors
            100..=107 => {
                grid.current_style_mut().bg_color = CellColor::Indexed((p - 100 + 8) as u8);
            }

            _ => {}
        }
        i += 1;
    }
}
