use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEventKind};
use ratatui::{backend::CrosstermBackend, layout::Rect, Terminal};
use std::io::Stdout;
use std::time::{Duration, Instant};

use crate::apply;
use crate::layout::{self, Direction, LayoutMonitor};
use crate::monitor::{self, MonitorInfo};
use crate::preset;

const SCALES: &[f32] = &[1.0, 1.2, 1.5, 2.0, 3.0];
const SLIDE_STEP: i32 = 50;
const CONFIRM_DURATION: Duration = Duration::from_secs(10);

struct DragState {
    monitor_idx: usize,
    offset_x: f64,
    offset_y: f64,
    orig_x: i32,
    orig_y: i32,
}

#[derive(Clone, Debug)]
pub enum Overlay {
    None,
    Confirm {
        countdown_start: Instant,
        duration: Duration,
        ready_for_input: bool,  // Prevents same keypress from confirming
    },
    Presets {
        selected: usize,
        names: Vec<String>,
        saving: bool,
        input: String,
    },
    ExternalChange,
}

pub struct App {
    pub monitors: Vec<MonitorInfo>,
    pub selected: usize,
    pub overlay: Overlay,
    pub status_msg: String,
    pub changed: bool,
    pub show_all_monitors: bool,
    initial_state: Vec<MonitorInfo>,
    prev_state: Option<Vec<MonitorInfo>>,
    pub list_area: Rect,
    pub canvas_area: Rect,
    drag: Option<DragState>,
    last_poll: Instant,
    external_state: Vec<MonitorInfo>,
    last_apply: Option<Instant>,  // Track when we last applied changes
}

impl App {
    pub fn new() -> Self {
        // Always fetch all monitors, we'll filter display based on show_all_monitors flag
        let mut monitors = monitor::fetch_monitors_all();

        // Restore workspace assignments from most recent save
        if let Some(recent) = preset::load_recent() {
            for config in &recent {
                if let Some(m) = monitors.iter_mut().find(|m| m.name == config.name) {
                    if !config.workspaces.is_empty() {
                        m.workspaces = config.workspaces.clone();
                    }
                }
            }
        }

        let initial_state = monitors.clone();
        let external_state = monitors.clone();
        App {
            monitors,
            selected: 0,
            overlay: Overlay::None,
            status_msg: "Welcome to monitui".to_string(),
            changed: false,
            show_all_monitors: false,
            initial_state,
            prev_state: None,
            list_area: Rect::default(),
            canvas_area: Rect::default(),
            drag: None,
            last_poll: Instant::now(),
            external_state,
            last_apply: None,
        }
    }

