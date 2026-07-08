// pub mod layout;
// pub mod side_panel;
pub mod widgets;

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, BorderType, Paragraph, Clear, List, ListItem, ListState},
};
use crate::app::App;
use crate::theme::Theme;
use widgets::{BarsWidget, WaveformWidget, CircleWidget, SpectrogramWidget, LissajousWidget, VuMeterWidget};

pub fn draw_app(f: &mut Frame, app: &mut App) {
    let area = f.size();
    let theme = &app.theme;

    // 自适应重置 DSP 频带个数，使其等于绘图区的宽度
    if app.config.visualizer.bar_count == 0 {
        app.dsp.resize_bars(area.width as usize);
    } else {
        app.dsp.resize_bars(app.config.visualizer.bar_count);
    }

    if area.width > 0 && area.height > 0 {
        match app.config.visualizer.mode.as_str() {
            "bars" => {
                let widget = BarsWidget::new(theme, &app.dsp.cur_bars, &app.dsp.peaks);
                f.render_widget(widget, area);
            }
            "waveform" => {
                let samples = app.get_mono_samples(512); // 取 512 个样本点绘制波形
                let widget = WaveformWidget::new(theme, &samples);
                f.render_widget(widget, area);
            }
            "circle" => {
                let widget = CircleWidget::new(theme, &app.dsp.cur_bars);
                f.render_widget(widget, area);
            }
            "spectrogram" => {
                let widget = SpectrogramWidget::new(theme, &app.waterfall_history);
                f.render_widget(widget, area);
            }
            "lissajous" => {
                let left = app.get_left_samples(600);
                let right = app.get_right_samples(600);
                let widget = LissajousWidget::new(theme, &left, &right);
                f.render_widget(widget, area);
            }
            "vu_meter" => {
                // 拆分声道获取单独的 RMS / Peak
                let (l_rms, l_peak, r_rms, r_peak) = app.get_stereo_metrics();
                
                let widget = VuMeterWidget::new(
                    theme,
                    l_rms,
                    l_peak,
                    r_rms,
                    r_peak,
                    app.vu_l_peak_hold,
                    app.vu_r_peak_hold,
                );
                f.render_widget(widget, area);
            }
            _ => {}
        }
    }

    // 渲染弹出窗口 (Popups) - 置于最顶层
    if app.show_help {
        draw_help_popup(f, area, theme);
    } else if app.show_device_select {
        draw_device_popup(f, area, theme, &app.audio_devices, &mut app.device_list_state);
    } else if app.show_theme_select {
        draw_theme_popup(f, area, theme, &app.themes, &mut app.theme_list_state);
    }
}

// 弹出窗口辅助定位
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

// 6.1 渲染帮助弹窗
fn draw_help_popup(f: &mut Frame, area: Rect, theme: &Theme) {
    let size = centered_rect(50, 45, area);
    f.render_widget(Clear, size); // 清空背景

    let help_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.accent1).bold())
        .title(Span::styled(" 󰞋 Mirage Help Menu ", Style::default().fg(theme.accent2).bold()));

    let help_text = vec![
        Line::from(vec![Span::styled(" Mirage —— Next-Generation TUI Audio Visualizer", Style::default().fg(theme.fg).bold())]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Tab  ", Style::default().fg(theme.accent3).bold()),
            Span::styled("Cycle Visualizer Mode (Bars/Wave/Circle/Water/Liss/VU)", Style::default().fg(theme.fg)),
        ]),
        Line::from(vec![
            Span::styled("   T   ", Style::default().fg(theme.accent3).bold()),
            Span::styled("Open Theme Selection Menu", Style::default().fg(theme.fg)),
        ]),
        Line::from(vec![
            Span::styled("   D   ", Style::default().fg(theme.accent3).bold()),
            Span::styled("Open Audio Device Selection Menu", Style::default().fg(theme.fg)),
        ]),
        Line::from(vec![
            Span::styled("   S   ", Style::default().fg(theme.accent3).bold()),
            Span::styled("Switch Input Source (Loopback / Microphone)", Style::default().fg(theme.fg)),
        ]),
        Line::from(vec![
            Span::styled("   1   ", Style::default().fg(theme.accent3).bold()),
            Span::styled("Directly Select System Audio (Loopback)", Style::default().fg(theme.fg)),
        ]),
        Line::from(vec![
            Span::styled("   2   ", Style::default().fg(theme.accent3).bold()),
            Span::styled("Directly Select Microphone (Mic)", Style::default().fg(theme.fg)),
        ]),
        Line::from(vec![
            Span::styled("  Esc  ", Style::default().fg(theme.accent3).bold()),
            Span::styled("Close popup menus", Style::default().fg(theme.fg)),
        ]),
        Line::from(vec![
            Span::styled("   Q   ", Style::default().fg(theme.accent3).bold()),
            Span::styled("Exit Application", Style::default().fg(theme.fg)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(" Edit config.toml to customize defaults (Hot Reload supported!)", Style::default().fg(theme.fg).dim())]),
    ];

    let help_para = Paragraph::new(help_text)
        .block(help_block)
        .alignment(Alignment::Left)
        .wrap(ratatui::widgets::Wrap { trim: false });
    
    f.render_widget(help_para, size);
}

// 6.2 渲染设备选择弹窗
fn draw_device_popup(
    f: &mut Frame,
    area: Rect,
    theme: &Theme,
    devices: &[String],
    state: &mut ListState,
) {
    let size = centered_rect(60, 50, area);
    f.render_widget(Clear, size);

    let popup_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.accent2).bold())
        .title(Span::styled(" 󰓃 Select Audio Device (Enter to select, Esc to exit) ", Style::default().fg(theme.fg).bold()));

    let items: Vec<ListItem> = devices
        .iter()
        .map(|d| {
            let icon = if d.contains("Default") || d == "default" { "󰓃 " } else { "󰎈 " };
            ListItem::new(format!("{}{}", icon, d)).style(Style::default().fg(theme.fg))
        })
        .collect();

    let list = List::new(items)
        .block(popup_block)
        .highlight_style(
            Style::default()
                .fg(theme.bg)
                .bg(theme.accent2)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, size, state);
}

// 6.3 渲染主题选择弹窗
fn draw_theme_popup(
    f: &mut Frame,
    area: Rect,
    theme: &Theme,
    themes: &[String],
    state: &mut ListState,
) {
    let size = centered_rect(30, 45, area);
    f.render_widget(Clear, size);

    let popup_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.accent3).bold())
        .title(Span::styled(" 󰏘 Select Theme ", Style::default().fg(theme.fg).bold()));

    let items: Vec<ListItem> = themes
        .iter()
        .map(|t| ListItem::new(format!(" 󰏘 {}", t.to_uppercase())).style(Style::default().fg(theme.fg)))
        .collect();

    let list = List::new(items)
        .block(popup_block)
        .highlight_style(
            Style::default()
                .fg(theme.bg)
                .bg(theme.accent3)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, size, state);
}
