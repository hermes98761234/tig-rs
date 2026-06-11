use std::io::{IsTerminal, Read};

use tig_rs::app::App;
use tig_rs::views::pager::PagerView;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("tig-rs {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    // Pager mode: data piped in on stdin.
    if !std::io::stdin().is_terminal() {
        let mut text = String::new();
        std::io::stdin().read_to_string(&mut text)?;
        return run_app(Box::new(PagerView::new("pager", &text)));
    }

    if !tig_rs::git::in_git_repo() {
        eprintln!("tig-rs: not a git repository (or any parent up to mount point)");
        std::process::exit(1);
    }

    // Temporary root view until the main view exists (Task 6):
    let log = tig_rs::git::run_git(&["log", "--oneline", "--decorate", "-100"])?;
    run_app(Box::new(PagerView::new("log", &log)))
}

fn run_app(root: Box<dyn tig_rs::views::View>) -> anyhow::Result<()> {
    let mut terminal = ratatui::init(); // installs panic hook + restore
    let result = App::new(root).run(&mut terminal);
    ratatui::restore();
    result
}