    pub fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> std::io::Result<()> {
        loop {
            terminal.draw(|f| crate::ui::draw(f, self))?;

            // Poll for external configuration changes every 3 seconds
            // Continue polling during ExternalChange to get latest state
            // But NEVER interrupt Confirm countdown or Presets menu
            // Also enforce grace period after apply/confirm/revert (5 seconds for Hyprland to stabilize)
            let in_grace_period = self.last_apply
                .map(|t| t.elapsed() < Duration::from_secs(5))
                .unwrap_or(false);

            let should_poll = self.last_poll.elapsed() >= Duration::from_secs(3)
                && !matches!(self.overlay, Overlay::Confirm { .. } | Overlay::Presets { .. })
                && !in_grace_period;

            if should_poll {
                self.last_poll = Instant::now();
                self.check_external_changes();
            }

            if let Overlay::Confirm { countdown_start, duration, ready_for_input } = &self.overlay {
                let remaining = duration.saturating_sub(countdown_start.elapsed());
                let elapsed = countdown_start.elapsed();

                // Make ready for input after 200ms to avoid same keypress
                if !ready_for_input && elapsed >= Duration::from_millis(200) {
                    self.overlay = Overlay::Confirm {
                        countdown_start: *countdown_start,
                        duration: *duration,
                        ready_for_input: true,
                    };
                }

                if remaining.is_zero() {
                    self.revert_changes();
                    self.status_msg = "Timeout — changes reverted".to_string();
                    continue;
                }
            }

            let poll_timeout = match &self.overlay {
                Overlay::Confirm { .. } => Duration::from_millis(100),
                _ => Duration::from_millis(200),
            };

            if crossterm::event::poll(poll_timeout)? {
                match event::read()? {
                    Event::Key(key) => {
                        if key.kind == KeyEventKind::Press && !self.handle_key(key) {
                            return Ok(());
                        }
                    }
                    Event::Mouse(mouse) => {
                        match mouse.kind {
                            MouseEventKind::Down(MouseButton::Left) => {
                                self.handle_mouse_down(mouse.column, mouse.row);
                            }
                            MouseEventKind::Drag(MouseButton::Left) => {
                                self.handle_mouse_drag(mouse.column, mouse.row);
                            }
                            MouseEventKind::Up(MouseButton::Left) => {
                                self.handle_mouse_up();
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        match &self.overlay {
            Overlay::Confirm { .. } => return self.handle_confirm_key(key),
            Overlay::ExternalChange => {
                return self.handle_external_change_key(key);
            }
            Overlay::Presets { saving, .. } => {
                if *saving {
                    self.handle_save_key(key);
                } else {
                    self.handle_preset_key(key);
                }
                return true;
            }
            Overlay::None => {}
        }

        let shift = key.modifiers.contains(KeyModifiers::SHIFT);

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return false,

            // Tab cycles monitor selection (only through visible monitors)
            KeyCode::Tab => {
                let visible = self.visible_monitors();
                if !visible.is_empty() {
                    let current_pos = visible.iter().position(|&i| i == self.selected);
                    let next_pos = match current_pos {
                        Some(pos) => (pos + 1) % visible.len(),
                        None => 0,
                    };
                    self.selected = visible[next_pos];
                }
            }
            KeyCode::BackTab => {
                let visible = self.visible_monitors();
                if !visible.is_empty() {
                    let current_pos = visible.iter().position(|&i| i == self.selected);
                    let next_pos = match current_pos {
                        Some(pos) => if pos == 0 { visible.len() - 1 } else { pos - 1 },
                        None => visible.len() - 1,
                    };
                    self.selected = visible[next_pos];
                }
            }

            // hjkl / arrows: move monitors (shift = snap to far side)
            KeyCode::Char('h') | KeyCode::Left if !shift => {
                self.canvas_move(Direction::Left, false);
            }
            KeyCode::Char('j') | KeyCode::Down if !shift => {
                self.canvas_move(Direction::Down, false);
            }
            KeyCode::Char('k') | KeyCode::Up if !shift => {
                self.canvas_move(Direction::Up, false);
            }
            KeyCode::Char('l') | KeyCode::Right if !shift => {
                self.canvas_move(Direction::Right, false);
            }
            KeyCode::Char('H') | KeyCode::Left if shift => self.canvas_move(Direction::Left, true),
            KeyCode::Char('J') | KeyCode::Down if shift => self.canvas_move(Direction::Down, true),
            KeyCode::Char('K') | KeyCode::Up if shift => self.canvas_move(Direction::Up, true),
            KeyCode::Char('L') | KeyCode::Right if shift => self.canvas_move(Direction::Right, true),

            KeyCode::Char('p') => self.open_presets(),
            KeyCode::Char('y') | KeyCode::Char(' ') | KeyCode::Enter => self.apply(),

            // Monitor config keys
            KeyCode::Char('d') => {
                if !self.monitors[self.selected].disabled {
                    self.monitors[self.selected].disabled = true;
                    self.changed = true;
                    self.status_msg = format!("Disabled {}", self.monitors[self.selected].name);
                }
            }
            KeyCode::Char('e') => {
                if self.monitors[self.selected].disabled {
                    self.monitors[self.selected].disabled = false;
                    self.changed = true;
                    self.apply_layout_adjustments();  // Auto-snap to avoid overlaps
                    self.status_msg = format!("Enabled {}", self.monitors[self.selected].name);
                }
            }
            KeyCode::Char('s') => self.cycle_scale(),
            KeyCode::Char('+') | KeyCode::Char('=') => self.scale_up(),
            KeyCode::Char('-') => self.scale_down(),
            KeyCode::Char('z') => {
                self.monitors[self.selected].cycle_resolution();
                self.changed = true;
                self.apply_layout_adjustments();
                self.status_msg = format!(
                    "{}: {}",
                    self.monitors[self.selected].name,
                    self.monitors[self.selected].resolution_string()
                );
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                self.monitors[self.selected].cycle_rotation();
                self.changed = true;
                self.apply_layout_adjustments();
                self.status_msg = format!(
                    "{}: rotation {}",
                    self.monitors[self.selected].name,
                    self.monitors[self.selected].rotation_string()
                );
            }
            KeyCode::Char('t') => self.toggle_show_all(),
            KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
                let ws = c as u32 - '0' as u32;
                for (i, m) in self.monitors.iter_mut().enumerate() {
                    if i != self.selected {
                        m.workspaces.retain(|&w| w != ws);
                    }
                }
                let m = &mut self.monitors[self.selected];
                if !m.workspaces.contains(&ws) {
                    m.workspaces.push(ws);
                    m.workspaces.sort();
                    self.changed = true;
                    self.status_msg = format!("Assigned WS {} to {}", ws, m.name);
                }
            }
            KeyCode::Char('W') => {
                self.monitors[self.selected].workspaces.clear();
                self.changed = true;
                self.status_msg = format!("Cleared workspaces from {}", self.monitors[self.selected].name);
            }
            _ => {}
        }
        true
    }

    fn canvas_move(&mut self, dir: Direction, snap: bool) {
        let mut layout_monitors = self.build_layout_monitors();
        if layout_monitors.is_empty() { return; }

        let enabled_idx = self.monitors.iter()
            .take(self.selected + 1)
            .filter(|m| !m.disabled)
            .count()
            .saturating_sub(1);

        if enabled_idx >= layout_monitors.len() { return; }

        let orig_x = layout_monitors[enabled_idx].x;
        let orig_y = layout_monitors[enabled_idx].y;

        if snap {
            layout::snap_to_far_side(&mut layout_monitors, enabled_idx, dir);
        } else {
            layout::move_monitor(&mut layout_monitors, enabled_idx, dir, SLIDE_STEP);
        }

        layout::auto_snap_all(&mut layout_monitors);
        layout::resolve_overlaps(&mut layout_monitors, enabled_idx, orig_x, orig_y);
        layout::normalize(&mut layout_monitors);
        self.apply_layout_to_monitors(&layout_monitors);
        self.changed = true;
        self.status_msg = "Layout updated".to_string();
    }

    fn build_layout_monitors(&self) -> Vec<LayoutMonitor> {
        self.monitors.iter()
            .filter(|m| !m.disabled)
            .map(|m| LayoutMonitor {
                id: m.name.clone(),
                x: m.x,
                y: m.y,
                w: m.logical_width(),
                h: m.logical_height(),
            })
            .collect()
    }

    fn apply_layout_to_monitors(&mut self, layout: &[LayoutMonitor]) {
        for lm in layout {
            if let Some(m) = self.monitors.iter_mut().find(|m| m.name == lm.id) {
                m.x = lm.x;
                m.y = lm.y;
            }
        }
    }

    fn apply_layout_adjustments(&mut self) {
        let mut layout_monitors = self.build_layout_monitors();
        if layout_monitors.is_empty() { return; }

        let enabled_idx = self.monitors.iter()
            .take(self.selected + 1)
            .filter(|m| !m.disabled)
            .count()
            .saturating_sub(1);

        if enabled_idx >= layout_monitors.len() { return; }

        let orig_x = layout_monitors[enabled_idx].x;
        let orig_y = layout_monitors[enabled_idx].y;

        layout::auto_snap_all(&mut layout_monitors);
        layout::resolve_overlaps(&mut layout_monitors, enabled_idx, orig_x, orig_y);
        layout::normalize(&mut layout_monitors);
        self.apply_layout_to_monitors(&layout_monitors);
    }

    fn apply_layout_snap_all(&mut self) {
        let mut layout_monitors = self.build_layout_monitors();
        if layout_monitors.is_empty() { return; }

        layout::auto_snap_all(&mut layout_monitors);
        layout::normalize(&mut layout_monitors);
        self.apply_layout_to_monitors(&layout_monitors);
    }

    // --- Mouse ---

    fn terminal_to_monitor_coords(&self, col: u16, row: u16) -> Option<(f64, f64)> {
        if col < self.canvas_area.x || col >= self.canvas_area.x + self.canvas_area.width
            || row < self.canvas_area.y || row >= self.canvas_area.y + self.canvas_area.height
        {
            return None;
        }

        let enabled: Vec<_> = self.monitors.iter().filter(|m| !m.disabled).collect();
        if enabled.is_empty() { return None; }

        let min_x = enabled.iter().map(|m| m.x).min().unwrap_or(0);
        let max_x = enabled.iter().map(|m| m.x + m.logical_width()).max().unwrap_or(1920);
        let min_y = enabled.iter().map(|m| m.y).min().unwrap_or(0);
        let max_y = enabled.iter().map(|m| m.y + m.logical_height()).max().unwrap_or(1080);

        let content_w = (max_x - min_x) as f64;
        let content_h = (max_y - min_y) as f64;
        if content_w <= 0.0 || content_h <= 0.0 { return None; }

        let inner_w = self.canvas_area.width.saturating_sub(2) as f64;
        let inner_h = self.canvas_area.height.saturating_sub(2) as f64;
        let click_x = (col - self.canvas_area.x).saturating_sub(1) as f64;
        let click_y = (row - self.canvas_area.y).saturating_sub(1) as f64;

        let char_aspect = 2.0;
        let eff_w = inner_w;
        let eff_h = inner_h * char_aspect;
        let scale_x = eff_w / content_w;
        let scale_y = eff_h / content_h;
        let scale = scale_x.min(scale_y);
        let scaled_w = content_w * scale;
        let scaled_h = content_h * scale;
        let pad_x = (eff_w - scaled_w) / 2.0;
        let pad_y = (eff_h - scaled_h) / 2.0;

        let mon_x = min_x as f64 + (click_x - pad_x) / scale;
        let mon_y = min_y as f64 + (click_y * char_aspect - pad_y) / scale;
        Some((mon_x, mon_y))
    }

    fn handle_mouse_down(&mut self, col: u16, row: u16) {
        if matches!(self.overlay, Overlay::Confirm { .. } | Overlay::Presets { .. }) {
            return;
        }

        // Check list pane click
        if col >= self.list_area.x && col < self.list_area.x + self.list_area.width
            && row >= self.list_area.y && row < self.list_area.y + self.list_area.height
        {
            let content_y = row.saturating_sub(self.list_area.y + 1);
            let mut y_offset = 0u16;
            for i in self.visible_monitors() {
                let m = &self.monitors[i];
                let item_height: u16 = if m.disabled { 2 } else { 4 };
                if content_y >= y_offset && content_y < y_offset + item_height {
                    self.selected = i;
                    return;
                }
                y_offset += item_height;
            }
            return;
        }

        // Check canvas pane click — start drag if monitor hit
        if let Some((mon_x, mon_y)) = self.terminal_to_monitor_coords(col, row) {
            let enabled: Vec<_> = self.monitors.iter().enumerate()
                .filter(|(_, m)| !m.disabled)
                .collect();

            for &(i, ref m) in &enabled {
                let mx = m.x as f64;
                let my = m.y as f64;
                let mw = m.logical_width() as f64;
                let mh = m.logical_height() as f64;
                if mon_x >= mx && mon_x < mx + mw && mon_y >= my && mon_y < my + mh {
                    self.selected = i;
                    self.drag = Some(DragState {
                        monitor_idx: i,
                        offset_x: mon_x - mx,
                        offset_y: mon_y - my,
                        orig_x: m.x,
                        orig_y: m.y,
                    });
                    return;
                }
            }
        }
    }

    fn handle_mouse_drag(&mut self, col: u16, row: u16) {
        let drag = match &self.drag {
            Some(d) => d,
            None => return,
        };
        let idx = drag.monitor_idx;
        let off_x = drag.offset_x;
        let off_y = drag.offset_y;

        if let Some((mon_x, mon_y)) = self.terminal_to_monitor_coords(col, row) {
            let new_x = (mon_x - off_x).round() as i32;
            let new_y = (mon_y - off_y).round() as i32;
            self.monitors[idx].x = new_x;
            self.monitors[idx].y = new_y;
            self.changed = true;
        }
    }

    fn handle_mouse_up(&mut self) {
        if let Some(drag) = self.drag.take() {
            let enabled_idx = self.monitors.iter()
                .take(drag.monitor_idx + 1)
                .filter(|m| !m.disabled)
                .count()
                .saturating_sub(1);

            let mut layout_monitors = self.build_layout_monitors();
            if enabled_idx < layout_monitors.len() {
                layout::auto_snap_all(&mut layout_monitors);
                layout::resolve_overlaps(&mut layout_monitors, enabled_idx, drag.orig_x, drag.orig_y);
                layout::normalize(&mut layout_monitors);
                self.apply_layout_to_monitors(&layout_monitors);
            }
            self.changed = true;
            self.status_msg = "Layout updated".to_string();
        }
    }

    // --- Confirm ---

    fn handle_confirm_key(&mut self, key: KeyEvent) -> bool {
        // Check if overlay is ready for input
        let ready = if let Overlay::Confirm { ready_for_input, .. } = &self.overlay {
            *ready_for_input
        } else {
            false
        };

        if !ready {
            return true;
        }

        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Char(' ') | KeyCode::Enter => {
                self.overlay = Overlay::None;
                // Confirmed — update the initial state to this new config
                self.initial_state = self.monitors.clone();
                self.external_state = self.monitors.clone();
                self.prev_state = None;
                self.last_apply = Some(Instant::now());  // Extend grace period
                preset::save_recent(&self.monitors);
                self.status_msg = "Configuration saved!".to_string();
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.revert_changes();
                self.status_msg = "Changes reverted".to_string();
            }
            _ => {}
        }
        true
    }

    fn revert_changes(&mut self) {
        // Revert to the state before apply (prev_state), or initial state as fallback
        let revert_to = self.prev_state.take()
            .unwrap_or_else(|| self.initial_state.clone());
        self.monitors = revert_to;
        match apply::apply_monitors(&self.monitors) {
            Ok(()) => {
                // Update external state to reflect the revert, so we don't trigger false external change detection
                self.external_state = self.monitors.clone();
                self.last_apply = Some(Instant::now());  // Extend grace period after revert
            }
            Err(e) => {
                self.status_msg = format!("Error reverting: {}", e);
            }
        }
        self.overlay = Overlay::None;
        self.changed = false;
    }

    // --- Presets ---

    fn open_presets(&mut self) {
        let names = preset::list_presets();
        self.overlay = Overlay::Presets {
            selected: 0,
            names,
            saving: false,
            input: String::new(),
        };
    }

    fn handle_preset_key(&mut self, key: KeyEvent) {
        if let Overlay::Presets { selected, names, .. } = &mut self.overlay {
            let total = 1 + names.len();
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    if *selected < total.saturating_sub(1) {
                        *selected += 1;
                    }
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    if *selected > 0 {
                        *selected -= 1;
                    }
                }
                KeyCode::Char('y') | KeyCode::Char(' ') | KeyCode::Enter => {
                    let sel = *selected;
                    let names_clone = names.clone();
                    self.load_preset_entry(sel, &names_clone);
                }
                KeyCode::Char('s') => {
                    if let Overlay::Presets { saving, input, .. } = &mut self.overlay {
                        *saving = true;
                        *input = String::new();
                    }
                }
                KeyCode::Char('d') => {
                    let sel = *selected;
                    if sel > 0 && sel <= names.len() {
                        let name = names[sel - 1].clone();
                        preset::delete_preset(&name).ok();
                        self.status_msg = format!("Deleted preset: {}", name);
                        self.open_presets();
                    }
                }
                KeyCode::Esc => {
                    self.overlay = Overlay::None;
                }
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    // 0 = Most Recent (index 0), 1-9 = presets (indices 1-9)
                    let idx = (c as u32 - '0' as u32) as usize;
                    if idx < total {
                        let names_clone = names.clone();
                        self.load_preset_entry(idx, &names_clone);
                    }
                }
                _ => {}
            }
        }
    }

