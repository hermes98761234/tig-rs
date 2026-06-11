use anyhow::Result;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::git::{run_git, run_git_stdin};
use crate::parse::extract_hunk;
use crate::ui::{diff_line_style, ListNav};
use crate::views::status::Section;
use crate::views::{nav_delta, NavMove, View, ViewAction};

pub struct StageView {
    section: Section,
    path: String,
    lines: Vec<String>,
    nav: ListNav,
    page: usize,
}

impl StageView {
    pub fn new(section: Section, path: &str) -> Result<Self> {
        let mut v = StageView {
            section,
            path: path.to_string(),
            lines: Vec::new(),
            nav: ListNav::default(),
            page: 1,
        };
        v.reload()?;
        Ok(v)
    }

    fn diff_text(&self) -> Result<String> {
        match self.section {
            Section::Staged => run_git(&["diff", "--cached", "--", &self.path]),
            Section::Unstaged => run_git(&["diff", "--", &self.path]),
            // Untracked files have no diff: show contents like a new-file.
            Section::Untracked => Ok(std::fs::read_to_string(&self.path).unwrap_or_default()),
        }
    }
}

impl View for StageView {
    fn title(&self) -> String {
        let what = match self.section {
            Section::Staged => "staged",
            Section::Unstaged => "unstaged",
            Section::Untracked => "untracked",
        };
        format!("stage: {} ({what}) — u: stage/unstage hunk", self.path)
    }

    fn reload(&mut self) -> Result<()> {
        let text = self.diff_text()?;
        self.lines = text.lines().map(|l| l.to_string()).collect();
        self.nav.clamp(self.lines.len());
        Ok(())
    }

    fn draw(&mut self, f: &mut Frame, area: Rect) {
        self.page = area.height as usize;
        let range = self.nav.visible(self.lines.len(), area.height as usize);
        let mut out: Vec<Line> = Vec::new();
        for i in range {
            let style = diff_line_style(&self.lines[i]);
            let style = if i == self.nav.selected {
                style.add_modifier(ratatui::style::Modifier::REVERSED)
            } else {
                style
            };
            out.push(Line::from(Span::styled(self.lines[i].clone(), style)));
        }
        f.render_widget(Paragraph::new(out), area);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<ViewAction> {
        if let Some(m) = nav_delta(&key, self.page) {
            match m {
                NavMove::By(d) => self.nav.move_by(d, self.lines.len()),
                NavMove::Home => self.nav.home(),
                NavMove::End => self.nav.end(self.lines.len()),
            }
            return Ok(ViewAction::None);
        }
        if key.code == KeyCode::Char('u') {
            match self.section {
                Section::Untracked => {
                    run_git(&["add", "--", &self.path])?;
                    return Ok(ViewAction::Pop); // file fully staged; back to status
                }
                Section::Unstaged => {
                    let diff = self.lines.join("\n") + "\n";
                    if let Some(patch) = extract_hunk(&diff, self.nav.selected) {
                        run_git_stdin(&["apply", "--cached", "-"], &patch)?;
                        self.reload()?;
                        if self.lines.is_empty() {
                            return Ok(ViewAction::Pop);
                        }
                    }
                }
                Section::Staged => {
                    let diff = self.lines.join("\n") + "\n";
                    if let Some(patch) = extract_hunk(&diff, self.nav.selected) {
                        run_git_stdin(&["apply", "--cached", "--reverse", "-"], &patch)?;
                        self.reload()?;
                        if self.lines.is_empty() {
                            return Ok(ViewAction::Pop);
                        }
                    }
                }
            }
            return Ok(ViewAction::None);
        }
        Ok(ViewAction::None)
    }

    fn text_lines(&self) -> Vec<String> {
        self.lines.clone()
    }

    fn select_line(&mut self, idx: usize) {
        self.nav.selected = idx.min(self.lines.len().saturating_sub(1));
    }
}
