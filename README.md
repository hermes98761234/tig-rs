# tig-rs

**tig-rs — text-mode interface for git, written in Rust**

[![CI](https://github.com/hermes98761234/tig-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/hermes98761234/tig-rs/actions/workflows/ci.yml)

## Screenshots

tig-rs provides a terminal UI with the following views:

- **Main view** — scrollable commit log with author, date, refs, and subject
- **Diff view** — per-commit diff opened from the main view
- **Status view** — staged, unstaged, and untracked files
- **Stage view** — per-file diff with hunk-level staging
- **Tree view** — browsable repository file tree (directories and blobs)
- **Blob view** — file contents opened from the tree view
- **Refs view** — branches, remotes, and tags
- **Help view** — in-app key binding reference
- **Pager view** — scrollable text used for help and blob display

## Features

- **Main view** — commit log of HEAD, Enter opens diff
- **Diff view** — full diff for a single commit
- **Status view** — working tree status with stage/unstage/revert
- **Stage view** — per-file diff with hunk-level staging (`u` to stage/unstage individual hunks)
- **Tree view** — browse the git tree hierarchy, Enter descends into directories or opens blobs
- **Blob view** — display file contents from the tree
- **Refs view** — list all branches, remotes, and tags with color-coded kinds
- **Help view** — static key binding reference (press `h`)
- **Pager mode** — scrollable text viewer used for help and blob content
- **Search** — `/` to search across any view, `n`/`N` for next/previous match
- **CLI subcommands** — `log`, `show`, `status`, `refs`, `tree`, `--version`, `--help`

## Install

### From source

```
cargo install --git https://github.com/hermes98761234/tig-rs
```

### Pre-built binaries

Download from the [releases page](https://github.com/hermes98761234/tig-rs/releases/tag/v0.1.0):

| Target | Architecture |
|---|---|
| `tig-rs-x86_64-unknown-linux-gnu` | Linux x86_64 (glibc) |
| `tig-rs-x86_64-unknown-linux-musl` | Linux x86_64 (musl) |
| `tig-rs-aarch64-unknown-linux-musl` | Linux aarch64 (musl) |
| `tig-rs-x86_64-apple-darwin` | macOS x86_64 |
| `tig-rs-aarch64-apple-darwin` | macOS aarch64 (Apple Silicon) |

SHA-256 checksums are provided in `SHA256SUMS.txt` on the release page.

## Usage

```
tig-rs [COMMAND] [ARGS]

COMMANDS:
  (none)            commit log of HEAD
  log [git args]    commit log with extra git-log arguments
  show [rev]        diff of one commit (default HEAD)
  status            working tree status
  refs              branches and tags
  tree [rev]        repository file tree (default HEAD)
```

### Key bindings

Global

| Key | Action |
|---|---|
| `j` / `Down` | Move down |
| `k` / `Up` | Move up |
| `PgDn` / `Ctrl-f` | Page down |
| `PgUp` / `Ctrl-b` | Page up |
| `g` / `Home` | First line |
| `G` / `End` | Last line |
| `Enter` | Open / drill in |
| `q` | Close view |
| `Q` | Quit |
| `R` | Refresh view |
| `/` | Search |
| `n` / `N` | Next / previous match |
| `h` | This help |

Views

| Key | Action |
|---|---|
| `m` | Main (commit log) |
| `s` | Status |
| `t` | Tree (HEAD) |
| `r` | Refs |

Main view: `Enter`/`d` — show commit diff
Refs view: `Enter` — log for ref
Tree view: `Enter` — descend / open blob
Status view: `u` — stage/unstage file, `!` — revert file (confirm with `y`), `Enter` — stage view for file
Stage view: `u` — stage/unstage hunk under cursor (whole file if untracked)

## Architecture

tig-rs shells out to the `git` CLI with machine-readable output formats and parses the results in pure functions (`src/parse.rs`). A stack of `View` trait objects (`src/views/`) drives the UI: Enter pushes child views, `q` pops, `Q` quits. The terminal UI is rendered with [ratatui](https://github.com/ratatui/ratatui) 0.29. See the [design spec](docs/superpowers/specs/2026-06-11-tig-rs-design.md) and [implementation plan](docs/superpowers/plans/2026-06-11-tig-rs-plan.md) under `docs/superpowers/`.

## Development

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt
```

Tests use throwaway temporary-directory git repos — no repository state is required to run the test suite.

## License

GPL-2.0. tig-rs is a rewrite of [tig](https://github.com/jonas/tig) by Jonas Fonseca.
