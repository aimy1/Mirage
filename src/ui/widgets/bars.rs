use ratatui::{
    prelude::*,
    widgets::{Block, Borders, BorderType, Widget},
};
use crate::theme::Theme;

pub struct BarsWidget<'a> {
    theme: &'a Theme,
    bars: &'a [f32],
    peaks: &'a [f32],
}

impl<'a> BarsWidget<'a> {
    pub fn new(theme: &'a Theme, bars: &'a [f32], peaks: &'a [f32]) -> Self {
        Self { theme, bars, peaks }
    }
}

impl<'a> Widget for BarsWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let num_bars = self.bars.len().min(area.width as usize);
        let height = area.height as usize;

        // 字符级垂直块 (8等分)
        let blocks = [" ", " ", "▂", "▃", "▄", "▅", "▆", "▇", "█"];

        for x_idx in 0..num_bars {
            let col_x = area.x + x_idx as u16;
            
            // 计算柱子高度和 Peak 高度（行数为单位）
            let raw_h = self.bars[x_idx] * height as f32;
            let peak_h = self.peaks[x_idx] * height as f32;
            
            let full_rows = raw_h.floor() as usize;
            let fraction = (raw_h - full_rows as f32) * 8.0;
            let block_idx = fraction.round() as usize;

            // 绘制主柱体 (从下往上)
            for y_row in 0..height {
                let col_y = area.y + area.height - 1 - y_row as u16;
                let cell = buf.get_mut(col_x, col_y);

                // 柱子的渐变配色：低音绿色，中音黄色，高音红色/紫色（通过 accents 列表）
                let color_idx = (x_idx * self.theme.accents.len() / num_bars).min(self.theme.accents.len() - 1);
                let fg_color = self.theme.accents[color_idx];

                if y_row < full_rows {
                    cell.set_char('█').set_fg(fg_color);
                } else if y_row == full_rows && block_idx > 0 {
                    cell.set_char(blocks[block_idx].chars().next().unwrap()).set_fg(fg_color);
                } else {
                    cell.set_char(' '); // 清空背景
                }
            }

            // 绘制 Peak Hold 顶部横线
            let peak_y_row = (peak_h.round() as usize).clamp(0, height - 1);
            if peak_y_row > 0 {
                let col_y = area.y + area.height - 1 - peak_y_row as u16;
                let cell = buf.get_mut(col_x, col_y);
                // 仅当该位置没有被全实心块覆盖时，绘制 Peak 横线
                if cell.symbol() != "█" {
                    cell.set_char('▔').set_fg(self.theme.accent4); // peak 采用粉色/红色
                }
            }
        }
    }
}
