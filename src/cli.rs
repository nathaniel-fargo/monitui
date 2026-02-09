use crate::{apply, monitor, preset};
use std::process;

pub fn print_help() {
    println!("monitui v{}", env!("CARGO_PKG_VERSION"));
    println!("{}", env!("CARGO_PKG_DESCRIPTION"));
    println!();
    println!("USAGE:");
    println!("    monitui                                    Launch interactive TUI");
    println!("    monitui --list                             List all monitors and their status");
    println!("    monitui --presets                          List all saved presets");
    println!("    monitui --preset <name>                    Apply saved preset");
    println!("    monitui --reload                           Reload most recent configuration");
    println!("    monitui --enable <monitor>                 Enable a monitor (e.g., DP-1)");
    println!("    monitui --disable <monitor>                Disable a monitor (e.g., DP-2)");
    println!("    monitui --set-workspace <num> <monitor>    Assign workspace to monitor");
    println!("    monitui --help                             Show this help message");
    println!();
    println!("EXAMPLES:");
    println!("    monitui --list                             Show all monitors");
    println!("    monitui --presets                          Show all presets");
    println!("    monitui --preset laptop                    Apply the 'laptop' preset");
    println!("    monitui --reload                           Reload last applied config");
    println!("    monitui --enable DP-1                      Enable DP-1 monitor");
    println!("    monitui --disable HDMI-A-1                 Disable HDMI-A-1 monitor");
    println!("    monitui --set-workspace 5 DP-1             Move workspace 5 to DP-1");
    println!();
    println!("For more information, visit: https://github.com/nathanielbd/monitui");
}

pub fn apply_preset(name: &str) {
    let preset_obj = match preset::load_preset(name) {
        Ok(p) => p,
        Err(_) => {
            eprintln!("Error: Preset '{}' not found", name);
            eprintln!("Available presets:");
            for preset_name in preset::list_presets() {
                eprintln!("  - {}", preset_name);
            }
            process::exit(1);
        }
    };

    // Get current monitors and apply preset configs
    let mut monitors = monitor::fetch_monitors_all();
    preset::apply_preset_to_monitors(&mut monitors, &preset_obj.monitors);

    println!("Applying preset '{}'...", name);
    match apply::apply_monitors(&monitors) {
        Ok(_) => {
            preset::save_recent(&monitors);
            println!("✓ Successfully applied preset '{}'", name);
        }
        Err(e) => {
            eprintln!("Error: Failed to apply preset: {}", e);
            process::exit(1);
        }
    }
}

pub fn reload_recent() {
    let configs = match preset::load_recent() {
        Some(c) => c,
        None => {
            eprintln!("Error: No recent configuration found");
            eprintln!("Apply a configuration first using the TUI or --preset");
            process::exit(1);
        }
    };

    // Get current monitors and apply recent configs
    let mut monitors = monitor::fetch_monitors_all();
    preset::apply_preset_to_monitors(&mut monitors, &configs);

    println!("Reloading most recent configuration...");
    match apply::apply_monitors(&monitors) {
        Ok(_) => {
            println!("✓ Successfully reloaded recent configuration");
        }
        Err(e) => {
            eprintln!("Error: Failed to reload config: {}", e);
            process::exit(1);
        }
    }
}

