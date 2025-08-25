#![allow(dead_code)]

//! Terminal user interface components built with ratatui and crossterm.

use std::time::{Duration, Instant};

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::{
    config::Config,
    data::{self, Feed, Group, Item},
};

/// Application focusable panes.
#[derive(Clone, Copy, Debug, Default)]
pub enum Pane {
    #[default]
    Groups,
    Feeds,
    Items,
    Preview,
}

/// Global application state.
pub struct AppState {
    pub focus: Pane,
    pub groups: Vec<Group>,
    pub feeds: Vec<Feed>,
    pub items: Vec<Item>,
    pub queue: Vec<Item>,
    pub search: String,
    pub show_help: bool,
    pub config: Config,
}

impl AppState {
    /// Create a new application state with loaded configuration and groups.
    pub fn new(config: Config, groups: Vec<Group>) -> Self {
        Self {
            focus: Pane::Groups,
            feeds: Vec::new(),
            items: Vec::new(),
            queue: Vec::new(),
            search: String::new(),
            show_help: false,
            config,
            groups,
        }
    }
}

/// Run the application event loop.
pub fn run_app(app: &mut AppState) -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let tick_rate = Duration::from_millis(250);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| ui(f, app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));
        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) => match key.code {
                    KeyCode::Char('q') => {
                        data::save_db(&app.groups)?;
                        app.config.save()?;
                        break;
                    }
                    KeyCode::F(1) => {
                        app.show_help = !app.show_help;
                    }
                    _ => {}
                },
                Event::Resize(_, _) => {
                    // just trigger a redraw on next loop
                }
                _ => {}
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

/// Draw the main UI layout.
fn ui(f: &mut Frame, app: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(30),
            Constraint::Percentage(50),
        ])
        .split(f.size());

    let groups_block = Block::default().title("Groups").borders(Borders::ALL);
    f.render_widget(groups_block, chunks[0]);

    let feeds_block = Block::default().title("Feeds").borders(Borders::ALL);
    f.render_widget(feeds_block, chunks[1]);

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[2]);

    let items_block = Block::default().title("Items").borders(Borders::ALL);
    f.render_widget(items_block, right_chunks[0]);

    let preview_block = Block::default().title("Preview").borders(Borders::ALL);
    f.render_widget(preview_block, right_chunks[1]);

    if app.show_help {
        draw_help(f, f.size());
    }
}

/// Render the help overlay showing key bindings.
fn draw_help(f: &mut Frame, area: Rect) {
    let block = Block::default().title("Help").borders(Borders::ALL);
    let text = vec![Line::from("F1: Toggle help"), Line::from("q: Quit")];
    let paragraph = Paragraph::new(text).block(block).style(Style::default());
    let popup_area = centered_rect(60, 40, area);
    f.render_widget(Clear, popup_area); // clear under the popup
    f.render_widget(paragraph, popup_area);
}

/// Helper to create a centered rect using up certain percentage of the available space.
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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
