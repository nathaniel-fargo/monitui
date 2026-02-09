mod app;
mod apply;
mod cli;
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
    let args: Vec<String> = std::env::args().collect();

    // Handle CLI commands
    if args.len() > 1 {
        match args[1].as_str() {
            "--help" | "-h" => {
                cli::print_help();
                return Ok(());
            }
            "--list" => {
                cli::list_monitors();
                return Ok(());
            }
            "--presets" => {
                cli::list_presets_cmd();
                return Ok(());
            }
            "--preset" => {
                if args.len() < 3 {
                    eprintln!("Error: --preset requires a preset name");
                    eprintln!("Usage: monitui --preset <name>");
                    std::process::exit(1);
                }
                cli::apply_preset(&args[2]);
                return Ok(());
            }
            "--reload" => {
                cli::reload_recent();
                return Ok(());
            }
            "--enable" => {
                if args.len() < 3 {
                    eprintln!("Error: --enable requires a monitor name");
                    eprintln!("Usage: monitui --enable <monitor>");
                    std::process::exit(1);
                }
                cli::enable_monitor(&args[2]);
                return Ok(());
            }
            "--disable" => {
                if args.len() < 3 {
                    eprintln!("Error: --disable requires a monitor name");
                    eprintln!("Usage: monitui --disable <monitor>");
                    std::process::exit(1);
                }
                cli::disable_monitor(&args[2]);
                return Ok(());
            }
            "--set-workspace" => {
                if args.len() < 4 {
                    eprintln!("Error: --set-workspace requires workspace number and monitor name");
                    eprintln!("Usage: monitui --set-workspace <num> <monitor>");
                    std::process::exit(1);
                }
                let workspace: u32 = match args[2].parse() {
                    Ok(n) => n,
                    Err(_) => {
                        eprintln!("Error: Invalid workspace number '{}'", args[2]);
                        std::process::exit(1);
                    }
                };
                cli::set_workspace(workspace, &args[3]);
                return Ok(());
            }
            _ => {
                eprintln!("Error: Unknown option '{}'", args[1]);
                eprintln!("Run 'monitui --help' for usage information");
                std::process::exit(1);
            }
        }
    }

    // No CLI args, launch TUI
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
