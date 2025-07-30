use anyhow::Result;
use clap::Args;
use std::collections::HashSet;

use crate::cli::output::{success_message, warning_message};
use crate::common::{Identity, IssueId, SystemEnvProvider};
use crate::storage::IssueStore;

use super::get_author_identity;

#[derive(Args)]
pub struct AssignArgs {
    /// Issue ID to assign
    pub id: IssueId,

    /// Assignee emails to add
    pub assignees: Vec<String>,
}

#[derive(Args)]
pub struct UnassignArgs {
    /// Issue ID to unassign
    pub id: IssueId,

    /// Assignee emails to remove (if none provided, removes all assignees)
    pub assignees: Vec<String>,
}

/// Handle assigning users to an issue
pub fn handle_assign(repo_path: std::path::PathBuf, args: AssignArgs) -> Result<()> {
    let mut store = IssueStore::open(&repo_path)?;
    let author = get_author_identity(None, None, &store, SystemEnvProvider)?;

    // Get the current issue to check existing assignees
    let current_issue = store.get_issue(args.id)?;
    let current_assignees: HashSet<String> = current_issue
        .assignees
        .iter()
        .map(|a| a.email.clone())
        .collect();

    let mut warnings = Vec::new();
    let mut new_assignees = current_issue.assignees.clone();
    let mut successfully_added = Vec::new();

    // If no assignees specified, assign to self
    let assignees_to_process = if args.assignees.is_empty() {
        vec![author.email.clone()]
    } else {
        args.assignees.clone()
    };

    // Process each assignee
    for email in &assignees_to_process {
        // Basic email validation
        if !email.contains('@') {
            return Err(anyhow::anyhow!("Invalid email format: {}", email));
        }

        if current_assignees.contains(email) {
            warnings.push(format!(
                "User '{}' is already assigned to issue #{}",
                email, args.id
            ));
        } else {
            let identity = Identity::new("", email);
            new_assignees.push(identity);
            successfully_added.push(email.clone());
        }
    }

    // Update assignees if there are changes
    if !successfully_added.is_empty() {
        store.update_assignees(args.id, new_assignees, author.clone())?;

        let message = if successfully_added.len() == 1 {
            if args.assignees.is_empty() && successfully_added[0] == author.email {
                format!("Assigned yourself to issue #{}", args.id)
            } else {
                format!("Assigned {} to issue #{}", successfully_added[0], args.id)
            }
        } else {
            format!(
                "Assigned {} users to issue #{}: {}",
                successfully_added.len(),
                args.id,
                successfully_added.join(", ")
            )
        };
        println!("{}", success_message(&message));
    }

    // Display warnings
    for warning in warnings {
        println!("{}", warning_message(&warning));
    }

    Ok(())
}

