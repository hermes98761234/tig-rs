# tig-rs Implementation Plan

> **For agentic workers:** This plan is executed via Hermes kanban tasks (board `tig-rs`), one task per section below. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `tig-rs`, a Rust rewrite of tig (text-mode interface for git) with main/diff/status/stage/tree/blob/refs/help/pager views.

**Architecture:** Single binary. All git data comes from shelling out to the `git` CLI with machine-readable formats (NUL / unit-separator delimited) and parsing in pure functions. A stack of `View` trait objects drives the UI; `Enter` pushes child views, `q` pops, `Q` quits. Rendering via ratatui.

**Tech Stack:** Rust 2021, ratatui 0.29 (re-exports crossterm — do NOT add a separate crossterm dependency), anyhow 1.

**Spec:** `docs/superpowers/specs/2026-06-11-tig-rs-design.md` (in this repo).

**Rules for every task:**
- Work dir: `/home/user/projects/tig-rs`. Never work elsewhere.
- Before claiming done: `cargo fmt`, then `cargo clippy --all-targets -- -D warnings` (fix ALL warnings), then `cargo test` (all green).
- Every task ends with `git add -A && git commit -m "<given message>" && git push origin main`.
- Match struct/function names EXACTLY as written here — later tasks depend on them.

---

### Task 1: Scaffold crate + GitHub repo

**Files:**
- Create: `Cargo.toml`, `src/main.rs`, `.gitignore`

- [ ] **Step 1: Scaffold**

In `/home/user/projects/tig-rs` (git repo already initialized, branch `main`, contains `docs/`):

```bash
cd /home/user/projects/tig-rs
cargo init --name tig-rs
```

Replace `Cargo.toml` with:

```toml
[package]
name = "tig-rs"
version = "0.1.0"
edition = "2021"
description = "Text-mode interface for git, in Rust (tig rewrite)"
license = "GPL-2.0"
repository = "https://github.com/hermes98761234/tig-rs"

[dependencies]
ratatui = "0.29"
anyhow = "1"

[profile.release]
strip = true
lto = true
```

Replace `src/main.rs` with:

```rust
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("tig-rs {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    println!("tig-rs: TUI not implemented yet");
}
```

Ensure `.gitignore` contains:

```
/target
```

- [ ] **Step 2: Verify build**

```bash
cargo build && cargo run -- --version
```

Expected output ends with: `tig-rs 0.1.0`

- [ ] **Step 3: Format, lint, commit**

```bash
cargo fmt && cargo clippy --all-targets -- -D warnings
git add -A && git commit -m "feat: scaffold tig-rs crate"
```

- [ ] **Step 4: Create the GitHub repo and push**

```bash
gh repo create tig-rs --public --description '📟 tig rewritten in Rust — text-mode interface for git' --source . --remote origin
gh repo edit hermes98761234/tig-rs --add-topic rust --add-topic git --add-topic tui --add-topic tig --add-topic ratatui
git push -u origin main
gh repo view hermes98761234/tig-rs --json url -q .url
```

Expected: prints `https://github.com/hermes98761234/tig-rs`. Report this URL.

---

### Task 2: git command runner + test-repo harness

**Files:**
- Create: `src/git.rs`
- Modify: `src/main.rs` (add `mod git;`)
- Test: `tests/git_runner.rs`, `tests/common/mod.rs`

- [ ] **Step 1: Write the failing tests**

Create `tests/common/mod.rs` — a scratch-repo harness reused by ALL later integration tests:

```rust
#![allow(dead_code)]
use std::path::{Path, PathBuf};
use std::process::Command;

/// A throwaway git repo in a temp dir. Deleted on drop.
pub struct TestRepo {
    pub dir: PathBuf,
}

impl TestRepo {
    pub fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "tig-rs-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let repo = TestRepo { dir };
        repo.git(&["init", "-b", "main"]);
        repo.git(&["config", "user.name", "Test User"]);
        repo.git(&["config", "user.email", "test@example.com"]);
        repo
    }

    /// Run git in the repo dir, panic on failure, return stdout.
    pub fn git(&self, args: &[&str]) -> String {
        let out = Command::new("git")
            .args(args)
            .current_dir(&self.dir)
            .env("GIT_AUTHOR_DATE", "2026-01-02T03:04:05+00:00")
            .env("GIT_COMMITTER_DATE", "2026-01-02T03:04:05+00:00")
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    pub fn write(&self, rel: &str, content: &str) {
        let p = self.dir.join(rel);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(p, content).unwrap();
    }

    pub fn commit_file(&self, rel: &str, content: &str, msg: &str) {
        self.write(rel, content);
        self.git(&["add", rel]);
        self.git(&["commit", "-m", msg]);
    }

    pub fn path(&self) -> &Path {
        &self.dir
    }
}

impl Drop for TestRepo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}
```

Create `tests/git_runner.rs`:

```rust
mod common;
use common::TestRepo;

#[test]
fn run_git_in_succeeds() {
    let repo = TestRepo::new();
    repo.commit_file("a.txt", "hello\n", "first");
    let out = tig_rs::git::run_git_in(repo.path(), &["log", "--format=%s"]).unwrap();
    assert_eq!(out.trim(), "first");
}

#[test]
fn run_git_in_reports_stderr_on_failure() {
    let repo = TestRepo::new();
    let err = tig_rs::git::run_git_in(repo.path(), &["log"]).unwrap_err();
    let msg = format!("{err}");
    assert!(msg.contains("git log failed"), "got: {msg}");
}
```

- [ ] **Step 2: Make the crate a lib+bin so tests can import it**

Integration tests import `tig_rs::...`, so the crate needs a library target.
Create `src/lib.rs`:

```rust
pub mod git;
```

In `src/main.rs` do NOT add `mod git;` — instead the binary will use the lib (`use tig_rs::...`) starting in Task 5. Leave `main.rs` unchanged for now.

- [ ] **Step 3: Run tests to verify they fail**

```bash
cargo test
```

Expected: compile error — `git` module missing.

- [ ] **Step 4: Implement `src/git.rs`**

```rust
use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};

/// Run `git <args>` in the current directory. Returns stdout (lossy UTF-8).
pub fn run_git(args: &[&str]) -> Result<String> {
    run_git_in(Path::new("."), args)
}

/// Run `git <args>` in `dir`. On non-zero exit, the error contains stderr.
pub fn run_git_in(dir: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .context("failed to spawn git — is git installed?")?;
    if !out.status.success() {
        bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Run `git <args>` feeding `input` to stdin (used by `git apply --cached -`).
pub fn run_git_stdin(args: &[&str], input: &str) -> Result<String> {
    use std::io::Write;
    let mut child = Command::new("git")
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to spawn git")?;
    child
        .stdin
        .as_mut()
        .expect("stdin piped")
        .write_all(input.as_bytes())?;
    let out = child.wait_with_output()?;
    if !out.status.success() {
        bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// True if the current directory is inside a git repository.
pub fn in_git_repo() -> bool {
    Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cargo test
```

Expected: `2 passed` (plus 0 from lib).

- [ ] **Step 6: Format, lint, commit, push**

```bash
cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test
git add -A && git commit -m "feat: git command runner with test-repo harness" && git push origin main
```

---

### Task 3: Commit log parser

**Files:**
- Create: `src/parse.rs`
- Modify: `src/lib.rs` (add `pub mod parse;`)
- Test: unit tests inside `src/parse.rs`, plus `tests/parse_log.rs`

The main view runs:

```
git log --format=%H%x1f%h%x1f%an%x1f%ad%x1f%D%x1f%s%x1e --date=short
```

Fields are separated by `\x1f` (unit separator), records by `\x1e` (record separator). `%D` is the ref decoration (e.g. `HEAD -> main, tag: v1.0`), empty for most commits.

- [ ] **Step 1: Write the failing unit tests**

Create `src/parse.rs` containing ONLY the tests for now:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_two_commits() {
        let raw = "aaaa1111\x1faaaa111\x1fAlice\x1f2026-01-02\x1fHEAD -> main, tag: v1.0\x1ffirst subject\x1e\
                   bbbb2222\x1fbbbb222\x1fBob Bobson\x1f2026-01-01\x1f\x1fsecond: with \x1f-free text\x1e";
        let commits = parse_commits(raw);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].id, "aaaa1111");
        assert_eq!(commits[0].short_id, "aaaa111");
        assert_eq!(commits[0].author, "Alice");
        assert_eq!(commits[0].date, "2026-01-02");
        assert_eq!(commits[0].refs, "HEAD -> main, tag: v1.0");
        assert_eq!(commits[0].subject, "first subject");
        assert_eq!(commits[1].refs, "");
        assert_eq!(commits[1].subject, "second: with \x1f-free text");
    }

    #[test]
    fn empty_input_gives_no_commits() {
        assert!(parse_commits("").is_empty());
        assert!(parse_commits("\n").is_empty());
    }

    #[test]
    fn skips_malformed_records() {
        // A record with too few fields must be skipped, not panic.
        let raw = "only-two-fields\x1foops\x1e";
        assert!(parse_commits(raw).is_empty());
    }
}
```

Note in the first test: the subject itself may contain anything except the separators; `parse_commits` must split each record into AT MOST 6 fields (use `splitn(6, '\x1f')`) so a subject containing `\x1f` would still terminate the record — the test's second record exercises `splitn` by including `\x1f` in the subject, which lands in field 6 because `splitn(6, ..)` stops splitting.

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test parse
```