    fn handle_save_key(&mut self, key: KeyEvent) {
        if let Overlay::Presets { input, .. } = &mut self.overlay {
            match key.code {
                KeyCode::Char(c) => {
                    input.push(c);
                }
                KeyCode::Backspace => {
                    input.pop();
                }
                KeyCode::Enter => {
                    if !input.is_empty() {
                        let name = input.clone();
                        match preset::save_preset(&name, &self.monitors) {
                            Ok(()) => self.status_msg = format!("Saved preset: {}", name),
                            Err(e) => self.status_msg = format!("Error saving: {}", e),
                        }
                        self.overlay = Overlay::None;
                    }
                }
                KeyCode::Esc => {
                    if let Overlay::Presets { saving, .. } = &mut self.overlay {
                        *saving = false;
                    }
                }
                _ => {}
            }
        }
    }

    fn load_preset_entry(&mut self, idx: usize, names: &[String]) {
        if idx == 0 {
            if let Some(configs) = preset::load_recent() {
                preset::apply_preset_to_monitors(&mut self.monitors, &configs);
                self.apply_layout_snap_all();  // Auto-snap after loading preset
                self.changed = true;
                self.overlay = Overlay::None;
                self.apply();  // Auto-apply preset
            } else {
                self.status_msg = "No recent configuration found".to_string();
                self.overlay = Overlay::None;
            }
        } else if idx <= names.len() {
            let name = &names[idx - 1];
            match preset::load_preset(name) {
                Ok(p) => {
                    preset::apply_preset_to_monitors(&mut self.monitors, &p.monitors);
                    self.apply_layout_snap_all();  // Auto-snap after loading preset
                    self.changed = true;
                    self.overlay = Overlay::None;
                    self.apply();  // Auto-apply preset
                }
                Err(e) => {
                    self.status_msg = format!("Error loading preset: {}", e);
                    self.overlay = Overlay::None;
                }
            }
        } else {
            self.overlay = Overlay::None;
        }
    }

