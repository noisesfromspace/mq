use crate::mail::EmailMetadata;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

pub struct UiState {
    pub search_query: String,
    pub search_cursor: usize,
    pub is_searching: bool,
    pub results: Vec<EmailMetadata>,
    pub selected_index: Option<usize>,
    pub selected_preview: Option<String>,
    pub selected_headers: Option<String>,
    pub preview_scroll_y: u16,
    pub preview_scroll_x: u16,
    pub show_folder_info: bool,
    pub show_headers: bool,
    pub show_help: bool,
    pub list_state: ListState,
}

impl UiState {
    pub fn new(initial_query: Option<String>) -> Self {
        let query = initial_query.unwrap_or_default();
        let cursor = query.chars().count();
        Self {
            search_query: query,
            search_cursor: cursor,
            is_searching: true,
            results: Vec::new(),
            selected_index: None,
            selected_preview: None,
            selected_headers: None,
            preview_scroll_y: 0,
            preview_scroll_x: 0,
            show_folder_info: false,
            show_headers: false,
            show_help: false,
            list_state: ListState::default(),
        }
    }
}

pub fn draw(f: &mut Frame, state: &mut UiState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(f.area());

    let active_border = Style::default().fg(Color::Yellow);
    let inactive_border = Style::default().fg(Color::DarkGray);

    let search_block = Block::default()
        .borders(Borders::ALL)
        .title(if state.is_searching {
            " Search "
        } else {
            " Search (/) "
        })
        .border_style(if state.is_searching {
            active_border
        } else {
            inactive_border
        });

    let mut query_spans = Vec::new();
    for (i, c) in state.search_query.chars().enumerate() {
        if state.is_searching && i == state.search_cursor {
            query_spans.push(Span::styled(
                c.to_string(),
                Style::default().add_modifier(Modifier::REVERSED),
            ));
        } else {
            query_spans.push(Span::raw(c.to_string()));
        }
    }
    if state.is_searching && state.search_cursor == state.search_query.chars().count() {
        query_spans.push(Span::styled(
            " ",
            Style::default().add_modifier(Modifier::REVERSED),
        ));
    }

    let search_text = Paragraph::new(Line::from(query_spans)).block(search_block);

    f.render_widget(search_text, chunks[0]);

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(chunks[1]);

    state.list_state.select(state.selected_index);

    let items: Vec<ListItem> = state
        .results
        .iter()
        .map(|metadata| {
            let date_str = chrono::DateTime::from_timestamp(metadata.date, 0)
                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                .unwrap_or_default();

            let content = Line::from(vec![
                Span::styled(
                    format!(
                        "{:<15} ",
                        &metadata.from.chars().take(15).collect::<String>()
                    ),
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw(format!("{} ", metadata.subject)),
                Span::styled(date_str, Style::default().fg(Color::DarkGray)),
            ]);

            ListItem::new(content)
        })
        .collect();

    let list_block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Results ({}) ", state.results.len()))
        .border_style(if !state.is_searching {
            active_border
        } else {
            inactive_border
        });

    let list = List::new(items)
        .block(list_block)
        .highlight_symbol(if state.is_searching { "  " } else { "> " })
        .highlight_style(if state.is_searching {
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::REVERSED)
                .add_modifier(Modifier::BOLD)
        });

    f.render_stateful_widget(list, main_chunks[0], &mut state.list_state);

    let preview_text = match &state.selected_preview {
        Some(text) => text.as_str(),
        None => {
            if state.results.is_empty() {
                "No results found."
            } else {
                "Select an email to preview."
            }
        }
    };

    let preview_block = Block::default()
        .borders(Borders::ALL)
        .title(" Preview ")
        .border_style(if !state.is_searching {
            active_border
        } else {
            inactive_border
        });

    let preview = Paragraph::new(preview_text)
        .block(preview_block)
        .scroll((state.preview_scroll_y, state.preview_scroll_x));

    f.render_widget(preview, main_chunks[1]);

    if state.show_folder_info {
        if let Some(idx) = state.selected_index {
            if let Some(metadata) = state.results.get(idx) {
                let area = centered_rect(60, 40, f.area());
                let popup_block = Block::default()
                    .title(" Folder Info ")
                    .borders(Borders::ALL)
                    .style(Style::default().bg(Color::Black));

                let text = format!(
                    "Found in: {}\nSubject:  {}\n\nTo open in aerc:\n1. Go to folder: {}\n2. Search: /{}",
                    metadata.folder, metadata.subject, metadata.folder, metadata.subject
                );

                let p = Paragraph::new(text)
                    .block(popup_block)
                    .wrap(Wrap { trim: false });

                f.render_widget(ratatui::widgets::Clear, area);
                f.render_widget(p, area);
            }
        }
    }

    if state.show_headers {
        if let Some(headers_text) = &state.selected_headers {
            let area = centered_rect(60, 60, f.area());
            let popup_block = Block::default()
                .title(" Important Headers ")
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Black));

            let p = Paragraph::new(headers_text.as_str())
                .block(popup_block)
                .wrap(Wrap { trim: false });

            f.render_widget(ratatui::widgets::Clear, area);
            f.render_widget(p, area);
        }
    }

    if state.show_help {
        let area = centered_rect(70, 80, f.area());
        let popup_block = Block::default()
            .title(" Help (?) ")
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Black));

        let help_text = vec![
            Line::from(vec![Span::styled(
                "KEYBINDINGS",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  /            : Focus search box"),
            Line::from("  Enter        : Open selected email in pager / Unfocus search"),
            Line::from("  j/k, ↓/↑     : Navigate email list"),
            Line::from("  h/l          : Scroll preview pane left/right"),
            Line::from("  Ctrl+d/u     : Jump down/up 10 items"),
            Line::from("  PgDn/PgUp    : Scroll preview pane down/up"),
            Line::from("  Ctrl+f/b     : Scroll preview pane down/up"),
            Line::from("  Ctrl+Left/Right: Move by word in search box"),
            Line::from("  Home/End     : Go to start/end of search box"),
            Line::from("  o            : Open HTML version in browser"),
            Line::from("  f            : Show aerc folder info"),
            Line::from("  H            : Show important mail headers"),
            Line::from("  ?            : Toggle this help screen"),
            Line::from("  q / Esc      : Close popups, unfocus search, or quit"),
            Line::from(""),
            Line::from(vec![Span::styled(
                "NOTMUCH SEARCH SYNTAX",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  from:        : e.g. from:alice@example.com or from:bob"),
            Line::from("  to:          : e.g. to:sales@example.com"),
            Line::from("  subject:     : e.g. subject:\"monthly report\""),
            Line::from(
                "  date:        : e.g. date:yesterday..today or date:2024-01-01..2024-02-01",
            ),
            Line::from("  tag:         : e.g. tag:unread or tag:inbox"),
            Line::from("  folder:      : e.g. folder:Work/receipts"),
            Line::from(
                "  body:        : e.g. body:\"tracking number\" (or just type the words directly)",
            ),
            Line::from(""),
            Line::from(vec![Span::styled(
                "LOGICAL OPERATORS & MODIFIERS",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  AND, OR, NOT : e.g. from:boss AND NOT tag:read"),
            Line::from(
                "  * (wildcard) : e.g. subject:proj* (Note: wildcards work at the end of words)",
            ),
        ];

        let p = Paragraph::new(help_text)
            .block(popup_block)
            .wrap(Wrap { trim: false });

        f.render_widget(ratatui::widgets::Clear, area);
        f.render_widget(p, area);
    }
}

fn centered_rect(
    percent_x: u16,
    percent_y: u16,
    r: ratatui::layout::Rect,
) -> ratatui::layout::Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
