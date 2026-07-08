use std::io;
use std::time::{Duration, Instant};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;

mod audio;
mod config;
mod dsp;
mod theme;
mod app;
mod ui;

use app::App;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 设置 Panic 钩子以确保崩溃时能够干净地恢复终端状态
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = execute!(stdout, LeaveAlternateScreen, crossterm::cursor::Show);
        default_hook(info);
    }));

    // 2. 初始化终端后端
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, crossterm::cursor::Hide)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 3. 创建 App 状态 (指向全局平台配置)
    let config_path = config::get_config_path();
    let mut app = App::new(config_path);

    // 4. 渲染主循环，目标 60 FPS
    let target_fps = 60;
    let frame_duration = Duration::from_secs_f64(1.0 / target_fps as f64);
    let mut last_frame_time = Instant::now();

    loop {
        let frame_start = Instant::now();
        let dt = frame_start.duration_since(last_frame_time).as_secs_f32();
        last_frame_time = frame_start;

        // 5. 更新应用数据 (处理音频、FFT、物理插值、系统指标等)
        app.update(dt);

        // 6. 渲染 UI
        terminal.draw(|f| ui::draw_app(f, &mut app))?;

        // 7. 处理终端事件输入 (非阻塞轮询)
        // 轮询 1ms 看看有没有按键，防止空转占用 100% CPU
        if event::poll(Duration::from_millis(1))? {
            if let Event::Key(key) = event::read()? {
                // Windows 平台要严格限定只处理 Press 事件，避免触发 Release 重复响应
                if key.kind == KeyEventKind::Press {
                    // 如果有弹窗处于打开状态，由弹窗接管部分按键
                    if app.show_device_select || app.show_theme_select {
                        match key.code {
                            KeyCode::Esc => app.menu_close(),
                            KeyCode::Up => app.menu_prev(),
                            KeyCode::Down => app.menu_next(),
                            KeyCode::Enter => app.menu_confirm(),
                            _ => {}
                        }
                    } else if app.show_help {
                        match key.code {
                            KeyCode::Esc | KeyCode::F(1) => app.menu_close(),
                            KeyCode::Char('q') | KeyCode::Char('Q') => break,
                            _ => {}
                        }
                    } else {
                        // 正常全局快捷键处理
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Char('Q') => {
                                break; // 退出程序
                            }
                            KeyCode::Tab => {
                                // 轮转可视化模式
                                let next_mode = match app.config.visualizer.mode.as_str() {
                                    "bars" => "waveform",
                                    "waveform" => "circle",
                                    "circle" => "spectrogram",
                                    "spectrogram" => "lissajous",
                                    "lissajous" => "vu_meter",
                                    _ => "bars",
                                };
                                app.config.visualizer.mode = next_mode.to_string();
                                // 保存新配置到全局配置文件
                                app.save_config();
                            }
                            KeyCode::Char('t') | KeyCode::Char('T') => {
                                app.open_theme_select();
                            }
                            KeyCode::Char('d') | KeyCode::Char('D') => {
                                app.open_device_select();
                            }
                            KeyCode::Char('p') | KeyCode::Char('P') => {
                                // 切换侧边栏
                                app.config.visualizer.show_side_panel = !app.config.visualizer.show_side_panel;
                                // 保存配置到全局配置文件
                                app.save_config();
                            }
                            KeyCode::F(1) => {
                                app.show_help = true;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        // 8. 帧率睡眠控制以保持 ~60 FPS
        let elapsed = frame_start.elapsed();
        if elapsed < frame_duration {
            std::thread::sleep(frame_duration - elapsed);
        }
    }

    // 9. 干净退出，还原终端
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        crossterm::cursor::Show
    )?;

    Ok(())
}
