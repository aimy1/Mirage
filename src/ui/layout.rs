use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    prelude::*,
};

pub struct MirageLayout {
    pub top_rect: Rect,
    pub main_rect: Rect,
    pub side_rect: Rect,
    pub bottom_rect: Rect,
    pub has_side_panel: bool,
}

impl MirageLayout {
    pub fn new(area: Rect, has_side_panel: bool) -> Self {
        // 1. 垂直划分：Top Bar (3行), Main + Side Area (自适应), Bottom Bar (1行)
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Top Bar
                Constraint::Min(5),    // Main + Side Area
                Constraint::Length(1), // Bottom Bar
            ])
            .split(area);

        let top_rect = main_chunks[0];
        let bottom_rect = main_chunks[2];
        let content_rect = main_chunks[1];

        // 2. 水平划分 Main Area 和 Side Panel
        let (main_rect, side_rect) = if has_side_panel {
            let sub_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(80), // Main Area
                    Constraint::Percentage(20), // Side Panel
                ])
                .split(content_rect);
            (sub_chunks[0], sub_chunks[1])
        } else {
            (content_rect, Rect::default())
        };

        Self {
            top_rect,
            main_rect,
            side_rect,
            bottom_rect,
            has_side_panel,
        }
    }
}