    // --- Apply ---

    fn apply(&mut self) {
        if !self.changed {
            self.status_msg = "No changes to apply".to_string();
            return;
        }
        self.prev_state = Some(self.initial_state.clone());
        match apply::apply_monitors(&self.monitors) {
            Ok(()) => {
                // Update external state to reflect our changes, so we don't trigger false external change detection
                self.external_state = self.monitors.clone();
                self.last_apply = Some(Instant::now());  // Start grace period
                self.overlay = Overlay::Confirm {
                    countdown_start: Instant::now(),
                    duration: CONFIRM_DURATION,
                    ready_for_input: false,  // Will become true after a brief delay
                };
                self.status_msg = "Applied — confirm to keep".to_string();
                self.changed = false;
            }
            Err(e) => {
                self.status_msg = format!("Error applying: {}", e);
                self.prev_state = None;
            }
        }
    }

    // --- Scale ---

    fn cycle_scale(&mut self) {
        let m = &mut self.monitors[self.selected];
        if m.disabled { return; }
        let idx = SCALES.iter().position(|&s| (s - m.scale).abs() < 0.01).unwrap_or(0);
        let next = (idx + 1) % SCALES.len();
        m.scale = SCALES[next];
        self.changed = true;
        self.status_msg = format!("{}: scale {:.2}x", m.name, m.scale);
    }

