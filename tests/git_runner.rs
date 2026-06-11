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
