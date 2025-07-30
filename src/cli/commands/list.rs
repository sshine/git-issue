use anyhow::Result;
use clap::Args;

use crate::cli::output::{format_issue_compact, format_issue_detailed};
use crate::common::IssueStatus;
use crate::storage::IssueStore;

use super::parse_status;

#[derive(Args)]
pub struct ListArgs {
    /// Filter by status
    #[arg(short, long)]
    pub status: Option<String>,

    /// Show only issue IDs and titles
    #[arg(short, long)]
    pub compact: bool,

    /// Show all issues including completed ones
    #[arg(short, long)]
    pub all: bool,
}

pub fn handle_list(repo_path: std::path::PathBuf, args: ListArgs) -> Result<()> {
    let store = IssueStore::open(&repo_path)?;
    let issues = store.list_issues()?;

    let filtered_issues = if let Some(status_filter) = args.status {
        let status = parse_status(&status_filter)?;
        issues
            .into_iter()
            .filter(|issue| issue.status == status)
            .collect()
    } else if args.all {
        // Show all issues when --all flag is specified
        issues
    } else {
        // By default, exclude "done" issues
        issues
            .into_iter()
            .filter(|issue| issue.status != IssueStatus::Done)
            .collect()
    };

    if args.compact {
        for issue in filtered_issues {
            println!("{}", format_issue_compact(&issue));
        }
    } else {
        for issue in filtered_issues {
            print!("{}", format_issue_detailed(&issue));
        }
    }

    Ok(())
}