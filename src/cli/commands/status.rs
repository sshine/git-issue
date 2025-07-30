use anyhow::Result;
use clap::Args;

use crate::cli::output::success_message;
use crate::common::{IssueId, SystemEnvProvider};
use crate::storage::IssueStore;

use super::{get_author_identity, parse_status};

#[derive(Args)]
pub struct StatusArgs {
    /// Issue ID to update
    pub id: IssueId,

    /// New status (todo, in-progress, done)
    pub status: String,

    /// Author name (defaults to git config)
    #[arg(long)]
    pub author_name: Option<String>,

    /// Author email (defaults to git config)
    #[arg(long)]
    pub author_email: Option<String>,
}

pub fn handle_status(repo_path: std::path::PathBuf, args: StatusArgs) -> Result<()> {
    let mut store = IssueStore::open(&repo_path)?;
    let author = get_author_identity(
        args.author_name,
        args.author_email,
        &store,
        SystemEnvProvider,
    )?;
    let new_status = parse_status(&args.status)?;

    store.update_issue_status(args.id, new_status, author)?;

    println!(
        "{}",
        success_message(&format!(
            "Updated issue #{} status to {}",
            args.id, new_status
        ))
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::IssueStatus;

    #[test]
    fn test_parse_status_valid() {
        assert_eq!(parse_status("todo").unwrap(), IssueStatus::Todo);
        assert_eq!(parse_status("open").unwrap(), IssueStatus::Todo);
        assert_eq!(parse_status("TODO").unwrap(), IssueStatus::Todo);

        assert_eq!(
            parse_status("in-progress").unwrap(),
            IssueStatus::InProgress
        );
        assert_eq!(parse_status("inprogress").unwrap(), IssueStatus::InProgress);
        assert_eq!(parse_status("progress").unwrap(), IssueStatus::InProgress);
        assert_eq!(
            parse_status("IN-PROGRESS").unwrap(),
            IssueStatus::InProgress
        );

        assert_eq!(parse_status("done").unwrap(), IssueStatus::Done);
        assert_eq!(parse_status("closed").unwrap(), IssueStatus::Done);
        assert_eq!(parse_status("complete").unwrap(), IssueStatus::Done);
        assert_eq!(parse_status("DONE").unwrap(), IssueStatus::Done);
    }

    #[test]
    fn test_parse_status_invalid() {
        let result = parse_status("invalid");
        assert!(result.is_err(), "Should return error for invalid status");
        assert!(result.unwrap_err().to_string().contains("Invalid status"));
    }
}
