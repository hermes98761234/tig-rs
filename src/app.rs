use anyhow::Result;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout};
use ratatui::DefaultTerminal;

use crate::ui::draw_status_bar;
use crate::views::{View, ViewAction};

pub struct App {
    views: Vec<Box<dyn View>>,
    status_msg: String,
}

impl App {
    pub fn new(root: Box<dyn View>) -> Self {
        App {
            views: vec![root],
            status_msg: String::from("h: help, q: close view, Q: quit"),
        }
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
                draw_status_bar(f, status, &title, &self.status_msg);
            })?;

            let Event::Key(key) = event::read()? else {
                continue;
            };
            if key.kind != KeyEventKind::Press {
                continue;
            }

            // Global keys first.
            match key.code {
                KeyCode::Char('Q') => return Ok(()),
                KeyCode::Char('q') => {
                    self.views.pop();
                    if self.views.is_empty() {
                        return Ok(());
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
                _ => {}
            }

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
