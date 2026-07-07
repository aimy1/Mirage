use std::collections::VecDeque;
use std::time::{Duration, Instant};
use crossbeam_channel::{unbounded, Receiver, Sender};
use ratatui::widgets::ListState;
use sysinfo::System;

use crate::audio::AudioEngine;
use crate::config::{Config, load_config, watch_config};
use crate::dsp::DspProcessor;
use crate::theme::Theme;

pub struct App {
    // 配置与主题
    pub config: Config,
    pub theme: Theme,
    pub themes: Vec<String>,
    config_path: std::path::PathBuf,
    config_rx: Receiver<Config>,
    _config_watcher: Option<notify::RecommendedWatcher>,

    // 音频捕获与处理
    pub audio_engine: AudioEngine,
    audio_rx: Receiver<f32>,
    audio_tx: Sender<f32>,
    pub dsp: DspProcessor,

    // 时域环形队列 (长度固定为 2048)
    left_channel_ring: VecDeque<f32>,
    right_channel_ring: VecDeque<f32>,
    mono_channel_ring: VecDeque<f32>,

    // 频谱图滚动历史 (用于瀑布图)
    pub waterfall_history: VecDeque<Vec<f32>>,

    // 系统指标与计算指标
    sys: System,
    pub sys_cpu: f32,
    pub sys_mem: f32,
    sys_last_update: Instant,

    // BPM & 音频指标
    pub bpm: f32,
    beat_history_rms: VecDeque<f32>,
    last_beat_time: Instant,

    // VU Peak Hold
    pub vu_l_peak_hold: f32,
    pub vu_r_peak_hold: f32,

    // 交互状态
    pub show_help: bool,
    pub show_device_select: bool,
    pub show_theme_select: bool,

    // 列表状态 (弹窗)
    pub audio_devices: Vec<String>,
    pub device_list_state: ListState,
    pub theme_list_state: ListState,

    // FPS 计算
    pub fps: u32,
    fps_counter: u32,
    fps_last_update: Instant,
}

impl App {
    pub fn new(config_path: std::path::PathBuf) -> Self {
        let config = load_config(&config_path);
        let theme = Theme::from_name(&config.theme.name);

        // 初始化通知通道用于 config.toml 热加载
        let (config_tx, config_rx) = unbounded();
        let _config_watcher = watch_config(&config_path, config_tx);

        // 初始化音频通道
        let (audio_tx, audio_rx) = unbounded();
        let mut audio_engine = AudioEngine::new();

        // 尝试启动默认设备音频流
        let _ = audio_engine.start(&config.audio.device, &config.audio.source, audio_tx.clone());

        // 初始化 DSP 处理器，FFT 长度 2048，自适应初始 bars = 64
        let mut dsp = DspProcessor::new(2048, config.visualizer.bar_count);
        dsp.update_params(
            config.physics.spring_k,
            config.physics.spring_damping,
            config.physics.gravity,
            config.visualizer.sensitivity,
            config.physics.smoothing,
        );

        // 初始化系统监视器
        let mut sys = System::new_all();
        sys.refresh_cpu();
        sys.refresh_memory();

        // 获取设备列表用于菜单
        let audio_devices: Vec<String> = audio_engine
            .list_devices()
            .iter()
            .map(|d| d.name.clone())
            .collect();

        let themes = vec![
            "tokyo_night".to_string(),
            "catppuccin".to_string(),
            "gruvbox".to_string(),
            "nord".to_string(),
            "dracula".to_string(),
            "everforest".to_string(),
        ];

        Self {
            config,
            theme,
            themes,
            config_path,
            config_rx,
            _config_watcher,
            audio_engine,
            audio_rx,
            audio_tx,
            dsp,
            left_channel_ring: VecDeque::from(vec![0.0; 2048]),
            right_channel_ring: VecDeque::from(vec![0.0; 2048]),
            mono_channel_ring: VecDeque::from(vec![0.0; 2048]),
            waterfall_history: VecDeque::new(),
            sys,
            sys_cpu: 0.0,
            sys_mem: 0.0,
            sys_last_update: Instant::now(),
            bpm: 120.0,
            beat_history_rms: VecDeque::new(),
            last_beat_time: Instant::now(),
            vu_l_peak_hold: 0.0,
            vu_r_peak_hold: 0.0,
            show_help: false,
            show_device_select: false,
            show_theme_select: false,
            audio_devices,
            device_list_state: ListState::default(),
            theme_list_state: ListState::default(),
            fps: 0,
            fps_counter: 0,
            fps_last_update: Instant::now(),
        }
    }

