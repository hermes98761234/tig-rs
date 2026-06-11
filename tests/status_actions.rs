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
    assert!(st
        .iter()
        .any(|e| e.path == "a.txt" && e.unstaged == 'M' && e.staged == '.'));

    // revert worktree change
    git::run_git_in(repo.path(), &["checkout", "HEAD", "--", "a.txt"]).unwrap();
    let st = status(&repo);
    assert!(!st.iter().any(|e| e.path == "a.txt"));
}

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
