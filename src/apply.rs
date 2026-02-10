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
    lines.push("# Managed by monitui — https://github.com/nathaniel-fargo/monitui".to_string());
    lines.push("# Manual edits will be overwritten on next apply.".to_string());
    lines.push("# Disabled monitors are not persisted; they are applied at runtime only.".to_string());
    lines.push(String::new());

    for m in monitors {
        if m.disabled {
            continue;
        }
        let mode = m.mode_string();
        let pos = format!("{}x{}", m.x, m.y);
        let scale = format_scale(m.scale);
        let transform = format!("transform, {}", m.transform);
        lines.push(format!("monitor = {}, {}, {}, {}, {}", m.name, mode, pos, scale, transform));
    }

    lines.push(String::new());
    lines.join("\n")
}

/// Apply monitor configuration via hyprctl AND write monitors.conf.
pub fn apply_monitors(monitors: &[MonitorInfo]) -> Result<(), String> {
    // Write monitors.conf first so persisted state does not include disabled outputs.
    let conf_path = monitors_conf_path();
    let content = generate_monitors_conf(monitors);
    fs::write(&conf_path, &content)
        .map_err(|e| format!("Failed to write {}: {}", conf_path.display(), e))?;

    // Reload Hyprland configuration so file-backed state is active first.
    let reload_output = Command::new("hyprctl")
        .args(["reload"])
        .output()
        .map_err(|e| format!("Failed to run hyprctl reload: {}", e))?;
    if !reload_output.status.success() {
        return Err(format!(
            "hyprctl reload failed: {}",
            String::from_utf8_lossy(&reload_output.stderr).trim()
        ));
    }

    // Then apply runtime state (including temporary disables) on top of the persisted config.
    for monitor in monitors {
        let cmd = if monitor.disabled {
            format!("{},disable", monitor.name)
        } else {
            let mode = monitor.mode_string();
            let pos = format!("{}x{}", monitor.x, monitor.y);
            let scale = format_scale(monitor.scale);
            format!("{},{},{},{},transform,{}", monitor.name, mode, pos, scale, monitor.transform)
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

#[cfg(test)]
mod tests {
    use super::generate_monitors_conf;
    use crate::monitor::MonitorInfo;

    fn test_monitor(name: &str, disabled: bool) -> MonitorInfo {
        MonitorInfo {
            name: name.to_string(),
            description: "Test monitor".to_string(),
            width: 1920,
            height: 1080,
            refresh_rate: 60.0,
            x: 0,
            y: 0,
            scale: 1.0,
            disabled,
            transform: 0,
            workspaces: vec![],
            available_modes: vec![],
            selected_mode: None,
        }
    }

    #[test]
    fn monitors_conf_excludes_disabled_monitors() {
        let monitors = vec![
            test_monitor("DP-1", false),
            test_monitor("HDMI-A-1", true),
        ];

        let content = generate_monitors_conf(&monitors);

        assert!(content.contains("monitor = DP-1, preferred, 0x0, 1, transform, 0"));
        assert!(!content.contains("HDMI-A-1, disable"));
        assert!(!content.contains("monitor = HDMI-A-1"));
    }
}
