use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use std::time::Duration;

use super::centered_rect;

pub fn draw(f: &mut Frame, remaining: Duration, area: Rect) {
    let popup = centered_rect(40, 20, area);
    f.render_widget(Clear, popup);

    let secs = remaining.as_secs();
    let bar_width = 20u16;
    let filled = ((secs as f64 / 10.0) * bar_width as f64).ceil() as usize;
    let empty = bar_width as usize - filled;
    let bar = format!("[{}{}]", "█".repeat(filled), "░".repeat(empty));

    let color = if secs <= 3 { Color::Red } else { Color::Yellow };

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Keep this configuration?",
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("Reverting in {}s", secs),
            Style::default().fg(color),
        )),
        Line::from(Span::styled(bar, Style::default().fg(color))),
        Line::from(""),
        Line::from(Span::styled(
            "[Y / Space] Keep   [N / Esc] Revert",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let para = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Confirm ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(color)),
        )
        .alignment(Alignment::Center);

    f.render_widget(para, popup);
}
