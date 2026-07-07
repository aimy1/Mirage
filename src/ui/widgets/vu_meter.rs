use ratatui::{
    prelude::*,
    widgets::{Block, Borders, BorderType, Widget},
};
use crate::theme::Theme;

pub struct VuMeterWidget<'a> {
    theme: &'a Theme,
    left_rms: f32,   // 左右声道均方根值 (0.0 到 1.0)
    left_peak: f32,  // 左右声道峰值 (0.0 到 1.0)
    right_rms: f32,
    right_peak: f32,
    // 用于 Peak Hold 指针缓慢下落的平滑值
    left_peak_hold: f32,
    right_peak_hold: f32,
}

impl<'a> VuMeterWidget<'a> {
    pub fn new(
        theme: &'a Theme,
        left_rms: f32,
        left_peak: f32,
        right_rms: f32,
        right_peak: f32,
        left_peak_hold: f32,
        right_peak_hold: f32,
    ) -> Self {
        Self {
            theme,
            left_rms,
            left_peak,
            right_rms,
            right_peak,
            left_peak_hold,
            right_peak_hold,
        }
    }
}

impl<'a> Widget for VuMeterWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        // 垂直平分 L 和 R 声道渲染区，并加一些间距
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(45), // Left Channel
                Constraint::Percentage(10), // Gap
                Constraint::Percentage(45), // Right Channel
            ])
            .split(area);

        self.draw_channel(chunks[0], "L (LEFT CHANNEL)", self.left_rms, self.left_peak, self.left_peak_hold, buf);
        self.draw_channel(chunks[2], "R (RIGHT CHANNEL)", self.right_rms, self.right_peak, self.right_peak_hold, buf);
    }
}

impl<'a> VuMeterWidget<'a> {
    fn draw_channel(&self, area: Rect, title: &str, rms: f32, peak: f32, peak_hold: f32, buf: &mut Buffer) {
        if area.width < 10 || area.height < 3 {
            return;
        }

        // 外层卡片
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(self.theme.border))
            .title(Span::styled(format!(" {} ", title), Style::default().fg(self.theme.fg).bold()));
        
        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height < 1 {
            return;
        }

        // 分贝转换函数
        let to_db = |val: f32| -> f32 {
            if val > 0.0 { 20.0 * val.log10() } else { -90.0 }
        };

        let rms_db = to_db(rms);
        let peak_db = to_db(peak);
        let peak_hold_db = to_db(peak_hold);

        // 映射 -60dB 到 0dB 为 0% 到 100% 进度
        let db_to_percent = |db: f32| -> f32 {
            ((db + 50.0) / 50.0).clamp(0.0, 1.0)
        };

        let rms_pct = db_to_percent(rms_db);
        let peak_pct = db_to_percent(peak_db);
        let peak_hold_pct = db_to_percent(peak_hold_db);

        // 分割内部区域：左侧是进度条，右侧是数值面板
        let layouts = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(10),       // Progress Bar
                Constraint::Length(26),    // Values panel
            ])
            .split(inner);

        let bar_area = layouts[0];
        let val_area = layouts[1];

        // 1. 绘制水平音量条
        // 留出边框 [] 符号位置
        if bar_area.width > 2 {
            let left_bracket_col = bar_area.x;
            let right_bracket_col = bar_area.x + bar_area.width - 1;
            
            // 写入两端括号
            for y in 0..bar_area.height {
                buf.get_mut(left_bracket_col, bar_area.y + y).set_char('[').set_fg(self.theme.border);
                buf.get_mut(right_bracket_col, bar_area.y + y).set_char(']').set_fg(self.theme.border);
            }

            let bar_len = (bar_area.width - 2) as usize;
            let rms_cells = (rms_pct * bar_len as f32).round() as usize;
            let peak_hold_cell = (peak_hold_pct * bar_len as f32).round() as usize;

            for x_cell in 0..bar_len {
                let col_x = bar_area.x + 1 + x_cell as u16;
                
                // 确定进度条内每个网格的颜色：前面 60% 蓝/绿，60%-85% 橙，85%-100% 红
                let pct = x_cell as f32 / bar_len as f32;
                let cell_color = if pct < 0.6 {
                    self.theme.accent1 // 绿色/蓝色
                } else if pct < 0.85 {
                    self.theme.accent3 // 橙色/黄色
                } else {
                    self.theme.accent4 // 红色/粉色
                };

                for y in 0..bar_area.height {
                    let col_y = bar_area.y + y;
                    let cell = buf.get_mut(col_x, col_y);

                    if x_cell < rms_cells {
                        cell.set_char('█').set_fg(cell_color);
                    } else if x_cell == peak_hold_cell {
                        // 在 Peak Hold 对应的网格上绘制纵向短线表示指针
                        cell.set_char('┃').set_fg(self.theme.accent4);
                    } else {
                        cell.set_char('░').set_fg(self.theme.border);
                    }
                }
            }
        }

        // 2. 绘制数值文字
        let rms_str = if rms_db <= -49.0 { "-∞ dB".to_string() } else { format!("{:.1} dB", rms_db) };
        let peak_str = if peak_db <= -49.0 { "-∞ dB".to_string() } else { format!("{:.1} dB", peak_db) };
        let peak_hold_str = if peak_hold_db <= -49.0 { "-∞ dB".to_string() } else { format!("{:.1} dB", peak_hold_db) };

        let lines = vec![
            Line::from(vec![
                Span::styled("  RMS  Power: ", Style::default().fg(self.theme.fg).dim()),
                Span::styled(format!("{:>9}", rms_str), Style::default().fg(self.theme.accent1).bold()),
            ]),
            Line::from(vec![
                Span::styled("  Peak Power: ", Style::default().fg(self.theme.fg).dim()),
                Span::styled(format!("{:>9}", peak_str), Style::default().fg(self.theme.accent3).bold()),
            ]),
            Line::from(vec![
                Span::styled("  Peak Hold:  ", Style::default().fg(self.theme.fg).dim()),
                Span::styled(format!("{:>9}", peak_hold_str), Style::default().fg(self.theme.accent4).bold()),
            ]),
        ];
        
        // 居中垂直渲染文字
        let text_height = lines.len() as u16;
        let text_y = val_area.y + (val_area.height.saturating_sub(text_height)) / 2;
        f32_render_text(buf, val_area, lines, text_y);
    }
}

// 辅助渲染多行文本
fn f32_render_text(buf: &mut Buffer, area: Rect, lines: Vec<Line>, start_y: u16) {
    for (i, line) in lines.into_iter().enumerate() {
        let y = start_y + i as u16;
        if y >= area.y + area.height {
            break;
        }
        let mut x = area.x;
        for span in line.spans {
            if x >= area.x + area.width {
                break;
            }
            buf.set_stringn(x, y, &span.content, (area.x + area.width - x) as usize, span.style);
            x += span.content.chars().count() as u16;
        }
    }
}
