use std::io::{IsTerminal, Read};

use tig_rs::app::App;
use tig_rs::views::pager::PagerView;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("tig-rs {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
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

    let root: Box<dyn tig_rs::views::View> = match args.first().map(String::as_str) {
        None => Box::new(tig_rs::views::main_view::MainView::new(Vec::new())?),
        Some("log") => Box::new(tig_rs::views::main_view::MainView::new(args[1..].to_vec())?),
        Some("show") => {
            let rev = args.get(1).map(String::as_str).unwrap_or("HEAD");
            tig_rs::views::diff::commit_diff_view(rev)?
        }
        Some("status") => Box::new(tig_rs::views::status::StatusView::new()?),
        Some("refs") => Box::new(tig_rs::views::refs::RefsView::new()?),
        Some("tree") => {
            let rev = args.get(1).map(String::as_str).unwrap_or("HEAD");
            Box::new(tig_rs::views::tree::TreeView::new(rev, "")?)
        }
        Some(other) => {
            eprintln!("tig-rs: unknown command '{other}' (try --help)");
            std::process::exit(1);
        }
    };
    run_app(root)
}

fn print_help() {
    println!(
        "tig-rs {} — text-mode interface for git

USAGE:
  tig-rs [COMMAND] [ARGS]

COMMANDS:
  (none)            commit log of HEAD
  log [git args]    commit log with extra git-log arguments
  show [rev]        diff of one commit (default HEAD)
  status            working tree status
  refs              branches and tags
  tree [rev]        repository file tree (default HEAD)

Press 'h' inside the TUI for key bindings.",
        env!("CARGO_PKG_VERSION")
    );
}

fn run_app(root: Box<dyn tig_rs::views::View>) -> anyhow::Result<()> {
    let mut terminal = ratatui::init(); // installs panic hook + restore
    let result = App::new(root).run(&mut terminal);
    ratatui::restore();
    result
}
