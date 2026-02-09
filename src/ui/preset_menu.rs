use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use super::centered_rect;

pub fn draw(f: &mut Frame, selected: usize, names: &[String], saving: bool, input: &str, area: Rect) {
    let popup = centered_rect(50, 60, area);
    f.render_widget(Clear, popup);

    if saving {
        draw_save_dialog(f, input, popup);
    } else {
        draw_preset_list(f, selected, names, popup);
    }
}

fn draw_preset_list(f: &mut Frame, selected: usize, names: &[String], area: Rect) {
    let mut items = Vec::new();

    // "Most Recent Apply" entry
    items.push(ListItem::new(Line::from(vec![
        Span::styled("  ↻ ", Style::default().fg(Color::Blue)),
        Span::styled("Most Recent Apply", Style::default().fg(Color::Blue)),
    ])));

    // Saved presets
    for name in names {
        items.push(ListItem::new(Line::from(vec![
            Span::styled("  ● ", Style::default().fg(Color::Cyan)),
            Span::styled(name.clone(), Style::default().fg(Color::White)),
        ])));
    }

    if items.len() == 1 {
        items.push(ListItem::new(Line::from(Span::styled(
            "  No saved presets",
            Style::default().fg(Color::DarkGray),
        ))));
    }

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Presets ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Magenta)),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD | Modifier::REVERSED),
        );

    let mut state = ListState::default();
    state.select(Some(selected));
    f.render_stateful_widget(list, area, &mut state);
}

fn draw_save_dialog(f: &mut Frame, input: &str, area: Rect) {
    let inner = centered_rect(80, 30, area);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "Enter preset name:",
            Style::default().fg(Color::White),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("▸ {}_", input),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "[Enter] Save  [Esc] Cancel",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let para = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Save Preset ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green)),
        )
        .alignment(Alignment::Center);

    f.render_widget(para, inner);
}
