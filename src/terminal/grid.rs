use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CellStyle {
    pub fg_color: CellColor,
    pub bg_color: CellColor,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub inverse: bool,
    pub dim: bool,
}

impl Default for CellStyle {
    fn default() -> Self {
        Self {
            fg_color: CellColor::Default,
            bg_color: CellColor::DefaultBg,
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            inverse: false,
            dim: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CellColor {
    Default,
    DefaultBg,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

#[derive(Debug, Clone, Copy)]
pub struct Cell {
    pub c: char,
    pub style: CellStyle,
    pub dirty: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            c: ' ',
            style: CellStyle::default(),
            dirty: true,
        }
    }
}

impl Cell {
    pub fn new(c: char) -> Self {
        Self {
            c,
            style: CellStyle::default(),
            dirty: true,
        }
    }

    pub fn with_style(c: char, style: CellStyle) -> Self {
        Self {
            c,
            style,
            dirty: true,
        }
    }
}

pub struct Grid {
    cols: u16,
    rows: u16,
    cells: Vec<Cell>,
    scrollback: VecDeque<Vec<Cell>>,
    scrollback_limit: usize,
    unlimited_scrollback: bool,
    cursor_x: u16,
    cursor_y: u16,
    cursor_visible: bool,
    scroll_offset: usize,
    current_style: CellStyle,

    // Saved cursor position (for ESC 7 / ESC 8)
    saved_cursor_x: u16,
    saved_cursor_y: u16,
    saved_style: CellStyle,

    // Scroll region
    scroll_top: u16,
    scroll_bottom: u16,

    // Modes
    pub origin_mode: bool,
    pub auto_wrap: bool,
    pub insert_mode: bool,
    pub linefeed_mode: bool,

    // Alternate screen buffer
    alt_screen_active: bool,
    main_screen_cells: Option<Vec<Cell>>,
    main_cursor_x: u16,
    main_cursor_y: u16,
}

impl Grid {
    pub fn new(cols: u16, rows: u16, scrollback_limit: usize) -> Self {
        let cell_count = (cols as usize) * (rows as usize);
        let cells = vec![Cell::default(); cell_count];

        Self {
            cols,
            rows,
            cells,
            scrollback: VecDeque::new(),
            scrollback_limit,
            unlimited_scrollback: false,
            cursor_x: 0,
            cursor_y: 0,
            cursor_visible: true,
            scroll_offset: 0,
            current_style: CellStyle::default(),
            saved_cursor_x: 0,
            saved_cursor_y: 0,
            saved_style: CellStyle::default(),
            scroll_top: 0,
            scroll_bottom: rows.saturating_sub(1),
            origin_mode: false,
            auto_wrap: true,
            insert_mode: false,
            linefeed_mode: false,
            alt_screen_active: false,
            main_screen_cells: None,
            main_cursor_x: 0,
            main_cursor_y: 0,
        }
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        if cols == self.cols && rows == self.rows {
            return;
        }

        let mut new_cells = vec![Cell::default(); (cols as usize) * (rows as usize)];

        // Copy existing content
        let copy_cols = cols.min(self.cols) as usize;
        let copy_rows = rows.min(self.rows) as usize;

        for y in 0..copy_rows {
            for x in 0..copy_cols {
                let old_idx = y * (self.cols as usize) + x;
                let new_idx = y * (cols as usize) + x;
                new_cells[new_idx] = self.cells[old_idx].clone();
            }
        }

        self.cols = cols;
        self.rows = rows;
        self.cells = new_cells;
        self.cursor_x = self.cursor_x.min(cols.saturating_sub(1));
        self.cursor_y = self.cursor_y.min(rows.saturating_sub(1));
        self.scroll_bottom = rows.saturating_sub(1);
    }

    pub fn cols(&self) -> u16 {
        self.cols
    }

    pub fn rows(&self) -> u16 {
        self.rows
    }

    pub fn cursor(&self) -> (u16, u16) {
        (self.cursor_x, self.cursor_y)
    }

    pub fn cursor_visible(&self) -> bool {
        self.cursor_visible
    }

    pub fn set_cursor_visible(&mut self, visible: bool) {
        self.cursor_visible = visible;
    }

    pub fn set_cursor(&mut self, x: u16, y: u16) {
        self.cursor_x = x.min(self.cols.saturating_sub(1));
        self.cursor_y = y.min(self.rows.saturating_sub(1));
    }

    pub fn move_cursor(&mut self, dx: i16, dy: i16) {
        let new_x = (self.cursor_x as i32 + dx as i32).clamp(0, (self.cols - 1) as i32) as u16;
        let new_y = (self.cursor_y as i32 + dy as i32).clamp(0, (self.rows - 1) as i32) as u16;
        self.cursor_x = new_x;
        self.cursor_y = new_y;
    }

    fn index(&self, x: u16, y: u16) -> usize {
        (y as usize) * (self.cols as usize) + (x as usize)
    }

    pub fn get(&self, x: u16, y: u16) -> Option<&Cell> {
        if x < self.cols && y < self.rows {
            Some(&self.cells[self.index(x, y)])
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, x: u16, y: u16) -> Option<&mut Cell> {
        if x < self.cols && y < self.rows {
            let idx = self.index(x, y);
            Some(&mut self.cells[idx])
        } else {
            None
        }
    }

    pub fn put_char(&mut self, c: char) {
        if c == '\r' {
            self.cursor_x = 0;
            return;
        }

        if c == '\n' {
            self.linefeed();
            return;
        }

        if c == '\t' {
            // Move to next tab stop (every 8 columns)
            let next_tab = ((self.cursor_x / 8) + 1) * 8;
            self.cursor_x = next_tab.min(self.cols.saturating_sub(1));
            return;
        }

        if c == '\x08' {
            // Backspace
            self.cursor_x = self.cursor_x.saturating_sub(1);
            return;
        }

        if c == '\x07' {
            // Bell - ignore for now
            return;
        }

        // Handle auto-wrap
        if self.cursor_x >= self.cols {
            if self.auto_wrap {
                self.cursor_x = 0;
                self.linefeed();
            } else {
                self.cursor_x = self.cols.saturating_sub(1);
            }
        }

        // Insert the character
        let idx = self.index(self.cursor_x, self.cursor_y);
        self.cells[idx] = Cell::with_style(c, self.current_style);

        self.cursor_x += 1;
    }

    pub fn linefeed(&mut self) {
        if self.cursor_y >= self.scroll_bottom {
            self.scroll_up(1);
        } else {
            self.cursor_y += 1;
        }
    }

    pub fn reverse_linefeed(&mut self) {
        if self.cursor_y <= self.scroll_top {
            self.scroll_down(1);
        } else {
            self.cursor_y -= 1;
        }
    }

    pub fn scroll_up(&mut self, count: u16) {
        let cols = self.cols as usize;

        for _ in 0..count {
            // Save the top line to scrollback
            let top_start = self.scroll_top as usize * cols;
            let top_line = self.cells[top_start..top_start + cols].to_vec();
            self.scrollback.push_back(top_line);

            // Trim scrollback if needed (unless unlimited)
            if !self.unlimited_scrollback && self.scrollback.len() > self.scrollback_limit {
                let excess = self.scrollback.len() - self.scrollback_limit;
                self.scrollback.drain(..excess);
            }

            // Move lines up within scroll region using copy_within
            let src_start = (self.scroll_top as usize + 1) * cols;
            let src_end = (self.scroll_bottom as usize + 1) * cols;
            let dst_start = self.scroll_top as usize * cols;
            self.cells.copy_within(src_start..src_end, dst_start);

            // Clear bottom line
            let bottom_start = self.scroll_bottom as usize * cols;
            for cell in &mut self.cells[bottom_start..bottom_start + cols] {
                *cell = Cell::default();
            }
        }
    }

    pub fn scroll_down(&mut self, count: u16) {
        let cols = self.cols as usize;

        for _ in 0..count {
            // Move lines down within scroll region using copy_within
            let src_start = self.scroll_top as usize * cols;
            let src_end = self.scroll_bottom as usize * cols;
            let dst_start = (self.scroll_top as usize + 1) * cols;
            self.cells.copy_within(src_start..src_end, dst_start);

            // Clear top line
            let top_start = self.scroll_top as usize * cols;
            for cell in &mut self.cells[top_start..top_start + cols] {
                *cell = Cell::default();
            }
        }
    }

    pub fn set_scroll_region(&mut self, top: u16, bottom: u16) {
        self.scroll_top = top.min(self.rows.saturating_sub(1));
        self.scroll_bottom = bottom.min(self.rows.saturating_sub(1));
        if self.scroll_top > self.scroll_bottom {
            std::mem::swap(&mut self.scroll_top, &mut self.scroll_bottom);
        }
    }

    pub fn clear_all(&mut self) {
        for cell in &mut self.cells {
            *cell = Cell::default();
        }
    }

    pub fn clear_line(&mut self, mode: u16) {
        let y = self.cursor_y;
        match mode {
            0 => {
                // Clear from cursor to end of line
                for x in self.cursor_x..self.cols {
                    let idx = self.index(x, y);
                    self.cells[idx] = Cell::default();
                }
            }
            1 => {
                // Clear from start of line to cursor
                for x in 0..=self.cursor_x {
                    let idx = self.index(x, y);
                    self.cells[idx] = Cell::default();
                }
            }
            2 => {
                // Clear entire line
                for x in 0..self.cols {
                    let idx = self.index(x, y);
                    self.cells[idx] = Cell::default();
                }
            }
            _ => {}
        }
    }

    pub fn clear_screen(&mut self, mode: u16) {
        match mode {
            0 => {
                // Clear from cursor to end of screen
                self.clear_line(0);
                for y in (self.cursor_y + 1)..self.rows {
                    for x in 0..self.cols {
                        let idx = self.index(x, y);
                        self.cells[idx] = Cell::default();
                    }
                }
            }
            1 => {
                // Clear from start of screen to cursor
                for y in 0..self.cursor_y {
                    for x in 0..self.cols {
                        let idx = self.index(x, y);
                        self.cells[idx] = Cell::default();
                    }
                }
                self.clear_line(1);
            }
            2 | 3 => {
                // Clear entire screen
                self.clear_all();
            }
            _ => {}
        }
    }

    pub fn delete_chars(&mut self, count: u16) {
        let y = self.cursor_y;
        let count = count.min(self.cols - self.cursor_x);

        // Shift characters left
        for x in self.cursor_x..(self.cols - count) {
            let src_idx = self.index(x + count, y);
            let dst_idx = self.index(x, y);
            self.cells[dst_idx] = self.cells[src_idx].clone();
        }

        // Clear the end
        for x in (self.cols - count)..self.cols {
            let idx = self.index(x, y);
            self.cells[idx] = Cell::default();
        }
    }

    pub fn insert_chars(&mut self, count: u16) {
        let y = self.cursor_y;
        let count = count.min(self.cols - self.cursor_x);

        // Shift characters right
        for x in ((self.cursor_x + count)..self.cols).rev() {
            let src_idx = self.index(x - count, y);
            let dst_idx = self.index(x, y);
            self.cells[dst_idx] = self.cells[src_idx].clone();
        }

        // Clear the inserted area
        for x in self.cursor_x..(self.cursor_x + count) {
            let idx = self.index(x, y);
            self.cells[idx] = Cell::default();
        }
    }

    pub fn delete_lines(&mut self, count: u16) {
        let count = count.min(self.scroll_bottom - self.cursor_y + 1);
        for _ in 0..count {
            // Move lines up
            for y in self.cursor_y..self.scroll_bottom {
                for x in 0..self.cols {
                    let src_idx = self.index(x, y + 1);
                    let dst_idx = self.index(x, y);
                    self.cells[dst_idx] = self.cells[src_idx].clone();
                }
            }
            // Clear bottom line
            for x in 0..self.cols {
                let idx = self.index(x, self.scroll_bottom);
                self.cells[idx] = Cell::default();
            }
        }
    }

    pub fn insert_lines(&mut self, count: u16) {
        let count = count.min(self.scroll_bottom - self.cursor_y + 1);
        for _ in 0..count {
            // Move lines down
            for y in (self.cursor_y + 1..=self.scroll_bottom).rev() {
                for x in 0..self.cols {
                    let src_idx = self.index(x, y - 1);
                    let dst_idx = self.index(x, y);
                    self.cells[dst_idx] = self.cells[src_idx].clone();
                }
            }
            // Clear current line
            for x in 0..self.cols {
                let idx = self.index(x, self.cursor_y);
                self.cells[idx] = Cell::default();
            }
        }
    }

    pub fn set_style(&mut self, style: CellStyle) {
        self.current_style = style;
    }

    pub fn current_style(&self) -> CellStyle {
        self.current_style
    }

    pub fn current_style_mut(&mut self) -> &mut CellStyle {
        &mut self.current_style
    }

    pub fn save_cursor(&mut self) {
        self.saved_cursor_x = self.cursor_x;
        self.saved_cursor_y = self.cursor_y;
        self.saved_style = self.current_style;
    }

    pub fn restore_cursor(&mut self) {
        self.cursor_x = self.saved_cursor_x;
        self.cursor_y = self.saved_cursor_y;
        self.current_style = self.saved_style;
    }

    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    pub fn scrollback(&self) -> &VecDeque<Vec<Cell>> {
        &self.scrollback
    }

    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    pub fn set_scroll_offset(&mut self, offset: usize) {
        self.scroll_offset = offset.min(self.scrollback.len());
    }

    pub fn set_scrollback_limit(&mut self, limit: usize) {
        self.scrollback_limit = limit;
    }

    pub fn set_unlimited_scrollback(&mut self, unlimited: bool) {
        self.unlimited_scrollback = unlimited;
    }

    pub fn scrollback_limit(&self) -> usize {
        self.scrollback_limit
    }

    pub fn is_unlimited_scrollback(&self) -> bool {
        self.unlimited_scrollback
    }

    /// Switch to alternate screen buffer (used by programs like top, vim, less)
    pub fn enter_alt_screen(&mut self) {
        if self.alt_screen_active {
            return;
        }

        // Save current screen content and cursor
        self.main_screen_cells = Some(self.cells.clone());
        self.main_cursor_x = self.cursor_x;
        self.main_cursor_y = self.cursor_y;

        // Clear screen for alternate buffer
        self.clear_all();
        self.cursor_x = 0;
        self.cursor_y = 0;
        self.alt_screen_active = true;

        log::debug!("Entered alternate screen buffer");
    }

    /// Switch back to main screen buffer
    pub fn exit_alt_screen(&mut self) {
        if !self.alt_screen_active {
            return;
        }

        // Restore main screen content and cursor
        if let Some(main_cells) = self.main_screen_cells.take() {
            let current_size = (self.cols as usize) * (self.rows as usize);
            if main_cells.len() == current_size {
                // Same size - direct restore
                self.cells = main_cells;
            } else {
                // Terminal was resized while in alt screen - copy what fits
                let mut new_cells = vec![Cell::default(); current_size];
                let old_cols = if self.rows > 0 {
                    main_cells.len() / self.rows as usize
                } else {
                    self.cols as usize
                };
                // Try to figure out old dimensions from saved data
                // Use main_cursor to estimate old cols (heuristic)
                let saved_cols = if main_cells.len() > 0 && self.main_cursor_y > 0 {
                    // Best guess: try current cols first
                    self.cols as usize
                } else {
                    old_cols
                };
                let saved_rows = if saved_cols > 0 {
                    main_cells.len() / saved_cols
                } else {
                    self.rows as usize
                };

                let copy_cols = (self.cols as usize).min(saved_cols);
                let copy_rows = (self.rows as usize).min(saved_rows);

                for y in 0..copy_rows {
                    for x in 0..copy_cols {
                        let src_idx = y * saved_cols + x;
                        let dst_idx = y * (self.cols as usize) + x;
                        if src_idx < main_cells.len() && dst_idx < new_cells.len() {
                            new_cells[dst_idx] = main_cells[src_idx];
                        }
                    }
                }
                self.cells = new_cells;
            }
        }
        self.cursor_x = self.cursor_x.min(self.cols.saturating_sub(1));
        self.cursor_y = self.cursor_y.min(self.rows.saturating_sub(1));
        self.alt_screen_active = false;

        log::debug!("Exited alternate screen buffer");
    }

    pub fn is_alt_screen(&self) -> bool {
        self.alt_screen_active
    }
}
