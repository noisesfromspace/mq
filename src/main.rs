use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::path::PathBuf;
use std::sync::Arc;

pub mod config;
pub mod events;
pub mod mail;
pub mod ui;

use config::Config;
use events::{AppEvent, EventHandler};
use mail::Searcher;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Initial search query
    pub query: Option<String>,

    /// Optional path to alternative configuration file
    #[arg(short, long)]
    pub config: Option<String>,
}

enum AppMessage {
    SearchResults(Vec<mail::EmailMetadata>),
    PreviewLoaded(String, String),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let app_config = Arc::new(Config::load(cli.config.as_deref()).unwrap_or_default());

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = ui::UiState::new(cli.query);
    let searcher = Arc::new(Searcher::new(app_config.database_path.clone()));

    let res = run_app(&mut terminal, &mut state, searcher, app_config).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen,)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    state: &mut ui::UiState,
    searcher: Arc<Searcher>,
    config: Arc<Config>,
) -> Result<()> {
    let limit = config.max_results.unwrap_or(100);
    let mut events = EventHandler::new(250);
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<AppMessage>();

    // Send initial search
    if !state.search_query.is_empty() {
        let q = state.search_query.clone();
        let s = searcher.clone();
        let t = tx.clone();
        tokio::task::spawn_blocking(move || {
            if let Ok(results) = s.search(&q, limit) {
                let _ = t.send(AppMessage::SearchResults(results));
            }
        });
    }

    let mut search_task: Option<tokio::task::JoinHandle<()>> = None;
    let mut preview_task: Option<tokio::task::JoinHandle<()>> = None;
    let mut last_query = state.search_query.clone();

    loop {
        terminal.draw(|f| ui::draw(f, state))?;

        tokio::select! {
            Some(msg) = rx.recv() => {
                match msg {
                    AppMessage::SearchResults(results) => {
                        state.results = results;
                        if state.results.is_empty() {
                            state.selected_index = None;
                            state.selected_preview = None;
                            state.selected_headers = None;
                            state.preview_scroll_y = 0;
                        } else {
                            state.selected_index = Some(0);
                            state.preview_scroll_y = 0;
                            state.preview_scroll_x = 0;
                            trigger_preview(&state.results[0].path, &tx, &mut preview_task);
                        }
                    }
                    AppMessage::PreviewLoaded(preview, headers) => {
                        state.selected_preview = Some(preview);
                        state.selected_headers = Some(headers);
                    }
                }
            }
            Some(event) = events.next() => {
                match event {
                    AppEvent::Input(crossterm::event::Event::Key(key)) => {
                        if key.kind == KeyEventKind::Press {
                            if key.modifiers.contains(KeyModifiers::CONTROL) {
                                match key.code {
                                    KeyCode::Char('c') => return Ok(()),
                                    KeyCode::Char('w') => {
                                        if state.is_searching && state.search_cursor > 0 {
                                            let s = state.search_query.clone();
                                            let chars: Vec<char> = s.chars().collect();
                                            let mut idx = state.search_cursor - 1;
                                            while idx > 0 && chars[idx].is_whitespace() {
                                                idx -= 1;
                                            }
                                            while idx > 0 && !chars[idx-1].is_whitespace() {
                                                idx -= 1;
                                            }

                                            let mut new_chars = Vec::new();
                                            for (i, c) in chars.iter().enumerate() {
                                                if i < idx || i >= state.search_cursor {
                                                    new_chars.push(*c);
                                                }
                                            }
                                            state.search_query = new_chars.into_iter().collect();
                                            state.search_cursor = idx;
                                        }
                                    }
                                    KeyCode::Char('d') => {
                                        if state.is_searching && state.search_cursor < state.search_query.chars().count() {
                                            // Forward delete (delete character under cursor)
                                            let mut chars: Vec<char> = state.search_query.chars().collect();
                                            chars.remove(state.search_cursor);
                                            state.search_query = chars.into_iter().collect();
                                        } else if !state.is_searching {
                                            if let Some(idx) = state.selected_index {
                                                let new_idx = (idx + 10).min(state.results.len().saturating_sub(1));
                                                if new_idx != idx {
                                                    state.selected_index = Some(new_idx);
                                                    state.preview_scroll_y = 0;
                                                    state.preview_scroll_x = 0;
                                                    trigger_preview(&state.results[new_idx].path, &tx, &mut preview_task);
                                                }
                                            }
                                        }
                                    }
                                    KeyCode::Char('u') => {
                                        if state.is_searching {
                                            // Clear from cursor to beginning of line
                                            let chars: Vec<char> = state.search_query.chars().collect();
                                            let mut new_chars = Vec::new();
                                            for (i, c) in chars.iter().enumerate() {
                                                if i >= state.search_cursor {
                                                    new_chars.push(*c);
                                                }
                                            }
                                            state.search_query = new_chars.into_iter().collect();
                                            state.search_cursor = 0;
                                        } else {
                                            if let Some(idx) = state.selected_index {
                                                let new_idx = idx.saturating_sub(10);
                                                if new_idx != idx {
                                                    state.selected_index = Some(new_idx);
                                                    state.preview_scroll_y = 0;
                                                    state.preview_scroll_x = 0;
                                                    trigger_preview(&state.results[new_idx].path, &tx, &mut preview_task);
                                                }
                                            }
                                        }
                                    }
                                    KeyCode::Char('f') => {
                                        if state.is_searching && state.search_cursor < state.search_query.chars().count() {
                                            state.search_cursor += 1;
                                        } else if !state.is_searching && state.selected_preview.is_some() {
                                            state.preview_scroll_y = state.preview_scroll_y.saturating_add(10);
                                        }
                                    }
                                    KeyCode::Char('b') => {
                                        if state.is_searching && state.search_cursor > 0 {
                                            state.search_cursor -= 1;
                                        } else if !state.is_searching && state.selected_preview.is_some() {
                                            state.preview_scroll_y = state.preview_scroll_y.saturating_sub(10);
                                        }
                                    }
                                    KeyCode::Char('a') => {
                                        if state.is_searching {
                                            state.search_cursor = 0;
                                        }
                                    }
                                    KeyCode::Char('e') => {
                                        if state.is_searching {
                                            state.search_cursor = state.search_query.chars().count();
                                        }
                                    }
                                    KeyCode::Char('k') => {
                                        if state.is_searching && state.search_cursor < state.search_query.chars().count() {
                                            // Clear from cursor to end of line
                                            let chars: Vec<char> = state.search_query.chars().collect();
                                            let mut new_chars = Vec::new();
                                            for (i, c) in chars.iter().enumerate() {
                                                if i < state.search_cursor {
                                                    new_chars.push(*c);
                                                }
                                            }
                                            state.search_query = new_chars.into_iter().collect();
                                        }
                                    }
                                    KeyCode::Left => {
                                        if state.is_searching && state.search_cursor > 0 {
                                            state.search_cursor = jump_word_left(&state.search_query, state.search_cursor);
                                        }
                                    }
                                    KeyCode::Right => {
                                        if state.is_searching && state.search_cursor < state.search_query.chars().count() {
                                            state.search_cursor = jump_word_right(&state.search_query, state.search_cursor);
                                        }
                                    }
                                    _ => {}
                                }
                            } else if key.modifiers.contains(KeyModifiers::ALT) {
                                match key.code {
                                    KeyCode::Char('b') => {
                                        if state.is_searching && state.search_cursor > 0 {
                                            state.search_cursor = jump_word_left(&state.search_query, state.search_cursor);
                                        }
                                    }
                                    KeyCode::Char('f') => {
                                        if state.is_searching && state.search_cursor < state.search_query.chars().count() {
                                            state.search_cursor = jump_word_right(&state.search_query, state.search_cursor);
                                        }
                                    }
                                    KeyCode::Char('d') => {
                                        if state.is_searching && state.search_cursor < state.search_query.chars().count() {
                                            let chars: Vec<char> = state.search_query.chars().collect();
                                            let mut idx = state.search_cursor;
                                            // Skip whitespace
                                            while idx < chars.len() && chars[idx].is_whitespace() {
                                                idx += 1;
                                            }
                                            // Skip word characters
                                            while idx < chars.len() && !chars[idx].is_whitespace() {
                                                idx += 1;
                                            }
                                            // Remove characters from cursor to idx
                                            let mut new_chars = Vec::new();
                                            for (i, c) in chars.iter().enumerate() {
                                                if i < state.search_cursor || i >= idx {
                                                    new_chars.push(*c);
                                                }
                                            }
                                            state.search_query = new_chars.into_iter().collect();
                                        }
                                    }
                                    _ => {}
                                }
                            } else {
                                match key.code {
                                KeyCode::Esc => {
                                    if state.show_help {
                                        state.show_help = false;
                                    } else if state.show_folder_info {
                                        state.show_folder_info = false;
                                    } else if state.show_headers {
                                        state.show_headers = false;
                                    }  else {
                                        return Ok(());
                                    }
                                }
                                KeyCode::Char(c) => {
                                    if state.is_searching {
                                        let mut chars: Vec<char> = state.search_query.chars().collect();
                                        chars.insert(state.search_cursor, c);
                                        state.search_query = chars.into_iter().collect();
                                        state.search_cursor += 1;
                                    } else if c == 'q' {
                                        if state.show_help {
                                            state.show_help = false;
                                        } else if state.show_folder_info {
                                            state.show_folder_info = false;
                                        } else if state.show_headers {
                                            state.show_headers = false;
                                        } else if state.is_searching {
                                            state.is_searching = false;
                                        }
                                    } else if c == '?' {
                                        state.show_help = !state.show_help;
                                    } else if c == '/'{
                                        state.is_searching = true;
                                    } else if c == 'j' {
                                        if let Some(idx) = state.selected_index {
                                            if idx + 1 < state.results.len() {
                                                state.selected_index = Some(idx + 1);
                                                state.preview_scroll_y = 0;
                                                state.preview_scroll_x = 0;
                                                trigger_preview(&state.results[idx + 1].path, &tx, &mut preview_task);
                                            }
                                        }
                                    } else if c == 'k' {
                                        if let Some(idx) = state.selected_index {
                                            if idx > 0 {
                                                state.selected_index = Some(idx - 1);
                                                state.preview_scroll_y = 0;
                                                state.preview_scroll_x = 0;
                                                trigger_preview(&state.results[idx - 1].path, &tx, &mut preview_task);
                                            }
                                        }
                                    } else if c == 'h' {
                                        if !state.is_searching && state.selected_preview.is_some() {
                                            state.preview_scroll_x = state.preview_scroll_x.saturating_sub(5);
                                        }
                                    } else if c == 'l' {
                                        if !state.is_searching && state.selected_preview.is_some() {
                                            state.preview_scroll_x = state.preview_scroll_x.saturating_add(5);
                                        }
                                    } else if c == 'o' {
                                        if let Some(idx) = state.selected_index {
                                            if let Some(metadata) = state.results.get(idx) {
                                                let path = metadata.path.clone();
                                                let message_id = metadata.message_id.clone();
                                                let browser_cmd = config.browser.clone();
                                                tokio::task::spawn_blocking(move || {
                                                    if let Ok(Some(html)) = mail::preview::extract_html(&path) {
                                                        use sha2::{Sha256, Digest};
                                                        let mut hasher = Sha256::new();
                                                        hasher.update(message_id.as_bytes());
                                                        let hash = format!("{:x}", hasher.finalize());
                                                        // Truncate hash to 16 chars for cleaner temp files
                                                        let safe_filename = format!("mq-{}.html", &hash[..16]);

                                                        let temp_path = std::env::temp_dir().join(safe_filename);
                                                        if std::fs::write(&temp_path, html).is_ok() {
                                                            if let Some(cmd_str) = browser_cmd {
                                                                let mut parts = cmd_str.split_whitespace();
                                                                if let Some(cmd) = parts.next() {
                                                                    let mut proc = std::process::Command::new(cmd);
                                                                    for arg in parts {
                                                                        proc.arg(arg);
                                                                    }
                                                                    proc.arg(temp_path);
                                                                    let _ = proc.spawn();
                                                                }
                                                            } else {
                                                                let _ = open::that(temp_path);
                                                            }
                                                        }
                                                    }
                                                });
                                            }
                                        }
                                    } else if c == 'f' {
                                        if !state.results.is_empty() && !state.is_searching {
                                            state.show_folder_info = !state.show_folder_info;
                                            state.show_headers = false;
                                        }
                                    } else if c == 'H' {
                                        if !state.results.is_empty() && !state.is_searching {
                                            state.show_headers = !state.show_headers;
                                            state.show_folder_info = false;
                                        }
                                    }
                                }
                                KeyCode::Backspace => {
                                    if state.is_searching && state.search_cursor > 0 {
                                        let mut chars: Vec<char> = state.search_query.chars().collect();
                                        chars.remove(state.search_cursor - 1);
                                        state.search_query = chars.into_iter().collect();
                                        state.search_cursor -= 1;
                                    }
                                }
                                KeyCode::Delete => {
                                    if state.is_searching && state.search_cursor < state.search_query.chars().count(){
                                        let mut chars: Vec<char> = state.search_query.chars().collect();
                                        chars.remove(state.search_cursor);
                                        state.search_query = chars.into_iter().collect();
                                    }
                                }
                                KeyCode::Left => {
                                    if state.is_searching && state.search_cursor > 0 {
                                        state.search_cursor -= 1;
                                    }
                                }
                                KeyCode::Right => {
                                    if state.is_searching && state.search_cursor < state.search_query.chars().count() {
                                        state.search_cursor += 1;
                                    }
                                }
                                KeyCode::Home => {
                                    if state.is_searching {
                                        state.search_cursor = 0;
                                    }
                                }
                                KeyCode::End => {
                                    if state.is_searching {
                                        state.search_cursor = state.search_query.chars().count();
                                    }
                                }
                                KeyCode::Down => {
                                    if !state.is_searching {
                                        if let Some(idx) = state.selected_index {
                                            if idx + 1 < state.results.len() {
                                                state.selected_index = Some(idx + 1);
                                                state.preview_scroll_y = 0;
                                                state.preview_scroll_x = 0;
                                                trigger_preview(&state.results[idx + 1].path, &tx, &mut preview_task);
                                            }
                                        }
                                    } else {
                                            state.is_searching = false
                                        }
                                }
                                KeyCode::Up => {
                                    if !state.is_searching {
                                        if let Some(idx) = state.selected_index {
                                            if idx > 0 {
                                                state.selected_index = Some(idx - 1);
                                                state.preview_scroll_y = 0;
                                                state.preview_scroll_x = 0;
                                                trigger_preview(&state.results[idx - 1].path, &tx, &mut preview_task);
                                            } else {
                                                    state.is_searching = true
                                                }
                                        }
                                    }
                                }
                                KeyCode::PageDown => {
                                    if !state.is_searching && state.selected_preview.is_some() {
                                        state.preview_scroll_y = state.preview_scroll_y.saturating_add(10);
                                    }
                                }
                                KeyCode::PageUp => {
                                    if !state.is_searching && state.selected_preview.is_some() {
                                        state.preview_scroll_y = state.preview_scroll_y.saturating_sub(10);
                                    }
                                }
                                KeyCode::Enter => {
                                    if state.show_help {
                                        state.show_help = false;
                                    } else if state.show_folder_info {
                                        state.show_folder_info = false;
                                    } else if state.show_headers {
                                        state.show_headers = false;
                                    } else if state.is_searching {
                                        state.is_searching = false;
                                    } else if let Some(idx) = state.selected_index {
                                        if let Some(metadata) = state.results.get(idx) {
                                            let path = metadata.path.clone();
                                            // Suspend TUI and open pager
                                            disable_raw_mode()?;
                                            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

                                            let pager = config.pager.clone()
                                                .unwrap_or_else(|| std::env::var("PAGER").unwrap_or_else(|_| "less -R".to_string()));
                                            let mut parts = pager.split_whitespace();
                                            if let Some(cmd) = parts.next() {
                                                let mut proc = std::process::Command::new(cmd);
                                                for arg in parts {
                                                    proc.arg(arg);
                                                }
                                                proc.arg(path);
                                                let _ = proc.status();
                                            }

                                            // Restore TUI
                                            enable_raw_mode()?;
                                            execute!(terminal.backend_mut(), EnterAlternateScreen)?;
                                            terminal.clear()?;
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        }
                    }
                    AppEvent::Tick => {
                        if state.search_query != last_query {
                            last_query = state.search_query.clone();
                            if let Some(task) = search_task.take() {
                                task.abort();
                            }

                            let q = state.search_query.clone();
                            let s = searcher.clone();
                            let t = tx.clone();

                            search_task = Some(tokio::spawn(async move {
                                tokio::time::sleep(std::time::Duration::from_millis(300)).await;
                                let res = tokio::task::spawn_blocking(move || {
                                    s.search(&q, limit)
                                }).await;

                                if let Ok(Ok(results)) = res {
                                    let _ = t.send(AppMessage::SearchResults(results));
                                }
                            }));
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

fn jump_word_left(s: &str, mut idx: usize) -> usize {
    if idx == 0 {
        return 0;
    }
    let chars: Vec<char> = s.chars().collect();
    idx -= 1;
    while idx > 0 && chars[idx].is_whitespace() {
        idx -= 1;
    }
    while idx > 0 && !chars[idx - 1].is_whitespace() {
        idx -= 1;
    }
    idx
}

fn jump_word_right(s: &str, mut idx: usize) -> usize {
    let chars: Vec<char> = s.chars().collect();
    if idx >= chars.len() {
        return chars.len();
    }
    while idx < chars.len() && chars[idx].is_whitespace() {
        idx += 1;
    }
    while idx < chars.len() && !chars[idx].is_whitespace() {
        idx += 1;
    }
    idx
}

fn trigger_preview(
    path: &PathBuf,
    tx: &tokio::sync::mpsc::UnboundedSender<AppMessage>,
    preview_task: &mut Option<tokio::task::JoinHandle<()>>,
) {
    if let Some(task) = preview_task.take() {
        task.abort();
    }

    let path = path.clone();
    let t = tx.clone();
    *preview_task = Some(tokio::spawn(async move {
        let res = tokio::task::spawn_blocking(move || mail::preview::generate_preview(&path)).await;

        if let Ok(Ok((preview, headers))) = res {
            let _ = t.send(AppMessage::PreviewLoaded(preview, headers));
        }
    }));
}
