#![allow(dead_code)]

//! Terminal user interface components built with ratatui and crossterm.

use std::time::{Duration, Instant};
use std::{
    io::{self, Write},
    process::Command,
};

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
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
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
    pub selected_group: usize,
    pub selected_feed: usize,
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
            selected_group: 0,
            selected_feed: 0,
        }
    }
}

fn prompt(msg: &str) -> Option<String> {
    disable_raw_mode().ok()?;
    print!("{} ", msg);
    let _ = io::stdout().flush();
    let mut input = String::new();
    if io::stdin().read_line(&mut input).ok()? == 0 {
        let _ = enable_raw_mode();
        return None;
    }
    let _ = enable_raw_mode();
    let s = input.trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

fn confirm(msg: &str) -> bool {
    if let Some(ans) = prompt(&format!("{} [y/N]", msg)) {
        matches!(ans.to_lowercase().as_str(), "y" | "yes")
    } else {
        false
    }
}

fn open_link(opener: &str, url: &str) {
    let _ = Command::new(opener).arg(url).spawn();
}

fn mark_feed_read(feed: &mut Feed) {
    for item in &mut feed.items {
        item.read = true;
    }
}

fn mark_group_read(group: &mut Group) {
    for feed in &mut group.feeds {
        mark_feed_read(feed);
    }
    group.update_unread();
}

fn open_unread_feed(feed: &mut Feed, opener: &str) {
    for item in &mut feed.items {
        if !item.read {
            open_link(opener, &item.link);
            item.read = true;
        }
    }
}

fn open_unread_group(group: &mut Group, opener: &str) {
    for feed in &mut group.feeds {
        open_unread_feed(feed, opener);
    }
    group.update_unread();
}

fn handle_groups_key(code: KeyCode, app: &mut AppState) -> Result<(), Box<dyn std::error::Error>> {
    match code {
        KeyCode::Up => {
            if app.selected_group > 0 {
                app.selected_group -= 1;
                app.selected_feed = 0;
            }
        }
        KeyCode::Down => {
            if app.selected_group + 1 < app.groups.len() {
                app.selected_group += 1;
                app.selected_feed = 0;
            }
        }
        KeyCode::Right => {
            app.focus = Pane::Feeds;
        }
        KeyCode::Char('a') => {
            if let Some(name) = prompt("New group name:") {
                app.groups.push(Group {
                    name,
                    ..Group::default()
                });
                app.selected_group = app.groups.len() - 1;
                app.selected_feed = 0;
            }
        }
        KeyCode::Char('d') => {
            if !app.groups.is_empty() {
                let name = app.groups[app.selected_group].name.clone();
                if confirm(&format!("Delete group '{}' ?", name)) {
                    app.groups.remove(app.selected_group);
                    if app.selected_group >= app.groups.len() && app.selected_group > 0 {
                        app.selected_group -= 1;
                    }
                    app.selected_feed = 0;
                }
            }
        }
        KeyCode::Char('r') => {
            if let Some(group) = app.groups.get_mut(app.selected_group) {
                if let Some(name) = prompt("Rename group:") {
                    group.name = name;
                }
            }
        }
        KeyCode::Char('A') => {
            if let Some(group) = app.groups.get_mut(app.selected_group) {
                mark_group_read(group);
            }
        }
        KeyCode::Char('O') => {
            if let Some(group) = app.groups.get_mut(app.selected_group) {
                if confirm("Open all unread items in group?") {
                    let opener = app.config.opener.command.clone();
                    open_unread_group(group, &opener);
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_feeds_key(code: KeyCode, app: &mut AppState) -> Result<(), Box<dyn std::error::Error>> {
    if app.groups.is_empty() {
        return Ok(());
    }
    let g = app.selected_group;
    match code {
        KeyCode::Up => {
            if app.selected_feed > 0 {
                app.selected_feed -= 1;
            }
        }
        KeyCode::Down => {
            if app.selected_feed + 1 < app.groups[g].feeds.len() {
                app.selected_feed += 1;
            }
        }
        KeyCode::Left => {
            app.focus = Pane::Groups;
        }
        KeyCode::Char('a') => {
            if let Some(url) = prompt("Feed URL:") {
                let feed = Feed {
                    url: url.clone(),
                    title: url,
                    ..Feed::default()
                };
                app.groups[g].feeds.push(feed);
                app.selected_feed = app.groups[g].feeds.len() - 1;
            }
        }
        KeyCode::Char('d') => {
            if !app.groups[g].feeds.is_empty() {
                let title = app.groups[g].feeds[app.selected_feed].title.clone();
                if confirm(&format!("Delete feed '{}' ?", title)) {
                    app.groups[g].feeds.remove(app.selected_feed);
                    if app.selected_feed >= app.groups[g].feeds.len() && app.selected_feed > 0 {
                        app.selected_feed -= 1;
                    }
                    app.groups[g].update_unread();
                }
            }
        }
        KeyCode::Char('A') => {
            if let Some(feed) = app.groups[g].feeds.get_mut(app.selected_feed) {
                mark_feed_read(feed);
                app.groups[g].update_unread();
            }
        }
        KeyCode::Char('O') => {
            if let Some(feed) = app.groups[g].feeds.get_mut(app.selected_feed) {
                let opener = app.config.opener.command.clone();
                open_unread_feed(feed, &opener);
                app.groups[g].update_unread();
            }
        }
        _ => {}
    }
    Ok(())
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
                    _ => match app.focus {
                        Pane::Groups => handle_groups_key(key.code, app)?,
                        Pane::Feeds => handle_feeds_key(key.code, app)?,
                        _ => {}
                    },
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

    let group_items: Vec<ListItem> = app
        .groups
        .iter()
        .map(|g| ListItem::new(g.name.clone()))
        .collect();
    let groups_list =
        List::new(group_items).block(Block::default().title("Groups").borders(Borders::ALL));
    let mut group_state = ListState::default();
    if !app.groups.is_empty() {
        group_state.select(Some(app.selected_group.min(app.groups.len() - 1)));
    }
    f.render_stateful_widget(groups_list, chunks[0], &mut group_state);

    let feeds = app
        .groups
        .get(app.selected_group)
        .map(|g| g.feeds.as_slice())
        .unwrap_or(&[]);
    let feed_items: Vec<ListItem> = feeds
        .iter()
        .map(|f| ListItem::new(f.title.clone()))
        .collect();
    let feeds_list =
        List::new(feed_items).block(Block::default().title("Feeds").borders(Borders::ALL));
    let mut feed_state = ListState::default();
    if !feeds.is_empty() {
        feed_state.select(Some(app.selected_feed.min(feeds.len() - 1)));
    }
    f.render_stateful_widget(feeds_list, chunks[1], &mut feed_state);

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
