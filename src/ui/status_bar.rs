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

    match &app.overlay {
        Overlay::Confirm { .. } => {
            lines.push(Line::from(Span::styled("[Y/Space] Keep  [N] Revert  [Esc] Revert", Style::default().fg(Color::DarkGray))));
        }
        Overlay::ExternalChange => {
            lines.push(Line::from(Span::styled("[O] Override (keep edits)  [P] Pull (reload from system)  [Q] Quit", Style::default().fg(Color::DarkGray))));
        }
        Overlay::Presets { saving: true, .. } => {
            lines.push(Line::from(Span::styled("Type name, [Enter] Save  [Esc] Cancel", Style::default().fg(Color::DarkGray))));
        }
        Overlay::Presets { .. } => {
            lines.push(Line::from(Span::styled("[j/k] Nav  [Enter] Load  [s] Save  [d] Delete  [Esc] Close", Style::default().fg(Color::DarkGray))));
        }
        Overlay::None => {
            lines.push(Line::from(Span::styled(
                "[Tab] Select  [hjkl] Move  [HJKL] Snap  [d/e] Dis/En  [s] Scale  [z] Res  [r] Rotate  [1-9] WS",
                Style::default().fg(Color::DarkGray)
            )));
            if app.changed {
                lines.push(Line::from(Span::styled(
                    "[t] Toggle All  [y] Apply  [p] Presets  [q] Quit",
                    Style::default().fg(Color::DarkGray)
                )));
            } else {
                lines.push(Line::from(Span::styled(
                    "[t] Toggle All  [p] Presets  [q] Quit",
                    Style::default().fg(Color::DarkGray)
                )));
            }
        }
    };

    let para = Paragraph::new(lines).alignment(Alignment::Center);
    f.render_widget(para, area);
}
