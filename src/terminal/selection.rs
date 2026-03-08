use super::grid::Grid;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: u16,
    pub y: u16,
}

impl Point {
    pub fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone)]
pub struct Selection {
    start: Option<Point>,
    end: Option<Point>,
    active: bool,
}

impl Selection {
    pub fn new() -> Self {
        Self {
            start: None,
            end: None,
            active: false,
        }
    }

    pub fn start_selection(&mut self, point: Point) {
        self.start = Some(point);
        self.end = Some(point);
        self.active = true;
    }

    pub fn update_selection(&mut self, point: Point) {
        if self.active {
            self.end = Some(point);
        }
    }

    pub fn end_selection(&mut self) {
        self.active = false;
    }

    pub fn clear(&mut self) {
        self.start = None;
        self.end = None;
        self.active = false;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn has_selection(&self) -> bool {
        self.start.is_some() && self.end.is_some()
    }

    pub fn normalized(&self) -> Option<(Point, Point)> {
        match (self.start, self.end) {
            (Some(start), Some(end)) => {
                let (s, e) = if start.y < end.y || (start.y == end.y && start.x <= end.x) {
                    (start, end)
                } else {
                    (end, start)
                };
                Some((s, e))
            }
            _ => None,
        }
    }

    pub fn contains(&self, x: u16, y: u16) -> bool {
        if let Some((start, end)) = self.normalized() {
            if y < start.y || y > end.y {
                return false;
            }
            if y == start.y && y == end.y {
                return x >= start.x && x <= end.x;
            }
            if y == start.y {
                return x >= start.x;
            }
            if y == end.y {
                return x <= end.x;
            }
            true
        } else {
            false
        }
    }

    pub fn get_text(&self, grid: &Grid) -> String {
        let Some((start, end)) = self.normalized() else {
            return String::new();
        };

        let mut result = String::new();
        let cols = grid.cols();

        for y in start.y..=end.y {
            let line_start = if y == start.y { start.x } else { 0 };
            let line_end = if y == end.y { end.x } else { cols - 1 };

            for x in line_start..=line_end {
                if let Some(cell) = grid.get(x, y) {
                    result.push(cell.c);
                }
            }

            // Add newline between lines (but not after the last line)
            if y != end.y {
                result.push('\n');
            }
        }

        // Trim trailing whitespace from each line
        result
            .lines()
            .map(|line| line.trim_end())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl Default for Selection {
    fn default() -> Self {
        Self::new()
    }
}