    fn scale_up(&mut self) {
        let m = &mut self.monitors[self.selected];
        if m.disabled { return; }
        let idx = SCALES.iter().position(|&s| (s - m.scale).abs() < 0.01).unwrap_or(0);
        if idx < SCALES.len() - 1 {
            m.scale = SCALES[idx + 1];
            self.changed = true;
            self.status_msg = format!("{}: scale {:.2}x", m.name, m.scale);
        }
    }

    fn scale_down(&mut self) {
        let m = &mut self.monitors[self.selected];
        if m.disabled { return; }
        let idx = SCALES.iter().position(|&s| (s - m.scale).abs() < 0.01).unwrap_or(0);
        if idx > 0 {
            m.scale = SCALES[idx - 1];
            self.changed = true;
            self.status_msg = format!("{}: scale {:.2}x", m.name, m.scale);
        }
    }

    fn toggle_show_all(&mut self) {
        self.show_all_monitors = !self.show_all_monitors;

        // Just toggle the visibility flag - don't reload to preserve edits
        // Ensure selection is valid for visible monitors
        let visible_monitors = self.visible_monitors();
        if visible_monitors.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.monitors.len() {
            self.selected = 0;
        } else if !self.is_monitor_visible(self.selected) {
            // Selected monitor is now hidden, select first visible
            self.selected = visible_monitors[0];
        }

        self.status_msg = if self.show_all_monitors {
            "Showing all monitors (including HEADLESS)".to_string()
        } else {
            "Showing active monitors only".to_string()
        };
    }

