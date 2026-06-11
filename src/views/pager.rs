use anyhow::Result;
use ratatui::crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::ui::{diff_line_style, ListNav};
use crate::views::{nav_delta, NavMove, View, ViewAction};

pub struct PagerView {
    title: String,
    lines: Vec<String>,
    nav: ListNav,
    page: usize,
}

impl PagerView {
    pub fn new(title: impl Into<String>, text: &str) -> Self {
        PagerView {
            title: title.into(),
            lines: text.lines().map(|l| l.to_string()).collect(),
            nav: ListNav::default(),
            page: 1,
        }
    }
}

impl View for PagerView {
    fn title(&self) -> String {
        format!(
            "{} [{}/{}]",
            self.title,
            self.nav.selected + 1,
            self.lines.len()
        )
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
