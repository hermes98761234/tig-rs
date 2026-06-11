use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::Frame;

/// Cursor + scroll state for a list of `len` items rendered in a viewport.
#[derive(Debug, Default, Clone)]
pub struct ListNav {
    pub selected: usize,
    pub offset: usize,
}

impl ListNav {
    pub fn clamp(&mut self, len: usize) {
        if len == 0 {
            self.selected = 0;
            self.offset = 0;
            return;
        }
        self.selected = self.selected.min(len - 1);
        self.offset = self.offset.min(self.selected);
    }

    pub fn move_by(&mut self, delta: isize, len: usize) {
        if len == 0 {
            return;
        }
        let new = self.selected as isize + delta;
        self.selected = new.clamp(0, len as isize - 1) as usize;
    }

    pub fn home(&mut self) {
        self.selected = 0;
    }

    pub fn end(&mut self, len: usize) {
        self.selected = len.saturating_sub(1);
    }

    /// Call during draw with the viewport height; keeps selection visible
    /// and returns the range of indices to render.
    pub fn visible(&mut self, len: usize, height: usize) -> std::ops::Range<usize> {
        if height == 0 || len == 0 {
            return 0..0;
        }
        self.clamp(len);
        if self.selected < self.offset {
            self.offset = self.selected;
        }
        if self.selected >= self.offset + height {
            self.offset = self.selected + 1 - height;
        }
        self.offset..len.min(self.offset + height)
    }
}

/// Bottom status bar: view title on the left, message on the right.
pub fn draw_status_bar(f: &mut Frame, area: Rect, title: &str, msg: &str) {
    let style = Style::default().fg(Color::Black).bg(Color::Cyan);
    let text = format!(" {title} — {msg}");
    let mut line = Line::from(Span::styled(text, style));
    line = line.style(style);
    f.render_widget(ratatui::widgets::Paragraph::new(line).style(style), area);
}

/// Style a diff/pager line by its prefix (used by pager, diff, stage views).
pub fn diff_line_style(line: &str) -> Style {
    if line.starts_with("diff --git")
        || line.starts_with("index ")
        || line.starts_with("---")
        || line.starts_with("+++")
    {
        Style::default().fg(Color::Yellow)
    } else if line.starts_with("@@") {
        Style::default().fg(Color::Magenta)
    } else if line.starts_with('+') {
        Style::default().fg(Color::Green)
    } else if line.starts_with('-') {
        Style::default().fg(Color::Red)
    } else if line.starts_with("commit ") {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    }
}
