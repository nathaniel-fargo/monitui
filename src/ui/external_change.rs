use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub fn draw(f: &mut Frame, area: Rect) {
    let popup = centered_rect_with_min_size(60, 16, area);
    f.render_widget(Clear, popup);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "⚠ External Configuration Change Detected",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "The monitor configuration has changed externally",
            Style::default().fg(Color::White),
        )),
        Line::from(Span::styled(
            "(e.g., monitor unplugged, hyprctl command run)",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "What would you like to do?",
            Style::default().fg(Color::White),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "[O] Override - Keep your current edits",
            Style::default().fg(Color::Cyan),
        )),
        Line::from(Span::styled(
            "[P] Pull - Reload from system configuration",
            Style::default().fg(Color::Green),
        )),
        Line::from(Span::styled(
            "[Q/Esc] Quit application",
            Style::default().fg(Color::Red),
        )),
    ];

    let para = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Configuration Change ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .alignment(Alignment::Center);

    f.render_widget(para, popup);
}

/// Create a centered popup rect with minimum dimensions
fn centered_rect_with_min_size(min_width: u16, min_height: u16, area: Rect) -> Rect {
    let width = min_width.max((area.width * 60) / 100);
    let height = min_height.max((area.height * 30) / 100);

    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((area.height.saturating_sub(height)) / 2),
            Constraint::Length(height),
            Constraint::Length((area.height.saturating_sub(height)) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((area.width.saturating_sub(width)) / 2),
            Constraint::Length(width),
            Constraint::Length((area.width.saturating_sub(width)) / 2),
        ])
        .split(popup_layout[1])[1]
}
