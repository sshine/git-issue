use anyhow::Result;
use clap::Args;

use crate::cli::output::{success_message, warning_message};
use crate::common::{IssueId, SystemEnvProvider};
use crate::storage::IssueStore;

use super::get_author_identity;

#[derive(Args)]
pub struct LabelArgs {
    /// Issue ID to modify labels for
    pub id: IssueId,

    /// Labels to add or remove (use +label to add, -label to remove). Use -- before -label if needed.
    pub labels: Vec<String>,

    /// Author name (defaults to git config)
    #[arg(short = 'n', long)]
    pub author_name: Option<String>,

    /// Author email (defaults to git config)
    #[arg(short = 'e', long)]
    pub author_email: Option<String>,
}

/// Parse label operations from arguments with +/- prefixes
fn parse_label_operations(labels: &[String]) -> Result<(Vec<String>, Vec<String>)> {
    let mut add_labels = Vec::new();
    let mut remove_labels = Vec::new();

    for label_arg in labels {
        if label_arg.is_empty() {
            continue;
        }

        if let Some(label) = label_arg.strip_prefix('+') {
            if label.is_empty() {
                return Err(anyhow::anyhow!("Empty label after '+' prefix"));
            }
            validate_label_name(label)?;
            add_labels.push(label.to_string());
        } else if let Some(label) = label_arg.strip_prefix('-') {
            if label.is_empty() {
                return Err(anyhow::anyhow!("Empty label after '-' prefix"));
            }
            validate_label_name(label)?;
            remove_labels.push(label.to_string());
        } else {
            return Err(anyhow::anyhow!(
                "Label '{}' must start with '+' (to add) or '-' (to remove)",
                label_arg
            ));
        }
    }

    // Note: We allow empty operations to show warnings for invalid attempts

    Ok((add_labels, remove_labels))
}

/// Validate that a label name contains valid characters
fn validate_label_name(label: &str) -> Result<()> {
    if label.trim() != label {
        return Err(anyhow::anyhow!(
            "Label '{}' has leading or trailing whitespace",
            label
        ));
    }

    if label.contains(' ') {
        return Err(anyhow::anyhow!("Label '{}' contains spaces", label));
    }

    if label.is_empty() {
        return Err(anyhow::anyhow!("Label cannot be empty"));
    }

    Ok(())
}

