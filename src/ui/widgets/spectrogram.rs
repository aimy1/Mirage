use std::collections::VecDeque;
use ratatui::{
    prelude::*,
    widgets::Widget,
};
use crate::theme::Theme;

pub struct SpectrogramWidget<'a> {
    theme: &'a Theme,
    // 滚动历史，每一项是一个 bars 数组 (已归一化到 0.0~1.0)
    // 列表的第一个元素是最新的一帧，渲染在最上面
    history: &'a VecDeque<Vec<f32>>,
}

impl<'a> SpectrogramWidget<'a> {
    pub fn new(theme: &'a Theme, history: &'a VecDeque<Vec<f32>>) -> Self {
        Self { theme, history }
    }
}

impl<'a> Widget for SpectrogramWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 || self.history.is_empty() {
            return;
        }

        let width = area.width as usize;
        let height = area.height as usize;

        // 决定使用多少行历史记录，最多填满可用高度
        let lines_to_draw = self.history.len().min(height);

        // 字符密度表表示能量大小
        let density = [' ', '░', '▒', '▓', '█'];

        for y in 0..lines_to_draw {
            let row_y = area.y + y as u16; // 最新一帧在最上方滚动向下
            let frame = &self.history[y];
            
            if frame.is_empty() {
                continue;
            }

            for x in 0..width {
                let col_x = area.x + x as u16;
                
                // 将横坐标映射到频带索引
                let bar_idx = (x * frame.len() / width).min(frame.len() - 1);
                let val = frame[bar_idx];

                // 计算能量等级
                let d_idx = (val * (density.len() - 1) as f32).round() as usize;
                let d_idx = d_idx.clamp(0, density.len() - 1);
                let ch = density[d_idx];

                if d_idx > 0 {
                    // 渐变色：低能量冷色，高能量暖色
                    let color_idx = (val * (self.theme.accents.len() - 1) as f32).round() as usize;
                    let color_idx = color_idx.min(self.theme.accents.len() - 1);
                    let fg_color = self.theme.accents[color_idx];

                    buf.get_mut(col_x, row_y)
                        .set_char(ch)
                        .set_fg(fg_color);
                } else {
                    buf.get_mut(col_x, row_y).set_char(' ');
                }
            }
        }

        // 清理剩余行
        for y in lines_to_draw..height {
            let row_y = area.y + y as u16;
            for x in 0..width {
                let col_x = area.x + x as u16;
                buf.get_mut(col_x, row_y).set_char(' ');
            }
        }
    }
}
