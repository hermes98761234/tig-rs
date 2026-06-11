mod common;
use common::TestRepo;
use tig_rs::{git, parse};

#[test]
fn parses_real_git_log() {
    let repo = TestRepo::new();
    repo.commit_file("a.txt", "one\n", "first commit");
    repo.commit_file("a.txt", "two\n", "second commit");
    repo.git(&["tag", "v1.0"]);
    let raw = git::run_git_in(repo.path(), &["log", parse::LOG_FORMAT, "--date=short"]).unwrap();
    let commits = parse::parse_commits(&raw);
    assert_eq!(commits.len(), 2);
    assert_eq!(commits[0].subject, "second commit");
    assert!(commits[0].refs.contains("tag: v1.0"));
    assert_eq!(commits[1].subject, "first commit");
    assert_eq!(commits[0].date, "2026-01-02");
}