Expected: compile error — `parse_commits` / `Commit` not defined.

- [ ] **Step 3: Implement above the tests in `src/parse.rs`**

```rust
/// One commit from `git log --format=%H%x1f%h%x1f%an%x1f%ad%x1f%D%x1f%s%x1e`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commit {
    pub id: String,
    pub short_id: String,
    pub author: String,
    pub date: String,
    pub refs: String,
    pub subject: String,
}

/// The exact git log format string that `parse_commits` understands.
pub const LOG_FORMAT: &str = "--format=%H%x1f%h%x1f%an%x1f%ad%x1f%D%x1f%s%x1e";

pub fn parse_commits(raw: &str) -> Vec<Commit> {
    raw.split('\x1e')
        .filter_map(|record| {
            let record = record.trim_start_matches(['\n', '\r']);
            if record.is_empty() {
                return None;
            }
            let f: Vec<&str> = record.splitn(6, '\x1f').collect();
            if f.len() != 6 {
                return None;
            }
            Some(Commit {
                id: f[0].to_string(),
                short_id: f[1].to_string(),
                author: f[2].to_string(),
                date: f[3].to_string(),
                refs: f[4].to_string(),
                subject: f[5].to_string(),
            })
        })
        .collect()
}
```

Add `pub mod parse;` to `src/lib.rs`.

- [ ] **Step 4: Run unit tests to verify they pass**

```bash
cargo test parse
```

Expected: `3 passed`.

- [ ] **Step 5: Add a live integration test**

Create `tests/parse_log.rs`:

```rust
mod common;
use common::TestRepo;
use tig_rs::{git, parse};

#[test]
fn parses_real_git_log() {
    let repo = TestRepo::new();
    repo.commit_file("a.txt", "one\n", "first commit");
    repo.commit_file("a.txt", "two\n", "second commit");
    repo.git(&["tag", "v1.0"]);
    let raw = git::run_git_in(
        repo.path(),
        &["log", parse::LOG_FORMAT, "--date=short"],
    )
    .unwrap();
    let commits = parse::parse_commits(&raw);
    assert_eq!(commits.len(), 2);
    assert_eq!(commits[0].subject, "second commit");
    assert!(commits[0].refs.contains("tag: v1.0"));
    assert_eq!(commits[1].subject, "first commit");
    assert_eq!(commits[0].date, "2026-01-02");
}
```

- [ ] **Step 6: Run all tests, format, lint, commit, push**

```bash
cargo test && cargo fmt && cargo clippy --all-targets -- -D warnings
git add -A && git commit -m "feat: commit log parser" && git push origin main
```

---

### Task 4: Refs, status, and tree parsers

**Files:**
- Modify: `src/parse.rs` (append; do not touch `Commit`/`parse_commits`)
- Test: unit tests in `src/parse.rs`, plus `tests/parse_live.rs`

- [ ] **Step 1: Write the failing unit tests** (append to the `tests` module in `src/parse.rs`)

```rust
    #[test]
    fn parses_refs() {
        let raw = "refs/heads/main\x1fmain\x1fabc1234\x1ftip subject\n\
                   refs/remotes/origin/main\x1forigin/main\x1fabc1234\x1ftip subject\n\
                   refs/tags/v1.0\x1fv1.0\x1fdef5678\x1ftagged subject\n\
                   refs/stash\x1fstash\x1f111aaaa\x1fwip\n";
        let refs = parse_refs(raw);
        assert_eq!(refs.len(), 4);
        assert_eq!(refs[0].short, "main");
        assert_eq!(refs[0].kind, RefKind::Branch);
        assert_eq!(refs[1].kind, RefKind::Remote);
        assert_eq!(refs[2].kind, RefKind::Tag);
        assert_eq!(refs[3].kind, RefKind::Other);
        assert_eq!(refs[2].oid, "def5678");
    }

    #[test]
    fn parses_status_porcelain_v2() {
        // git status --porcelain=v2 -z output. Entries NUL-terminated.
        // '1' = ordinary change, '2' = rename (extra NUL-separated origPath
        // FOLLOWS the entry), '?' = untracked, 'u' = unmerged (treat like '1').
        let raw = "1 M. N... 100644 100644 100644 aaa bbb staged.txt\0\
                   1 .M N... 100644 100644 100644 aaa bbb unstaged.txt\0\
                   2 R. N... 100644 100644 100644 aaa bbb R100 new name.txt\0old name.txt\0\
                   ? untracked file.txt\0";
        let entries = parse_status(raw);
        assert_eq!(entries.len(), 4);

        assert_eq!(entries[0].path, "staged.txt");
        assert_eq!(entries[0].staged, 'M');
        assert_eq!(entries[0].unstaged, '.');
        assert!(!entries[0].untracked);

        assert_eq!(entries[1].staged, '.');
        assert_eq!(entries[1].unstaged, 'M');

        assert_eq!(entries[2].path, "new name.txt");
        assert_eq!(entries[2].orig_path.as_deref(), Some("old name.txt"));
        assert_eq!(entries[2].staged, 'R');

        assert_eq!(entries[3].path, "untracked file.txt");
        assert!(entries[3].untracked);
    }

    #[test]
    fn parses_tree() {
        let raw = "100644 blob e69de29bb2d1d6434b8b29ae775ad8c2e48c5391\tREADME.md\n\
                   040000 tree d564d0bc3dd917926892c55e3706cc116d5b165e\tsrc\n\
                   120000 blob 473a0f4c3be8a93681a267e3b1e9a7dcda1185436\tlink name\n";
        let entries = parse_tree(raw);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].name, "README.md");
        assert_eq!(entries[0].kind, TreeEntryKind::Blob);
        assert_eq!(entries[1].kind, TreeEntryKind::Tree);
        assert_eq!(entries[1].name, "src");
        assert_eq!(entries[2].name, "link name");
        assert_eq!(entries[0].mode, "100644");
    }
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test parse
```

Expected: compile errors for the new types.