    /// 执行帧更新（处理数据、计算物理平滑与动画，被渲染主循环调用，~16.6ms）
    pub fn update(&mut self, dt: f32) {
        // 1. 处理配置热加载
        if let Ok(new_config) = self.config_rx.try_recv() {
            self.apply_new_config(new_config);
        }

        // 2. 接收新的音频样本
        self.pull_audio_samples();

        // 3. 执行 FFT 与 DSP
        let mono_vec: Vec<f32> = self.mono_channel_ring.iter().copied().collect();
        if mono_vec.len() >= 2048 {
            self.dsp.compute_bars(&mono_vec, self.audio_engine.sample_rate, dt);
            
            // 4. 将新频段帧压入 Spectrogram 瀑布历史
            self.waterfall_history.push_front(self.dsp.cur_bars.clone());
            if self.waterfall_history.len() > 100 {
                self.waterfall_history.pop_back();
            }

            // 5. 估计 BPM 与更新双声道 VU 指针
            let (rms, _) = self.get_audio_metrics();
            self.estimate_bpm(rms);
            let (_, l_peak, _, r_peak) = self.get_stereo_metrics();
            self.update_vu_peaks(l_peak, r_peak);
        }

        // 6. 每秒更新系统资源占用 (CPU/Memory)，降低系统资源开销
        let now = Instant::now();
        if now.duration_since(self.sys_last_update) >= Duration::from_secs(1) {
            self.sys.refresh_cpu();
            self.sys.refresh_memory();
            self.sys_cpu = self.sys.global_cpu_info().cpu_usage();
            self.sys_mem = (self.sys.used_memory() as f32 / self.sys.total_memory() as f32) * 100.0;
            self.sys_last_update = now;
        }

        // 7. FPS 计数器
        self.fps_counter += 1;
        let elapsed_fps = now.duration_since(self.fps_last_update);
        if elapsed_fps >= Duration::from_secs(1) {
            self.fps = (self.fps_counter as f32 / elapsed_fps.as_secs_f32()).round() as u32;
            self.fps_counter = 0;
            self.fps_last_update = now;
        }
    }

    /// 从 cpal 音频流通道拉取并交错解包所有样本
    fn pull_audio_samples(&mut self) {
        let mut temp = Vec::new();
        while let Ok(sample) = self.audio_rx.try_recv() {
            temp.push(sample);
        }

        if temp.is_empty() {
            return;
        }

        let channels = self.audio_engine.channels as usize;
        if channels == 0 {
            return;
        }

        // 提取交错的多声道采样，归档到左右与单声道环形缓冲区中
        for chunk in temp.chunks_exact(channels) {
            let left = chunk[0];
            let right = if channels > 1 { chunk[1] } else { chunk[0] };
            let mono = (left + right) / 2.0;

            self.left_channel_ring.push_back(left);
            self.right_channel_ring.push_back(right);
            self.mono_channel_ring.push_back(mono);

            // 保持缓冲区长度固定在 2048 (满足 FFT 尺寸即可)
            if self.left_channel_ring.len() > 2048 { self.left_channel_ring.pop_front(); }
            if self.right_channel_ring.len() > 2048 { self.right_channel_ring.pop_front(); }
            if self.mono_channel_ring.len() > 2048 { self.mono_channel_ring.pop_front(); }
        }
    }

    /// 重启音频捕获设备/源
    pub fn restart_audio(&mut self) {
        let _ = self.audio_engine.start(
            &self.config.audio.device,
            &self.config.audio.source,
            self.audio_tx.clone()
        );
        // 清空以前的旧采样缓冲，防止突变
        self.left_channel_ring = VecDeque::from(vec![0.0; 2048]);
        self.right_channel_ring = VecDeque::from(vec![0.0; 2048]);
        self.mono_channel_ring = VecDeque::from(vec![0.0; 2048]);
    }

    /// 应用修改后的配置并重置部分依赖模块
    pub fn apply_new_config(&mut self, new_config: Config) {
        let old_device = self.config.audio.device.clone();
        let old_source = self.config.audio.source.clone();

        self.config = new_config;
        
        // 1. 重载物理与增益参数
        self.dsp.update_params(
            self.config.physics.spring_k,
            self.config.physics.spring_damping,
            self.config.physics.gravity,
            self.config.visualizer.sensitivity,
            self.config.physics.smoothing,
        );

        // 2. 重新初始化主题
        self.theme = Theme::from_name(&self.config.theme.name);

        // 3. 判断是否需要重启音频流 (设备或源变更)
        if self.config.audio.device != old_device || self.config.audio.source != old_source {
            self.restart_audio();
        }
    }

