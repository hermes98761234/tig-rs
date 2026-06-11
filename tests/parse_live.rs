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
    assert!(refs
        .iter()
        .any(|r| r.short == "main" && r.kind == parse::RefKind::Branch));
    assert!(refs
        .iter()
        .any(|r| r.short == "v0.1" && r.kind == parse::RefKind::Tag));

    // status: one staged, one modified, one untracked
    repo.write("staged.txt", "s\n");
    repo.git(&["add", "staged.txt"]);
    repo.write("README.md", "changed\n");
    repo.write("untracked.txt", "u\n");
    let raw = git::run_git_in(repo.path(), &["status", "--porcelain=v2", "-z"]).unwrap();
    let st = parse::parse_status(&raw);
    assert!(st.iter().any(|e| e.path == "staged.txt" && e.staged == 'A'));
    assert!(st
        .iter()
        .any(|e| e.path == "README.md" && e.unstaged == 'M'));
    assert!(st.iter().any(|e| e.path == "untracked.txt" && e.untracked));

    // tree
    let raw = git::run_git_in(repo.path(), &["ls-tree", "HEAD"]).unwrap();
    let tree = parse::parse_tree(&raw);
    assert!(tree
        .iter()
        .any(|t| t.name == "src" && t.kind == parse::TreeEntryKind::Tree));
    assert!(tree
        .iter()
        .any(|t| t.name == "README.md" && t.kind == parse::TreeEntryKind::Blob));
}

#[test]
fn git_show_produces_patch_text() {
    let repo = TestRepo::new();
    repo.commit_file("a.txt", "one\n", "first");
    repo.commit_file("a.txt", "one\ntwo\n", "second");
    let head = git::run_git_in(repo.path(), &["rev-parse", "HEAD"]).unwrap();
    let raw = git::run_git_in(
        repo.path(),
        &[
            "show",
            "--stat",
            "--patch",
            "--format=fuller",
            "--decorate",
            head.trim(),
        ],
    )
    .unwrap();
    assert!(raw.contains("commit "));
    assert!(raw.contains("+two"));
    assert!(raw.contains("a.txt"));
}
