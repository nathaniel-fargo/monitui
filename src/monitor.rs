use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AvailableMode {
    pub width: u32,
    pub height: u32,
    pub refresh: f32,
}

impl std::fmt::Display for AvailableMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}x{}@{:.0}Hz", self.width, self.height, self.refresh)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct MonitorInfo {
    pub name: String,
    pub description: String,
    pub width: u32,
    pub height: u32,
    pub refresh_rate: f32,
    pub x: i32,
    pub y: i32,
    pub scale: f32,
    pub disabled: bool,
    pub transform: u8,
    pub workspaces: Vec<u32>,
    pub available_modes: Vec<AvailableMode>,
    pub selected_mode: Option<usize>,
}

impl MonitorInfo {
    pub fn logical_width(&self) -> i32 {
        ((self.width as f32) / self.scale).ceil() as i32
    }

    pub fn logical_height(&self) -> i32 {
        ((self.height as f32) / self.scale).ceil() as i32
    }

    pub fn resolution_string(&self) -> String {
        if self.disabled {
            "Disabled".to_string()
        } else {
            format!("{}x{}@{:.0}Hz", self.width, self.height, self.refresh_rate)
        }
    }

    pub fn cycle_resolution(&mut self) {
        if self.available_modes.is_empty() {
            return;
        }
        let next = match self.selected_mode {
            Some(i) => (i + 1) % self.available_modes.len(),
            None => 0,
        };
        self.selected_mode = Some(next);
        let mode = &self.available_modes[next];
        self.width = mode.width;
        self.height = mode.height;
        self.refresh_rate = mode.refresh;
    }

    pub fn mode_string(&self) -> String {
        if self.selected_mode.is_some() {
            format!("{}x{}@{:.0}", self.width, self.height, self.refresh_rate)
        } else {
            "preferred".to_string()
        }
    }
}

fn parse_mode(mode_str: &str) -> Option<AvailableMode> {
    // "1920x1080@60.00Hz"
    let parts: Vec<&str> = mode_str.split('@').collect();
    if parts.len() != 2 {
        return None;
    }
    let res: Vec<&str> = parts[0].split('x').collect();
    if res.len() != 2 {
        return None;
    }
    let width = res[0].parse().ok()?;
    let height = res[1].parse().ok()?;
    let refresh = parts[1].trim_end_matches("Hz").parse().ok()?;
    Some(AvailableMode { width, height, refresh })
}

pub fn fetch_monitors() -> Vec<MonitorInfo> {
    let output = match Command::new("hyprctl")
        .args(["-j", "monitors", "all"])
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => {
            eprintln!("Failed to run hyprctl -j monitors all");
            std::process::exit(1);
        }
    };

    let raw: Vec<serde_json::Value> = match serde_json::from_slice(&output.stdout) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Failed to parse hyprctl output: {}", e);
            vec![]
        }
    };

    let mut all_monitors = Vec::new();
    let mut non_headless_monitors = Vec::new();

    for m in raw {
        let name = match m.get("name").and_then(|v| v.as_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };

        let description = m.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let width = m.get("width").and_then(|v| v.as_u64()).unwrap_or(1920) as u32;
        let height = m.get("height").and_then(|v| v.as_u64()).unwrap_or(1080) as u32;
        let refresh_rate = m.get("refreshRate").and_then(|v| v.as_f64()).unwrap_or(60.0) as f32;
        let x = m.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let y = m.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
        let scale = m.get("scale").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
        let disabled = m.get("disabled").and_then(|v| v.as_bool()).unwrap_or(false);
        let transform = m.get("transform").and_then(|v| v.as_u64()).unwrap_or(0) as u8;

        let workspaces = m.get("activeWorkspace")
            .and_then(|v| v.as_object())
            .and_then(|obj| obj.get("id"))
            .and_then(|v| v.as_u64())
            .map(|id| vec![id as u32])
            .unwrap_or_default();

        let available_modes = m.get("availableModes")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().and_then(parse_mode))
                    .collect()
            })
            .unwrap_or_default();

        let monitor = MonitorInfo {
            name: name.clone(),
            description,
            width,
            height,
            refresh_rate,
            x,
            y,
            scale,
            disabled,
            transform,
            workspaces,
            available_modes,
            selected_mode: None,
        };

        all_monitors.push(monitor.clone());
        if !name.starts_with("HEADLESS-") {
            non_headless_monitors.push(monitor);
        }
    }

    // Smart auto-detection: show HEADLESS monitors only when no enabled physical monitors exist
    let has_enabled_physical = non_headless_monitors.iter().any(|m| !m.disabled);
    let mut monitors = if has_enabled_physical {
        non_headless_monitors
    } else {
        all_monitors
    };

    // Sort: enabled first by x position, disabled at bottom
    monitors.sort_by(|a, b| {
        a.disabled.cmp(&b.disabled).then_with(|| a.x.cmp(&b.x))
    });

    monitors
}