    /// 获取最近 1024 点单声道采样的瞬时 RMS (均方根) 和 Peak (最大幅值)
    pub fn get_audio_metrics(&self) -> (f32, f32) {
        let count = 1024.min(self.mono_channel_ring.len());
        if count == 0 {
            return (0.0, 0.0);
        }

        let start = self.mono_channel_ring.len() - count;
        let mut sum_sq = 0.0;
        let mut max_val = 0.0;

        for i in 0..count {
            let val = self.mono_channel_ring[start + i];
            sum_sq += val * val;
            let abs_val = val.abs();
            if abs_val > max_val {
                max_val = abs_val;
            }
        }

        let rms = (sum_sq / count as f32).sqrt();
        (rms, max_val)
    }

    /// 获取左右双声道各自独立的 (L_RMS, L_Peak, R_RMS, R_Peak)
    pub fn get_stereo_metrics(&self) -> (f32, f32, f32, f32) {
        let count = 1024.min(self.left_channel_ring.len()).min(self.right_channel_ring.len());
        if count == 0 {
            return (0.0, 0.0, 0.0, 0.0);
        }

        let l_start = self.left_channel_ring.len() - count;
        let r_start = self.right_channel_ring.len() - count;
        
        let mut l_sum_sq = 0.0;
        let mut r_sum_sq = 0.0;
        let mut l_max = 0.0;
        let mut r_max = 0.0;

        for i in 0..count {
            let l_val = self.left_channel_ring[l_start + i];
            let r_val = self.right_channel_ring[r_start + i];

            l_sum_sq += l_val * l_val;
            r_sum_sq += r_val * r_val;

            let abs_l = l_val.abs();
            let abs_r = r_val.abs();

            if abs_l > l_max { l_max = abs_l; }
            if abs_r > r_max { r_max = abs_r; }
        }

        (
            (l_sum_sq / count as f32).sqrt(),
            l_max,
            (r_sum_sq / count as f32).sqrt(),
            r_max,
        )
    }

    /// 更新 VU 仪表的缓降 Peak Hold
    pub fn update_vu_peaks(&mut self, cur_l_peak: f32, cur_r_peak: f32) {
        if cur_l_peak >= self.vu_l_peak_hold {
            self.vu_l_peak_hold = cur_l_peak;
        } else {
            self.vu_l_peak_hold = self.vu_l_peak_hold * 0.93 + cur_l_peak * 0.07;
        }

        if cur_r_peak >= self.vu_r_peak_hold {
            self.vu_r_peak_hold = cur_r_peak;
        } else {
            self.vu_r_peak_hold = self.vu_r_peak_hold * 0.93 + cur_r_peak * 0.07;
        }
    }

    /// 估计 BPM：轻量级包络能量历史均值差分法
    fn estimate_bpm(&mut self, cur_rms: f32) {
        if self.beat_history_rms.is_empty() {
            self.beat_history_rms.push_back(cur_rms);
            return;
        }

        let avg_rms = self.beat_history_rms.iter().sum::<f32>() / self.beat_history_rms.len() as f32;
        self.beat_history_rms.push_back(cur_rms);
        if self.beat_history_rms.len() > 43 {
            self.beat_history_rms.pop_front();
        }

        let threshold = avg_rms * 1.35;
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_beat_time).as_secs_f32();

