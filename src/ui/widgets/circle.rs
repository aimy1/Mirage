use ratatui::{
    prelude::*,
    widgets::Widget,
};
use crate::theme::Theme;

pub struct CircleWidget<'a> {
    theme: &'a Theme,
    bars: &'a [f32],
}

impl<'a> CircleWidget<'a> {
    pub fn new(theme: &'a Theme, bars: &'a [f32]) -> Self {
        Self { theme, bars }
    }
}

impl<'a> Widget for CircleWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 || self.bars.is_empty() {
            return;
        }

        let char_w = area.width as usize;
        let char_h = area.height as usize;

        // 盲文 2x4 分辨率
        let pixel_w = char_w * 2;
        let pixel_h = char_h * 4;

        let mut grid = vec![false; pixel_w * pixel_h];

        // 逻辑中心点
        let pcx = pixel_w as f32 / 2.0;
        let pcy = pixel_h as f32 / 2.0;

        // 基础半径 (取逻辑高度的 20%)
        let base_radius = (pixel_h as f32 * 0.18).max(6.0);
        // 最大扩散长度
        let max_extension = (pcx.min(pcy) - base_radius - 2.0).max(5.0);

        let num_bars = self.bars.len();

        // 1. 绘制内圆底盘 (360个点点亮)
        let num_base_pts = 180;
        for j in 0..num_base_pts {
            let theta = (j as f32 / num_base_pts as f32) * 2.0 * std::f32::consts::PI;
            // 微微加上宽高补偿，使在非完美 1:2 终端上也偏向圆形
            let px = (pcx + base_radius * theta.cos() * 1.05) as i32;
            let py = (pcy + base_radius * theta.sin()) as i32;
            if px >= 0 && px < pixel_w as i32 && py >= 0 && py < pixel_h as i32 {
                grid[py as usize * pixel_w + px as usize] = true;
            }
        }

        // 2. 沿着角度放射频带柱子
        for i in 0..num_bars {
            let val = self.bars[i];
            // 将角度环绕分布
            let theta = (i as f32 / num_bars as f32) * 2.0 * std::f32::consts::PI;
            
            let r_start = base_radius;
            let r_end = base_radius + val * max_extension;

            let cos_t = theta.cos() * 1.05; // 宽高补偿
            let sin_t = theta.sin();

            // 在起止半径内渲染线段
            let steps = (val * max_extension).round() as usize;
            let steps = steps.max(2);
            for step in 0..=steps {
                let t = step as f32 / steps as f32;
                let r = r_start + (r_end - r_start) * t;
                let px = (pcx + r * cos_t) as i32;
                let py = (pcy + r * sin_t) as i32;
                
                if px >= 0 && px < pixel_w as i32 && py >= 0 && py < pixel_h as i32 {
                    grid[py as usize * pixel_w + px as usize] = true;
                }
            }
        }

        // 3. 映射到盲文并输出
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

                if braille_char > 0 {
                    let cell_char = std::char::from_u32(0x2800 + braille_char).unwrap_or(' ');
                    let col_x = area.x + cx as u16;
                    let col_y = area.y + cy as u16;

                    // 圆形渐变色：根据与中心点的距离来着色，内圈蓝绿，外圈粉红/紫色
                    let dx = cx as f32 - (char_w as f32 / 2.0);
                    let dy = cy as f32 - (char_h as f32 / 2.0);
                    let dist = (dx * dx * 1.5 + dy * dy).sqrt();
                    let max_dist = (char_w as f32 / 2.0).min(char_h as f32 / 2.0).max(1.0);
                    let color_ratio = (dist / max_dist).clamp(0.0, 1.0);
                    
                    let color_idx = (color_ratio * (self.theme.accents.len() - 1) as f32) as usize;
                    let fg_color = self.theme.accents[color_idx.min(self.theme.accents.len() - 1)];

                    buf.get_mut(col_x, col_y)
                        .set_char(cell_char)
                        .set_fg(fg_color);
                } else {
                    let col_x = area.x + cx as u16;
                    let col_y = area.y + cy as u16;
                    buf.get_mut(col_x, col_y).set_char(' '); // 填充空格
                }
            }
        }
    }
}
