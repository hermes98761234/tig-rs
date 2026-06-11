use anyhow::Result;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::git::run_git;
use crate::parse::{parse_status, StatusEntry};
use crate::ui::ListNav;
use crate::views::{nav_delta, NavMove, View, ViewAction};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Staged,
    Unstaged,
    Untracked,
}

pub enum Row {
    Header(&'static str),
    File(Section, StatusEntry),
}

pub struct StatusView {
    rows: Vec<Row>,
    nav: ListNav,
    page: usize,
    /// Some(path) while waiting for y/n confirmation of `!` revert.
    pending_revert: Option<(Section, String)>,
}

impl StatusView {
    pub fn new() -> Result<Self> {
        let mut v = StatusView {
            rows: Vec::new(),
            nav: ListNav::default(),
            page: 1,
            pending_revert: None,
        };
        v.reload()?;
        Ok(v)
    }

    fn selected_file(&self) -> Option<(Section, &StatusEntry)> {
        match self.rows.get(self.nav.selected) {
            Some(Row::File(s, e)) => Some((*s, e)),
            _ => None,
        }
    }
}

impl View for StatusView {
    fn title(&self) -> String {
        if let Some((_, path)) = &self.pending_revert {
            return format!("status — revert {path}? press y to confirm, any other key to cancel");
        }
        let files = self
            .rows
            .iter()
            .filter(|r| matches!(r, Row::File(..)))
            .count();
        format!("status [{files} files]")
    }

    fn reload(&mut self) -> Result<()> {
        let raw = run_git(&["status", "--porcelain=v2", "-z"])?;
        let entries = parse_status(&raw);
        let mut staged: Vec<StatusEntry> = Vec::new();
        let mut unstaged: Vec<StatusEntry> = Vec::new();
        let mut untracked: Vec<StatusEntry> = Vec::new();
        for e in entries {
            if e.untracked {
                untracked.push(e);
                continue;
            }
            // One file can appear in both sections (e.g. "MM").
            if e.staged != '.' {
                staged.push(e.clone());
            }
            if e.unstaged != '.' {
                unstaged.push(e);
            }
        }
        self.rows.clear();
        self.rows.push(Row::Header("Changes to be committed:"));
        for e in staged {
            self.rows.push(Row::File(Section::Staged, e));
        }
        self.rows.push(Row::Header("Changes not staged:"));
        for e in unstaged {
            self.rows.push(Row::File(Section::Unstaged, e));
        }
        self.rows.push(Row::Header("Untracked files:"));
        for e in untracked {
            self.rows.push(Row::File(Section::Untracked, e));
        }
        self.nav.clamp(self.rows.len());
        Ok(())
    }

    fn draw(&mut self, f: &mut Frame, area: Rect) {
        self.page = area.height as usize;
        let range = self.nav.visible(self.rows.len(), area.height as usize);
        let mut out: Vec<Line> = Vec::new();
        for i in range {
            let mut line = match &self.rows[i] {
                Row::Header(h) => Line::from(Span::styled(
                    h.to_string(),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )),
                Row::File(section, e) => {
                    let status_char = match section {
                        Section::Staged => e.staged,
                        Section::Unstaged => e.unstaged,
                        Section::Untracked => '?',
                    };
                    let color = match section {
                        Section::Staged => Color::Green,
                        Section::Unstaged => Color::Red,
                        Section::Untracked => Color::Magenta,
                    };
                    let name = match &e.orig_path {
                        Some(orig) => format!("{} -> {}", orig, e.path),
                        None => e.path.clone(),
                    };
                    Line::from(vec![
                        Span::styled(format!("  {status_char} "), Style::default().fg(color)),
                        Span::raw(name),
                    ])
                }
            };
            if i == self.nav.selected {
                line = line.style(Style::default().add_modifier(Modifier::REVERSED));
            }
            out.push(line);
        }
        f.render_widget(Paragraph::new(out), area);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<ViewAction> {
        // Pending y/n confirmation for revert?
        if let Some((section, path)) = self.pending_revert.take() {
            if key.code == KeyCode::Char('y') {
                match section {
                    Section::Staged | Section::Unstaged => {
                        run_git(&["checkout", "HEAD", "--", &path])?;
                    }
                    Section::Untracked => {
                        std::fs::remove_file(&path)?;
                    }
                }
                self.reload()?;
            }
            return Ok(ViewAction::None);
        }

        if let Some(m) = nav_delta(&key, self.page) {
            match m {
                NavMove::By(d) => self.nav.move_by(d, self.rows.len()),
                NavMove::Home => self.nav.home(),
                NavMove::End => self.nav.end(self.rows.len()),
            }
            return Ok(ViewAction::None);
        }
        match key.code {
            KeyCode::Char('R') => Ok(ViewAction::Refresh),
            KeyCode::Char('u') => {
                if let Some((section, e)) = self.selected_file() {
                    let path = e.path.clone();
                    match section {
                        Section::Staged => {
                            run_git(&["restore", "--staged", "--", &path])?;
                        }
                        Section::Unstaged | Section::Untracked => {
                            run_git(&["add", "--", &path])?;
                        }
                    }
                    self.reload()?;
                }
                Ok(ViewAction::None)
            }
            KeyCode::Char('!') => {
                if let Some((section, e)) = self.selected_file() {
                    self.pending_revert = Some((section, e.path.clone()));
                }
                Ok(ViewAction::None)
            }
            KeyCode::Enter => {
                if let Some((section, e)) = self.selected_file() {
                    let v = crate::views::stage::StageView::new(section, &e.path)?;
                    return Ok(ViewAction::Push(Box::new(v)));
                }
                Ok(ViewAction::None)
            }
            _ => Ok(ViewAction::None),
        }
    }

    fn text_lines(&self) -> Vec<String> {
        self.rows
            .iter()
            .map(|r| match r {
                Row::Header(h) => h.to_string(),
                Row::File(_, e) => e.path.clone(),
            })
            .collect()
    }

    fn select_line(&mut self, idx: usize) {
        self.nav.selected = idx.min(self.rows.len().saturating_sub(1));
    }
}
