use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use crate::app::App;

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .monitors
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let is_selected = i == app.selected;

            let name_style = if m.disabled {
                Style::default().fg(Color::DarkGray)
            } else if is_selected {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Cyan)
            };

            let marker = if is_selected { "▸ " } else { "  " };

            let mut lines = vec![Line::from(vec![
                Span::styled(marker, name_style),
                Span::styled(
                    m.description.chars().take(40).collect::<String>(),
                    name_style,
                ),
            ])];

            if m.disabled {
                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled("[DISABLED]", Style::default().fg(Color::Red)),
                    Span::styled(
                        format!("  {}", m.name),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
            } else {
                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled(m.resolution_string(), Style::default().fg(Color::Green)),
                    Span::styled(format!("  {:.2}x", m.scale), Style::default().fg(Color::Green)),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled(format!("Pos: {}x{}", m.x, m.y), Style::default().fg(Color::Blue)),
                    Span::styled(format!("  {}", m.name), Style::default().fg(Color::DarkGray)),
                ]));
                let ws_text = if m.workspaces.is_empty() {
                    "WS: -".to_string()
                } else {
                    format!(
                        "WS: {}",
                        m.workspaces.iter().map(|w| w.to_string()).collect::<Vec<_>>().join(", ")
                    )
                };
                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::styled(ws_text, Style::default().fg(Color::Magenta)),
                ]));
            }

            ListItem::new(lines)
        })
        .collect();

    let title = if app.changed { " Monitors * " } else { " Monitors " };

    let list = List::new(items)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut state = ListState::default();
    state.select(Some(app.selected));
    f.render_stateful_widget(list, area, &mut state);
}
