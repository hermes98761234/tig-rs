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
        let raw = "1 M. N... 100644 100644 100644 aaa bbb staged.txt\x00\
                   1 .M N... 100644 100644 100644 aaa bbb unstaged.txt\x00\
                   2 R. N... 100644 100644 100644 aaa bbb R100 new name.txt\x00old name.txt\x00\
                   ? untracked file.txt\x00";
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
}
