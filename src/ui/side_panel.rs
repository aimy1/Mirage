use ratatui::{
    prelude::*,
    widgets::{Block, Borders, BorderType, Paragraph, Gauge},
};
use crate::theme::Theme;

pub fn render_side_panel(
    f: &mut Frame,
    area: Rect,
    theme: &Theme,
    cpu_usage: f32,
    mem_usage: f32,
    peak: f32,
    rms: f32,
    bpm: f32,
    latency_ms: f32,
    audio_source: &str,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme.border))
        .title(Span::styled(" 󱝁 Status ", Style::default().fg(theme.accent1).bold()));

    f.render_widget(block.clone(), area);

    // 内部区域稍微缩进一行一列
    let inner_area = block.inner(area);
    if inner_area.width < 5 || inner_area.height < 10 {
        return; // 区域太小不渲染
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Source Title
            Constraint::Length(2), // Source Content
            Constraint::Length(1), // Separator
            Constraint::Length(4), // Audio Metrics (BPM, Peak, RMS)
            Constraint::Length(1), // Separator
            Constraint::Length(5), // System Metrics (CPU, MEM)
            Constraint::Min(0),    // Padding
        ])
        .split(inner_area);

    // 1. Source Info
    f.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled("󰎈 Source: ", Style::default().fg(theme.fg).dim()),
            ]),
            Line::from(vec![
                Span::styled(format!("  {}", audio_source.to_uppercase()), Style::default().fg(theme.accent2).bold()),
            ]),
        ]),
        chunks[0].union(chunks[1])
    );

    // Separator 1
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("─".repeat(inner_area.width as usize), Style::default().fg(theme.border).dim())
        ])),
        chunks[2]
    );

    // 2. Audio Metrics
    let rms_db = if rms > 0.0 { 20.0 * rms.log10() } else { -90.0 };
    let rms_percent = ((rms_db + 60.0) / 60.0).clamp(0.0, 1.0); // 映射 -60dB~0dB 到 0%~100%
    
    let audio_text = vec![
        Line::from(vec![
            Span::styled("󱐋 Peak: ", Style::default().fg(theme.fg).dim()),
            Span::styled(format!("{:.2}", peak), Style::default().fg(theme.accent4).bold()),
        ]),
        Line::from(vec![
            Span::styled("󰓎 RMS:  ", Style::default().fg(theme.fg).dim()),
            Span::styled(format!("{:.1} dB", rms_db), Style::default().fg(theme.accent3).bold()),
        ]),
        Line::from(vec![
            Span::styled("󰏔 BPM:  ", Style::default().fg(theme.fg).dim()),
            Span::styled(format!("{:.0}", bpm), Style::default().fg(theme.accent1).bold()),
        ]),
        Line::from(vec![
            Span::styled("󰑮 Latency: ", Style::default().fg(theme.fg).dim()),
            Span::styled(format!("{:.1}ms", latency_ms), Style::default().fg(theme.fg)),
        ]),
    ];
    f.render_widget(Paragraph::new(audio_text), chunks[3]);

    // Separator 2
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("─".repeat(inner_area.width as usize), Style::default().fg(theme.border).dim())
        ])),
        chunks[4]
    );

    // 3. System Metrics
    let cpu_gauge = Gauge::default()
        .gauge_style(Style::default().fg(theme.accent1).bg(theme.border))
        .label(format!("CPU {:.0}%", cpu_usage))
        .percent(cpu_usage.clamp(0.0, 100.0) as u16);

    let mem_gauge = Gauge::default()
        .gauge_style(Style::default().fg(theme.accent2).bg(theme.border))
        .label(format!("MEM {:.0}%", mem_usage))
        .percent(mem_usage.clamp(0.0, 100.0) as u16);

    let sys_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Title
            Constraint::Length(1), // CPU Gauge
            Constraint::Length(1), // Spacer
            Constraint::Length(1), // MEM Gauge
            Constraint::Min(0),
        ])
        .split(chunks[5]);

    f.render_widget(Paragraph::new(Span::styled("󰢚 Resources:", Style::default().fg(theme.fg).dim())), sys_chunks[0]);
    f.render_widget(cpu_gauge, sys_chunks[1]);
    f.render_widget(mem_gauge, sys_chunks[3]);
}