/// Handle unassigning users from an issue
pub fn handle_unassign(repo_path: std::path::PathBuf, args: UnassignArgs) -> Result<()> {
    let mut store = IssueStore::open(&repo_path)?;
    let author = get_author_identity(None, None, &store, SystemEnvProvider)?;

    // Get the current issue to check existing assignees
    let current_issue = store.get_issue(args.id)?;
    let current_assignees: HashSet<String> = current_issue
        .assignees
        .iter()
        .map(|a| a.email.clone())
        .collect();

    if current_assignees.is_empty() {
        println!("Issue #{} has no assignees to remove", args.id);
        return Ok(());
    }

    let mut warnings = Vec::new();
    let mut successfully_removed = Vec::new();

    let new_assignees = if args.assignees.is_empty() {
        // Remove all assignees
        successfully_removed.extend(current_assignees);
        Vec::new()
    } else {
        // Remove specific assignees
        let mut remaining_assignees = current_issue.assignees.clone();

        for email in &args.assignees {
            if current_assignees.contains(email) {
                remaining_assignees.retain(|a| a.email != *email);
                successfully_removed.push(email.clone());
            } else {
                warnings.push(format!(
                    "User '{}' is not assigned to issue #{}",
                    email, args.id
                ));
            }
        }

        remaining_assignees
    };

    // Update assignees if there are changes
    if !successfully_removed.is_empty() {
        store.update_assignees(args.id, new_assignees, author)?;

        let message = if args.assignees.is_empty() {
            format!("Unassigned all users from issue #{}", args.id)
        } else if successfully_removed.len() == 1 {
            format!(
                "Unassigned {} from issue #{}",
                successfully_removed[0], args.id
            )
        } else {
            format!(
                "Unassigned {} users from issue #{}: {}",
                successfully_removed.len(),
                args.id,
                successfully_removed.join(", ")
            )
        };
        println!("{}", success_message(&message));
    }

    // Display warnings
    for warning in warnings {
        println!("{}", warning_message(&warning));
    }

    // If no successful operations occurred, show a message
    if successfully_removed.is_empty() && !args.assignees.is_empty() {
        println!("No assignments were removed from issue #{}", args.id);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::test_helpers::*;
    use tempfile::TempDir;

    fn setup_temp_assign_repo() -> (TempDir, std::path::PathBuf, IssueId) {
        let temp_dir = TempDir::new().expect("Failed to create temporary directory");
        let repo_path = temp_dir.path().to_path_buf();

        // Create a test issue
        let mut store = IssueStore::init(&repo_path).expect("Failed to initialize store");
        let author = create_test_identity();
        let issue_id = store
            .create_issue(
                "Test Issue".to_string(),
                "Test description".to_string(),
                author,
            )
            .expect("Failed to create test issue");

        (temp_dir, repo_path, issue_id)
    }

    #[test]
    fn test_assign_single_user() {
        let (_temp_dir, repo_path, issue_id) = setup_temp_assign_repo();

        let args = AssignArgs {
            id: issue_id,
            assignees: vec!["user1@example.com".to_string()],
        };

        let result = handle_assign(repo_path.clone(), args);
        assert!(result.is_ok(), "Assign should succeed");

        // Verify the assignment
        let store = IssueStore::open(&repo_path).expect("Should open store");
        let issue = store.get_issue(issue_id).expect("Should get issue");
        assert_eq!(issue.assignees.len(), 1);
        assert_eq!(issue.assignees[0].email, "user1@example.com");
    }

    #[test]
    fn test_assign_multiple_users() {
        let (_temp_dir, repo_path, issue_id) = setup_temp_assign_repo();

        let args = AssignArgs {
            id: issue_id,
            assignees: vec![
                "user1@example.com".to_string(),
                "user2@example.com".to_string(),
            ],
        };

        let result = handle_assign(repo_path.clone(), args);
        assert!(result.is_ok(), "Assign multiple should succeed");

        // Verify the assignments
        let store = IssueStore::open(&repo_path).expect("Should open store");
        let issue = store.get_issue(issue_id).expect("Should get issue");
        assert_eq!(issue.assignees.len(), 2);
        let emails: Vec<String> = issue.assignees.iter().map(|a| a.email.clone()).collect();
        assert!(emails.contains(&"user1@example.com".to_string()));
        assert!(emails.contains(&"user2@example.com".to_string()));
    }

    #[test]
    fn test_assign_duplicate_user() {
        let (_temp_dir, repo_path, issue_id) = setup_temp_assign_repo();

        // First assignment
        let args1 = AssignArgs {
            id: issue_id,
            assignees: vec!["user1@example.com".to_string()],
        };
        handle_assign(repo_path.clone(), args1).expect("First assign should succeed");

        // Try to assign the same user again
        let args2 = AssignArgs {
            id: issue_id,
            assignees: vec!["user1@example.com".to_string()],
        };
        let result = handle_assign(repo_path.clone(), args2);
        assert!(result.is_ok(), "Duplicate assign should succeed but warn");

        // Verify still only one assignee
        let store = IssueStore::open(&repo_path).expect("Should open store");
        let issue = store.get_issue(issue_id).expect("Should get issue");
        assert_eq!(issue.assignees.len(), 1);
    }

    #[test]
    fn test_unassign_all() {
        let (_temp_dir, repo_path, issue_id) = setup_temp_assign_repo();

        // First assign some users
        let assign_args = AssignArgs {
            id: issue_id,
            assignees: vec![
                "user1@example.com".to_string(),
                "user2@example.com".to_string(),
            ],
        };
        handle_assign(repo_path.clone(), assign_args).expect("Assign should succeed");

        // Unassign all
        let unassign_args = UnassignArgs {
            id: issue_id,
            assignees: Vec::new(), // Empty means unassign all
        };
        let result = handle_unassign(repo_path.clone(), unassign_args);
        assert!(result.is_ok(), "Unassign all should succeed");

        // Verify no assignees
        let store = IssueStore::open(&repo_path).expect("Should open store");
        let issue = store.get_issue(issue_id).expect("Should get issue");
        assert!(issue.assignees.is_empty());
    }

    #[test]
    fn test_unassign_specific_user() {
        let (_temp_dir, repo_path, issue_id) = setup_temp_assign_repo();

        // First assign some users
        let assign_args = AssignArgs {
            id: issue_id,
            assignees: vec![
                "user1@example.com".to_string(),
                "user2@example.com".to_string(),
            ],
        };
        handle_assign(repo_path.clone(), assign_args).expect("Assign should succeed");

        // Unassign one specific user
        let unassign_args = UnassignArgs {
            id: issue_id,
            assignees: vec!["user1@example.com".to_string()],
        };
        let result = handle_unassign(repo_path.clone(), unassign_args);
        assert!(result.is_ok(), "Unassign specific should succeed");

        // Verify only user2 remains
        let store = IssueStore::open(&repo_path).expect("Should open store");
        let issue = store.get_issue(issue_id).expect("Should get issue");
        assert_eq!(issue.assignees.len(), 1);
        assert_eq!(issue.assignees[0].email, "user2@example.com");
    }

    #[test]
    fn test_assign_invalid_email() {
        let (_temp_dir, repo_path, issue_id) = setup_temp_assign_repo();

        let args = AssignArgs {
            id: issue_id,
            assignees: vec!["invalid-email".to_string()],
        };

        let result = handle_assign(repo_path, args);
        assert!(result.is_err(), "Should fail with invalid email");
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid email format")
        );
    }

    #[test]
    fn test_assign_no_assignees_assigns_self() {
        let (_temp_dir, repo_path, issue_id) = setup_temp_assign_repo();

        // Get the expected author identity (same as what handle_assign will use)
        let store = IssueStore::open(&repo_path).expect("Should open store");
        let expected_author = crate::cli::commands::get_author_identity(
            None,
            None,
            &store,
            crate::common::SystemEnvProvider,
        )
        .expect("Should get author");

        let args = AssignArgs {
            id: issue_id,
            assignees: Vec::new(),
        };

        let result = handle_assign(repo_path.clone(), args);
        assert!(result.is_ok(), "Should succeed and assign to self");

        // Verify the assignment to the current user
        let store = IssueStore::open(&repo_path).expect("Should open store");
        let issue = store.get_issue(issue_id).expect("Should get issue");
        assert_eq!(issue.assignees.len(), 1);
        assert_eq!(issue.assignees[0].email, expected_author.email);
    }
}