    /// Returns indices of visible monitors based on show_all_monitors flag
    fn visible_monitors(&self) -> Vec<usize> {
        self.monitors
            .iter()
            .enumerate()
            .filter(|(_, m)| self.is_monitor_visible_by_ref(m))
            .map(|(i, _)| i)
            .collect()
    }

    fn is_monitor_visible(&self, index: usize) -> bool {
        if index >= self.monitors.len() {
            return false;
        }
        self.is_monitor_visible_by_ref(&self.monitors[index])
    }

    fn is_monitor_visible_by_ref(&self, monitor: &MonitorInfo) -> bool {
        if self.show_all_monitors {
            true
        } else {
            // Hide HEADLESS monitors unless show_all is enabled
            !monitor.name.starts_with("HEADLESS-")
        }
    }

    // --- External Change Detection ---

    fn check_external_changes(&mut self) {
        // Always fetch all monitors to match our internal storage
        let current_external = monitor::fetch_monitors_all();

        // Compare with last known external state
        if !monitors_equal(&self.external_state, &current_external) {
            // If already showing ExternalChange overlay, just update silently to latest state
            // This ensures user acts on the most recent change, not stale data
            if matches!(self.overlay, Overlay::ExternalChange) {
                self.external_state = current_external;
            } else {
                // New external change detected, show overlay
                self.external_state = current_external;
                self.overlay = Overlay::ExternalChange;
                self.status_msg = "External monitor configuration change detected!".to_string();
            }
        }
    }

