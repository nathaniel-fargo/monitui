use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Stdout};
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Deserialize, PartialEq)]
struct MonitorInfo {
    id: String,
    name: String,
    description: String,
    width: u32,
    height: u32,
    refresh_rate: f32,
    x: i32,
    y: i32,
    scale: f32,
    disabled: bool,
    workspaces: Vec<u32>,
    resolution_string: String,
}

struct App {
    monitors: Vec<MonitorInfo>,
    selected: usize,
    status_msg: String,
    changed: bool,
    show_confirm: bool,
    countdown_start: Option<Instant>,
    countdown_duration: Duration,
    prev_state: Option<Vec<MonitorInfo>>,
}

impl App {
    fn new() -> Self {
        let monitors = Self::fetch_monitors();
        App {
            monitors,
            selected: 0,
            status_msg: "Press 'a' to apply changes, 'q' to quit".to_string(),
            changed: false,
            show_confirm: false,
            countdown_start: None,
            countdown_duration: Duration::from_secs(10),
            prev_state: None,
        }
    }

    fn fetch_monitors() -> Vec<MonitorInfo> {
        let output = Command::new("hyprctl")
            .args(["-j", "monitors"])
            .output()
            .unwrap_or_else(|e| {
                eprintln!("Failed to run hyprctl: {}", e);
                std::process::exit(1);
            });

        let mut monitors_map: HashMap<String, MonitorInfo> = HashMap::new();
        let config_path = Path::new(&std::env::var("HOME").unwrap()).join(".config/hypr/monitors.conf");

        if !output.status.success() {
            eprintln!("hyprctl failed with: {}", String::from_utf8_lossy(&output.stderr));
            std::process::exit(1);
        }

        let raw: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
            eprintln!("Failed to parse hyprctl output: {}", e);
            vec![]
        });

        let mut monitors = Vec::new();

        for m in raw {
            if let (Some(name), Some(description), Some(width), Some(height)) = (
                m.get("name").and_then(|v| v.as_str()),
                m.get("description").and_then(|v| v.as_str()),
                m.get("width").and_then(|v| v.as_u64()),
                m.get("height").and_then(|v| v.as_u64()),
            ) {
                let scale = m.get("scale").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
                let x = m.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let y = m.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let refresh_rate = m.get("refreshRate").and_then(|v| v.as_f64()).unwrap_or(60.0) as f32;
                let disabled = m.get("disabled").and_then(|v| v.as_bool()).unwrap_or(false);
                let workspaces = m.get("activeWorkspace")
                    .and_then(|v| v.as_object())
                    .and_then(|obj| obj.get("id"))
                    .and_then(|v| v.as_u64())
                    .map(|id| vec![id as u32])
                    .unwrap_or_default();

                let resolution_string = if disabled {
                    "Disabled".to_string()
                } else {
                    format!("{}x{}@{:.1}Hz", width, height, refresh_rate)
                };

                monitors.push(MonitorInfo {
                    id: m.get("id").and_then(|v| v.as_u64()).unwrap_or(0).to_string(),
                    name: name.to_string(),
                    description: description.to_string(),
                    width: width as u32,
                    height: height as u32,
                    refresh_rate,
                    x,
                    y,
                    scale,
                    disabled,
                    workspaces,
                    resolution_string,
                });
            }
        }

        monitors.sort_by(|a, b| a.x.cmp(&b.x));
        monitors
    }

    fn get_preset(&self, num: usize) -> Option<Vec<MonitorInfo>> {
        let current = self.monitors.clone();

        match num {
            1 => {
                let mut monitors = current;
                for m in &mut monitors {
                    m.disabled = m.name.contains("DP-2");
                    if !m.disabled {
                        m.scale = if m.name.contains("DP-2") { 2.0 } else { 1.0 };
                    }
                }
                monitors.sort_by(|a, b| a.disabled.cmp(&b.disabled).then_with(|| a.name.cmp(&b.name)));
                Some(monitors)
            }
            2 => {
                let mut monitors = current;
                for m in &mut monitors {
                    m.disabled = m.name.contains("DP-1");
                    if !m.disabled {
                        m.scale = if m.name.contains("DP-2") { 2.0 } else { 1.0 };
                    }
                }
                monitors.sort_by(|a, b| a.disabled.cmp(&b.disabled).then_with(|| a.name.cmp(&b.name)));
                Some(monitors)
            }
            3 => {
                let mut monitors = current;
                for m in &mut monitors {
                    m.disabled = false;
                    m.scale = if m.name.contains("DP-2") { 2.0 } else { 1.0 };
                }
                monitors.sort_by(|a, b| a.name.cmp(&b.name));
                Some(monitors)
            }
            _ => None,
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if self.show_confirm {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    self.prev_state = None;
                    self.show_confirm = false;
                    self.countdown_start = None;
                    return false;
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    if let Some(prev) = self.prev_state.take() {
                        self.monitors = prev;
                        self.recalculate_positions();
                    }
                    self.show_confirm = false;
                    self.countdown_start = None;
                    self.changed = false;
                    return true;
                }
                _ => {
                    if let Some(start) = self.countdown_start {
                        if start.elapsed() > self.countdown_duration {
                            if let Some(prev) = self.prev_state.take() {
                                self.monitors = prev;
                                self.recalculate_positions();
                            }
                            self.show_confirm = false;
                            self.countdown_start = None;
                            self.changed = false;
                        }
                    }
                    return true;
                }
            }
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected < self.monitors.len() - 1 {
                    self.selected += 1;
                }
            }
            KeyCode::Char('d') => {
                self.monitors[self.selected].disabled = true;
                self.changed = true;
                self.status_msg = format!("Disabling {}", self.monitors[self.selected].description);
            }
            KeyCode::Char('e') => {
                self.monitors[self.selected].disabled = false;
                self.changed = true;
                self.status_msg = format!("Enabling {}", self.monitors[self.selected].description);
            }
            KeyCode::Char('s') => {
                self.cycle_scale();
            }
            KeyCode::Char('+') => {
                if !self.monitors[self.selected].disabled {
                    self.cycle_scale_up();
                }
            }
            KeyCode::Char('-') => {
                if !self.monitors[self.selected].disabled {
                    self.cycle_scale_down();
                }
            }
            KeyCode::Char('K') => {
                if self.selected > 0 {
                    self.monitors.swap(self.selected - 1, self.selected);
                    self.selected -= 1;
                    self.recalculate_positions();
                    self.changed = true;
                    self.status_msg = "Moved monitor up".to_string();
                }
            }
            KeyCode::Char('J') => {
                if self.selected < self.monitors.len() - 1 {
                    self.monitors.swap(self.selected, self.selected + 1);
                    self.selected += 1;
                    self.recalculate_positions();
                    self.changed = true;
                    self.status_msg = "Moved monitor down".to_string();
                }
            }
            KeyCode::Char('1') => { if let Some(preset) = self.get_preset(1) {
                if preset != self.monitors {
                    self.prev_state = Some(self.monitors.clone());
                    self.monitors = preset;
                    self.recalculate_positions();
                    self.changed = true;
                    self.apply_changes();
                    self.show_confirm = true;
                    self.countdown_start = Some(Instant::now());
                    self.status_msg = "Preset 1 applied - Press 'y' to keep, 'n' to revert".to_string();
                }
            }}
            KeyCode::Char('2') => { if let Some(preset) = self.get_preset(2) {
                if preset != self.monitors {
                    self.prev_state = Some(self.monitors.clone());
                    self.monitors = preset;
                    self.recalculate_positions();
                    self.changed = true;
                    self.apply_changes();
                    self.show_confirm = true;
                    self.countdown_start = Some(Instant::now());
                    self.status_msg = "Preset 2 applied - Press 'y' to keep, 'n' to revert".to_string();
                }
            }}
            KeyCode::Char('3') => { if let Some(preset) = self.get_preset(3) {
                if preset != self.monitors {
                    self.prev_state = Some(self.monitors.clone());
                    self.monitors = preset;
                    self.recalculate_positions();
                    self.changed = true;
                    self.apply_changes();
                    self.show_confirm = true;
                    self.countdown_start = Some(Instant::now());
                    self.status_msg = "Preset 3 applied - Press 'y' to keep, 'n' to revert".to_string();
                }
            }}
            KeyCode::Char('4') | KeyCode::Char('5') | KeyCode::Char('6')
            | KeyCode::Char('7') | KeyCode::Char('8') | KeyCode::Char('9') => {
                let num = match key.code {
                    KeyCode::Char('4') => 4u32,
                    KeyCode::Char('5') => 5u32,
                    KeyCode::Char('6') => 6u32,
                    KeyCode::Char('7') => 7u32,
                    KeyCode::Char('8') => 8u32,
                    KeyCode::Char('9') => 9u32,
                    _ => return true,
                };
                let monitor = &mut self.monitors[self.selected];
                if !monitor.workspaces.contains(&num) {
                    monitor.workspaces.push(num);
                    monitor.workspaces.sort();
                    self.changed = true;
                    self.status_msg = format!("Added workspace {} to {}", num, monitor.description);
                }
            }
            KeyCode::Char('0') => {
                let monitor = &mut self.monitors[self.selected];
                if !monitor.workspaces.contains(&10) {
                    monitor.workspaces.push(10);
                    monitor.workspaces.sort();
                    self.changed = true;
                    self.status_msg = format!("Added workspace 10 to {}", monitor.description);
                }
            }
            KeyCode::Char('W') => {
                let monitor = &mut self.monitors[self.selected];
                if !monitor.workspaces.is_empty() {
                    monitor.workspaces.clear();
                    self.changed = true;
                    self.status_msg = format!("Cleared workspaces from {}", monitor.description);
                }
            }
            KeyCode::Left | KeyCode::Char('<') => {
                if self.selected > 0 {
                    let from_monitor = self.selected;
                    let to_monitor = self.selected - 1;
                    if !self.monitors[from_monitor].workspaces.is_empty() {
                        if let Some(ws) = self.monitors[from_monitor].workspaces.pop() {
                            self.monitors[to_monitor].workspaces.push(ws);
                            self.monitors[to_monitor].workspaces.sort();
                            self.changed = true;
                            self.status_msg = format!("Moved workspace {} to {}", ws, self.monitors[to_monitor].description);
                        }
                    }
                }
            }
            KeyCode::Right | KeyCode::Char('>') => {
                if self.selected < self.monitors.len() - 1 {
                    let from_monitor = self.selected;
                    let to_monitor = self.selected + 1;
                    if !self.monitors[from_monitor].workspaces.is_empty() {
                        if let Some(ws) = self.monitors[from_monitor].workspaces.pop() {
                            self.monitors[to_monitor].workspaces.push(ws);
                            self.monitors[to_monitor].workspaces.sort();
                            self.changed = true;
                            self.status_msg = format!("Moved workspace {} to {}", ws, self.monitors[to_monitor].description);
                        }
                    }
                }
            }
            KeyCode::Char('a') => {
                if self.changed {
                    self.prev_state = Some(self.monitors.clone());
                    self.apply_changes();
                    self.show_confirm = true;
                    self.countdown_start = Some(Instant::now());
                    self.changed = false;
                } else {
                    self.status_msg = "No changes to apply".to_string();
                }
            }
            KeyCode::Char('q') | KeyCode::Esc => {
                return false;
            }
            _ => {}
        }
        true
    }

    fn cycle_scale(&mut self) {
        let monitor = &mut self.monitors[self.selected];
        if monitor.disabled {
            self.status_msg = "Cannot scale disabled monitor".to_string();
            return;
        }

        let scales = vec![1.0, 1.25, 1.5, 1.6666667, 2.0, 2.5, 3.0];
        let current_idx = scales.iter().position(|&s| (s - monitor.scale).abs() < 0.01).unwrap_or(0);
        let next_idx = (current_idx + 1) % scales.len();
        monitor.scale = scales[next_idx];
        self.changed = true;
        self.status_msg = format!("{} scale: {:.2}x", monitor.description, monitor.scale);
    }

    fn cycle_scale_up(&mut self) {
        let monitor = &mut self.monitors[self.selected];
        let scales = vec![1.0, 1.25, 1.5, 1.6666667, 2.0, 2.5, 3.0];
        let current_idx = scales.iter().position(|&s| (s - monitor.scale).abs() < 0.01).unwrap_or(0);
        if current_idx < scales.len() - 1 {
            monitor.scale = scales[current_idx + 1];
            self.changed = true;
            self.status_msg = format!("{} scale: {:.2}x", monitor.description, monitor.scale);
        }
    }

    fn cycle_scale_down(&mut self) {
        let monitor = &mut self.monitors[self.selected];
        let scales = vec![1.0, 1.25, 1.5, 1.6666667, 2.0, 2.5, 3.0];
        let current_idx = scales.iter().position(|&s| (s - monitor.scale).abs() < 0.01).unwrap_or(0);
        if current_idx > 0 {
            monitor.scale = scales[current_idx - 1];
            self.changed = true;
            self.status_msg = format!("{} scale: {:.2}x", monitor.description, monitor.scale);
        }
    }

    fn recalculate_positions(&mut self) {
        let mut x = 0;
        for monitor in &mut self.monitors {
            if !monitor.disabled {
                monitor.x = x;
                let scaled_width = ((monitor.width as f32) / monitor.scale).ceil() as i32;
                x += scaled_width;
            }
        }
    }

    fn apply_changes(&mut self) {
        self.status_msg = "Applying changes...".to_string();

        for monitor in &self.monitors {
            if monitor.disabled {
                Command::new("hyprctl")
                    .args(["keyword", "monitor", &format!("{},disable", monitor.name)])
                    .output()
                    .ok();
            } else {
                let pos = format!("{}x{}", monitor.x, monitor.y);
                let scale_str = if monitor.scale == 1.0 { "1".to_string() } else if monitor.scale == 1.6666667 { "1.666667".to_string() } else { format!("{}", monitor.scale) };

                Command::new("hyprctl")
                    .args(["keyword", "monitor", &format!("{},preferred,{},{}", monitor.name, pos, scale_str)])
                    .output()
                    .ok();

                if !monitor.workspaces.is_empty() {
                    for ws in &monitor.workspaces {
                        Command::new("hyprctl")
                            .args(["dispatch", "moveworkspacetomonitor", &ws.to_string(), &monitor.name])
                            .output()
                            .ok();
                    }
                }
            }
        }

        Command::new("notify-send")
            .args(["Monitor Config", "Monitor configuration updated"])
            .output()
            .ok();

        self.status_msg = "Changes applied!".to_string();
        self.changed = false;
    }

    fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> io::Result<()> {
        loop {
            terminal.draw(|f| self.draw(f))?;

            let timeout = if self.show_confirm && self.countdown_start.is_some() {
                if let Some(start) = self.countdown_start {
                    let elapsed = start.elapsed();
                    let remaining = self.countdown_duration.saturating_sub(elapsed);
                    if remaining.is_zero() {
                        if let Some(prev) = self.prev_state.take() {
                            self.monitors = prev;
                            self.recalculate_positions();
                        }
                        self.show_confirm = false;
                        self.countdown_start = None;
                        self.changed = false;
                        self.status_msg = "Timeout - Changes reverted".to_string();
                        Some(Duration::from_millis(100))
                    } else {
                        Some(remaining)
                    }
                } else {
                    None
                }
            } else {
                None
            };

            if let Some(t) = timeout {
                if crossterm::event::poll(t)? {
                    if let Event::Key(key) = event::read()? {
                        if key.kind == KeyEventKind::Press {
                            if !self.handle_key(key) {
                                return Ok(());
                            }
                        }
                    }
                }
            } else if crossterm::event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        if !self.handle_key(key) {
                            return Ok(());
                        }
                    }
                }
            }
        }
    }

    fn draw(&self, f: &mut Frame) {
        let size = f.size();

        if size.width < 80 || size.height < 20 {
            f.render_widget(
                Paragraph::new("Terminal too small (min 80x20)").style(Style::default().fg(Color::Red)),
                Rect { x: 0, y: 0, width: size.width, height: size.height }
            );
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Min(10), Constraint::Length(3)].as_ref())
            .split(size);

        let monitor_items: Vec<ListItem> = self.monitors.iter().enumerate().map(|(i, m)| {
            let is_selected = i == self.selected;
            let status_style = if m.disabled {
                Style::default().fg(Color::DarkGray)
            } else if is_selected {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Cyan)
            };

            let mut lines = vec![
                Line::from(vec![
                    Span::styled(if i == 0 { "├─" } else if i == self.monitors.len() - 1 { "└─" } else { "├─" }, Style::default().fg(Color::Gray)),
                    Span::styled(m.description.chars().take(50).collect::<String>(), status_style),
                ])
            ];

            if !m.disabled {
                lines.extend(vec![
                    Line::from(vec![
                        Span::styled("  │    ", Style::default().fg(Color::Gray)),
                        Span::styled(format!("Res: {} | Scale: {:.2}x", m.resolution_string, m.scale), Style::default().fg(Color::Green)),
                    ]),
                    Line::from(vec![
                        Span::styled("  │    ", Style::default().fg(Color::Gray)),
                        Span::styled(format!("Pos: {}x{}", m.x, m.y), Style::default().fg(Color::Green)),
                        Span::styled(format!(" | Name: {}", m.name), Style::default().fg(Color::DarkGray)),
                    ]),
                    Line::from(vec![
                        Span::styled("  │    ", Style::default().fg(Color::Gray)),
                        Span::styled(if m.workspaces.is_empty() {
                            "Workspaces: None (press 0-9 to add)".to_string()
                        } else {
                            format!("Workspaces: {}", m.workspaces.iter().map(|w| w.to_string()).collect::<Vec<_>>().join(", "))
                        }, Style::default().fg(Color::Magenta)),
                    ]),
                ]);
            } else {
                lines.push(Line::from(vec![
                    Span::styled("  │    ", Style::default().fg(Color::Gray)),
                    Span::styled("Disabled (press 'e' to enable)", Style::default().fg(Color::Red)),
                ]));
            }

            ListItem::new(lines)
        }).collect();

        let mut list_state = ListState::default();
        list_state.select(Some(self.selected));

        let monitor_list = List::new(monitor_items)
            .block(Block::default().title("Monitors").borders(Borders::ALL))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        f.render_stateful_widget(monitor_list, chunks[0], &mut list_state);

        let mut status_lines = vec![Line::from(self.status_msg.clone())];

        if self.show_confirm {
            if let Some(start) = self.countdown_start {
                let elapsed = start.elapsed();
                let remaining = self.countdown_duration.saturating_sub(elapsed);
                let seconds_left = (remaining.as_secs_f32() + 0.5) as u32;
                status_lines.push(Line::from(vec![
                    Span::styled(format!("Confirm in {} seconds? [Y]es [N]o", seconds_left), Style::default().fg(Color::Yellow)),
                ]));
            }
        }

        let help = if self.show_confirm {
            "[Y] Keep [N] Cancel"
        } else if self.changed {
            "[↑↓] Nav [e/d] On/Off [K/J] Reorder [s] Scale [4-9] WS [</>] Move WS [a] Apply [q] Quit"
        } else {
            "[↑↓] Nav [e/d] On/Off [K/J] Reorder [s] Scale [4-9] WS [</>] Move WS [q] Quit"
        };
        status_lines.push(Line::from(vec![Span::styled(help, Style::default().fg(Color::Gray))]));

        let status = Paragraph::new(status_lines).alignment(Alignment::Center);

        f.render_widget(status, chunks[1]);
    }
}

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    let result = app.run(&mut terminal);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    result
}
