pub mod diff;
pub mod main_view;
pub mod pager;
pub mod refs;
pub mod tree;

use anyhow::Result;
use ratatui::crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;

pub enum ViewAction {
    None,
    Push(Box<dyn View>),
    Pop,
    Quit,
    /// Reload the current view's data (e.g. after staging a file).
    Refresh,
}

pub trait View {
    fn title(&self) -> String;
    fn draw(&mut self, f: &mut Frame, area: Rect);
    fn handle_key(&mut self, key: KeyEvent) -> Result<ViewAction>;
    fn reload(&mut self) -> Result<()> {
        Ok(())
    }
    /// Plain-text content used by `/` search (Task 11). Default: nothing.
    fn text_lines(&self) -> Vec<String> {
        Vec::new()
    }
    /// Jump selection/scroll to line `idx` (search match). Default: ignore.
    fn select_line(&mut self, _idx: usize) {}
}

/// Standard movement keys shared by all list-like views.
/// Returns Some(delta_or_jump) handled, None if the key is not a move key.
pub fn nav_delta(key: &KeyEvent, page: usize) -> Option<NavMove> {
    use ratatui::crossterm::event::{KeyCode, KeyModifiers};
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    Some(match key.code {
        KeyCode::Char('j') | KeyCode::Down => NavMove::By(1),
        KeyCode::Char('k') | KeyCode::Up => NavMove::By(-1),
        KeyCode::PageDown => NavMove::By(page as isize),
        KeyCode::PageUp => NavMove::By(-(page as isize)),
        KeyCode::Char('f') if ctrl => NavMove::By(page as isize),
        KeyCode::Char('b') if ctrl => NavMove::By(-(page as isize)),
        KeyCode::Char('g') | KeyCode::Home => NavMove::Home,
        KeyCode::Char('G') | KeyCode::End => NavMove::End,
        _ => return None,
    })
}

pub enum NavMove {
    By(isize),
    Home,
    End,
}