    fn handle_external_change_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Char('o') | KeyCode::Char('O') => {
                // Override - keep current edits, ignore external change
                // Mark as changed so user can re-apply their configuration
                self.changed = true;
                self.overlay = Overlay::None;
                self.status_msg = "Keeping your current configuration (override) - press 'y' to reapply".to_string();
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                // Pull - reload from external state
                self.monitors = self.external_state.clone();
                self.initial_state = self.external_state.clone();
                self.changed = false;
                self.overlay = Overlay::None;
                self.selected = self.selected.min(self.monitors.len().saturating_sub(1));
                self.status_msg = "Pulled latest configuration from system".to_string();
            }
            KeyCode::Char('q') | KeyCode::Esc => {
                // Quit application
                return false;
            }
            _ => {}
        }
        true
    }
}

/// Compare two monitor lists for equality (ignores workspaces which change frequently)
/// Matches monitors by NAME, not by array position (Hyprland can reorder them)
fn monitors_equal(a: &[MonitorInfo], b: &[MonitorInfo]) -> bool {
    use std::collections::HashMap;

    // Build maps of monitor name -> monitor info
    let map_a: HashMap<_, _> = a.iter().map(|m| (&m.name, m)).collect();
    let map_b: HashMap<_, _> = b.iter().map(|m| (&m.name, m)).collect();

    // Check for added monitors (monitors in b that aren't in a)
    for name in map_b.keys() {
        if !map_a.contains_key(name) {
            return false;
        }
    }

    // Check for removed monitors, BUT ignore monitors we disabled
    // (Hyprland may stop reporting disabled monitors)
    for (name, monitor_a) in &map_a {
        if !map_b.contains_key(name) {
            if !monitor_a.disabled {
                // Only care if an enabled monitor disappeared
                return false;
            }
            // Disabled monitor not in new list is fine - Hyprland may not report it
            continue;
        }
    }

    // Allow uniform x/y translation differences between snapshots.
    // Hyprland can preserve absolute coordinates after unplug/plug events,
    // which may shift the whole layout while keeping relative placement intact.
    let mut offset: Option<(i32, i32)> = None;

    for (name, m1) in &map_a {
        let Some(m2) = map_b.get(name) else {
            continue;
        };
        if m1.disabled || m2.disabled {
            continue;
        }
        offset = Some((m1.x - m2.x, m1.y - m2.y));
        break;
    }

    // Compare each monitor by name
    for (name, m1) in &map_a {
        let m2 = map_b.get(name).unwrap();  // Safe because we checked above

        if m1.width != m2.width {
            return false;
        }
        if m1.height != m2.height {
            return false;
        }
        if !m1.disabled && !m2.disabled {
            if let Some((dx, dy)) = offset {
                if m1.x - m2.x != dx || m1.y - m2.y != dy {
                    return false;
                }
            } else if m1.x != m2.x || m1.y != m2.y {
                return false;
            }
        } else if m1.x != m2.x || m1.y != m2.y {
            // Keep strict position checks for disabled monitors.
            return false;
        }
        if m1.scale != m2.scale {
            return false;
        }
        if m1.disabled != m2.disabled {
            return false;
        }
        if m1.transform != m2.transform {
            return false;
        }
    }

    true
}
