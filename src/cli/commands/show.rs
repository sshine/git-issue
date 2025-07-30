use anyhow::Result;
use clap::Args;

use crate::cli::output::format_issue_detailed;
use crate::common::IssueId;
use crate::storage::IssueStore;

#[derive(Args)]
pub struct ShowArgs {
    /// Issue ID to show
    pub id: IssueId,
}

pub fn handle_show(repo_path: std::path::PathBuf, args: ShowArgs) -> Result<()> {
    let store = IssueStore::open(&repo_path)?;
    let issue = store.get_issue(args.id)?;

    print!("{}", format_issue_detailed(&issue));

    Ok(())
}
