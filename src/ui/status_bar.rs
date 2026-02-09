use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use crate::app::{App, Overlay};

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let mut lines = Vec::new();

    let msg_color = if app.status_msg.contains("Error") || app.status_msg.contains("revert") {
        Color::Red
    } else if app.status_msg.contains("saved") || app.status_msg.contains("Saved") {
        Color::Green
    } else {
        Color::White
    };
    lines.push(Line::from(Span::styled(&app.status_msg, Style::default().fg(msg_color))));

    let help = match &app.overlay {
        Overlay::Confirm { .. } => "[Y/Space] Keep  [N/Esc] Revert",
        Overlay::Presets { saving: true, .. } => "Type name, [Enter] Save  [Esc] Cancel",
        Overlay::Presets { .. } => "[j/k] Nav  [Enter] Load  [s] Save  [d] Delete  [Esc] Close",
        Overlay::None => {
            if app.changed {
                "[Tab] Select  [hjkl] Move  [HJKL] Snap  [d/e] Dis/En  [s] Scale  [r] Res  [a] Apply  [p] Presets  [q] Quit"
            } else {
                "[Tab] Select  [hjkl] Move  [HJKL] Snap  [d/e] Dis/En  [s] Scale  [r] Res  [p] Presets  [q] Quit"
            }
        }
    };
    lines.push(Line::from(Span::styled(help, Style::default().fg(Color::DarkGray))));

    let para = Paragraph::new(lines).alignment(Alignment::Center);
    f.render_widget(para, area);
}
