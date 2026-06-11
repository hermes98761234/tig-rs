use anyhow::Result;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::git::run_git;
use crate::parse::{parse_refs, RefEntry, RefKind, REF_FORMAT};
use crate::ui::ListNav;
use crate::views::main_view::MainView;
use crate::views::{nav_delta, NavMove, View, ViewAction};

pub struct RefsView {
    refs: Vec<RefEntry>,
    nav: ListNav,
    page: usize,
}

impl RefsView {
    pub fn new() -> Result<Self> {
        let mut v = RefsView {
            refs: Vec::new(),
            nav: ListNav::default(),
            page: 1,
        };
        v.reload()?;
        Ok(v)
    }
}

impl View for RefsView {
    fn title(&self) -> String {
        format!("refs [{}/{}]", self.nav.selected + 1, self.refs.len())
    }

    fn reload(&mut self) -> Result<()> {
        let raw = run_git(&["for-each-ref", REF_FORMAT])?;
        self.refs = parse_refs(&raw);
        self.nav.clamp(self.refs.len());
        Ok(())
    }

    fn draw(&mut self, f: &mut Frame, area: Rect) {
        self.page = area.height as usize;
        let range = self.nav.visible(self.refs.len(), area.height as usize);
        let mut out: Vec<Line> = Vec::new();
        for i in range {
            let r = &self.refs[i];
            let (label, color) = match r.kind {
                RefKind::Branch => ("branch", Color::Green),
                RefKind::Remote => ("remote", Color::Yellow),
                RefKind::Tag => ("tag   ", Color::Magenta),
                RefKind::Other => ("other ", Color::DarkGray),
            };
            let mut line = Line::from(vec![
                Span::styled(format!("{label} "), Style::default().fg(color)),
                Span::styled(format!("{:<8} ", r.oid), Style::default().fg(Color::Blue)),
                Span::styled(format!("{:<28.28} ", r.short), Style::default().fg(color)),
                Span::raw(r.subject.clone()),
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
                NavMove::By(d) => self.nav.move_by(d, self.refs.len()),
                NavMove::Home => self.nav.home(),
                NavMove::End => self.nav.end(self.refs.len()),
            }
            return Ok(ViewAction::None);
        }
        match key.code {
            KeyCode::Char('R') => Ok(ViewAction::Refresh),
            KeyCode::Enter => {
                if let Some(r) = self.refs.get(self.nav.selected) {
                    let v = MainView::new(vec![r.short.clone()])?;
                    return Ok(ViewAction::Push(Box::new(v)));
                }
                Ok(ViewAction::None)
            }
            _ => Ok(ViewAction::None),
        }
    }

    fn text_lines(&self) -> Vec<String> {
        self.refs
            .iter()
            .map(|r| format!("{} {} {}", r.oid, r.short, r.subject))
            .collect()
    }

    fn select_line(&mut self, idx: usize) {
        self.nav.selected = idx.min(self.refs.len().saturating_sub(1));
    }

    fn selected_index(&self) -> usize {
        self.nav.selected
    }
}