pub fn enable_monitor(monitor_name: &str) {
    let mut monitors = monitor::fetch_monitors_all();

    let monitor = match monitors.iter_mut().find(|m| m.name == monitor_name) {
        Some(m) => m,
        None => {
            eprintln!("Error: Monitor '{}' not found", monitor_name);
            eprintln!("Available monitors:");
            for m in &monitors {
                eprintln!("  - {} ({})", m.name, if m.disabled { "disabled" } else { "enabled" });
            }
            process::exit(1);
        }
    };

    if !monitor.disabled {
        println!("Monitor '{}' is already enabled", monitor_name);
        return;
    }

    monitor.disabled = false;

    println!("Enabling monitor '{}'...", monitor_name);
    match apply::apply_monitors(&monitors) {
        Ok(_) => {
            preset::save_recent(&monitors);
            println!("✓ Successfully enabled '{}'", monitor_name);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}

pub fn disable_monitor(monitor_name: &str) {
    let mut monitors = monitor::fetch_monitors_all();

    let monitor = match monitors.iter_mut().find(|m| m.name == monitor_name) {
        Some(m) => m,
        None => {
            eprintln!("Error: Monitor '{}' not found", monitor_name);
            eprintln!("Available monitors:");
            for m in &monitors {
                eprintln!("  - {} ({})", m.name, if m.disabled { "disabled" } else { "enabled" });
            }
            process::exit(1);
        }
    };

    if monitor.disabled {
        println!("Monitor '{}' is already disabled", monitor_name);
        return;
    }

    monitor.disabled = true;

    println!("Disabling monitor '{}'...", monitor_name);
    match apply::apply_monitors(&monitors) {
        Ok(_) => {
            preset::save_recent(&monitors);
            println!("✓ Successfully disabled '{}'", monitor_name);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}

pub fn set_workspace(workspace: u32, monitor_name: &str) {
    let monitors = monitor::fetch_monitors_all();

    let monitor = match monitors.iter().find(|m| m.name == monitor_name) {
        Some(m) => m,
        None => {
            eprintln!("Error: Monitor '{}' not found", monitor_name);
            eprintln!("Available monitors:");
            for m in &monitors {
                eprintln!("  - {} ({})", m.name, if m.disabled { "disabled" } else { "enabled" });
            }
            process::exit(1);
        }
    };

    if monitor.disabled {
        eprintln!("Error: Cannot assign workspace to disabled monitor '{}'", monitor_name);
        eprintln!("Enable it first with: monitui --enable {}", monitor_name);
        process::exit(1);
    }

    println!("Moving workspace {} to '{}'...", workspace, monitor_name);

    // Use hyprctl to move the workspace
    let output = std::process::Command::new("hyprctl")
        .args(["dispatch", "moveworkspacetomonitor", &workspace.to_string(), monitor_name])
        .output();

    match output {
        Ok(o) if o.status.success() => {
            println!("✓ Successfully moved workspace {} to '{}'", workspace, monitor_name);
        }
        Ok(o) => {
            eprintln!("Error: hyprctl command failed:");
            eprintln!("{}", String::from_utf8_lossy(&o.stderr));
            process::exit(1);
        }
        Err(e) => {
            eprintln!("Error: Failed to run hyprctl: {}", e);
            process::exit(1);
        }
    }
}

pub fn list_monitors() {
    let monitors = monitor::fetch_monitors_all();

    println!("Monitors:");
    println!();

    for m in &monitors {
        let status = if m.disabled { "DISABLED" } else { "enabled" };
        let ws_text = if m.workspaces.is_empty() {
            "no workspaces".to_string()
        } else {
            format!("WS: {}", m.workspaces.iter().map(|w| w.to_string()).collect::<Vec<_>>().join(", "))
        };

        println!("  {} - {} | {} | {} | Pos: {}x{} | Scale: {:.2}x | Rotation: {}",
            m.name,
            status,
            m.resolution_string(),
            ws_text,
            m.x,
            m.y,
            m.scale,
            m.rotation_string()
        );
    }
}

pub fn list_presets_cmd() {
    let preset_names = preset::list_presets();

    if preset_names.is_empty() {
        println!("No presets found.");
        println!("Create presets using the interactive TUI (press 'p', then 's')");
        return;
    }

    println!("Available presets:");
    println!();

    for name in &preset_names {
        match preset::load_preset(name) {
            Ok(p) => {
                println!("  {}:", name);

                // Sort monitors by position: left to right, top to bottom for ties
                let mut enabled: Vec<_> = p.monitors.iter()
                    .filter(|m| !m.disabled)
                    .collect();
                enabled.sort_by(|a, b| a.y.cmp(&b.y).then_with(|| a.x.cmp(&b.x)));

                if enabled.is_empty() {
                    println!("    (no monitors enabled)");
                } else {
                    for m in enabled {
                        let ws_text = if m.workspaces.is_empty() {
                            "no WS".to_string()
                        } else {
                            format!("WS: {}", m.workspaces.iter().map(|w| w.to_string()).collect::<Vec<_>>().join(", "))
                        };
                        // Build resolution string accounting for rotation
                        let (w, h) = match m.transform {
                            1 | 3 => (m.height, m.width),  // 90° or 270° - swap dimensions
                            _ => (m.width, m.height),      // 0° or 180° - keep dimensions
                        };
                        let resolution = format!("{}x{}@{:.0}Hz", w, h, m.refresh_rate);
                        println!("    - {} ({}) | {} | Pos: {}x{} | Scale: {:.2}x",
                            m.name,
                            ws_text,
                            resolution,
                            m.x,
                            m.y,
                            m.scale
                        );
                    }
                }
                println!();
            }
            Err(e) => {
                println!("  {} (error loading: {})", name, e);
            }
        }
    }
}
