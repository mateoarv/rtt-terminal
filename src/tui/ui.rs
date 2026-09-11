use ratatui::layout::{Constraint, Direction, Layout, Position};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};
use ratatui::Frame;

use super::ansi::parse_ansi_to_line;
use super::app::AppState;

pub fn render(frame: &mut Frame, state: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(5),    // Log viewport
            Constraint::Length(1), // Quick command shortcuts
            Constraint::Length(3), // Input prompt
        ])
        .split(frame.area());

    render_header(frame, state, chunks[0]);
    render_logs(frame, state, chunks[1]);
    render_hints(frame, chunks[2]);
    render_input(frame, state, chunks[3]);
}

fn render_header(frame: &mut Frame, state: &AppState, area: ratatui::layout::Rect) {
    let status_color = match state.core_status.as_str() {
        "RUNNING" => Color::Green,
        "HALTED" => Color::Yellow,
        _ => Color::Red,
    };

    let title = Line::from(vec![
        Span::styled(" rtt-terminal ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw("── [Probe: "),
        Span::styled(&state.probe_name, Style::default().fg(Color::Green)),
        Span::raw("] ── [Target: "),
        Span::styled(&state.target_name, Style::default().fg(Color::Yellow)),
        Span::raw("] ── [Status: "),
        Span::styled(&state.core_status, Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
        Span::raw("] "),
    ]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(title)
        .border_style(Style::default().fg(Color::DarkGray));

    let content = Line::from(vec![
        Span::raw(" Active Channel: "),
        Span::styled(format!("CH{} (Terminal)", state.active_channel), Style::default().fg(Color::Cyan)),
        Span::raw(" │ EOL: "),
        Span::styled(&state.eol, Style::default().fg(Color::White)),
        Span::raw(" │ Timestamps: "),
        Span::styled(
            if state.timestamps_enabled { "ON" } else { "OFF" },
            Style::default().fg(if state.timestamps_enabled { Color::Green } else { Color::DarkGray }),
        ),
        Span::raw(" │ Auto-Reconnect: "),
        Span::styled(
            if state.auto_reconnect_enabled { "ON" } else { "OFF" },
            Style::default().fg(Color::Green),
        ),
        Span::raw(" │ Log: "),
        Span::styled(state.log_file.as_deref().unwrap_or("None"), Style::default().fg(Color::Magenta)),
    ]);

    let paragraph = Paragraph::new(content).block(block);
    frame.render_widget(paragraph, area);
}

fn render_logs(frame: &mut Frame, state: &mut AppState, area: ratatui::layout::Rect) {
    let visible_height = area.height.saturating_sub(2) as usize;
    state.viewport_height = visible_height.max(1);

    let total_logs = state.logs.len();
    let max_scroll = total_logs.saturating_sub(visible_height);
    let effective_scroll = state.scroll_offset.min(max_scroll);
    state.scroll_offset = effective_scroll;

    let title_scroll = if state.auto_scroll || effective_scroll == 0 {
        Span::styled(" [ Auto-Scroll: ON ] ", Style::default().fg(Color::Green))
    } else {
        Span::styled(
            format!(" [ Scrolled: +{} ] ", effective_scroll),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Plain)
        .title(Line::from(vec![title_scroll]))
        .title_alignment(ratatui::layout::Alignment::Right)
        .border_style(Style::default().fg(Color::DarkGray));

    // Slice visible lines based on effective_scroll
    let lines_to_render: Vec<Line> = if total_logs == 0 {
        vec![Line::from(Span::styled(
            "Waiting for RTT output from microcontroller...",
            Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
        ))]
    } else {
        let end_idx = total_logs.saturating_sub(effective_scroll);
        let start_idx = end_idx.saturating_sub(visible_height);

        state
            .logs
            .range(start_idx..end_idx)
            .map(|entry| {
                let mut line = parse_ansi_to_line(&entry.raw_text);
                if state.timestamps_enabled {
                    let ts_span = Span::styled(
                        format!("[{}] ", entry.timestamp),
                        Style::default().fg(Color::DarkGray),
                    );
                    line.spans.insert(0, ts_span);
                }
                line
            })
            .collect()
    };

    let paragraph = Paragraph::new(lines_to_render).block(block);
    frame.render_widget(paragraph, area);

    // Render scrollbar if total logs exceed visible area
    if total_logs > visible_height && max_scroll > 0 {
        let mut scrollbar_state = ScrollbarState::new(max_scroll)
            .position(max_scroll.saturating_sub(effective_scroll));
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .symbols(ratatui::symbols::scrollbar::VERTICAL);
        frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}

fn render_hints(frame: &mut Frame, area: ratatui::layout::Rect) {
    let hints = Line::from(vec![
        Span::raw(" Quick shortcuts: "),
        Span::styled("[/help]", Style::default().fg(Color::Cyan)),
        Span::raw("  "),
        Span::styled("[/channel]", Style::default().fg(Color::Cyan)),
        Span::raw("  "),
        Span::styled("[/raw]", Style::default().fg(Color::Cyan)),
        Span::raw("  "),
        Span::styled("[/clear]", Style::default().fg(Color::Cyan)),
        Span::raw("  "),
        Span::styled("[/reset]", Style::default().fg(Color::Cyan)),
        Span::raw("  "),
        Span::styled("[/timestamps]", Style::default().fg(Color::Cyan)),
        Span::raw("  "),
        Span::styled("[/log]", Style::default().fg(Color::Cyan)),
        Span::raw("  "),
        Span::styled("[/quit]", Style::default().fg(Color::Cyan)),
        Span::styled(" (PageUp/Down or Mouse Wheel to scroll)", Style::default().fg(Color::DarkGray)),
    ]);
    frame.render_widget(Paragraph::new(hints), area);
}

fn render_input(frame: &mut Frame, state: &AppState, area: ratatui::layout::Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan));

    let input_line = Line::from(vec![
        Span::styled("> ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::raw(&state.input),
    ]);

    let paragraph = Paragraph::new(input_line).block(block);
    frame.render_widget(paragraph, area);

    // Position cursor at the active character index (+3 accounts for border and "> ")
    let cursor_x = area.x + 3 + state.cursor_position as u16;
    let cursor_y = area.y + 1;
    frame.set_cursor_position(Position::new(cursor_x, cursor_y));
}
