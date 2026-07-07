use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::Sender;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct AudioDevice {
    pub name: String,
    pub is_output: bool, // true 表示输出设备(用于环回捕获)，false 表示输入设备(如麦克风)
}

pub struct AudioEngine {
    host: cpal::Host,
    stream: Option<cpal::Stream>,
    pub current_device: String,
    pub channels: u16,
    pub sample_rate: u32,
}

#[cfg(target_os = "windows")]
const USE_OUTPUT_DEVICE_FOR_LOOPBACK: bool = true;
#[cfg(not(target_os = "windows"))]
const USE_OUTPUT_DEVICE_FOR_LOOPBACK: bool = false;

impl AudioEngine {
    pub fn new() -> Self {
        Self {
            host: cpal::default_host(),
            stream: None,
            current_device: "None".to_string(),
            channels: 2,
            sample_rate: 44100,
        }
    }

    /// 获取所有可用的音频设备列表（包括输入和输出）
    pub fn list_devices(&self) -> Vec<AudioDevice> {
        let mut devices = Vec::new();

        // 列出输出设备（用于 Loopback 捕获系统播放的音乐）
        if let Ok(output_devices) = self.host.output_devices() {
            for device in output_devices {
                if let Ok(name) = device.name() {
                    devices.push(AudioDevice {
                        name,
                        is_output: true,
                    });
                }
            }
        }

        // 列出输入设备（如麦克风）
        if let Ok(input_devices) = self.host.input_devices() {
            for device in input_devices {
                if let Ok(name) = device.name() {
                    // 避免重复添加名称相同的设备
                    if !devices.iter().any(|d| d.name == name && d.is_output) {
                        devices.push(AudioDevice {
                            name,
                            is_output: false,
                        });
                    }
                }
            }
        }

        devices
    }

    /// 启动音频流捕获
    pub fn start(
        &mut self,
        device_name: &str,
        source_type: &str, // "loopback" 或 "mic"
        tx: Sender<f32>,
    ) -> Result<(), String> {
        // 停止之前的流
        self.stream = None;

        let is_loopback = source_type == "loopback";
        let search_output = is_loopback && USE_OUTPUT_DEVICE_FOR_LOOPBACK;
        
        // 1. 选择设备
        let device = if device_name == "default" {
            if search_output {
                self.host.default_output_device()
                    .ok_or_else(|| "No default output device (loopback) found".to_string())?
            } else {
                // 如果在 Linux 下要获取环回流且设备名是 default，我们要主动搜寻 monitor 虚拟输入设备
                let mut found_monitor = None;
                if is_loopback {
                    if let Ok(input_devices) = self.host.input_devices() {
                        for d in input_devices {
                            if let Ok(name) = d.name() {
                                let lower_name = name.to_lowercase();
                                if lower_name.contains("monitor") || lower_name.contains("loopback") {
                                    found_monitor = Some(d);
                                    break;
                                }
                            }
                        }
                    }
                }

                match found_monitor {
                    Some(d) => d,
                    None => self.host.default_input_device()
                        .ok_or_else(|| "No default input device (microphone) found".to_string())?
                }
            }
        } else {
            let mut found = None;
            if search_output {
                if let Ok(devices) = self.host.output_devices() {
                    for d in devices {
                        if d.name().unwrap_or_default() == device_name {
                            found = Some(d);
                            break;
                        }
                    }
                }
            } else {
                if let Ok(devices) = self.host.input_devices() {
                    for d in devices {
                        if d.name().unwrap_or_default() == device_name {
                            found = Some(d);
                            break;
                        }
                    }
                }
            }

            // 如果指定名字的设备找不到，尝试回退到默认
            match found {
                Some(d) => d,
                None => {
                    if search_output {
                        self.host.default_output_device()
                            .ok_or_else(|| format!("Device '{}' not found, and no default output device available", device_name))?
                    } else {
                        self.host.default_input_device()
                            .ok_or_else(|| format!("Device '{}' not found, and no default input device available", device_name))?
                    }
                }
            }
        };

        self.current_device = device.name().unwrap_or_else(|_| "Unknown Device".to_string());

        // 2. 获取配置
        // 如果是 Windows 环回(loopback)，我们需要使用输出配置
        let supported_config = if search_output {
            device.default_output_config()
                .map_err(|e| format!("Failed to get default output config: {}", e))?
        } else {
            device.default_input_config()
                .map_err(|e| format!("Failed to get default input config: {}", e))?
        };

        let stream_config = supported_config.config();
        self.channels = stream_config.channels;
        self.sample_rate = stream_config.sample_rate.0;

        let sample_format = supported_config.sample_format();
        let err_fn = |err| eprintln!("An error occurred on the audio stream: {}", err);

        // 3. 根据采样格式构建输入流
        // 在 Windows Loopback 模式下，我们虽然使用输出设备，但是将其作为输入流打开以录制其声音。
        let stream = match sample_format {
            cpal::SampleFormat::F32 => {
                device.build_input_stream(
                    &stream_config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        for &sample in data {
                            let _ = tx.try_send(sample);
                        }
                    },
                    err_fn,
                    None,
                )
            }
            cpal::SampleFormat::I16 => {
                device.build_input_stream(
                    &stream_config,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        for &sample in data {
                            // 归一化 i16 (-32768 到 32767) 为 f32 (-1.0 到 1.0)
                            let float_sample = sample as f32 / 32768.0;
                            let _ = tx.try_send(float_sample);
                        }
                    },
                    err_fn,
                    None,
                )
            }
            cpal::SampleFormat::U16 => {
                device.build_input_stream(
                    &stream_config,
                    move |data: &[u16], _: &cpal::InputCallbackInfo| {
                        for &sample in data {
                            // 归一化 u16 (0 到 65535) 为 f32 (-1.0 到 1.0)
                            let float_sample = (sample as f32 - 32768.0) / 32768.0;
                            let _ = tx.try_send(float_sample);
                        }
                    },
                    err_fn,
                    None,
                )
            }
            _ => return Err(format!("Unsupported sample format: {:?}", sample_format)),
        }.map_err(|e| format!("Failed to build input stream: {}", e))?;

        // 4. 启动流
        stream.play().map_err(|e| format!("Failed to play stream: {}", e))?;
        self.stream = Some(stream);

        Ok(())
    }
}
