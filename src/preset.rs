use crate::monitor::MonitorInfo;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Preset {
    pub name: String,
    pub monitors: Vec<MonitorConfig>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MonitorConfig {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub refresh_rate: f32,
    pub x: i32,
    pub y: i32,
    pub scale: f32,
    pub disabled: bool,
    pub workspaces: Vec<u32>,
}

impl From<&MonitorInfo> for MonitorConfig {
    fn from(m: &MonitorInfo) -> Self {
        MonitorConfig {
            name: m.name.clone(),
            width: m.width,
            height: m.height,
            refresh_rate: m.refresh_rate,
            x: m.x,
            y: m.y,
            scale: m.scale,
            disabled: m.disabled,
            workspaces: m.workspaces.clone(),
        }
    }
}

fn presets_dir() -> PathBuf {
    let dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("monitui")
        .join("presets");
    fs::create_dir_all(&dir).ok();
    dir
}

fn recent_path() -> PathBuf {
    let dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("monitui");
    fs::create_dir_all(&dir).ok();
    dir.join("recent.json")
}

pub fn save_preset(name: &str, monitors: &[MonitorInfo]) -> Result<(), String> {
    let preset = Preset {
        name: name.to_string(),
        monitors: monitors.iter().map(MonitorConfig::from).collect(),
    };
    let path = presets_dir().join(format!("{}.json", sanitize_filename(name)));
    let json = serde_json::to_string_pretty(&preset).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())
}

pub fn load_preset(name: &str) -> Result<Preset, String> {
    let path = presets_dir().join(format!("{}.json", sanitize_filename(name)));
    let json = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

pub fn delete_preset(name: &str) -> Result<(), String> {
    let path = presets_dir().join(format!("{}.json", sanitize_filename(name)));
    fs::remove_file(&path).map_err(|e| e.to_string())
}

pub fn list_presets() -> Vec<String> {
    let dir = presets_dir();
    let mut names = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.path().file_stem().and_then(|s| s.to_str()) {
                names.push(name.to_string());
            }
        }
    }
    names.sort();
    names
}

pub fn save_recent(monitors: &[MonitorInfo]) {
    let configs: Vec<MonitorConfig> = monitors.iter().map(MonitorConfig::from).collect();
    if let Ok(json) = serde_json::to_string_pretty(&configs) {
        fs::write(recent_path(), json).ok();
    }
}

pub fn load_recent() -> Option<Vec<MonitorConfig>> {
    let json = fs::read_to_string(recent_path()).ok()?;
    serde_json::from_str(&json).ok()
}

/// Apply a preset's monitor configs to the current monitor list.
/// Matches by monitor name; unmatched monitors keep their current state.
pub fn apply_preset_to_monitors(monitors: &mut Vec<MonitorInfo>, configs: &[MonitorConfig]) {
    for config in configs {
        if let Some(m) = monitors.iter_mut().find(|m| m.name == config.name) {
            m.width = config.width;
            m.height = config.height;
            m.refresh_rate = config.refresh_rate;
            m.x = config.x;
            m.y = config.y;
            m.scale = config.scale;
            m.disabled = config.disabled;
            m.workspaces = config.workspaces.clone();
        }
    }
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn make_test_monitor(name: &str) -> MonitorInfo {
        MonitorInfo {
            name: name.to_string(),
            description: format!("Test {}", name),
            width: 1920,
            height: 1080,
            refresh_rate: 60.0,
            x: 0,
            y: 0,
            scale: 1.0,
            disabled: false,
            transform: 0,
            workspaces: vec![1],
            available_modes: vec![],
            selected_mode: None,
        }
    }

    #[test]
    fn test_monitor_config_roundtrip() {
        let monitor = make_test_monitor("DP-1");
        let config = MonitorConfig::from(&monitor);
        assert_eq!(config.name, "DP-1");
        assert_eq!(config.width, 1920);
        assert_eq!(config.scale, 1.0);
    }

    #[test]
    fn test_apply_preset_to_monitors() {
        let mut monitors = vec![
            make_test_monitor("DP-1"),
            make_test_monitor("DP-2"),
        ];
        let configs = vec![
            MonitorConfig {
                name: "DP-1".to_string(),
                width: 2560,
                height: 1440,
                refresh_rate: 144.0,
                x: 0,
                y: 0,
                scale: 1.5,
                disabled: false,
                workspaces: vec![1, 2],
            },
        ];
        apply_preset_to_monitors(&mut monitors, &configs);
        assert_eq!(monitors[0].width, 2560);
        assert_eq!(monitors[0].scale, 1.5);
        assert_eq!(monitors[1].width, 1920); // DP-2 unchanged
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("my preset!"), "my_preset_");
        assert_eq!(sanitize_filename("work-setup_2"), "work-setup_2");
    }
}
