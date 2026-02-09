use crate::monitor::MonitorInfo;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn monitors_conf_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("hypr")
        .join("monitors.conf")
}

/// Generate monitors.conf content from current monitor state.
fn generate_monitors_conf(monitors: &[MonitorInfo]) -> String {
    let mut lines = Vec::new();
    lines.push("# Managed by monitui — https://github.com/your/monitui".to_string());
    lines.push("# Manual edits will be overwritten on next apply.".to_string());
    lines.push(String::new());

    for m in monitors {
        if m.disabled {
            lines.push(format!("monitor = {}, disable", m.name));
        } else {
            let mode = m.mode_string();
            let pos = format!("{}x{}", m.x, m.y);
            let scale = format_scale(m.scale);
            lines.push(format!("monitor = {}, {}, {}, {}", m.name, mode, pos, scale));
        }
    }

    lines.push(String::new());
    lines.join("\n")
}

/// Apply monitor configuration via hyprctl AND write monitors.conf.
pub fn apply_monitors(monitors: &[MonitorInfo]) -> Result<(), String> {
    // First apply live via hyprctl
    for monitor in monitors {
        let cmd = if monitor.disabled {
            format!("{},disable", monitor.name)
        } else {
            let mode = monitor.mode_string();
            let pos = format!("{}x{}", monitor.x, monitor.y);
            let scale = format_scale(monitor.scale);
            format!("{},{},{},{}", monitor.name, mode, pos, scale)
        };

        let output = Command::new("hyprctl")
            .args(["keyword", "monitor", &cmd])
            .output()
            .map_err(|e| format!("Failed to run hyprctl: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("hyprctl failed for {}: {}", monitor.name, stderr));
        }

        if !monitor.disabled {
            for ws in &monitor.workspaces {
                Command::new("hyprctl")
                    .args(["dispatch", "moveworkspacetomonitor", &ws.to_string(), &monitor.name])
                    .output()
                    .ok();
            }
        }
    }

    // Write monitors.conf so it persists
    let conf_path = monitors_conf_path();
    let content = generate_monitors_conf(monitors);
    fs::write(&conf_path, &content)
        .map_err(|e| format!("Failed to write {}: {}", conf_path.display(), e))?;

    Command::new("notify-send")
        .args(["monitui", "Monitor configuration applied"])
        .output()
        .ok();

    Ok(())
}

fn format_scale(scale: f32) -> String {
    if (scale - scale.round()).abs() < 0.001 {
        format!("{}", scale as u32)
    } else {
        format!("{:.6}", scale)
    }
}
