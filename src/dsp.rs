use realfft::{RealFftPlanner, RealToComplex};
use std::sync::Arc;

pub struct DspProcessor {
    fft_size: usize,
    r2c: Arc<dyn RealToComplex<f32>>,
    window: Vec<f32>,
    pub bars_count: usize,
    
    // 物理参数
    spring_k: f32,
    spring_damping: f32,
    gravity: f32,
    sensitivity: f32,
    smoothing: f32,
    
    // 柱子的物理状态
    pub cur_bars: Vec<f32>,       // 柱子的当前高度
    velocities: Vec<f32>,       // 柱子的速度
    pub peaks: Vec<f32>,          // 峰值高度
    peak_velocities: Vec<f32>,   // 峰值下降速度
    peak_hold_timers: Vec<u32>,  // 峰值保持计时器 (帧数)
}

impl DspProcessor {
    pub fn new(fft_size: usize, initial_bars_count: usize) -> Self {
        let mut planner = RealFftPlanner::<f32>::new();
        let r2c = planner.plan_fft_forward(fft_size);
        
        // Hanning 窗口预计算
        let window: Vec<f32> = (0..fft_size)
            .map(|i| 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (fft_size - 1) as f32).cos()))
            .collect();

        let bars_count = if initial_bars_count == 0 { 64 } else { initial_bars_count };

        Self {
            fft_size,
            r2c,
            window,
            bars_count,
            spring_k: 15.0,
            spring_damping: 1.8,
            gravity: 1.5,
            sensitivity: 1.0,
            smoothing: 0.7,
            cur_bars: vec![0.0; bars_count],
            velocities: vec![0.0; bars_count],
            peaks: vec![0.0; bars_count],
            peak_velocities: vec![0.0; bars_count],
            peak_hold_timers: vec![0; bars_count],
        }
    }

    /// 在运行时更新物理和平滑参数
    pub fn update_params(&mut self, k: f32, damping: f32, gravity: f32, sensitivity: f32, smoothing: f32) {
        self.spring_k = k;
        self.spring_damping = damping;
        self.gravity = gravity;
        self.sensitivity = sensitivity;
        self.smoothing = smoothing;
    }

    /// 动态调整柱子个数（自适应终端宽度）
    pub fn resize_bars(&mut self, new_count: usize) {
        if new_count == 0 || new_count == self.bars_count {
            return;
        }
        self.bars_count = new_count;
        self.cur_bars.resize(new_count, 0.0);
        self.velocities.resize(new_count, 0.0);
        self.peaks.resize(new_count, 0.0);
        self.peak_velocities.resize(new_count, 0.0);
        self.peak_hold_timers.resize(new_count, 0);
    }

    /// 线性插值采样幅值谱
    fn sample_magnitude(&self, magnitudes: &[f32], float_idx: f32) -> f32 {
        let i0 = float_idx.floor() as usize;
        let i1 = float_idx.ceil() as usize;
        if i1 >= magnitudes.len() {
            return magnitudes[magnitudes.len() - 1];
        }
        let t = float_idx - float_idx.floor();
        magnitudes[i0] * (1.0 - t) + magnitudes[i1] * t
    }

    /// 处理输入的时域 PCM 样本（必须长度等于 fft_size）
    /// 返回平滑后的柱状图幅值，高度已缩放到 0.0 ~ 1.0 范围
    pub fn compute_bars(&mut self, pcm_data: &[f32], sample_rate: u32, dt: f32) {
        if pcm_data.len() < self.fft_size {
            return;
        }

        // 1. 加窗 (Hanning Window)
        let mut indata = self.r2c.make_input_vec();
        for i in 0..self.fft_size {
            indata[i] = pcm_data[i] * self.window[i];
        }

        // 2. FFT 前向计算
        let mut outdata = self.r2c.make_output_vec();
        if self.r2c.process(&mut indata, &mut outdata).is_err() {
            return;
        }

        // 3. 计算幅值谱 (FFT前半部分，对称)
        let mag_len = self.fft_size / 2 + 1;
        let mut magnitudes = vec![0.0; mag_len];
        for i in 0..mag_len {
            let re = outdata[i].re;
            let im = outdata[i].im;
            // 计算幅值，加入归一化因子
            let raw_mag = (re * re + im * im).sqrt() / (self.fft_size as f32);
            // 对数响应或根号响应，让人耳感觉更自然
            magnitudes[i] = (raw_mag * self.sensitivity).sqrt();
        }

        // 4. 对数频率分箱 (Logarithmic Binning)
        // 频率范围: 20Hz ~ 20000Hz (或根据采样率调整)
        let f_min = 20.0;
        let f_max = (sample_rate as f32 / 2.0).min(20000.0);
        
        let mut bin_indices = vec![0.0; self.bars_count + 1];
        for i in 0..=self.bars_count {
            let f = f_min * (f_max / f_min).powf(i as f32 / self.bars_count as f32);
            let idx = f * (self.fft_size as f32) / (sample_rate as f32);
            bin_indices[i] = idx.clamp(0.0, (mag_len - 1) as f32);
        }

        // 5. 采样并映射为目标柱子高度
        let mut target_bars = vec![0.0; self.bars_count];
        for i in 0..self.bars_count {
            let start = bin_indices[i];
            let end = bin_indices[i + 1];
            
            // 使用 5 点均匀插值求平均值，获得极佳平滑度
            let mut sum = 0.0;
            for step in 0..5 {
                let t = step as f32 / 4.0;
                let idx = start + (end - start) * t;
                sum += self.sample_magnitude(&magnitudes, idx);
            }
            
            // 对特定频段做微调加权 (高频衰减补偿，因为高频段能量通常偏低)
            let freq = f_min * (f_max / f_min).powf((i as f32 + 0.5) / self.bars_count as f32);
            let weight = 1.0 + (freq / 500.0).ln().max(0.0) * 0.3; // 补偿高音
            
            // 归一化限制在 0.0 ~ 1.0 之间
            let val = (sum / 5.0 * weight).clamp(0.0, 1.0);
            target_bars[i] = val;
        }

        // 6. 弹簧阻尼物理动画与 Peak Hold 仿真
        for i in 0..self.bars_count {
            let target = target_bars[i];
            let current = self.cur_bars[i];
            
            // 弹簧阻尼系统: F = -k * x - c * v
            // 其中 x 为位移偏移量 (current - target)
            let x = current - target;
            let force = -self.spring_k * x - self.spring_damping * self.velocities[i];
            
            // 更新速度与位移
            self.velocities[i] += force * dt;
            self.cur_bars[i] += self.velocities[i] * dt;
            
            // 额外应用 Attack/Release 混合，防止回弹过剧烈
            if self.cur_bars[i] < target {
                // 快速上升 (Attack)
                self.cur_bars[i] = self.cur_bars[i] * (1.0 - self.smoothing) + target * self.smoothing;
                self.velocities[i] = 0.0; // 上升不积累下落速度
            }
            
            // 限制取值范围
            if self.cur_bars[i] < 0.0 {
                self.cur_bars[i] = 0.0;
                self.velocities[i] = 0.0;
            } else if self.cur_bars[i] > 1.0 {
                self.cur_bars[i] = 1.0;
                self.velocities[i] = 0.0;
            }

            // --- Peak Hold 峰值保持逻辑 ---
            let bar_val = self.cur_bars[i];
            if bar_val >= self.peaks[i] {
                self.peaks[i] = bar_val;
                self.peak_velocities[i] = 0.0;
                self.peak_hold_timers[i] = 15; // 保持 15 帧 (约 250ms)
            } else {
                if self.peak_hold_timers[i] > 0 {
                    self.peak_hold_timers[i] -= 1;
                } else {
                    // 重力下落: v = v + g * dt, p = p - v * dt
                    self.peak_velocities[i] += self.gravity * dt;
                    self.peaks[i] -= self.peak_velocities[i] * dt;
                    if self.peaks[i] < 0.0 {
                        self.peaks[i] = 0.0;
                        self.peak_velocities[i] = 0.0;
                    }
                }
            }
        }
    }
}
