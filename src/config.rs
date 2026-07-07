use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use crossbeam_channel::Sender;
use notify::{Watcher, RecursiveMode, RecommendedWatcher};

pub fn get_config_path() -> PathBuf {
    let mut path = if let Ok(appdata) = std::env::var("APPDATA") {
        PathBuf::from(appdata)
    } else if let Ok(home) = std::env::var("HOME") {
        let mut p = PathBuf::from(home);
        p.push(".config");
        p
    } else {
        PathBuf::from(".")
    };
    path.push("mirage");
    let _ = fs::create_dir_all(&path);
    path.push("config.toml");
    path
}


#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VisualizerConfig {
    pub mode: String,
    pub fps: u32,
    pub bar_count: usize,
    pub sensitivity: f32,
    pub show_side_panel: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AudioConfig {
    pub device: String,
    pub source: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ThemeConfig {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PhysicsConfig {
    pub smoothing: f32,
    pub spring_k: f32,
    pub spring_damping: f32,
    pub gravity: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub visualizer: VisualizerConfig,
    pub audio: AudioConfig,
    pub theme: ThemeConfig,
    pub physics: PhysicsConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            visualizer: VisualizerConfig {
                mode: "bars".to_string(),
                fps: 60,
                bar_count: 0,
                sensitivity: 1.0,
                show_side_panel: true,
            },
            audio: AudioConfig {
                device: "default".to_string(),
                source: "loopback".to_string(),
            },
            theme: ThemeConfig {
                name: "tokyo_night".to_string(),
            },
            physics: PhysicsConfig {
                smoothing: 0.7,
                spring_k: 15.0,
                spring_damping: 1.8,
                gravity: 1.5,
            },
        }
    }
}

pub fn load_config<P: AsRef<Path>>(path: P) -> Config {
    let path = path.as_ref();
    if !path.exists() {
        // 如果文件不存在，尝试把默认配置写入该路径
        let default_config = Config::default();
        if let Ok(toml_str) = toml::to_string_pretty(&default_config) {
            let _ = fs::write(path, toml_str);
        }
        return default_config;
    }

    match fs::read_to_string(path) {
        Ok(content) => match toml::from_str(&content) {
            Ok(config) => config,
            Err(e) => {
                eprintln!("Error parsing config.toml: {}. Using default configuration.", e);
                Config::default()
            }
        },
        Err(e) => {
            eprintln!("Error reading config.toml: {}. Using default configuration.", e);
            Config::default()
        }
    }
}

// 监控配置文件，发现变动就将新配置发给通道
pub fn watch_config<P: AsRef<Path>>(path: P, tx: Sender<Config>) -> Option<RecommendedWatcher> {
    let path_buf = PathBuf::from(path.as_ref());
    if !path_buf.exists() {
        return None;
    }

    let (event_tx, event_rx) = std::sync::mpsc::channel();
    let mut watcher = match RecommendedWatcher::new(event_tx, notify::Config::default()) {
        Ok(w) => w,
        Err(_) => return None,
    };

    if watcher.watch(&path_buf, RecursiveMode::NonRecursive).is_err() {
        return None;
    }

    std::thread::spawn(move || {
        // 保持 watcher 不被释放
        for res in event_rx {
            match res {
                Ok(event) => {
                    // 检测是否为修改或写入事件
                    if event.kind.is_modify() || event.kind.is_create() {
                        // 延迟 50ms 避开写入时序冲突
                        std::thread::sleep(std::time::Duration::from_millis(50));
                        if let Ok(content) = fs::read_to_string(&path_buf) {
                            if let Ok(config) = toml::from_str::<Config>(&content) {
                                let _ = tx.send(config);
                            }
                        }
                    }
                }
                Err(_) => break,
            }
        }
    });

    Some(watcher)
}