        // 限制两次鼓点识别的最短时间 (0.33s -> 最快 180BPM，0.85s -> 最慢 70BPM)
        if cur_rms > threshold && elapsed > 0.33 && elapsed < 1.0 {
            let instantaneous_bpm = 60.0 / elapsed;
            self.bpm = self.bpm * 0.8 + instantaneous_bpm * 0.2; // 阻尼估计
            self.last_beat_time = now;
        } else if elapsed >= 1.2 {
            // 如果长时间没有检测到显著鼓点，使 BPM 缓慢往 100/120 靠拢
            self.bpm = self.bpm * 0.99 + 100.0 * 0.01;
        }
    }

    /// 获取拉伸的单声道时域采样数据 (用于波形 Widget 绘制)
    pub fn get_mono_samples(&self, count: usize) -> Vec<f32> {
        let available = self.mono_channel_ring.len();
        let count = count.min(available);
        if count == 0 {
            return Vec::new();
        }
        let start = available - count;
        self.mono_channel_ring.iter().skip(start).copied().collect()
    }

    /// 获取左声道时域数据
    pub fn get_left_samples(&self, count: usize) -> Vec<f32> {
        let available = self.left_channel_ring.len();
        let count = count.min(available);
        if count == 0 {
            return Vec::new();
        }
        let start = available - count;
        self.left_channel_ring.iter().skip(start).copied().collect()
    }

    /// 获取右声道时域数据
    pub fn get_right_samples(&self, count: usize) -> Vec<f32> {
        let available = self.right_channel_ring.len();
        let count = count.min(available);
        if count == 0 {
            return Vec::new();
        }
        let start = available - count;
        self.right_channel_ring.iter().skip(start).copied().collect()
    }

    /// 获取音频延迟 ms
    pub fn get_latency_ms(&self) -> f32 {
        // 延迟近似于 采样缓冲区(512样点) / 采样率
        let sample_rate = self.audio_engine.sample_rate as f32;
        if sample_rate > 0.0 {
            (1024.0 / sample_rate) * 1000.0
        } else {
            0.0
        }
    }

    // 交互菜单快捷操作
    pub fn open_device_select(&mut self) {
        // 重新获取当前最新设备列表
        self.audio_devices = self.audio_engine
            .list_devices()
            .iter()
            .map(|d| d.name.clone())
            .collect();
        
        self.show_device_select = true;
        self.show_help = false;
        self.show_theme_select = false;
        
        // 默认定位到当前激活设备
        let idx = self.audio_devices
            .iter()
            .position(|d| d == &self.audio_engine.current_device)
            .unwrap_or(0);
        self.device_list_state.select(Some(idx));
    }

    pub fn open_theme_select(&mut self) {
        self.show_theme_select = true;
        self.show_help = false;
        self.show_device_select = false;
        
        let idx = self.themes
            .iter()
            .position(|t| t == &self.config.theme.name)
            .unwrap_or(0);
        self.theme_list_state.select(Some(idx));
    }

    pub fn menu_next(&mut self) {
        if self.show_device_select {
            let i = match self.device_list_state.selected() {
                Some(i) => {
                    if i >= self.audio_devices.len() - 1 { 0 } else { i + 1 }
                }
                None => 0,
            };
            self.device_list_state.select(Some(i));
        } else if self.show_theme_select {
            let i = match self.theme_list_state.selected() {
                Some(i) => {
                    if i >= self.themes.len() - 1 { 0 } else { i + 1 }
                }
                None => 0,
            };
            self.theme_list_state.select(Some(i));
        }
    }

    pub fn menu_prev(&mut self) {
        if self.show_device_select {
            let i = match self.device_list_state.selected() {
                Some(i) => {
                    if i == 0 { self.audio_devices.len() - 1 } else { i - 1 }
                }
                None => 0,
            };
            self.device_list_state.select(Some(i));
        } else if self.show_theme_select {
            let i = match self.theme_list_state.selected() {
                Some(i) => {
                    if i == 0 { self.themes.len() - 1 } else { i - 1 }
                }
                None => 0,
            };
            self.theme_list_state.select(Some(i));
        }
    }

    pub fn menu_confirm(&mut self) {
        if self.show_device_select {
            if let Some(idx) = self.device_list_state.selected() {
                if idx < self.audio_devices.len() {
                    let selected_device = self.audio_devices[idx].clone();
                    self.config.audio.device = selected_device;
                    self.restart_audio();
                    
                    // 保存新配置到本地 config.toml 文件
                    if let Ok(toml_str) = toml::to_string_pretty(&self.config) {
                        let _ = std::fs::write(&self.config_path, toml_str);
                    }
                }
            }
            self.show_device_select = false;
        } else if self.show_theme_select {
            if let Some(idx) = self.theme_list_state.selected() {
                if idx < self.themes.len() {
                    let selected_theme = self.themes[idx].clone();
                    self.config.theme.name = selected_theme.clone();
                    self.theme = Theme::from_name(&selected_theme);
                    
                    // 保存新配置到本地
                    if let Ok(toml_str) = toml::to_string_pretty(&self.config) {
                        let _ = std::fs::write(&self.config_path, toml_str);
                    }
                }
            }
            self.show_theme_select = false;
        }
    }

    pub fn menu_close(&mut self) {
        self.show_help = false;
        self.show_device_select = false;
        self.show_theme_select = false;
    }
}
