#![allow(dead_code)]

//! Terminal user interface components built with ratatui and crossterm.

use std::time::{Duration, Instant};
use std::{
    io::{self, Write},
    process::Command,
};

use chrono::{TimeZone, Utc};
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
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Pane {
    #[default]
    Groups,
    Feeds,
    Items,
    Preview,
    Queue,
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
    pub selected_item: usize,
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
            selected_item: 0,
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
    if opener.trim().is_empty() {
        let _ = open::that_in_background(url);
        return;
    }

    #[cfg(target_os = "windows")]
    if opener == "start" {
        let _ = Command::new("cmd").args(["/c", "start", "", url]).spawn();
        return;
    }

    let mut parts = opener.split_whitespace();
    if let Some(cmd) = parts.next() {
        let mut command = Command::new(cmd);
        let mut replaced = false;
        for part in parts {
            if part == "%u" {
                command.arg(url);
                replaced = true;
            } else {
                command.arg(part);
            }
        }
        if !replaced {
            command.arg(url);
        }
        let _ = command.spawn();
    } else {
        let _ = open::that_in_background(url);
    }
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
                app.selected_item = 0;
            }
        }
        KeyCode::Down => {
            if app.selected_group + 1 < app.groups.len() {
                app.selected_group += 1;
                app.selected_feed = 0;
                app.selected_item = 0;
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
                app.selected_item = 0;
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
                    app.selected_item = 0;
                }
            }
        }
        KeyCode::Char('r') => {
            if let Some(group) = app.groups.get_mut(app.selected_group)
                && let Some(name) = prompt("Rename group:")
            {
                group.name = name;
            }
        }
        KeyCode::Char('A') => {
            if let Some(group) = app.groups.get_mut(app.selected_group) {
                mark_group_read(group);
            }
        }
        KeyCode::Char('O') => {
            if let Some(group) = app.groups.get_mut(app.selected_group)
                && confirm("Open all unread items in group?")
            {
                let opener = app.config.opener.command.clone();
                open_unread_group(group, &opener);
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
                app.selected_item = 0;
            }
        }
        KeyCode::Down => {
            if app.selected_feed + 1 < app.groups[g].feeds.len() {
                app.selected_feed += 1;
                app.selected_item = 0;
            }
        }
        KeyCode::Left => {
            app.focus = Pane::Groups;
        }
        KeyCode::Right => {
            app.focus = Pane::Items;
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
                app.selected_item = 0;
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
                    app.selected_item = 0;
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

fn handle_items_key(code: KeyCode, app: &mut AppState) -> Result<(), Box<dyn std::error::Error>> {
    if app.groups.is_empty() {
        return Ok(());
    }
    let g = app.selected_group;
    if app.groups[g].feeds.is_empty() {
        return Ok(());
    }
    let f = app.selected_feed;
    let items_len = app.groups[g].feeds[f].items.len();
    if items_len == 0 {
        return Ok(());
    }
    match code {
        KeyCode::Up => {
            if app.selected_item > 0 {
                app.selected_item -= 1;
            }
        }
        KeyCode::Down => {
            if app.selected_item + 1 < items_len {
                app.selected_item += 1;
            }
        }
        KeyCode::Left => {
            app.focus = Pane::Feeds;
        }
        KeyCode::Enter => {
            let opener = app.config.opener.command.clone();
            let item = &app.groups[g].feeds[f].items[app.selected_item];
            open_link(&opener, &item.link);
        }
        KeyCode::Char(' ') => {
            let item = &mut app.groups[g].feeds[f].items[app.selected_item];
            item.read = !item.read;
            app.groups[g].update_unread();
        }
        KeyCode::Char('m') => {
            let item = &mut app.groups[g].feeds[f].items[app.selected_item];
            item.read = true;
            app.groups[g].update_unread();
        }
        KeyCode::Char('M') => {
            let item = &mut app.groups[g].feeds[f].items[app.selected_item];
            item.read = false;
            app.groups[g].update_unread();
        }
        KeyCode::Char('q') => {
            let item = &mut app.groups[g].feeds[f].items[app.selected_item];
            item.queued = !item.queued;
            if item.queued {
                app.queue.push(item.clone());
            } else {
                app.queue.retain(|i| i.id != item.id);
            }
        }
        KeyCode::Delete => {
            let item = &mut app.groups[g].feeds[f].items[app.selected_item];
            if item.queued {
                item.queued = false;
                app.queue.retain(|i| i.id != item.id);
            }
        }
        KeyCode::Char('Q') => {
            app.focus = Pane::Queue;
        }
        _ => {}
    }
    Ok(())
}

fn handle_queue_key(code: KeyCode, app: &mut AppState) -> Result<(), Box<dyn std::error::Error>> {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.focus = Pane::Items;
        }
        KeyCode::Enter => {
            let opener = app.config.opener.command.clone();
            let ids: Vec<String> = app.queue.iter().map(|i| i.id.clone()).collect();
            for id in ids {
                for group in &mut app.groups {
                    for feed in &mut group.feeds {
                        if let Some(item) = feed.items.iter_mut().find(|it| it.id == id) {
                            open_link(&opener, &item.link);
                            item.read = true;
                            item.queued = false;
                        }
                    }
                    group.update_unread();
                }
            }
            app.queue.clear();
            app.focus = Pane::Items;
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
                Event::Key(key) => {
                    if key.code == KeyCode::F(1) {
                        app.show_help = !app.show_help;
                    } else if key.code == KeyCode::Char('Q') {
                        app.focus = Pane::Queue;
                    } else if key.code == KeyCode::Char('q')
                        && app.focus != Pane::Items
                        && app.focus != Pane::Queue
                    {
                        data::save_db(&app.groups)?;
                        app.config.save()?;
                        break;
                    } else {
                        match app.focus {
                            Pane::Groups => handle_groups_key(key.code, app)?,
                            Pane::Feeds => handle_feeds_key(key.code, app)?,
                            Pane::Items => handle_items_key(key.code, app)?,
                            Pane::Queue => handle_queue_key(key.code, app)?,
                            _ => {}
                        }
                    }
                }
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

    let items = feeds
        .get(app.selected_feed)
        .map(|f| f.items.as_slice())
        .unwrap_or(&[]);
    let item_entries: Vec<ListItem> = items
        .iter()
        .map(|i| {
            let badge = if i.read { " " } else { "●" };
            let ts = Utc
                .timestamp_opt(i.timestamp, 0)
                .single()
                .unwrap_or_else(|| Utc.timestamp_opt(0, 0).unwrap())
                .format("%m-%d %H:%M")
                .to_string();
            ListItem::new(format!("{} {} {}", badge, ts, i.title))
        })
        .collect();
    let items_list =
        List::new(item_entries).block(Block::default().title("Items").borders(Borders::ALL));
    let mut item_state = ListState::default();
    if !items.is_empty() {
        item_state.select(Some(app.selected_item.min(items.len() - 1)));
    }
    f.render_stateful_widget(items_list, right_chunks[0], &mut item_state);

    let preview_lines = if let Some(item) = items.get(app.selected_item) {
        vec![
            Line::from(item.title.clone()),
            Line::from(""),
            Line::from(item.desc.clone()),
        ]
    } else {
        vec![Line::from("")]
    };
    let preview = Paragraph::new(preview_lines)
        .block(Block::default().title("Preview").borders(Borders::ALL));
    f.render_widget(preview, right_chunks[1]);

    if app.focus == Pane::Queue {
        draw_queue(f, f.size(), app);
    }
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

fn draw_queue(f: &mut Frame, area: Rect, app: &AppState) {
    let block = Block::default().title("Queue").borders(Borders::ALL);
    let items: Vec<ListItem> = app
        .queue
        .iter()
        .map(|i| ListItem::new(i.title.clone()))
        .collect();
    let list = List::new(items).block(block);
    let popup_area = centered_rect(60, 60, area);
    f.render_widget(Clear, popup_area);
    f.render_widget(list, popup_area);
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
