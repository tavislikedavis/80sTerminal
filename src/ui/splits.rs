use egui::Rect;

#[derive(Clone, Copy, PartialEq)]
pub enum SplitDirection {
    Horizontal,
    Vertical,
}

pub enum SplitNode {
    Leaf { terminal_id: usize },
    Split {
        direction: SplitDirection,
        ratio: f32,
        first: Box<SplitNode>,
        second: Box<SplitNode>,
    },
}

impl SplitNode {
    pub fn new_leaf(terminal_id: usize) -> Self {
        SplitNode::Leaf { terminal_id }
    }

    pub fn split(self, direction: SplitDirection, new_terminal_id: usize) -> Self {
        SplitNode::Split {
            direction,
            ratio: 0.5,
            first: Box::new(self),
            second: Box::new(SplitNode::Leaf {
                terminal_id: new_terminal_id,
            }),
        }
    }

    pub fn calculate_rects(&self, available: Rect) -> Vec<(usize, Rect)> {
        match self {
            SplitNode::Leaf { terminal_id } => {
                vec![(*terminal_id, available)]
            }
            SplitNode::Split {
                direction,
                ratio,
                first,
                second,
            } => {
                let (first_rect, second_rect) = match direction {
                    SplitDirection::Horizontal => {
                        let split_x = available.min.x + available.width() * ratio;
                        (
                            Rect::from_min_max(
                                available.min,
                                egui::pos2(split_x - 2.0, available.max.y),
                            ),
                            Rect::from_min_max(
                                egui::pos2(split_x + 2.0, available.min.y),
                                available.max,
                            ),
                        )
                    }
                    SplitDirection::Vertical => {
                        let split_y = available.min.y + available.height() * ratio;
                        (
                            Rect::from_min_max(
                                available.min,
                                egui::pos2(available.max.x, split_y - 2.0),
                            ),
                            Rect::from_min_max(
                                egui::pos2(available.min.x, split_y + 2.0),
                                available.max,
                            ),
                        )
                    }
                };

                let mut rects = first.calculate_rects(first_rect);
                rects.extend(second.calculate_rects(second_rect));
                rects
            }
        }
    }

    pub fn find_focused_terminal(&self, focus_id: usize) -> Option<usize> {
        match self {
            SplitNode::Leaf { terminal_id } => {
                if *terminal_id == focus_id {
                    Some(*terminal_id)
                } else {
                    None
                }
            }
            SplitNode::Split { first, second, .. } => first
                .find_focused_terminal(focus_id)
                .or_else(|| second.find_focused_terminal(focus_id)),
        }
    }

    pub fn terminal_count(&self) -> usize {
        match self {
            SplitNode::Leaf { .. } => 1,
            SplitNode::Split { first, second, .. } => {
                first.terminal_count() + second.terminal_count()
            }
        }
    }
}

pub struct SplitPane {
    root: SplitNode,
    focused_terminal: usize,
    next_terminal_id: usize,
}

impl SplitPane {
    pub fn new() -> Self {
        Self {
            root: SplitNode::new_leaf(0),
            focused_terminal: 0,
            next_terminal_id: 1,
        }
    }

    pub fn split_horizontal(&mut self) -> usize {
        let new_id = self.next_terminal_id;
        self.next_terminal_id += 1;

        let old_root = std::mem::replace(&mut self.root, SplitNode::new_leaf(0));
        self.root = old_root.split(SplitDirection::Horizontal, new_id);
        self.focused_terminal = new_id;

        new_id
    }

    pub fn split_vertical(&mut self) -> usize {
        let new_id = self.next_terminal_id;
        self.next_terminal_id += 1;

        let old_root = std::mem::replace(&mut self.root, SplitNode::new_leaf(0));
        self.root = old_root.split(SplitDirection::Vertical, new_id);
        self.focused_terminal = new_id;

        new_id
    }

    pub fn focused_terminal(&self) -> usize {
        self.focused_terminal
    }

    pub fn set_focused_terminal(&mut self, id: usize) {
        self.focused_terminal = id;
    }

    pub fn calculate_rects(&self, available: Rect) -> Vec<(usize, Rect)> {
        self.root.calculate_rects(available)
    }

    pub fn terminal_count(&self) -> usize {
        self.root.terminal_count()
    }
}

impl Default for SplitPane {
    fn default() -> Self {
        Self::new()
    }
}
