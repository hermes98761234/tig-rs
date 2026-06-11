# tig-rs — Design Spec

Date: 2026-06-11
Status: Approved (autonomous session — recommended options chosen, alternatives recorded)

## What

`tig-rs` is a Rust rewrite of [tig](https://github.com/jonas/tig), the text-mode
interface for git. It is a terminal UI (TUI) that lets you browse a git
repository: commit history, diffs, the working-tree status, file trees, file
contents, and refs — navigating between views with single-key commands.

## Why

- Original tig is C + ncurses; a Rust rewrite gets memory safety, easy static
  binaries, and a modern TUI stack.
- Sibling projects (`tio-rs`, `wrk-rs`, `valkey-rs`, `svgo-rs`) follow the same
  pattern: Rust rewrite, GitHub repo under `hermes98761234`, CI + tag-driven
  release builds.

## Decisions (with rejected alternatives)

| Decision | Chosen | Rejected | Why |
|---|---|---|---|
| TUI stack | `ratatui` + `crossterm` | `cursive`, raw ncurses bindings | De facto standard, best docs/examples — important because implementation is delegated to weaker models |
| Git access | Shell out to `git` CLI, parse machine-readable output | `git2` (libgit2 bindings) | Matches tig's own architecture; smaller API surface; deterministic parsing with NUL/US separators; no native-lib build issues |
| Config | Hardcoded tig-default keybindings + colors | tigrc config language | YAGNI for v0.1; tigrc parsing is a large feature with little payoff |
| Scope v0.1 | main, diff, status, stage, tree, blob, refs, help, pager views | blame, stash, grep views, mouse support | Core daily-driver views first; the rest are additive later |
| Testing | Parser unit tests on captured git output + integration tests against temp git repos | TUI snapshot testing of every view | Parsers are where bugs live; snapshot tests are fragile for weaker models. A few `ratatui::backend::TestBackend` render tests are allowed but not required |

## Architecture

Single binary crate `tig-rs` (binary name `tig-rs`). Modules:

```
src/
  main.rs        CLI arg parsing (view subcommand, revision args), terminal setup/teardown, panic hook
  app.rs         App struct: view stack, event loop, global key dispatch
  git.rs         run_git() command runner + error type
  parse.rs       Pure parsers: commits, refs, status entries, tree entries, diff classification
  views/
    mod.rs       View trait + ViewAction enum
    main_view.rs Commit log (tig's "main view")
    diff.rs      Diff/show view
    status.rs    Working tree status view
    stage.rs     Stage view (per-file diff, hunk staging)
    tree.rs      Tree browser
    blob.rs      File contents
    refs.rs      Branches/tags/remotes
    help.rs      Keybinding help
    pager.rs     Generic scrollable text (also used for stdin pager mode)
  ui.rs          Shared drawing helpers: list rendering, status/title bar, diff line coloring
```

### Core abstractions

- **`View` trait**: `title()`, `draw(frame, area)`, `handle_key(key) -> ViewAction`,
  `reload()`. Each view owns its data and cursor/scroll state.
- **`ViewAction` enum**: `None`, `Push(Box<dyn View>)`, `Pop`, `Quit`, `Refresh`.
  The app maintains a stack of views; `Enter` typically pushes a child view
  (main → diff, status → stage, tree → blob/tree, refs → main filtered),
  `q` pops, popping the last view (or pressing `Q`) quits.
- **`run_git(args: &[&str]) -> Result<String>`**: runs `git` with
  `Command`, captures stdout as UTF-8 (lossy), returns stderr in the error.
  All views go through it. Working directory: process cwd (must be in a git repo;
  checked at startup with `git rev-parse --git-dir`).

### Git plumbing formats (exact)

- Log: `git log --format=%H%x1f%h%x1f%an%x1f%ad%x1f%D%x1f%s%x1e --date=short [revisions]`
  — fields split on `\x1f` (unit separator), records on `\x1e` (record separator).
- Refs: `git for-each-ref --format=%(refname)%x1f%(refname:short)%x1f%(objectname:short)%x1f%(contents:subject)`
- Status: `git status --porcelain=v2 -z` — NUL-separated entries; type `2`
  (rename) entries are followed by an extra NUL-separated original path.
- Tree: `git ls-tree <rev> -- [path]` — `<mode> <type> <oid>\t<name>` per line.
- Diff: raw `git show`/`git diff` text, displayed as-is and colorized by line
  prefix (`diff --git`, `index`, `---`/`+++`, `@@`, `+`, `-`, default).
- Hunk staging: build a minimal patch (file header + one hunk) and pipe to
  `git apply --cached [--reverse] -` .

### Keybindings (hardcoded, tig defaults subset)

Global: `j`/`k`/`Up`/`Down` move, `PageUp`/`PageDown`/`Ctrl-f`/`Ctrl-b` page,
`g`/`Home` first line, `G`/`End` last line, `Enter` open/child view, `q` close
view, `Q` quit, `R` refresh, `/` search, `n`/`N` next/prev match, `h` help,
`m` main view, `s` status view, `t` tree view, `r` refs view.
Status/stage views: `u` stage/unstage (file in status, hunk in stage), `!` checkout/revert file (with confirm).

### CLI

`tig-rs [log|show|status|refs|tree] [git log/show args…]`. No subcommand =
main view. If stdin is not a TTY, read stdin into the pager view (tig's pager
mode). `--version`/`-h` supported. Not a git repo → exit 1 with clear message.

### Error handling

- Startup: verify `git rev-parse --git-dir` succeeds, else friendly error.
- Any failing git command while running: show stderr in the status bar; never panic.
- Panic hook restores the terminal (disable raw mode, leave alternate screen)
  before printing the panic.

## Testing strategy

- `parse.rs` functions are pure (`&str -> Vec<T>`) — unit tests with literal
  fixture strings copied from real git output, including edge cases (empty
  repo, renames, detached HEAD, multi-parent commits, filenames with spaces).
- Integration tests (`tests/`) build a scratch repo in a tempdir via
  `std::process::Command` git calls, then exercise `run_git` + parsers
  end-to-end. Each test sets `GIT_AUTHOR_*`/`GIT_COMMITTER_*` env vars so
  commits work on CI.
- CI gate: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test`.

## Delivery

- Public GitHub repo `hermes98761234/tig-rs`, default branch `main`, pushed
  from the first scaffold task onward (every task ends with a push).
- CI workflow (fmt/clippy/test) + tag-driven release workflow producing
  Linux x86_64/aarch64 (gnu + musl) and macOS binaries with SHA256SUMS,
  adapted from `tio-rs` (no system deps needed — pure Rust crates only).
- v0.1.0 tagged once CI is green; README last.

## Execution

Implementation is delegated to Hermes kanban workers on board `tig-rs`,
one linear task chain (shared workspace `/home/user/projects/tig-rs` — no
parallel tasks, to avoid checkout clobbering). The implementation plan with
full task bodies lives at `docs/superpowers/plans/2026-06-11-tig-rs-plan.md`.
