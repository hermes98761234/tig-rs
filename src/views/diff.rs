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
