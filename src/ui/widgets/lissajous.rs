use ratatui::{
    prelude::*,
    widgets::Widget,
};
use crate::theme::Theme;

pub struct LissajousWidget<'a> {
    theme: &'a Theme,
    left_samples: &'a [f32],
    right_samples: &'a [f32],
}

impl<'a> LissajousWidget<'a> {
    pub fn new(theme: &'a Theme, left_samples: &'a [f32], right_samples: &'a [f32]) -> Self {
        Self {
            theme,
            left_samples,
            right_samples,
        }
    }
}

impl<'a> Widget for LissajousWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 || self.left_samples.is_empty() || self.right_samples.is_empty() {
            return;
        }

        // 为了保持利萨茹示波器形状，我们取一个居中的正方形区域
        let size = area.width.min(area.height * 2) as u16; // 宽为高的两倍
        let char_w = size;
        let char_h = (size / 2).max(1);

        let offset_x = area.x + (area.width - char_w) / 2;
        let offset_y = area.y + (area.height - char_h) / 2;

        let pixel_w = char_w as usize * 2;
        let pixel_h = char_h as usize * 4;

        let mut grid = vec![false; pixel_w * pixel_h];

        // 决定使用多少采样点（最多使用可用采样数和 400 个点，防止点太密集变成一团墨）
        let num_points = self.left_samples.len().min(self.right_samples.len()).min(400);
        let start_idx = self.left_samples.len().min(self.right_samples.len()) - num_points;

        for i in 0..num_points {
            let idx = start_idx + i;
            let l_val = self.left_samples[idx];
            let r_val = self.right_samples[idx];

            // 映射到 0.0 ~ 1.0
            let norm_x = (l_val + 1.0) / 2.0;
            // Y 轴反转，1.0 对应顶部，-1.0 对应底部
            let norm_y = (1.0 - r_val) / 2.0;

            let px = (norm_x * (pixel_w - 1) as f32).round() as usize;
            let py = (norm_y * (pixel_h - 1) as f32).round() as usize;

            let px = px.clamp(0, pixel_w - 1);
            let py = py.clamp(0, pixel_h - 1);

            grid[py * pixel_w + px] = true;
        }

        // 渲染盲文
        for cx in 0..char_w as usize {
            for cy in 0..char_h as usize {
                let px_start = cx * 2;
                let py_start = cy * 4;
                
                let mut braille_char = 0u32;
                
                if grid[(py_start + 0) * pixel_w + px_start] { braille_char |= 0x01; }
                if grid[(py_start + 1) * pixel_w + px_start] { braille_char |= 0x02; }
                if grid[(py_start + 2) * pixel_w + px_start] { braille_char |= 0x04; }
                if grid[(py_start + 3) * pixel_w + px_start] { braille_char |= 0x40; }
                
                if grid[(py_start + 0) * pixel_w + px_start + 1] { braille_char |= 0x08; }
                if grid[(py_start + 1) * pixel_w + px_start + 1] { braille_char |= 0x10; }
                if grid[(py_start + 2) * pixel_w + px_start + 1] { braille_char |= 0x20; }
                if grid[(py_start + 3) * pixel_w + px_start + 1] { braille_char |= 0x80; }

                let col_x = offset_x + cx as u16;
                let col_y = offset_y + cy as u16;

                if braille_char > 0 {
                    let cell_char = std::char::from_u32(0x2800 + braille_char).unwrap_or(' ');
                    
                    // 利萨茹轨迹颜色：采用中间冷、外围暖的渐变
                    let dx = cx as f32 - (char_w as f32 / 2.0);
                    let dy = cy as f32 - (char_h as f32 / 2.0);
                    let dist = (dx * dx * 1.5 + dy * dy).sqrt();
                    let max_dist = (char_w as f32 / 2.0).max(1.0);
                    let ratio = (dist / max_dist).clamp(0.0, 1.0);
                    
                    let color_idx = (ratio * (self.theme.accents.len() - 1) as f32) as usize;
                    let fg_color = self.theme.accents[color_idx.min(self.theme.accents.len() - 1)];

                    buf.get_mut(col_x, col_y)
                        .set_char(cell_char)
                        .set_fg(fg_color);
                } else {
                    buf.get_mut(col_x, col_y).set_char(' ');
                }
            }
        }
    }
}
