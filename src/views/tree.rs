use anyhow::Result;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::git::run_git;
use crate::parse::{parse_tree, TreeEntry, TreeEntryKind};
use crate::ui::ListNav;
use crate::views::pager::PagerView;
use crate::views::{nav_delta, NavMove, View, ViewAction};

pub struct TreeView {
    rev: String,
    /// Path prefix inside the repo, "" for root, always ends with '/' if non-empty.
    prefix: String,
    entries: Vec<TreeEntry>,
    nav: ListNav,
    page: usize,
}

impl TreeView {
    pub fn new(rev: &str, prefix: &str) -> Result<Self> {
        let mut v = TreeView {
            rev: rev.to_string(),
            prefix: prefix.to_string(),
            entries: Vec::new(),
            nav: ListNav::default(),
            page: 1,
        };
        v.reload()?;
        Ok(v)
    }
}

impl View for TreeView {
    fn title(&self) -> String {
        format!(
            "tree: {}:/{} [{}/{}]",
            self.rev,
            self.prefix,
            self.nav.selected + 1,
            self.entries.len()
        )
    }

    fn reload(&mut self) -> Result<()> {
        let spec = format!("{}^{{tree}}", self.rev);
        let raw = if self.prefix.is_empty() {
            run_git(&["ls-tree", &spec])?
        } else {
            run_git(&["ls-tree", &spec, "--", &self.prefix])?
        };
        self.entries = parse_tree(&raw);
        self.nav.clamp(self.entries.len());
        Ok(())
    }

    fn draw(&mut self, f: &mut Frame, area: Rect) {
        self.page = area.height as usize;
        let range = self.nav.visible(self.entries.len(), area.height as usize);
        let mut out: Vec<Line> = Vec::new();
        for i in range {
            let e = &self.entries[i];
            let (marker, color) = match e.kind {
                TreeEntryKind::Tree => ("/", Color::Blue),
                TreeEntryKind::Blob => ("", Color::Reset),
                TreeEntryKind::Other => ("?", Color::DarkGray),
            };
            // ls-tree returns the full path when a prefix is given; show the
            // last component only.
            let display = e.name.rsplit('/').next().unwrap_or(&e.name).to_string();
            let mut line = Line::from(vec![
                Span::styled(format!("{} ", e.mode), Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{display}{marker}"), Style::default().fg(color)),
            ]);
            if i == self.nav.selected {
                line = line.style(Style::default().add_modifier(Modifier::REVERSED));
            }
            out.push(line);
        }
        f.render_widget(Paragraph::new(out), area);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<ViewAction> {
        if let Some(m) = nav_delta(&key, self.page) {
            match m {
                NavMove::By(d) => self.nav.move_by(d, self.entries.len()),
                NavMove::Home => self.nav.home(),
                NavMove::End => self.nav.end(self.entries.len()),
            }
            return Ok(ViewAction::None);
        }
        match key.code {
            KeyCode::Char('R') => Ok(ViewAction::Refresh),
            KeyCode::Enter => {
                let Some(e) = self.entries.get(self.nav.selected) else {
                    return Ok(ViewAction::None);
                };
                match e.kind {
                    TreeEntryKind::Tree => {
                        let v = TreeView::new(&self.rev, &format!("{}/", e.name))?;
                        Ok(ViewAction::Push(Box::new(v)))
                    }
                    TreeEntryKind::Blob => {
                        let spec = format!("{}:{}", self.rev, e.name);
                        let raw = run_git(&["show", &spec])?;
                        let v = PagerView::new(format!("blob {}", e.name), &raw);
                        Ok(ViewAction::Push(Box::new(v)))
                    }
                    TreeEntryKind::Other => Ok(ViewAction::None),
                }
            }
            _ => Ok(ViewAction::None),
        }
    }

    fn text_lines(&self) -> Vec<String> {
        self.entries.iter().map(|e| e.name.clone()).collect()
    }

    fn select_line(&mut self, idx: usize) {
        self.nav.selected = idx.min(self.entries.len().saturating_sub(1));
    }

    fn selected_index(&self) -> usize {
        self.nav.selected
    }
}
