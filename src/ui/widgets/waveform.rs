use ratatui::{
    prelude::*,
    widgets::Widget,
};
use crate::theme::Theme;

pub struct WaveformWidget<'a> {
    theme: &'a Theme,
    samples: &'a [f32], // 传入时域采样数据，应足够长（如 1024 或自适应）
}

impl<'a> WaveformWidget<'a> {
    pub fn new(theme: &'a Theme, samples: &'a [f32]) -> Self {
        Self { theme, samples }
    }
}

impl<'a> Widget for WaveformWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 || self.samples.is_empty() {
            return;
        }

        let char_w = area.width as usize;
        let char_h = area.height as usize;

        // 盲文网格的分辨率是字符分辨率的 2x4 倍
        let pixel_w = char_w * 2;
        let pixel_h = char_h * 4;

        // 创建像素点亮网格
        let mut grid = vec![false; pixel_w * pixel_h];

        // 计算所有采样点的像素 Y 坐标并保存
        let mut points = Vec::with_capacity(pixel_w);
        for px in 0..pixel_w {
            let sample_idx = (px as f32 / pixel_w as f32 * self.samples.len() as f32) as usize;
            if sample_idx >= self.samples.len() {
                break;
            }
            let sample_val = self.samples[sample_idx];
            
            // 归一化 Y 坐标，0.0 在最顶部，1.0 在最底部
            let norm_y = (1.0 - sample_val) / 2.0;
            let py = (norm_y * (pixel_h - 1) as f32).round() as usize;
            let py = py.clamp(0, pixel_h - 1);
            points.push((px, py));
        }

        // 用 Bresenham 直线算法连接所有相邻的采样点，形成平滑的示波器线条
        for i in 0..points.len().saturating_sub(1) {
            let (x0, y0) = points[i];
            let (x1, y1) = points[i + 1];

            let dx = (x1 as i32 - x0 as i32).abs();
            let dy = (y1 as i32 - y0 as i32).abs();
            let sx = if x0 < x1 { 1 } else { -1 };
            let sy = if y0 < y1 { 1 } else { -1 };
            let mut err = dx - dy;
            let mut x = x0 as i32;
            let mut y = y0 as i32;

            loop {
                if x >= 0 && x < pixel_w as i32 && y >= 0 && y < pixel_h as i32 {
                    grid[(y as usize) * pixel_w + (x as usize)] = true;
                }
                if x == x1 as i32 && y == y1 as i32 {
                    break;
                }
                let e2 = 2 * err;
                if e2 > -dy {
                    err -= dy;
                    x += sx;
                }
                if e2 < dx {
                    err += dx;
                    y += sy;
                }
            }
        }

        // 把 2x4 的像素块拼接为 Braille 字符并写入 Buffer
        // 盲文点位置对应二进制位：
        // (x=0, y=0) -> bit 0 (0x01)    (x=1, y=0) -> bit 3 (0x08)
        // (x=0, y=1) -> bit 1 (0x02)    (x=1, y=1) -> bit 4 (0x10)
        // (x=0, y=2) -> bit 2 (0x04)    (x=1, y=2) -> bit 5 (0x20)
        // (x=0, y=3) -> bit 6 (0x40)    (x=1, y=3) -> bit 7 (0x80)
        for cx in 0..char_w {
            for cy in 0..char_h {
                let px_start = cx * 2;
                let py_start = cy * 4;
                
                let mut braille_char = 0u32;
                
                // 检查左列
                if grid[(py_start + 0) * pixel_w + px_start] { braille_char |= 0x01; }
                if grid[(py_start + 1) * pixel_w + px_start] { braille_char |= 0x02; }
                if grid[(py_start + 2) * pixel_w + px_start] { braille_char |= 0x04; }
                if grid[(py_start + 3) * pixel_w + px_start] { braille_char |= 0x40; }
                
                // 检查右列
                if grid[(py_start + 0) * pixel_w + px_start + 1] { braille_char |= 0x08; }
                if grid[(py_start + 1) * pixel_w + px_start + 1] { braille_char |= 0x10; }
                if grid[(py_start + 2) * pixel_w + px_start + 1] { braille_char |= 0x20; }
                if grid[(py_start + 3) * pixel_w + px_start + 1] { braille_char |= 0x80; }

                let cell_char = if braille_char == 0 {
                    ' ' // 全空显示为空格
                } else {
                    std::char::from_u32(0x2800 + braille_char).unwrap_or(' ')
                };

                let col_x = area.x + cx as u16;
                let col_y = area.y + cy as u16;
                
                // 设置字符和渐变色
                let color_idx = (cx * self.theme.accents.len() / char_w).min(self.theme.accents.len() - 1);
                buf.get_mut(col_x, col_y)
                    .set_char(cell_char)
                    .set_fg(self.theme.accents[color_idx]);
            }
        }
    }
}
