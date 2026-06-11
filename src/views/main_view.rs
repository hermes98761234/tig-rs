use anyhow::Result;
use ratatui::crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::git::run_git;
use crate::parse::{parse_commits, Commit, LOG_FORMAT};
use crate::ui::ListNav;
use crate::views::{nav_delta, NavMove, View, ViewAction};

pub struct MainView {
    /// Extra args appended to `git log` (revisions, paths). Empty = HEAD.
    rev_args: Vec<String>,
    commits: Vec<Commit>,
    nav: ListNav,
    page: usize,
}

impl MainView {
    pub fn new(rev_args: Vec<String>) -> Result<Self> {
        let mut v = MainView {
            rev_args,
            commits: Vec::new(),
            nav: ListNav::default(),
            page: 1,
        };
        v.reload()?;
        Ok(v)
    }

    fn selected_commit(&self) -> Option<&Commit> {
        self.commits.get(self.nav.selected)
    }
}

impl View for MainView {
    fn title(&self) -> String {
        let what = if self.rev_args.is_empty() {
            "HEAD".to_string()
        } else {
            self.rev_args.join(" ")
        };
        format!(
            "main: {} [{}/{}]",
            what,
            self.nav.selected + 1,
            self.commits.len()
        )
    }

    fn reload(&mut self) -> Result<()> {
        let mut args: Vec<&str> = vec!["log", LOG_FORMAT, "--date=short"];
        for a in &self.rev_args {
            args.push(a);
        }
        let raw = run_git(&args)?;
        self.commits = parse_commits(&raw);
        self.nav.clamp(self.commits.len());
        Ok(())
    }

    fn draw(&mut self, f: &mut Frame, area: Rect) {
        self.page = area.height as usize;
        let range = self.nav.visible(self.commits.len(), area.height as usize);
        let mut out: Vec<Line> = Vec::new();
        for i in range {
            let c = &self.commits[i];
            let mut spans = vec![
                Span::styled(c.date.clone(), Style::default().fg(Color::Blue)),
                Span::raw(" "),
                Span::styled(
                    format!("{:<16.16}", c.author),
                    Style::default().fg(Color::Green),
                ),
                Span::raw(" "),
            ];
            if !c.refs.is_empty() {
                spans.push(Span::styled(
                    format!("[{}] ", c.refs),
                    Style::default().fg(Color::Yellow),
                ));
            }
            spans.push(Span::raw(c.subject.clone()));
            let mut line = Line::from(spans);
            if i == self.nav.selected {
                line = line.style(Style::default().add_modifier(Modifier::REVERSED));
            }
            out.push(line);
        }
        f.render_widget(Paragraph::new(out), area);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<ViewAction> {
        use ratatui::crossterm::event::KeyCode;
        if let Some(m) = nav_delta(&key, self.page) {
            match m {
                NavMove::By(d) => self.nav.move_by(d, self.commits.len()),
                NavMove::Home => self.nav.home(),
                NavMove::End => self.nav.end(self.commits.len()),
            }
            return Ok(ViewAction::None);
        }
        match key.code {
            KeyCode::Char('R') => Ok(ViewAction::Refresh),
            KeyCode::Enter => {
                // Diff view arrives in Task 7. For now: no-op.
                let _ = self.selected_commit();
                Ok(ViewAction::None)
            }
            _ => Ok(ViewAction::None),
        }
    }

    fn text_lines(&self) -> Vec<String> {
        self.commits
            .iter()
            .map(|c| format!("{} {} {} {}", c.date, c.author, c.refs, c.subject))
            .collect()
    }

    fn select_line(&mut self, idx: usize) {
        self.nav.selected = idx.min(self.commits.len().saturating_sub(1));
    }
}
