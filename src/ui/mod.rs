pub mod list_pane;
pub mod canvas_pane;
pub mod preset_menu;
pub mod status_bar;
pub mod confirm;
pub mod external_change;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    Frame,
};

use crate::app::{App, Overlay};

pub fn draw(f: &mut Frame, app: &mut App) {
    let size = f.size();

    if size.width < 60 || size.height < 15 {
        let msg = ratatui::widgets::Paragraph::new("Terminal too small (min 60x15)")
            .style(Style::default().fg(Color::Red));
        f.render_widget(msg, size);
        return;
    }

    // Main layout: content + status bar
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(10), Constraint::Length(3)])
        .split(size);

    // Split pane: list | canvas (or top/bottom if narrow)
    let (list_area, canvas_area) = if size.width >= 100 {
        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
            .split(outer[0]);
        (panes[0], panes[1])
    } else {
        let panes = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(outer[0]);
        (panes[0], panes[1])
    };

    app.list_area = list_area;
    app.canvas_area = canvas_area;
    list_pane::draw(f, app, list_area);
    canvas_pane::draw(f, app, canvas_area);
    status_bar::draw(f, app, outer[1]);

    // Overlays
    match &app.overlay {
        Overlay::Confirm { countdown_start, duration, .. } => {
            let elapsed = countdown_start.elapsed();
            let remaining = duration.saturating_sub(elapsed);
            confirm::draw(f, remaining, size);
        }
        Overlay::ExternalChange => {
            external_change::draw(f, size);
        }
        Overlay::Presets { selected, names, saving, input } => {
            preset_menu::draw(f, *selected, names, *saving, input, size);
        }
        Overlay::None => {}
    }
}

/// Create a centered popup rect of given percentage size.
pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