- [ ] **Step 3: Implement** (append to `src/parse.rs` above the tests)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefKind {
    Branch,
    Remote,
    Tag,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefEntry {
    pub full: String,
    pub short: String,
    pub oid: String,
    pub subject: String,
    pub kind: RefKind,
}

/// Format for `git for-each-ref`:
pub const REF_FORMAT: &str =
    "--format=%(refname)\x1f%(refname:short)\x1f%(objectname:short)\x1f%(contents:subject)";

pub fn parse_refs(raw: &str) -> Vec<RefEntry> {
    raw.lines()
        .filter_map(|line| {
            let f: Vec<&str> = line.splitn(4, '\x1f').collect();
            if f.len() != 4 {
                return None;
            }
            let kind = if f[0].starts_with("refs/heads/") {
                RefKind::Branch
            } else if f[0].starts_with("refs/remotes/") {
                RefKind::Remote
            } else if f[0].starts_with("refs/tags/") {
                RefKind::Tag
            } else {
                RefKind::Other
            };
            Some(RefEntry {
                full: f[0].to_string(),
                short: f[1].to_string(),
                oid: f[2].to_string(),
                subject: f[3].to_string(),
                kind,
            })
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusEntry {
    /// Index (staged) state: 'M', 'A', 'D', 'R', 'C', '.', etc.
    pub staged: char,
    /// Worktree (unstaged) state.
    pub unstaged: char,
    pub path: String,
    /// Original path for renames.
    pub orig_path: Option<String>,
    pub untracked: bool,
}

/// Parse `git status --porcelain=v2 -z` output.
/// Entry types: '1' ordinary, '2' rename/copy (followed by an extra
/// NUL-separated original path), 'u' unmerged, '?' untracked, '!' ignored
/// (ignored entries are skipped). With -z there is no quoting of paths.
pub fn parse_status(raw: &str) -> Vec<StatusEntry> {
    let mut out = Vec::new();
    let mut parts = raw.split('\0');
    while let Some(entry) = parts.next() {
        if entry.is_empty() {
            continue;
        }
        let kind = entry.chars().next().unwrap();
        match kind {
            '1' => {
                // "1 XY sub mH mI mW hH hI path" — split into exactly 9
                // chunks so the path (field index 8) keeps its spaces.
                let f: Vec<&str> = entry.splitn(9, ' ').collect();
                if f.len() != 9 {
                    continue;
                }
                let mut chars = f[1].chars();
                out.push(StatusEntry {
                    staged: chars.next().unwrap_or('.'),
                    unstaged: chars.next().unwrap_or('.'),
                    path: f[8].to_string(),
                    orig_path: None,
                    untracked: false,
                });
            }
            'u' => {
                // unmerged: "u XY sub m1 m2 m3 h1 h2 h3 path" — 11 chunks,
                // path is field index 10.
                let f: Vec<&str> = entry.splitn(11, ' ').collect();
                if f.len() != 11 {
                    continue;
                }
                let mut chars = f[1].chars();
                out.push(StatusEntry {
                    staged: chars.next().unwrap_or('.'),
                    unstaged: chars.next().unwrap_or('.'),
                    path: f[10].to_string(),
                    orig_path: None,
                    untracked: false,
                });
            }
            '2' => {
                // "2 XY sub mH mI mW hH hI Xscore path" then NUL origPath
                let f: Vec<&str> = entry.splitn(10, ' ').collect();
                if f.len() != 10 {
                    continue;
                }
                let xy = f[1];
                let orig = parts.next().unwrap_or("").to_string();
                let mut chars = xy.chars();
                out.push(StatusEntry {
                    staged: chars.next().unwrap_or('.'),
                    unstaged: chars.next().unwrap_or('.'),
                    path: f[9].to_string(),
                    orig_path: Some(orig),
                    untracked: false,
                });
            }
            '?' => {
                out.push(StatusEntry {
                    staged: '.',
                    unstaged: '.',
                    path: entry[2..].to_string(),
                    orig_path: None,
                    untracked: true,
                });
            }
            _ => {} // '#' branch headers, '!' ignored — skip
        }
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeEntryKind {
    Blob,
    Tree,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeEntry {
    pub mode: String,
    pub kind: TreeEntryKind,
    pub oid: String,
    pub name: String,
}

/// Parse `git ls-tree <rev> -- [path]`: "<mode> <type> <oid>\t<name>".
pub fn parse_tree(raw: &str) -> Vec<TreeEntry> {
    raw.lines()
        .filter_map(|line| {
            let (meta, name) = line.split_once('\t')?;
            let f: Vec<&str> = meta.split_whitespace().collect();
            if f.len() != 3 {
                return None;
            }
            let kind = match f[1] {
                "blob" => TreeEntryKind::Blob,
                "tree" => TreeEntryKind::Tree,
                _ => TreeEntryKind::Other,
            };
            Some(TreeEntry {
                mode: f[0].to_string(),
                kind,
                oid: f[2].to_string(),
                name: name.to_string(),
            })
        })
        .collect()
}
```

- [ ] **Step 4: Run unit tests**

```bash
cargo test parse
```

Expected: all parse tests pass (6 total now).

- [ ] **Step 5: Live integration test**

Create `tests/parse_live.rs`:

```rust
mod common;
use common::TestRepo;
use tig_rs::{git, parse};

#[test]
fn live_refs_status_tree() {
    let repo = TestRepo::new();
    repo.commit_file("README.md", "hi\n", "init");
    repo.write("src/main.rs", "fn main() {}\n");
    repo.git(&["add", "src/main.rs"]);
    repo.git(&["commit", "-m", "add src"]);
    repo.git(&["tag", "v0.1"]);

    // refs
    let raw = git::run_git_in(repo.path(), &["for-each-ref", parse::REF_FORMAT]).unwrap();
    let refs = parse::parse_refs(&raw);
    assert!(refs.iter().any(|r| r.short == "main" && r.kind == parse::RefKind::Branch));
    assert!(refs.iter().any(|r| r.short == "v0.1" && r.kind == parse::RefKind::Tag));

    // status: one staged, one modified, one untracked
    repo.write("staged.txt", "s\n");
    repo.git(&["add", "staged.txt"]);
    repo.write("README.md", "changed\n");
    repo.write("untracked.txt", "u\n");
    let raw = git::run_git_in(repo.path(), &["status", "--porcelain=v2", "-z"]).unwrap();
    let st = parse::parse_status(&raw);
    assert!(st.iter().any(|e| e.path == "staged.txt" && e.staged == 'A'));
    assert!(st.iter().any(|e| e.path == "README.md" && e.unstaged == 'M'));
    assert!(st.iter().any(|e| e.path == "untracked.txt" && e.untracked));

    // tree
    let raw = git::run_git_in(repo.path(), &["ls-tree", "HEAD"]).unwrap();
    let tree = parse::parse_tree(&raw);
    assert!(tree.iter().any(|t| t.name == "src" && t.kind == parse::TreeEntryKind::Tree));
    assert!(tree.iter().any(|t| t.name == "README.md" && t.kind == parse::TreeEntryKind::Blob));
}
```

- [ ] **Step 6: Run all tests, format, lint, commit, push**

```bash
cargo test && cargo fmt && cargo clippy --all-targets -- -D warnings
git add -A && git commit -m "feat: refs, status, and tree parsers" && git push origin main
```

---

### Task 5: App skeleton, View trait, pager view

**Files:**
- Create: `src/app.rs`, `src/ui.rs`, `src/views/mod.rs`, `src/views/pager.rs`
- Modify: `src/lib.rs`, `src/main.rs`

After this task `git log | cargo run` shows a scrollable colored-less pager and `q` quits.

- [ ] **Step 1: `src/ui.rs` — list navigation + status bar helpers**

```rust
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
    f.render_widget(
        ratatui::widgets::Paragraph::new(line).style(style),
        area,
    );
}

/// Style a diff/pager line by its prefix (used by pager, diff, stage views).
pub fn diff_line_style(line: &str) -> Style {
    if line.starts_with("diff --git") || line.starts_with("index ") {
        Style::default().fg(Color::Yellow)
    } else if line.starts_with("---") || line.starts_with("+++") {
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
```

- [ ] **Step 2: `src/views/mod.rs` — the View trait**

```rust
pub mod pager;

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
```

- [ ] **Step 3: `src/views/pager.rs`**

```rust
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
        format!("{} [{}/{}]", self.title, self.nav.selected + 1, self.lines.len())
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
```

- [ ] **Step 4: `src/app.rs` — view stack + event loop**

```rust
use anyhow::Result;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::layout::{Constraint, Layout};
use ratatui::DefaultTerminal;

use crate::ui::draw_status_bar;
use crate::views::{View, ViewAction};

pub struct App {
    views: Vec<Box<dyn View>>,
    status_msg: String,
}

impl App {
    pub fn new(root: Box<dyn View>) -> Self {
        App {
            views: vec![root],
            status_msg: String::from("h: help, q: close view, Q: quit"),
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        loop {
            terminal.draw(|f| {
                let [main, status] =
                    Layout::vertical([Constraint::Min(1), Constraint::Length(1)])
                        .areas(f.area());
                let title = self
                    .views
                    .last()
                    .map(|v| v.title())
                    .unwrap_or_default();
                if let Some(view) = self.views.last_mut() {
                    view.draw(f, main);
                }
                draw_status_bar(f, status, &title, &self.status_msg);
            })?;

            let Event::Key(key) = event::read()? else { continue };
            if key.kind != KeyEventKind::Press {
                continue;
            }

            // Global keys first.
            match key.code {
                KeyCode::Char('Q') => return Ok(()),
                KeyCode::Char('q') => {
                    self.views.pop();
                    if self.views.is_empty() {
                        return Ok(());
                    }
                    continue;
                }
                _ => {}
            }

            let action = match self.views.last_mut() {
                Some(view) => view.handle_key(key),
                None => return Ok(()),
            };
            match action {
                Ok(ViewAction::None) => {}
                Ok(ViewAction::Push(v)) => self.views.push(v),
                Ok(ViewAction::Pop) => {
                    self.views.pop();
                    if self.views.is_empty() {
                        return Ok(());
                    }
                }
                Ok(ViewAction::Quit) => return Ok(()),
                Ok(ViewAction::Refresh) => {
                    if let Some(view) = self.views.last_mut() {
                        if let Err(e) = view.reload() {
                            self.status_msg = format!("error: {e}");
                        }
                    }
                }
                Err(e) => {
                    // Never crash the TUI on a failed git command — show it.
                    self.status_msg = format!("error: {e}");
                }
            }
        }
    }
}
```

- [ ] **Step 5: Wire up `src/lib.rs` and `src/main.rs`**

`src/lib.rs`:

```rust
pub mod app;
pub mod git;
pub mod parse;
pub mod ui;
pub mod views;
```

`src/main.rs`:

```rust
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
```

NOTE on pager mode: crossterm reads keyboard input from `/dev/tty` on Unix,
so keys work even when stdin is a pipe. If `cargo test` and manual checks show
keys NOT working in pager mode, leave it — do not fight it in this task.

- [ ] **Step 6: Verify it compiles and tests still pass**

```bash
cargo build && cargo test && cargo fmt && cargo clippy --all-targets -- -D warnings
```

- [ ] **Step 7: Manual smoke test (use a detached tmux pane since this is a TUI)**

```bash
tmux new-session -d -s tigtest -x 100 -y 30 'cd /home/user/projects/tig-rs && cargo run 2>/tmp/tig-rs-err.log'
sleep 3
tmux capture-pane -t tigtest -p | head -20
tmux send-keys -t tigtest j j j
sleep 1
tmux capture-pane -t tigtest -p | head -5
tmux send-keys -t tigtest Q
sleep 1
tmux kill-session -t tigtest 2>/dev/null || true
```

Expected: first capture shows git oneline log lines and a cyan status bar
line containing `log [1/...]`; after `j j j` the highlighted line moved down.
If the pane is empty, check `/tmp/tig-rs-err.log`.

- [ ] **Step 8: Commit, push**

```bash
git add -A && git commit -m "feat: app event loop, View trait, pager view" && git push origin main
```

---

### Task 6: Main view (commit log)

**Files:**
- Create: `src/views/main_view.rs`
- Modify: `src/views/mod.rs` (add `pub mod main_view;`), `src/main.rs`

- [ ] **Step 1: Implement `src/views/main_view.rs`**

```rust
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
```

Add `pub mod main_view;` to `src/views/mod.rs`.

- [ ] **Step 2: Make MainView the startup view in `src/main.rs`**

Replace the "Temporary root view" block:

```rust
    let root = tig_rs::views::main_view::MainView::new(Vec::new())?;
    run_app(Box::new(root))
```

(Remove the now-unused `git::run_git` log call and the `PagerView` import if
no longer referenced in that path — pager mode still uses it.)

- [ ] **Step 3: Build, test, lint**

```bash
cargo build && cargo test && cargo fmt && cargo clippy --all-targets -- -D warnings
```

- [ ] **Step 4: Manual smoke test in tmux**

Same tmux recipe as Task 5 Step 7. Expected: commit list with blue dates,
green authors, yellow `[refs]`, reversed-video selected row; `j`/`k` move;
`G` jumps to the oldest commit; `R` reloads without crashing; `Q` quits.

- [ ] **Step 5: Commit, push**

```bash
git add -A && git commit -m "feat: main view (commit log)" && git push origin main
```

---

### Task 7: Diff view

**Files:**
- Create: `src/views/diff.rs`
- Modify: `src/views/mod.rs` (add `pub mod diff;`), `src/views/main_view.rs` (Enter opens diff)

- [ ] **Step 1: Implement `src/views/diff.rs`**

The diff view is a pager over `git show` output:

```rust
use anyhow::Result;

use crate::git::run_git;
use crate::views::pager::PagerView;
use crate::views::View;

/// Build a pager view showing one commit (header + stat + patch).
pub fn commit_diff_view(commit_id: &str) -> Result<Box<dyn View>> {
    let raw = run_git(&[
        "show",
        "--stat",
        "--patch",
        "--format=fuller",
        "--decorate",
        commit_id,
    ])?;
    let short: String = commit_id.chars().take(7).collect();
    Ok(Box::new(PagerView::new(format!("diff {short}"), &raw)))
}
```

(`PagerView` already colorizes diff lines via `ui::diff_line_style` and
supports navigation/search hooks — no new view struct is needed.)

- [ ] **Step 2: Open it from the main view**

In `src/views/main_view.rs` `handle_key`, replace the `KeyCode::Enter` arm:

```rust
            KeyCode::Enter | KeyCode::Char('d') => {
                if let Some(c) = self.selected_commit() {
                    let v = crate::views::diff::commit_diff_view(&c.id)?;
                    return Ok(ViewAction::Push(v));
                }
                Ok(ViewAction::None)
            }
```

Add `pub mod diff;` to `src/views/mod.rs`.

- [ ] **Step 3: Integration test for the data underneath**

Append to `tests/parse_live.rs`:

```rust
#[test]
fn git_show_produces_patch_text() {
    let repo = TestRepo::new();
    repo.commit_file("a.txt", "one\n", "first");
    repo.commit_file("a.txt", "one\ntwo\n", "second");
    let head = git::run_git_in(repo.path(), &["rev-parse", "HEAD"]).unwrap();
    let raw = git::run_git_in(
        repo.path(),
        &["show", "--stat", "--patch", "--format=fuller", "--decorate", head.trim()],
    )
    .unwrap();
    assert!(raw.contains("commit "));
    assert!(raw.contains("+two"));
    assert!(raw.contains("a.txt"));
}
```

- [ ] **Step 4: Build, test, lint**

```bash
cargo build && cargo test && cargo fmt && cargo clippy --all-targets -- -D warnings
```

- [ ] **Step 5: Manual smoke test in tmux**

Same tmux recipe as Task 5 Step 7, then: `tmux send-keys -t tigtest Enter`,
capture — expect a commit header (`commit …`, `Author:`), green `+` lines /
red `-` lines; `q` returns to the main view.

- [ ] **Step 6: Commit, push**

```bash
git add -A && git commit -m "feat: diff view from main view" && git push origin main
```

---

### Task 8: Refs, tree, and blob views

**Files:**
- Create: `src/views/refs.rs`, `src/views/tree.rs`
- Modify: `src/views/mod.rs`, `src/app.rs` (global view-switch keys)

- [ ] **Step 1: `src/views/refs.rs`**

A list of all refs. `Enter` opens a main view limited to that ref.

```rust
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
        let mut v = RefsView { refs: Vec::new(), nav: ListNav::default(), page: 1 };
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
}
```

- [ ] **Step 2: `src/views/tree.rs` — tree browser + blob open**

Behavior: shows `git ls-tree <rev> -- <prefix>` entries, directories first is
NOT required (keep git's order). `Enter` on a tree pushes a new TreeView with
`prefix = prefix + name + "/"`. `Enter` on a blob pushes a PagerView with
`git show <rev>:<prefix><name>`.

```rust
use anyhow::Result;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::git::run_git;
use crate::parse::{parse_tree, TreeEntry, TreeEntryKind};
use crate::ui::ListNav;
use crate::views::pager::PagerView;
use crate::views::{nav_delta, NavMove, View, ViewAction};

pub struct TreeView {
    rev: String,
    /// Path prefix inside the repo, "" for root, always ends with '/' if non-empty.
    prefix: String,
    entries: Vec<TreeEntry>,
    nav: ListNav,
    page: usize,
}

impl TreeView {
    pub fn new(rev: &str, prefix: &str) -> Result<Self> {
        let mut v = TreeView {
            rev: rev.to_string(),
            prefix: prefix.to_string(),
            entries: Vec::new(),
            nav: ListNav::default(),
            page: 1,
        };
        v.reload()?;
        Ok(v)
    }
}

impl View for TreeView {
    fn title(&self) -> String {
        format!("tree: {}:/{} [{}/{}]", self.rev, self.prefix, self.nav.selected + 1, self.entries.len())
    }

    fn reload(&mut self) -> Result<()> {
        let spec = format!("{}^{{tree}}", self.rev);
        let raw = if self.prefix.is_empty() {
            run_git(&["ls-tree", &spec])?
        } else {
            run_git(&["ls-tree", &spec, "--", &self.prefix])?
        };
        self.entries = parse_tree(&raw);
        self.nav.clamp(self.entries.len());
        Ok(())
    }

    fn draw(&mut self, f: &mut Frame, area: Rect) {
        self.page = area.height as usize;
        let range = self.nav.visible(self.entries.len(), area.height as usize);
        let mut out: Vec<Line> = Vec::new();
        for i in range {
            let e = &self.entries[i];
            let (marker, color) = match e.kind {
                TreeEntryKind::Tree => ("/", Color::Blue),
                TreeEntryKind::Blob => ("", Color::Reset),
                TreeEntryKind::Other => ("?", Color::DarkGray),
            };
            // ls-tree returns the full path when a prefix is given; show the
            // last component only.
            let display = e.name.rsplit('/').next().unwrap_or(&e.name).to_string();
            let mut line = Line::from(vec![
                Span::styled(format!("{} ", e.mode), Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{display}{marker}"), Style::default().fg(color)),
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
                NavMove::By(d) => self.nav.move_by(d, self.entries.len()),
                NavMove::Home => self.nav.home(),
                NavMove::End => self.nav.end(self.entries.len()),
            }
            return Ok(ViewAction::None);
        }
        match key.code {
            KeyCode::Char('R') => Ok(ViewAction::Refresh),
            KeyCode::Enter => {
                let Some(e) = self.entries.get(self.nav.selected) else {
                    return Ok(ViewAction::None);
                };
                match e.kind {
                    TreeEntryKind::Tree => {
                        let v = TreeView::new(&self.rev, &format!("{}/", e.name))?;
                        Ok(ViewAction::Push(Box::new(v)))
                    }
                    TreeEntryKind::Blob => {
                        let spec = format!("{}:{}", self.rev, e.name);
                        let raw = run_git(&["show", &spec])?;
                        let v = PagerView::new(format!("blob {}", e.name), &raw);
                        Ok(ViewAction::Push(Box::new(v)))
                    }
                    TreeEntryKind::Other => Ok(ViewAction::None),
                }
            }
            _ => Ok(ViewAction::None),
        }
    }

    fn text_lines(&self) -> Vec<String> {
        self.entries.iter().map(|e| e.name.clone()).collect()
    }

    fn select_line(&mut self, idx: usize) {
        self.nav.selected = idx.min(self.entries.len().saturating_sub(1));
    }
}
```

IMPORTANT ls-tree subtlety: `git ls-tree HEAD^{tree} -- src/` lists entries
INSIDE `src/` only when the path ends with `/`; entry names come back as full
paths (`src/main.rs`). That is why `prefix` always ends with `/` and drawing
shows only the last path component. Verify this against a real repo before
trusting it (`git ls-tree 'HEAD^{tree}' -- src/` in this very project) — if
names come back without the prefix on your git version, drop the
`rsplit('/')` display logic accordingly.

- [ ] **Step 3: Global view-switch keys in `src/app.rs`**

In `App::run`, extend the global-key `match key.code` block (BEFORE the
view-specific dispatch) with:

```rust
                KeyCode::Char('m') => {
                    match crate::views::main_view::MainView::new(Vec::new()) {
                        Ok(v) => self.views.push(Box::new(v)),
                        Err(e) => self.status_msg = format!("error: {e}"),
                    }
                    continue;
                }
                KeyCode::Char('r') => {
                    match crate::views::refs::RefsView::new() {
                        Ok(v) => self.views.push(Box::new(v)),
                        Err(e) => self.status_msg = format!("error: {e}"),
                    }
                    continue;
                }
                KeyCode::Char('t') => {
                    match crate::views::tree::TreeView::new("HEAD", "") {
                        Ok(v) => self.views.push(Box::new(v)),
                        Err(e) => self.status_msg = format!("error: {e}"),
                    }
                    continue;
                }
```

Add `pub mod refs;` and `pub mod tree;` to `src/views/mod.rs`.

CAUTION: `r`/`t`/`m` are global, so list views must NOT use those letters
for their own bindings (they don't).

- [ ] **Step 4: Build, test, lint**

```bash
cargo build && cargo test && cargo fmt && cargo clippy --all-targets -- -D warnings
```

- [ ] **Step 5: Manual smoke test in tmux**

tmux recipe from Task 5 Step 7, then verify: `r` shows refs (branch `main`
green, tags magenta), Enter on a ref shows its log, `q` back; `t` shows the
root tree, Enter on `src/` descends, Enter on a file shows its contents,
`q` walks back up. Capture panes at each step to confirm.

- [ ] **Step 6: Commit, push**

```bash
git add -A && git commit -m "feat: refs, tree, and blob views" && git push origin main
```

---

### Task 9: Status view with stage/unstage

**Files:**
- Create: `src/views/status.rs`
- Modify: `src/views/mod.rs`, `src/app.rs` (global `s` key)

The status view shows three sections — "Changes to be committed" (staged),
"Changes not staged" (unstaged), "Untracked files" — as one flat list with
section header rows. Keys: `u` stages/unstages the file under the cursor,
`!` reverts the file (with a y/n confirmation), `Enter` opens the stage view
(Task 10; until then `Enter` is a no-op), `R` refreshes.

- [ ] **Step 1: Implement `src/views/status.rs`**

```rust
use anyhow::Result;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::git::run_git;
use crate::parse::{parse_status, StatusEntry};
use crate::ui::ListNav;
use crate::views::{nav_delta, NavMove, View, ViewAction};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Staged,
    Unstaged,
    Untracked,
}

pub enum Row {
    Header(&'static str),
    File(Section, StatusEntry),
}

pub struct StatusView {
    rows: Vec<Row>,
    nav: ListNav,
    page: usize,
    /// Some(path) while waiting for y/n confirmation of `!` revert.
    pending_revert: Option<(Section, String)>,
}

impl StatusView {
    pub fn new() -> Result<Self> {
        let mut v = StatusView {
            rows: Vec::new(),
            nav: ListNav::default(),
            page: 1,
            pending_revert: None,
        };
        v.reload()?;
        Ok(v)
    }

    fn selected_file(&self) -> Option<(Section, &StatusEntry)> {
        match self.rows.get(self.nav.selected) {
            Some(Row::File(s, e)) => Some((*s, e)),
            _ => None,
        }
    }
}

impl View for StatusView {
    fn title(&self) -> String {
        let files = self.rows.iter().filter(|r| matches!(r, Row::File(..))).count();
        format!("status [{files} files]")
    }

    fn reload(&mut self) -> Result<()> {
        let raw = run_git(&["status", "--porcelain=v2", "-z"])?;
        let entries = parse_status(&raw);
        let mut staged: Vec<StatusEntry> = Vec::new();
        let mut unstaged: Vec<StatusEntry> = Vec::new();
        let mut untracked: Vec<StatusEntry> = Vec::new();
        for e in entries {
            if e.untracked {
                untracked.push(e);
                continue;
            }
            // One file can appear in both sections (e.g. "MM").
            if e.staged != '.' {
                staged.push(e.clone());
            }
            if e.unstaged != '.' {
                unstaged.push(e);
            }
        }
        self.rows.clear();
        self.rows.push(Row::Header("Changes to be committed:"));
        for e in staged {
            self.rows.push(Row::File(Section::Staged, e));
        }
        self.rows.push(Row::Header("Changes not staged:"));
        for e in unstaged {
            self.rows.push(Row::File(Section::Unstaged, e));
        }
        self.rows.push(Row::Header("Untracked files:"));
        for e in untracked {
            self.rows.push(Row::File(Section::Untracked, e));
        }
        self.nav.clamp(self.rows.len());
        Ok(())
    }

    fn draw(&mut self, f: &mut Frame, area: Rect) {
        self.page = area.height as usize;
        let range = self.nav.visible(self.rows.len(), area.height as usize);
        let mut out: Vec<Line> = Vec::new();
        for i in range {
            let mut line = match &self.rows[i] {
                Row::Header(h) => Line::from(Span::styled(
                    h.to_string(),
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )),
                Row::File(section, e) => {
                    let status_char = match section {
                        Section::Staged => e.staged,
                        Section::Unstaged => e.unstaged,
                        Section::Untracked => '?',
                    };
                    let color = match section {
                        Section::Staged => Color::Green,
                        Section::Unstaged => Color::Red,
                        Section::Untracked => Color::Magenta,
                    };
                    let name = match &e.orig_path {
                        Some(orig) => format!("{} -> {}", orig, e.path),
                        None => e.path.clone(),
                    };
                    Line::from(vec![
                        Span::styled(format!("  {status_char} "), Style::default().fg(color)),
                        Span::raw(name),
                    ])
                }
            };
            if i == self.nav.selected {
                line = line.style(Style::default().add_modifier(Modifier::REVERSED));
            }
            out.push(line);
        }
        f.render_widget(Paragraph::new(out), area);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<ViewAction> {
        // Pending y/n confirmation for revert?
        if let Some((section, path)) = self.pending_revert.take() {
            if key.code == KeyCode::Char('y') {
                match section {
                    Section::Staged | Section::Unstaged => {
                        run_git(&["checkout", "HEAD", "--", &path])?;
                    }
                    Section::Untracked => {
                        std::fs::remove_file(&path)?;
                    }
                }
                self.reload()?;
            }
            return Ok(ViewAction::None);
        }

        if let Some(m) = nav_delta(&key, self.page) {
            match m {
                NavMove::By(d) => self.nav.move_by(d, self.rows.len()),
                NavMove::Home => self.nav.home(),
                NavMove::End => self.nav.end(self.rows.len()),
            }
            return Ok(ViewAction::None);
        }
        match key.code {
            KeyCode::Char('R') => Ok(ViewAction::Refresh),
            KeyCode::Char('u') => {
                if let Some((section, e)) = self.selected_file() {
                    let path = e.path.clone();
                    match section {
                        Section::Staged => {
                            run_git(&["restore", "--staged", "--", &path])?;
                        }
                        Section::Unstaged | Section::Untracked => {
                            run_git(&["add", "--", &path])?;
                        }
                    }
                    self.reload()?;
                }
                Ok(ViewAction::None)
            }
            KeyCode::Char('!') => {
                if let Some((section, e)) = self.selected_file() {
                    self.pending_revert = Some((section, e.path.clone()));
                }
                Ok(ViewAction::None)
            }
            _ => Ok(ViewAction::None),
        }
    }

    fn text_lines(&self) -> Vec<String> {
        self.rows
            .iter()
            .map(|r| match r {
                Row::Header(h) => h.to_string(),
                Row::File(_, e) => e.path.clone(),
            })
            .collect()
    }

    fn select_line(&mut self, idx: usize) {
        self.nav.selected = idx.min(self.rows.len().saturating_sub(1));
    }
}
```

Display the pending confirmation: when `pending_revert` is `Some`, App's
status bar should warn. Simplest: in `title()` return
`format!("status — revert {path}? press y to confirm, any other key to cancel")`
when pending. Implement exactly that (check `self.pending_revert` in `title()`).

- [ ] **Step 2: Global `s` key in `src/app.rs`**

Add to the global key block (same pattern as `m`/`r`/`t`):

```rust
                KeyCode::Char('s') => {
                    match crate::views::status::StatusView::new() {
                        Ok(v) => self.views.push(Box::new(v)),
                        Err(e) => self.status_msg = format!("error: {e}"),
                    }
                    continue;
                }
```

Add `pub mod status;` to `src/views/mod.rs`.

- [ ] **Step 3: Integration test for the staging plumbing**

Create `tests/status_actions.rs`:

```rust
mod common;
use common::TestRepo;
use tig_rs::{git, parse};

fn status(repo: &TestRepo) -> Vec<parse::StatusEntry> {
    let raw = git::run_git_in(repo.path(), &["status", "--porcelain=v2", "-z"]).unwrap();
    parse::parse_status(&raw)
}

#[test]
fn stage_and_unstage_cycle() {
    let repo = TestRepo::new();
    repo.commit_file("a.txt", "one\n", "init");
    repo.write("a.txt", "one\ntwo\n");

    // unstaged -> stage
    git::run_git_in(repo.path(), &["add", "--", "a.txt"]).unwrap();
    let st = status(&repo);
    assert!(st.iter().any(|e| e.path == "a.txt" && e.staged == 'M'));

    // staged -> unstage
    git::run_git_in(repo.path(), &["restore", "--staged", "--", "a.txt"]).unwrap();
    let st = status(&repo);
    assert!(st.iter().any(|e| e.path == "a.txt" && e.unstaged == 'M' && e.staged == '.'));

    // revert worktree change
    git::run_git_in(repo.path(), &["checkout", "HEAD", "--", "a.txt"]).unwrap();
    let st = status(&repo);
    assert!(!st.iter().any(|e| e.path == "a.txt"));
}
```

- [ ] **Step 4: Build, test, lint**

```bash
cargo build && cargo test && cargo fmt && cargo clippy --all-targets -- -D warnings
```

- [ ] **Step 5: Manual smoke test in tmux**

In the tig-rs repo itself, `touch /tmp/nothing; echo x >> README.md` is NOT
acceptable — do not dirty this repo. Instead create a scratch repo:

```bash
rm -rf /tmp/tig-smoke && mkdir -p /tmp/tig-smoke && cd /tmp/tig-smoke
git init -b main && git config user.email t@t && git config user.name T
echo one > a.txt && git add a.txt && git commit -m init
echo two >> a.txt && echo new > untracked.txt
tmux new-session -d -s tigtest -x 100 -y 30 "cd /tmp/tig-smoke && /home/user/projects/tig-rs/target/debug/tig-rs 2>/tmp/tig-rs-err.log"
```

(`cargo build` first.) Then: `s` opens status (a.txt under "Changes not
staged", untracked.txt under "Untracked"); cursor onto a.txt, `u` moves it to
"Changes to be committed"; `u` again moves it back. `Q`, kill session,
`rm -rf /tmp/tig-smoke`.

- [ ] **Step 6: Commit, push**

```bash
cd /home/user/projects/tig-rs
git add -A && git commit -m "feat: status view with stage/unstage/revert" && git push origin main
```

---

### Task 10: Stage view (per-file diff + hunk staging)

**Files:**
- Create: `src/views/stage.rs`
- Modify: `src/views/mod.rs`, `src/views/status.rs` (Enter opens stage view)

The stage view shows the diff of ONE file from the status view. `u` on a line
inside a hunk stages (or unstages, for the staged section) JUST THAT HUNK by
building a one-hunk patch and piping it to `git apply --cached`.

- [ ] **Step 1: Hunk-extraction as a pure function with unit tests**

Append to `src/parse.rs`:

```rust
/// Given full `git diff` output for ONE file and a 0-based line index into
/// it, return a minimal patch containing the file header and only the hunk
/// containing that line. Returns None if the line is not inside a hunk.
pub fn extract_hunk(diff: &str, line_idx: usize) -> Option<String> {
    let lines: Vec<&str> = diff.lines().collect();
    if line_idx >= lines.len() {
        return None;
    }
    // Header: everything before the first "@@".
    let first_hunk = lines.iter().position(|l| l.starts_with("@@"))?;
    if line_idx < first_hunk {
        return None;
    }
    // Find the start of the hunk containing line_idx.
    let start = (0..=line_idx).rev().find(|&i| lines[i].starts_with("@@"))?;
    // Hunk ends at the next "@@" line or "diff --git" line or EOF.
    let end = lines[start + 1..]
        .iter()
        .position(|l| l.starts_with("@@") || l.starts_with("diff --git"))
        .map(|p| start + 1 + p)
        .unwrap_or(lines.len());
    let mut patch = String::new();
    for l in &lines[..first_hunk] {
        patch.push_str(l);
        patch.push('\n');
    }
    for l in &lines[start..end] {
        patch.push_str(l);
        patch.push('\n');
    }
    Some(patch)
}
```

Unit tests (append to the tests module in `src/parse.rs`):

```rust
    const TWO_HUNK_DIFF: &str = "\
diff --git a/f.txt b/f.txt
index 0000000..1111111 100644
--- a/f.txt
+++ b/f.txt
@@ -1,3 +1,4 @@
 line1
+added-in-hunk-1
 line2
 line3
@@ -10,3 +11,4 @@
 line10
+added-in-hunk-2
 line11
 line12
";

    #[test]
    fn extract_first_hunk() {
        // Line index 6 = "+added-in-hunk-1" (0-based, counting from "diff --git").
        let patch = extract_hunk(TWO_HUNK_DIFF, 6).unwrap();
        assert!(patch.contains("+added-in-hunk-1"));
        assert!(!patch.contains("hunk-2"));
        assert!(patch.starts_with("diff --git"));
        assert!(patch.contains("+++ b/f.txt"));
    }

    #[test]
    fn extract_second_hunk() {
        let patch = extract_hunk(TWO_HUNK_DIFF, 11).unwrap();
        assert!(patch.contains("+added-in-hunk-2"));
        assert!(!patch.contains("hunk-1"));
    }

    #[test]
    fn header_lines_are_not_in_a_hunk() {
        assert!(extract_hunk(TWO_HUNK_DIFF, 0).is_none());
        assert!(extract_hunk(TWO_HUNK_DIFF, 3).is_none());
        assert!(extract_hunk("not a diff", 0).is_none());
    }
```

Run `cargo test extract` — first fails to compile, then implement, then passes.

- [ ] **Step 2: Implement `src/views/stage.rs`**

```rust
use anyhow::Result;
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::git::{run_git, run_git_stdin};
use crate::parse::extract_hunk;
use crate::ui::{diff_line_style, ListNav};
use crate::views::status::Section;
use crate::views::{nav_delta, NavMove, View, ViewAction};

pub struct StageView {
    section: Section,
    path: String,
    lines: Vec<String>,
    nav: ListNav,
    page: usize,
}

impl StageView {
    pub fn new(section: Section, path: &str) -> Result<Self> {
        let mut v = StageView {
            section,
            path: path.to_string(),
            lines: Vec::new(),
            nav: ListNav::default(),
            page: 1,
        };
        v.reload()?;
        Ok(v)
    }

    fn diff_text(&self) -> Result<String> {
        match self.section {
            Section::Staged => run_git(&["diff", "--cached", "--", &self.path]),
            Section::Unstaged => run_git(&["diff", "--", &self.path]),
            // Untracked files have no diff: show contents like a new-file.
            Section::Untracked => {
                Ok(std::fs::read_to_string(&self.path).unwrap_or_default())
            }
        }
    }
}

impl View for StageView {
    fn title(&self) -> String {
        let what = match self.section {
            Section::Staged => "staged",
            Section::Unstaged => "unstaged",
            Section::Untracked => "untracked",
        };
        format!("stage: {} ({what}) — u: stage/unstage hunk", self.path)
    }

    fn reload(&mut self) -> Result<()> {
        let text = self.diff_text()?;
        self.lines = text.lines().map(|l| l.to_string()).collect();
        self.nav.clamp(self.lines.len());
        Ok(())
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
            return Ok(ViewAction::None);
        }
        if key.code == KeyCode::Char('u') {
            match self.section {
                Section::Untracked => {
                    run_git(&["add", "--", &self.path])?;
                    return Ok(ViewAction::Pop); // file fully staged; back to status
                }
                Section::Unstaged => {
                    let diff = self.lines.join("\n") + "\n";
                    if let Some(patch) = extract_hunk(&diff, self.nav.selected) {
                        run_git_stdin(&["apply", "--cached", "-"], &patch)?;
                        self.reload()?;
                        if self.lines.is_empty() {
                            return Ok(ViewAction::Pop);
                        }
                    }
                }
                Section::Staged => {
                    let diff = self.lines.join("\n") + "\n";
                    if let Some(patch) = extract_hunk(&diff, self.nav.selected) {
                        run_git_stdin(&["apply", "--cached", "--reverse", "-"], &patch)?;
                        self.reload()?;
                        if self.lines.is_empty() {
                            return Ok(ViewAction::Pop);
                        }
                    }
                }
            }
            return Ok(ViewAction::None);
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
```

Add `pub mod stage;` to `src/views/mod.rs`.

- [ ] **Step 3: Open from the status view**

In `src/views/status.rs` `handle_key`, add an `Enter` arm (NOT inside the
pending_revert branch):

```rust
            KeyCode::Enter => {
                if let Some((section, e)) = self.selected_file() {
                    let v = crate::views::stage::StageView::new(section, &e.path)?;
                    return Ok(ViewAction::Push(Box::new(v)));
                }
                Ok(ViewAction::None)
            }
```

ALSO: when the user returns from the stage view, the status view's data is
stale. Fix in `src/app.rs`: after every `ViewAction::Pop` (and after `q` pops
a view), call `reload()` on the newly-exposed top view, ignoring errors:

```rust
                // after self.views.pop() in BOTH places:
                if let Some(v) = self.views.last_mut() {
                    let _ = v.reload();
                }
```

- [ ] **Step 4: Integration test for hunk staging plumbing**

Append to `tests/status_actions.rs`:

```rust
fn run_stdin(dir: &std::path::Path, args: &[&str], input: &str) {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let mut child = Command::new("git")
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn hunk_staging_via_extract_and_apply() {
    let repo = TestRepo::new();
    // A file long enough that two edits produce two separate hunks.
    let base: String = (1..=30).map(|i| format!("line{i}\n")).collect();
    repo.commit_file("f.txt", &base, "init");
    let modified = base
        .replace("line3\n", "line3\nADDED-EARLY\n")
        .replace("line27\n", "line27\nADDED-LATE\n");
    repo.write("f.txt", &modified);

    let diff = git::run_git_in(repo.path(), &["diff", "--", "f.txt"]).unwrap();
    let lines: Vec<&str> = diff.lines().collect();
    let early_idx = lines.iter().position(|l| *l == "+ADDED-EARLY").unwrap();
    let patch = parse::extract_hunk(&diff, early_idx).unwrap();
    assert!(patch.contains("ADDED-EARLY"));
    assert!(!patch.contains("ADDED-LATE"));

    // Stage just that hunk, exactly as the stage view does.
    run_stdin(repo.path(), &["apply", "--cached", "-"], &patch);

    let st = status(&repo);
    let e = st.iter().find(|e| e.path == "f.txt").unwrap();
    // Hunk 1 staged, hunk 2 still unstaged => both flags set.
    assert_eq!(e.staged, 'M');
    assert_eq!(e.unstaged, 'M');
}
```

- [ ] **Step 5: Build, test, lint**

```bash
cargo build && cargo test && cargo fmt && cargo clippy --all-targets -- -D warnings
```

- [ ] **Step 6: Manual smoke test in tmux**

Scratch-repo recipe from Task 9 Step 5, but commit a 30-line file and edit
line 3 and line 27 (two hunks). `s` → cursor on the file → `Enter` → stage
view shows the diff with two `@@` hunks → cursor into the first hunk → `u` →
view reloads showing ONLY the second hunk → `q` → status shows the file in
BOTH staged and unstaged sections. Then clean up the scratch repo and tmux session.

- [ ] **Step 7: Commit, push**

```bash
cd /home/user/projects/tig-rs
git add -A && git commit -m "feat: stage view with hunk-level staging" && git push origin main
```

---

### Task 11: Search, help view, CLI args

**Files:**
- Create: `src/views/help.rs`
- Modify: `src/app.rs` (search prompt + `h` key), `src/main.rs` (subcommands), `src/views/mod.rs`

- [ ] **Step 1: Search in `src/app.rs`**

Add fields to `App`:

```rust
    /// Some(buffer) while the user is typing a /search query.
    search_input: Option<String>,
    /// Last submitted query, for n/N.
    last_search: String,
```

(Initialize: `search_input: None, last_search: String::new()` in `App::new`.)

Behavior:
- `/` sets `search_input = Some(String::new())` (global key block).
- While `search_input` is `Some`: printable chars append to the buffer,
  `Backspace` pops a char, `Esc` cancels (set to `None`), `Enter` submits:
  `last_search = buffer`, `search_input = None`, jump to first match at or
  after the current selection. NO other key handling while typing.
- `n` jumps to the next match after the current selection (wraps around),
  `N` to the previous one (wraps). Case-insensitive substring match over
  `view.text_lines()`.
- Status bar shows `/<buffer>` while typing (pass it as `msg` to
  `draw_status_bar`), and `match i/j for "<query>"` / `no match for "<query>"`
  after submit/n/N.

Implementation — add to `App`:

```rust
    fn find_match(&mut self, from: usize, forward: bool) {
        let Some(view) = self.views.last_mut() else { return };
        if self.last_search.is_empty() {
            return;
        }
        let needle = self.last_search.to_lowercase();
        let lines = view.text_lines();
        let n = lines.len();
        if n == 0 {
            self.status_msg = format!("no match for \"{}\"", self.last_search);
            return;
        }
        let hit = (1..=n).map(|step| {
            if forward {
                (from + step) % n
            } else {
                (from + n - step % n) % n
            }
        });
        for i in hit {
            if lines[i].to_lowercase().contains(&needle) {
                view.select_line(i);
                self.status_msg = format!("match {}/{} for \"{}\"", i + 1, n, self.last_search);
                return;
            }
        }
        self.status_msg = format!("no match for \"{}\"", self.last_search);
    }
```

To know "the current selection" for `from`, add a method to the `View` trait
in `src/views/mod.rs` with a default:

```rust
    /// Currently selected line index (for search start). Default 0.
    fn selected_index(&self) -> usize {
        0
    }
```

Implement `selected_index` in every view that has a `nav`
(pager, main_view, refs, tree, status, stage) as `self.nav.selected`.

Key-handling order inside the event loop becomes:

```rust
            // 1. search input mode swallows everything
            if let Some(buf) = self.search_input.as_mut() {
                match key.code {
                    KeyCode::Esc => self.search_input = None,
                    KeyCode::Backspace => { buf.pop(); }
                    KeyCode::Enter => {
                        self.last_search = self.search_input.take().unwrap_or_default();
                        let from = self.views.last().map(|v| v.selected_index()).unwrap_or(0);
                        // search starts AT current line: step back one so the
                        // first checked index is `from` itself
                        self.find_match(from.wrapping_sub(1).min(usize::MAX), true);
                        let _ = from;
                    }
                    KeyCode::Char(c) => buf.push(c),
                    _ => {}
                }
                continue;
            }
            // 2. global keys (Q, q, m, r, t, s, h, /, n, N)
            // 3. view handle_key
```

Simplification allowed for the Enter arm (the wrapping_sub trick is fragile):
it is fine to start the search strictly AFTER the current line:

```rust
                    KeyCode::Enter => {
                        self.last_search = self.search_input.take().unwrap_or_default();
                        let from = self.views.last().map(|v| v.selected_index()).unwrap_or(0);
                        self.find_match(from, true);
                    }
```

Use this simpler version. Add the `/`, `n`, `N` global keys:

```rust
                KeyCode::Char('/') => {
                    self.search_input = Some(String::new());
                    continue;
                }
                KeyCode::Char('n') => {
                    let from = self.views.last().map(|v| v.selected_index()).unwrap_or(0);
                    self.find_match(from, true);
                    continue;
                }
                KeyCode::Char('N') => {
                    let from = self.views.last().map(|v| v.selected_index()).unwrap_or(0);
                    self.find_match(from, false);
                    continue;
                }
```

And while drawing, if `search_input` is `Some(buf)`, the status-bar `msg`
must be `format!("/{buf}")` instead of `self.status_msg`.

- [ ] **Step 2: Help view**

`src/views/help.rs` — a static pager:

```rust
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
```

Global key in `src/app.rs`:

```rust
                KeyCode::Char('h') => {
                    self.views.push(crate::views::help::help_view());
                    continue;
                }
```

Add `pub mod help;` to `src/views/mod.rs`.

- [ ] **Step 3: CLI subcommands in `src/main.rs`**

Replace the argument handling so the binary supports:

```
tig-rs                      # main view of HEAD
tig-rs log [git log args]   # main view with extra args, e.g. tig-rs log v1.0..main -- src/
tig-rs show <rev>           # diff view of one commit
tig-rs status               # status view
tig-rs refs                 # refs view
tig-rs tree [rev]           # tree view (default HEAD)
tig-rs --version | -V
tig-rs --help | -h
```

```rust
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
        Some("log") => Box::new(tig_rs::views::main_view::MainView::new(
            args[1..].to_vec(),
        )?),
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
```

- [ ] **Step 4: Build, test, lint**

```bash
cargo build && cargo test && cargo fmt && cargo clippy --all-targets -- -D warnings
cargo run -- --help    # prints the help text above
cargo run -- bogus; echo "exit=$?"   # prints unknown-command error, exit=1
```

- [ ] **Step 5: Manual smoke test in tmux**

tmux recipe from Task 5 Step 7: `h` shows help; `q` closes; `/` then typing
`feat` then Enter jumps the main view to a line containing "feat"; `n`
advances; `N` goes back. Also `cargo run -- tree` starts in the tree view.

- [ ] **Step 6: Commit, push**

```bash
git add -A && git commit -m "feat: search, help view, CLI subcommands" && git push origin main
```

---

### Task 12: CI + release workflows, tag v0.1.0

**Files:**
- Create: `.github/workflows/ci.yml`, `.github/workflows/release.yml`

- [ ] **Step 1: `.github/workflows/ci.yml`**

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always

jobs:
  check:
    name: Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: cargo fmt
        run: cargo fmt --check
      - name: cargo clippy
        run: cargo clippy --all-targets -- -D warnings
      - name: cargo test
        run: cargo test
```

- [ ] **Step 2: `.github/workflows/release.yml`**

tig-rs has no native dependencies (pure Rust), so musl static builds work.

```yaml
name: Release

on:
  push:
    tags: ['v*']

permissions:
  contents: write

env:
  CARGO_TERM_COLOR: always

jobs:
  build:
    name: Build ${{ matrix.target }}
    strategy:
      fail-fast: false
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - os: ubuntu-latest
            target: x86_64-unknown-linux-musl
          - os: ubuntu-latest
            target: aarch64-unknown-linux-musl
            use_cross: true
          - os: macos-latest
            target: x86_64-apple-darwin
          - os: macos-latest
            target: aarch64-apple-darwin
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4

      - name: Install musl tools
        if: matrix.target == 'x86_64-unknown-linux-musl'
        run: sudo apt-get update && sudo apt-get install -y musl-tools

      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - uses: Swatinem/rust-cache@v2

      - name: Install cross
        if: matrix.use_cross == true
        run: cargo install cross --locked

      - name: Build (cross)
        if: matrix.use_cross == true
        run: cross build --release --target ${{ matrix.target }}

      - name: Build
        if: matrix.use_cross != true
        run: cargo build --release --target ${{ matrix.target }}

      - name: Prepare binary
        shell: bash
        run: |
          mkdir -p dist
          src="target/${{ matrix.target }}/release/tig-rs"
          cp "$src" "dist/tig-rs-${{ matrix.target }}"
          ls -l dist

      - name: Upload artifact
        uses: actions/upload-artifact@v4
        with:
          name: binaries-${{ matrix.target }}
          path: dist/*
          if-no-files-found: error

  release:
    name: Create GitHub Release
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Download all artifacts
        uses: actions/download-artifact@v4
        with:
          path: artifacts
          merge-multiple: true

      - name: Generate SHA-256 checksums
        run: |
          cd artifacts
          sha256sum tig-rs-* > SHA256SUMS.txt
          cat SHA256SUMS.txt

      - name: Generate release notes
        run: |
          PREV_TAG=$(git describe --tags --abbrev=0 HEAD^ 2>/dev/null || echo "")
          {
            if [ -n "$PREV_TAG" ]; then
              echo "## Changes since $PREV_TAG"
            else
              echo "## Changes"
            fi
            echo ""
            if [ -n "$PREV_TAG" ]; then
              git log --oneline --no-decorate "${PREV_TAG}..HEAD"
            else
              git log --oneline --no-decorate -20
            fi
          } > RELEASE_NOTES.md
          cat RELEASE_NOTES.md

      - name: Create GitHub Release
        uses: softprops/action-gh-release@v2
        with:
          tag_name: ${{ github.ref_name }}
          name: Release ${{ github.ref_name }}
          body_path: RELEASE_NOTES.md
          files: |
            artifacts/tig-rs-*
            artifacts/SHA256SUMS.txt
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

- [ ] **Step 3: Commit, push, watch CI until green**

```bash
git add -A && git commit -m "ci: add CI and release workflows" && git push origin main
sleep 30 && gh run list --limit 3
RUN_ID=$(gh run list --workflow CI --limit 1 --json databaseId -q '.[0].databaseId')
gh run watch "$RUN_ID" --exit-status
```

If CI fails: read `gh run view "$RUN_ID" --log-failed`, fix, commit
(`fix: <what>`), push, repeat watch. Do NOT tag until CI is green.

- [ ] **Step 4: Tag v0.1.0 and watch the release workflow**

```bash
git tag v0.1.0 && git push origin v0.1.0
sleep 30
RUN_ID=$(gh run list --workflow Release --limit 1 --json databaseId -q '.[0].databaseId')
gh run watch "$RUN_ID" --exit-status
gh release view v0.1.0
```

Expected: `gh release view v0.1.0` lists `tig-rs-<target>` binaries and
`SHA256SUMS.txt`. If exactly ONE matrix target keeps failing after one fix
attempt (likely the `cross` aarch64 build), REMOVE that matrix entry, commit
`ci: drop failing <target> release target`, delete and re-create the tag:

```bash
git tag -d v0.1.0 && git push origin :refs/tags/v0.1.0
git tag v0.1.0 && git push origin v0.1.0
```

and watch again. Report the release URL when done.

---

### Task 13: README

**Files:**
- Create: `README.md`

- [ ] **Step 1: Inspect the finished project**

```bash
cd /home/user/projects/tig-rs
ls src/ src/views/ && cargo run -- --help && gh release view v0.1.0 | head -20
```

- [ ] **Step 2: Write `README.md`** covering, in this order:

1. Title + one-liner: `tig-rs — text-mode interface for git, written in Rust` + a badge row (CI badge: `https://github.com/hermes98761234/tig-rs/actions/workflows/ci.yml/badge.svg`).
2. Screenshot placeholder section (text description of the views — no actual screenshot needed).
3. Features: the view list (main/diff/status/stage with hunk staging/tree/blob/refs/help/pager mode) — be accurate, don't invent features.
4. Install: `cargo install --git https://github.com/hermes98761234/tig-rs` AND pre-built binaries from the releases page (list the targets actually published in v0.1.0).
5. Usage: the CLI table from `--help`, plus the key-bindings table from the in-app help (copy from `src/views/help.rs` — keep them in sync).
6. Architecture: 5–10 lines — git CLI + parsers (`src/parse.rs`), View trait stack (`src/views/`), ratatui rendering; mention the spec and plan docs under `docs/superpowers/`.
7. Development: `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt`; tests use throwaway temp-dir git repos.
8. License: GPL-2.0 (same as original tig), credit `jonas/tig` as the original.

- [ ] **Step 3: Verify claims**

Every command and key binding in the README must exist in the code. Check the
key table against `src/views/help.rs` and the CLI table against `print_help()`.

- [ ] **Step 4: Commit, push**

```bash
git add README.md && git commit -m "docs: add README" && git push origin main
gh repo view hermes98761234/tig-rs --json url -q .url
```

Report the repo URL.
