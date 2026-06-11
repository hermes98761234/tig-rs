use anyhow::Result;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout};
use ratatui::DefaultTerminal;

use crate::ui::draw_status_bar;
use crate::views::{View, ViewAction};

pub struct App {
    views: Vec<Box<dyn View>>,
    status_msg: String,
    /// Some(buffer) while the user is typing a /search query.
    search_input: Option<String>,
    /// Last submitted query, for n/N.
    last_search: String,
}

impl App {
    pub fn new(root: Box<dyn View>) -> Self {
        App {
            views: vec![root],
            status_msg: String::from("h: help, q: close view, Q: quit"),
            search_input: None,
            last_search: String::new(),
        }
    }

    fn find_match(&mut self, from: usize, forward: bool) {
        let Some(view) = self.views.last_mut() else {
            return;
        };
        if self.last_search.is_empty() {
            return;
        }
        let needle = self.last_search.to_lowercase();
        let lines = view.text_lines();
        let n = lines.len();
        if n == 0 {
            self.status_msg = format!("no match for \"{}\"", self.last_search);
            return;
        }
        let hit = (1..=n).map(|step| {
            if forward {
                (from + step) % n
            } else {
                (from + n - step % n) % n
            }
        });
        for i in hit {
            if lines[i].to_lowercase().contains(&needle) {
                view.select_line(i);
                self.status_msg = format!("match {}/{} for \"{}\"", i + 1, n, self.last_search);
                return;
            }
        }
        self.status_msg = format!("no match for \"{}\"", self.last_search);
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        loop {
            terminal.draw(|f| {
                let [main, status] =
                    Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).areas(f.area());
                let title = self.views.last().map(|v| v.title()).unwrap_or_default();
                if let Some(view) = self.views.last_mut() {
                    view.draw(f, main);
                }
                let msg = if let Some(buf) = &self.search_input {
                    format!("/{buf}")
                } else {
                    self.status_msg.clone()
                };
                draw_status_bar(f, status, &title, &msg);
            })?;

            let Event::Key(key) = event::read()? else {
                continue;
            };
            if key.kind != KeyEventKind::Press {
                continue;
            }

            // 1. search input mode swallows everything
            if let Some(buf) = self.search_input.as_mut() {
                match key.code {
                    KeyCode::Esc => self.search_input = None,
                    KeyCode::Backspace => {
                        buf.pop();
                    }
                    KeyCode::Enter => {
                        self.last_search = self.search_input.take().unwrap_or_default();
                        let from = self.views.last().map(|v| v.selected_index()).unwrap_or(0);
                        self.find_match(from, true);
                    }
                    KeyCode::Char(c) => buf.push(c),
                    _ => {}
                }
                continue;
            }

            // 2. global keys
            match key.code {
                KeyCode::Char('Q') => return Ok(()),
                KeyCode::Char('q') => {
                    self.views.pop();
                    if self.views.is_empty() {
                        return Ok(());
                    }
                    if let Some(v) = self.views.last_mut() {
                        let _ = v.reload();
                    }
                    continue;
                }
                KeyCode::Char('m') => {
                    match crate::views::main_view::MainView::new(Vec::new()) {
                        Ok(v) => self.views.push(Box::new(v)),
                        Err(e) => self.status_msg = format!("error: {e}"),
                    }
                    continue;
                }
                KeyCode::Char('r') => {
                    match crate::views::refs::RefsView::new() {
                        Ok(v) => self.views.push(Box::new(v)),
                        Err(e) => self.status_msg = format!("error: {e}"),
                    }
                    continue;
                }
                KeyCode::Char('t') => {
                    match crate::views::tree::TreeView::new("HEAD", "") {
                        Ok(v) => self.views.push(Box::new(v)),
                        Err(e) => self.status_msg = format!("error: {e}"),
                    }
                    continue;
                }
                KeyCode::Char('s') => {
                    match crate::views::status::StatusView::new() {
                        Ok(v) => self.views.push(Box::new(v)),
                        Err(e) => self.status_msg = format!("error: {e}"),
                    }
                    continue;
                }
                KeyCode::Char('h') => {
                    self.views.push(crate::views::help::help_view());
                    continue;
                }
                KeyCode::Char('/') => {
                    self.search_input = Some(String::new());
                    continue;
                }
                KeyCode::Char('n') => {
                    let from = self.views.last().map(|v| v.selected_index()).unwrap_or(0);
                    self.find_match(from, true);
                    continue;
                }
                KeyCode::Char('N') => {
                    let from = self.views.last().map(|v| v.selected_index()).unwrap_or(0);
                    self.find_match(from, false);
                    continue;
                }
                _ => {}
            }

            // 3. view handle_key
            let action = match self.views.last_mut() {
                Some(view) => view.handle_key(key),
                None => return Ok(()),
            };
            match action {
                Ok(ViewAction::None) => {}
                Ok(ViewAction::Push(v)) => self.views.push(v),
                Ok(ViewAction::Pop) => {
                    self.views.pop();
                    if self.views.is_empty() {
                        return Ok(());
                    }
                    if let Some(v) = self.views.last_mut() {
                        let _ = v.reload();
                    }
                }
                Ok(ViewAction::Quit) => return Ok(()),
                Ok(ViewAction::Refresh) => {
                    if let Some(view) = self.views.last_mut() {
                        if let Err(e) = view.reload() {
                            self.status_msg = format!("error: {e}");
                        }
                    }
                }
                Err(e) => {
                    // Never crash the TUI on a failed git command — show it.
                    self.status_msg = format!("error: {e}");
                }
            }
        }
    }
}