pub fn handle_label(repo_path: std::path::PathBuf, args: LabelArgs) -> Result<()> {
    let mut store = IssueStore::open(&repo_path)?;
    let author = get_author_identity(
        args.author_name,
        args.author_email,
        &store,
        SystemEnvProvider,
    )?;

    // Get the current issue to check existing labels
    let current_issue = store.get_issue(args.id)?;
    let current_labels: std::collections::HashSet<String> =
        current_issue.labels.iter().cloned().collect();

    // Parse the label operations
    if args.labels.is_empty() {
        return Err(anyhow::anyhow!(
            "No label operations specified. Use +label to add or -label to remove"
        ));
    }

    let (add_labels, remove_labels) = parse_label_operations(&args.labels)?;

    let mut warnings = Vec::new();
    let mut successful_adds = Vec::new();
    let mut successful_removes = Vec::new();

    // Process additions
    for label in add_labels {
        if current_labels.contains(&label) {
            warnings.push(format!(
                "Label '{}' already exists on issue #{}",
                label, args.id
            ));
        } else {
            store.add_label(args.id, label.clone(), author.clone())?;
            successful_adds.push(label);
        }
    }

    // Process removals
    for label in remove_labels {
        if !current_labels.contains(&label) {
            warnings.push(format!("Label '{}' not found on issue #{}", label, args.id));
        } else {
            store.remove_label(args.id, label.clone(), author.clone())?;
            successful_removes.push(label);
        }
    }

    // Display results
    if !successful_adds.is_empty() || !successful_removes.is_empty() {
        let mut changes = Vec::new();

        if !successful_adds.is_empty() {
            changes.push(format!("Added: {}", successful_adds.join(", ")));
        }

        if !successful_removes.is_empty() {
            changes.push(format!("Removed: {}", successful_removes.join(", ")));
        }

        println!(
            "{}",
            success_message(&format!(
                "Updated labels for issue #{}: {}",
                args.id,
                changes.join("; ")
            ))
        );
    }

    // Display warnings
    for warning in warnings {
        println!("{}", warning_message(&warning));
    }

    // If no successful operations occurred, show a message
    if successful_adds.is_empty() && successful_removes.is_empty() {
        println!("No label changes were made to issue #{}", args.id);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::test_helpers::*;
    use tempfile::TempDir;

    fn setup_temp_label_repo() -> (TempDir, std::path::PathBuf, IssueId) {
        let temp_dir = TempDir::new().expect("Failed to create temporary directory");
        let repo_path = temp_dir.path().to_path_buf();

        // Create a test issue with some initial labels
        let mut store = IssueStore::init(&repo_path).expect("Failed to initialize store");
        let author = create_test_identity();
        let issue_id = store
            .create_issue(
                "Test Issue".to_string(),
                "Test description".to_string(),
                author.clone(),
            )
            .expect("Failed to create test issue");

        // Add some initial labels
        store
            .add_label(issue_id, "existing-label".to_string(), author.clone())
            .expect("Failed to add initial label");
        store
            .add_label(issue_id, "another-label".to_string(), author)
            .expect("Failed to add initial label");

        (temp_dir, repo_path, issue_id)
    }

    #[test]
    fn test_parse_label_operations_valid() {
        let labels = vec![
            "+bug".to_string(),
            "+feature".to_string(),
            "-old-label".to_string(),
        ];

        let (add_labels, remove_labels) = parse_label_operations(&labels).unwrap();

        assert_eq!(add_labels, vec!["bug", "feature"]);
        assert_eq!(remove_labels, vec!["old-label"]);
    }

    #[test]
    fn test_parse_label_operations_only_adds() {
        let labels = vec!["+bug".to_string(), "+feature".to_string()];

        let (add_labels, remove_labels) = parse_label_operations(&labels).unwrap();

        assert_eq!(add_labels, vec!["bug", "feature"]);
        assert!(remove_labels.is_empty());
    }

    #[test]
    fn test_parse_label_operations_only_removes() {
        let labels = vec!["-bug".to_string(), "-feature".to_string()];

        let (add_labels, remove_labels) = parse_label_operations(&labels).unwrap();

        assert!(add_labels.is_empty());
        assert_eq!(remove_labels, vec!["bug", "feature"]);
    }

    #[test]
    fn test_parse_label_operations_invalid_prefix() {
        let labels = vec!["bug".to_string()]; // Missing + or -

        let result = parse_label_operations(&labels);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must start with"));
    }

    #[test]
    fn test_parse_label_operations_empty_label() {
        let labels = vec!["+".to_string()]; // Empty label after +

        let result = parse_label_operations(&labels);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Empty label"));
    }

    #[test]
    fn test_validate_label_name_valid() {
        assert!(validate_label_name("bug").is_ok());
        assert!(validate_label_name("feature-request").is_ok());
        assert!(validate_label_name("v1.2.3").is_ok());
    }

    #[test]
    fn test_validate_label_name_with_spaces() {
        let result = validate_label_name("bug fix");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("contains spaces"));
    }

    #[test]
    fn test_validate_label_name_with_whitespace() {
        let result = validate_label_name(" bug ");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("whitespace"));
    }

    #[test]
    fn test_handle_label_add_new_labels() {
        let (_temp_dir, repo_path, issue_id) = setup_temp_label_repo();
        let author = create_test_identity();

        let args = LabelArgs {
            id: issue_id,
            labels: vec!["+bug".to_string(), "+feature".to_string()],
            author_name: Some(author.name.clone()),
            author_email: Some(author.email.clone()),
        };

        let result = handle_label(repo_path.clone(), args);
        assert!(result.is_ok(), "Handle label should succeed");

        // Verify labels were added
        let store = IssueStore::open(&repo_path).expect("Should open store");
        let issue = store.get_issue(issue_id).expect("Should get issue");
        assert!(issue.labels.contains(&"bug".to_string()));
        assert!(issue.labels.contains(&"feature".to_string()));
        assert!(issue.labels.contains(&"existing-label".to_string())); // Should still be there
    }

    #[test]
    fn test_handle_label_remove_existing_labels() {
        let (_temp_dir, repo_path, issue_id) = setup_temp_label_repo();
        let author = create_test_identity();

        let args = LabelArgs {
            id: issue_id,
            labels: vec!["-existing-label".to_string()],
            author_name: Some(author.name.clone()),
            author_email: Some(author.email.clone()),
        };

        let result = handle_label(repo_path.clone(), args);
        assert!(result.is_ok(), "Handle label should succeed");

        // Verify label was removed
        let store = IssueStore::open(&repo_path).expect("Should open store");
        let issue = store.get_issue(issue_id).expect("Should get issue");
        assert!(!issue.labels.contains(&"existing-label".to_string()));
        assert!(issue.labels.contains(&"another-label".to_string())); // Should still be there
    }

    #[test]
    fn test_handle_label_mixed_operations() {
        let (_temp_dir, repo_path, issue_id) = setup_temp_label_repo();
        let author = create_test_identity();

        let args = LabelArgs {
            id: issue_id,
            labels: vec![
                "+bug".to_string(),
                "-existing-label".to_string(),
                "+feature".to_string(),
            ],
            author_name: Some(author.name.clone()),
            author_email: Some(author.email.clone()),
        };

        let result = handle_label(repo_path.clone(), args);
        assert!(result.is_ok(), "Handle label should succeed");

        // Verify changes
        let store = IssueStore::open(&repo_path).expect("Should open store");
        let issue = store.get_issue(issue_id).expect("Should get issue");
        assert!(issue.labels.contains(&"bug".to_string()));
        assert!(issue.labels.contains(&"feature".to_string()));
        assert!(!issue.labels.contains(&"existing-label".to_string()));
        assert!(issue.labels.contains(&"another-label".to_string())); // Should still be there
    }

    #[test]
    fn test_handle_label_add_existing_label() {
        let (_temp_dir, repo_path, issue_id) = setup_temp_label_repo();
        let author = create_test_identity();

        let args = LabelArgs {
            id: issue_id,
            labels: vec!["+existing-label".to_string()], // Already exists
            author_name: Some(author.name.clone()),
            author_email: Some(author.email.clone()),
        };

        let result = handle_label(repo_path.clone(), args);
        assert!(
            result.is_ok(),
            "Handle label should succeed even with existing label"
        );

        // Verify no duplicate was added (label count should remain the same)
        let store = IssueStore::open(&repo_path).expect("Should open store");
        let issue = store.get_issue(issue_id).expect("Should get issue");
        assert!(issue.labels.contains(&"existing-label".to_string()));
        assert_eq!(issue.labels.len(), 2); // Should still be just the original 2 labels
    }

    #[test]
    fn test_handle_label_remove_nonexistent_label() {
        let (_temp_dir, repo_path, issue_id) = setup_temp_label_repo();
        let author = create_test_identity();

        let args = LabelArgs {
            id: issue_id,
            labels: vec!["-nonexistent".to_string()],
            author_name: Some(author.name.clone()),
            author_email: Some(author.email.clone()),
        };

        let result = handle_label(repo_path.clone(), args);
        assert!(
            result.is_ok(),
            "Handle label should succeed even with nonexistent label"
        );

        // Verify original labels are unchanged
        let store = IssueStore::open(&repo_path).expect("Should open store");
        let issue = store.get_issue(issue_id).expect("Should get issue");
        assert_eq!(issue.labels.len(), 2); // Should still be the original 2 labels
        assert!(issue.labels.contains(&"existing-label".to_string()));
        assert!(issue.labels.contains(&"another-label".to_string()));
    }

    #[test]
    fn test_handle_label_nonexistent_issue() {
        let (_temp_dir, repo_path, _issue_id) = setup_temp_label_repo();
        let author = create_test_identity();

        let args = LabelArgs {
            id: 9999, // Non-existent issue
            labels: vec!["+bug".to_string()],
            author_name: Some(author.name.clone()),
            author_email: Some(author.email.clone()),
        };

        let result = handle_label(repo_path, args);
        assert!(
            result.is_err(),
            "Handle label should fail for non-existent issue"
        );
    }
}
