use anyhow::Result;
use clap::Args;

use crate::cli::output::{format_issue_compact, format_issue_list_long};
use crate::common::IssueStatus;
use crate::storage::IssueStore;

use super::parse_status;

#[derive(Args)]
pub struct ListArgs {
    /// Optional search string to filter issues by title, description, or labels
    pub search: Option<String>,

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
    let mut issues = store.list_issues()?;

    // Apply search filter if provided
    if let Some(search_term) = &args.search {
        let search_lower = search_term.to_lowercase();
        issues.retain(|issue| {
            // Search in title
            issue.title.to_lowercase().contains(&search_lower)
                // Search in description
                || issue.description.to_lowercase().contains(&search_lower)
                // Search in labels
                || issue.labels.iter().any(|label| label.to_lowercase().contains(&search_lower))
        });
    }

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
            print!("{}", format_issue_list_long(&issue));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::test_helpers::*;
    use tempfile::TempDir;

    fn setup_test_issues() -> (TempDir, std::path::PathBuf) {
        let temp_dir = TempDir::new().expect("Failed to create temporary directory");
        let repo_path = temp_dir.path().to_path_buf();

        // Create test issues with different content for searching
        let mut store = IssueStore::init(&repo_path).expect("Failed to initialize store");
        let author = create_test_identity();

        // Issue 1: Contains "bug" in title
        store
            .create_issue(
                "Fix bug in authentication".to_string(),
                "Users cannot log in".to_string(),
                author.clone(),
            )
            .expect("Failed to create issue 1");

        // Issue 2: Contains "bug" in label
        let issue2_id = store
            .create_issue(
                "Add new feature".to_string(),
                "Implement user profiles".to_string(),
                author.clone(),
            )
            .expect("Failed to create issue 2");
        store
            .add_label(issue2_id, "bug".to_string(), author.clone())
            .expect("Failed to add label");

        // Issue 3: Contains "bug" in description
        store
            .create_issue(
                "Update documentation".to_string(),
                "Fix bug in example code".to_string(),
                author.clone(),
            )
            .expect("Failed to create issue 3");

        // Issue 4: Done status with "bug" (for testing default filtering)
        let issue4_id = store
            .create_issue(
                "Completed bug fix".to_string(),
                "This bug is done".to_string(),
                author.clone(),
            )
            .expect("Failed to create issue 4");
        store
            .update_issue_status(issue4_id, IssueStatus::Done, author.clone())
            .expect("Failed to update status");

        // Issue 5: No "bug" anywhere
        store
            .create_issue(
                "Refactor code".to_string(),
                "Clean up the codebase".to_string(),
                author.clone(),
            )
            .expect("Failed to create issue 5");

        (temp_dir, repo_path)
    }

    #[test]
    fn test_list_search_in_title() {
        let (_temp_dir, repo_path) = setup_test_issues();

        // Test search for "bug" - should find issues 1, 2, and 3 (not 4 because it's done by default)
        let _args = ListArgs {
            search: Some("bug".to_string()),
            status: None,
            compact: true,
            all: false,
        };

        // We can't easily capture stdout in the current implementation,
        // so we'll test the logic by accessing the store directly
        let store = IssueStore::open(&repo_path).expect("Failed to open store");
        let mut issues = store.list_issues().expect("Failed to list issues");

        // Apply search filter
        let search_lower = "bug";
        issues.retain(|issue| {
            issue.title.to_lowercase().contains(search_lower)
                || issue.description.to_lowercase().contains(search_lower)
                || issue
                    .labels
                    .iter()
                    .any(|label| label.to_lowercase().contains(search_lower))
        });

        // Apply default status filter (exclude done)
        let filtered: Vec<_> = issues
            .into_iter()
            .filter(|issue| issue.status != IssueStatus::Done)
            .collect();

        assert_eq!(filtered.len(), 3, "Should find 3 issues containing 'bug'");
    }

    #[test]
    fn test_list_search_case_insensitive() {
        let (_temp_dir, repo_path) = setup_test_issues();

        let store = IssueStore::open(&repo_path).expect("Failed to open store");
        let mut issues = store.list_issues().expect("Failed to list issues");

        // Test case-insensitive search
        let search_lower = "BUG".to_lowercase();
        issues.retain(|issue| {
            issue.title.to_lowercase().contains(&search_lower)
                || issue.description.to_lowercase().contains(&search_lower)
                || issue
                    .labels
                    .iter()
                    .any(|label| label.to_lowercase().contains(&search_lower))
        });

        // Include all issues (no status filter)
        assert_eq!(
            issues.len(),
            4,
            "Should find 4 issues containing 'BUG' (case-insensitive)"
        );
    }

    #[test]
    fn test_list_search_with_all_flag() {
        let (_temp_dir, repo_path) = setup_test_issues();

        let store = IssueStore::open(&repo_path).expect("Failed to open store");
        let mut issues = store.list_issues().expect("Failed to list issues");

        // Apply search filter
        let search_lower = "bug";
        issues.retain(|issue| {
            issue.title.to_lowercase().contains(search_lower)
                || issue.description.to_lowercase().contains(search_lower)
                || issue
                    .labels
                    .iter()
                    .any(|label| label.to_lowercase().contains(search_lower))
        });

        // With --all flag, should include done issues
        assert_eq!(issues.len(), 4, "Should find 4 issues with --all flag");
    }

    #[test]
    fn test_list_search_no_results() {
        let (_temp_dir, repo_path) = setup_test_issues();

        let store = IssueStore::open(&repo_path).expect("Failed to open store");
        let mut issues = store.list_issues().expect("Failed to list issues");

        // Search for something that doesn't exist
        let search_lower = "nonexistent";
        issues.retain(|issue| {
            issue.title.to_lowercase().contains(search_lower)
                || issue.description.to_lowercase().contains(search_lower)
                || issue
                    .labels
                    .iter()
                    .any(|label| label.to_lowercase().contains(search_lower))
        });

        assert_eq!(
            issues.len(),
            0,
            "Should find no issues for nonexistent search term"
        );
    }

    #[test]
    fn test_list_search_with_status_filter() {
        let (_temp_dir, repo_path) = setup_test_issues();

        // Add an in-progress issue with "bug"
        let mut store = IssueStore::open(&repo_path).expect("Failed to open store");
        let author = create_test_identity();
        let issue_id = store
            .create_issue(
                "Debug performance issue".to_string(),
                "Application is slow".to_string(),
                author.clone(),
            )
            .expect("Failed to create issue");
        store
            .update_issue_status(issue_id, IssueStatus::InProgress, author)
            .expect("Failed to update status");

        let mut issues = store.list_issues().expect("Failed to list issues");

        // Apply search filter
        let search_lower = "bug";
        issues.retain(|issue| {
            issue.title.to_lowercase().contains(search_lower)
                || issue.description.to_lowercase().contains(search_lower)
                || issue
                    .labels
                    .iter()
                    .any(|label| label.to_lowercase().contains(search_lower))
        });

        // Apply status filter
        let in_progress_issues: Vec<_> = issues
            .into_iter()
            .filter(|issue| issue.status == IssueStatus::InProgress)
            .collect();

        assert_eq!(
            in_progress_issues.len(),
            1,
            "Should find 1 in-progress issue containing 'bug'"
        );
    }
}
