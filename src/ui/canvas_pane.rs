use ratatui::{
    layout::Rect,
    style::{Color, Style},
    symbols::Marker,
    widgets::{
        canvas::{Canvas, Rectangle},
        Block, Borders,
    },
    Frame,
};

use crate::app::App;

pub fn draw(f: &mut Frame, app: &App, area: Rect) {
    let enabled: Vec<_> = app.monitors.iter().enumerate()
        .filter(|(_, m)| !m.disabled)
        .collect();

    if enabled.is_empty() {
        let block = Block::default()
            .title(" Layout ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));
        let msg = ratatui::widgets::Paragraph::new("No enabled monitors")
            .block(block)
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(msg, area);
        return;
    }

    let min_x = enabled.iter().map(|(_, m)| m.x).min().unwrap_or(0);
    let max_x = enabled.iter().map(|(_, m)| m.x + m.logical_width()).max().unwrap_or(1920);
    let min_y = enabled.iter().map(|(_, m)| m.y).min().unwrap_or(0);
    let max_y = enabled.iter().map(|(_, m)| m.y + m.logical_height()).max().unwrap_or(1080);

    let content_w = (max_x - min_x) as f64;
    let content_h = (max_y - min_y) as f64;

    if content_w <= 0.0 || content_h <= 0.0 { return; }

    // Available drawing area (inside borders, with padding)
    // Canvas uses ~2 braille dots per character width, ~4 per character height
    let canvas_chars_w = (area.width.saturating_sub(2)) as f64;
    let canvas_chars_h = (area.height.saturating_sub(2)) as f64;

    // Aspect ratio fitting: scale content to fit canvas while preserving proportions.
    // Braille cells are roughly 2:1 aspect (each cell is ~2px wide x 4px tall in dots,
    // but characters are taller than wide), so we compensate.
    let char_aspect = 2.0; // approximate width:height ratio of terminal characters
    let effective_canvas_w = canvas_chars_w;
    let effective_canvas_h = canvas_chars_h * char_aspect;

    let scale_x = effective_canvas_w / content_w;
    let scale_y = effective_canvas_h / content_h;
    let scale = scale_x.min(scale_y);

    // Scaled content dimensions in "character units"
    let scaled_w = content_w * scale;
    let scaled_h = content_h * scale;

    // Center the content: compute padding
    let pad_x = (effective_canvas_w - scaled_w) / 2.0;
    let pad_y = (effective_canvas_h - scaled_h) / 2.0;

    // Canvas coordinate space: we set bounds such that monitors map proportionally.
    // We work in a uniform coordinate system and offset by padding.
    let x_lo = min_x as f64 - pad_x / scale;
    let x_hi = max_x as f64 + pad_x / scale;
    let y_lo = min_y as f64 - pad_y / scale;
    let y_hi = max_y as f64 + pad_y / scale;

    let selected = app.selected;

    let canvas = Canvas::default()
        .block(
            Block::default()
                .title(" Layout ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .marker(Marker::Braille)
        .x_bounds([x_lo, x_hi])
        .y_bounds([y_lo, y_hi])
        .paint(move |ctx| {
            for &(i, ref m) in &enabled {
                let lw = m.logical_width() as f64;
                let lh = m.logical_height() as f64;

                let color = if i == selected {
                    Color::Yellow
                } else {
                    Color::Cyan
                };

                // Flip y: canvas y increases upward, we want it downward
                let flipped_y = (y_hi + y_lo) - m.y as f64 - lh;

                ctx.draw(&Rectangle {
                    x: m.x as f64,
                    y: flipped_y,
                    width: lw,
                    height: lh,
                    color,
                });

                let cx = m.x as f64 + lw / 2.0;
                let cy = flipped_y + lh / 2.0;
                ctx.print(cx, cy + lh * 0.12, ratatui::text::Line::from(
                    ratatui::text::Span::styled(m.name.clone(), Style::default().fg(color))
                ));
                ctx.print(cx, cy - lh * 0.12, ratatui::text::Line::from(
                    ratatui::text::Span::styled(
                        format!("{}x{}", m.width, m.height),
                        Style::default().fg(Color::DarkGray),
                    )
                ));
            }
        });

    f.render_widget(canvas, area);
}
