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
