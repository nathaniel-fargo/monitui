mod app;
mod apply;
mod layout;
mod monitor;
mod preset;
mod ui;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

fn main() -> io::Result<()> {
    if std::env::args().any(|a| a == "--everywhere") {
        return launch_everywhere();
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = app::App::new();
    let result = app.run(&mut terminal);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;

    result
}

fn launch_everywhere() -> io::Result<()> {
    // Query enabled monitors
    let output = std::process::Command::new("hyprctl")
        .args(["-j", "monitors"])
        .output()?;

    let monitors: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let exe = std::env::current_exe()?;
    let terminal = std::env::var("TERMINAL").unwrap_or_else(|_| "alacritty".to_string());

    let mut children: Vec<std::process::Child> = Vec::new();

    for mon in &monitors {
        let name = mon["name"].as_str().unwrap_or("").to_string();
        if name.is_empty() || name.starts_with("HEADLESS-") {
            continue;
        }
        let rule = format!("[float; monitor {}; size 80% 80%; center]", name);
        let child = std::process::Command::new("hyprctl")
            .args(["dispatch", "exec", &format!("{} {} -e {}", rule, terminal, exe.display())])
            .spawn()?;
        children.push(child);
    }

    if children.is_empty() {
        eprintln!("No monitors found");
        return Ok(());
    }

    // Wait for any child to exit, then kill the rest
    // Since hyprctl dispatch exec returns immediately, we just wait briefly
    // The actual monitui instances are grandchildren; we can't easily track them.
    // Instead, just wait for all hyprctl dispatches to finish.
    for child in &mut children {
        let _ = child.wait();
    }

    Ok(())
}
