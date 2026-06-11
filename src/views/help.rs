use crate::views::pager::PagerView;
use crate::views::View;

const HELP: &str = "\
tig-rs — key bindings

Global
  j / Down       move down          k / Up         move up
  PgDn / Ctrl-f  page down          PgUp / Ctrl-b  page up
  g / Home       first line         G / End        last line
  Enter          open / drill in    q              close view
  Q              quit               R              refresh view
  /              search             n / N          next / prev match
  h              this help

Views
  m              main (commit log)  s              status
  t              tree (HEAD)        r              refs

Main view        Enter/d: show commit diff
Refs view        Enter: log for ref
Tree view        Enter: descend / open blob
Status view      u: stage/unstage file   !: revert file (y to confirm)
                 Enter: stage view for file
Stage view       u: stage/unstage hunk under cursor (whole file if untracked)
";

pub fn help_view() -> Box<dyn View> {
    Box::new(PagerView::new("help", HELP))
}
