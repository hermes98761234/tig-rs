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
