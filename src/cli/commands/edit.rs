use anyhow::Result;
use clap::Args;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io::Write;

use crate::common::{Identity, IssueId, IssueStatus, SystemEnvProvider};
use crate::storage::IssueStore;

use super::get_author_identity;

#[derive(Debug, Serialize, Deserialize)]
struct EditableIssue {
    title: String,
    status: String,
    labels: Vec<String>,
    assignee: Option<String>,
    description: String,
}

#[derive(Args)]
pub struct EditArgs {
    /// Issue ID to edit
    pub id: IssueId,

    /// Set title directly (for programmatic access)
    #[arg(short = 't', long)]
    pub title: Option<String>,

    /// Set description directly (for programmatic access)
    #[arg(short = 'd', long)]
    pub description: Option<String>,

    /// Set status directly (for programmatic access)
    #[arg(short = 's', long)]
    pub status: Option<String>,

    /// Add a label (repeatable, for programmatic access)
    #[arg(long)]
    pub add_label: Vec<String>,

    /// Remove a label (repeatable, for programmatic access)
    #[arg(long)]
    pub remove_label: Vec<String>,

    /// Set assignee directly (for programmatic access)
    #[arg(short = 'a', long)]
    pub assignee: Option<String>,

    /// Skip interactive editor, use only CLI arguments
    #[arg(long)]
    pub no_editor: bool,

    /// Author name (defaults to git config)
    #[arg(short = 'n', long)]
    pub author_name: Option<String>,

    /// Author email (defaults to git config)
    #[arg(short = 'e', long)]
    pub author_email: Option<String>,
}

pub fn handle_edit(repo_path: std::path::PathBuf, args: EditArgs) -> Result<()> {
    let mut store = IssueStore::open(&repo_path)?;
    let author = get_author_identity(
        args.author_name.clone(),
        args.author_email.clone(),
        &store,
        SystemEnvProvider,
    )?;

    // Get the current issue
    let current_issue = store.get_issue(args.id)?;

    let editable_issue = if args.no_editor {
        // Programmatic mode - apply CLI arguments directly
        apply_cli_edits(&current_issue, &args, &author.email)?
    } else {
        // Interactive editor mode
        edit_with_editor(&current_issue, &args, &author.email)?
    };

    // Apply changes with change detection
    apply_changes(&mut store, args.id, &current_issue, &editable_issue, author)?;

    Ok(())
}

fn apply_cli_edits(
    current_issue: &crate::common::Issue,
    args: &EditArgs,
    _author_email: &str,
) -> Result<EditableIssue> {
    let mut editable = EditableIssue {
        title: current_issue.title.clone(),
        status: current_issue.status.to_string(),
        labels: current_issue.labels.clone(),
        assignee: current_issue.assignee.as_ref().map(|a| a.email.clone()),
        description: current_issue.description.clone(),
    };

    // Apply CLI overrides
    if let Some(ref title) = args.title {
        editable.title = title.clone();
    }
    if let Some(ref description) = args.description {
        editable.description = description.clone();
    }
    if let Some(ref status) = args.status {
        editable.status = status.clone();
    }
    if let Some(ref assignee) = args.assignee {
        editable.assignee = Some(assignee.clone());
    }

    // Handle labels
    let mut labels_set: HashSet<String> = editable.labels.into_iter().collect();
    for label in &args.add_label {
        labels_set.insert(label.trim().to_string());
    }
    for label in &args.remove_label {
        labels_set.remove(label.trim());
    }
    editable.labels = labels_set.into_iter().collect();
    editable.labels.sort();

    validate_editable_issue(&editable)?;
    Ok(editable)
}

fn edit_with_editor(
    current_issue: &crate::common::Issue,
    _args: &EditArgs,
    author_email: &str,
) -> Result<EditableIssue> {
    // Create default template with current issue or template values
    let template = create_template(current_issue, author_email);

    // Create temporary file with .yaml extension
    let mut temp_file = tempfile::Builder::new().suffix(".yaml").tempfile()?;

    // Write template to file
    writeln!(temp_file, "{}", template)?;
    temp_file.flush()?;

    // Open editor
    edit::edit_file(temp_file.path())?;

    // Read edited content
    let edited_content = fs::read_to_string(temp_file.path())?;

    // Parse YAML
    let editable: EditableIssue = serde_yaml::from_str(&edited_content)
        .map_err(|e| anyhow::anyhow!("Failed to parse YAML: {}", e))?;

    validate_editable_issue(&editable)?;
    Ok(editable)
}

fn create_template(issue: &crate::common::Issue, default_assignee_email: &str) -> String {
    format!(
        r#"# Edit the fields below. Save and close to apply changes.
# Leave fields unchanged to keep current values.
# Set assignee to null to unassign.

title: "{}"
status: {}  # Options: todo, in-progress, done
labels:
{}
assignee: {}  # Optional: email address or null
description: |
{}"#,
        issue.title,
        issue.status,
        if issue.labels.is_empty() {
            "  []".to_string()
        } else {
            issue
                .labels
                .iter()
                .map(|l| format!("  - {}", l))
                .collect::<Vec<_>>()
                .join("\n")
        },
        issue
            .assignee
            .as_ref()
            .map(|a| format!("\"{}\"", a.email))
            .unwrap_or_else(|| format!("\"{}\"", default_assignee_email)),
        issue
            .description
            .lines()
            .map(|line| format!("  {}", line))
            .collect::<Vec<_>>()
            .join("\n")
    )
}

fn validate_editable_issue(editable: &EditableIssue) -> Result<()> {
    // Title must be non-empty after trimming
    if editable.title.trim().is_empty() {
        return Err(anyhow::anyhow!("Title cannot be empty"));
    }

    // Status must be valid
    editable.status.parse::<IssueStatus>()?;

    // Labels must be trimmed and contain no internal spaces
    for label in &editable.labels {
        let trimmed = label.trim();
        if trimmed != label {
            return Err(anyhow::anyhow!(
                "Label '{}' has leading/trailing whitespace",
                label
            ));
        }
        if trimmed.contains(' ') {
            return Err(anyhow::anyhow!("Label '{}' contains spaces", label));
        }
        if trimmed.is_empty() {
            return Err(anyhow::anyhow!("Empty label found"));
        }
    }

    // Assignee email format (basic check)
    if let Some(ref email) = editable.assignee {
        if !email.contains('@') {
            return Err(anyhow::anyhow!("Invalid email format: {}", email));
        }
    }

    Ok(())
}

fn apply_changes(
    store: &mut IssueStore,
    issue_id: IssueId,
    original: &crate::common::Issue,
    edited: &EditableIssue,
    author: Identity,
) -> Result<()> {
    let mut changes = Vec::new();

    // Check title change
    let new_title = edited.title.trim().to_string();
    if original.title != new_title {
        store.update_title(issue_id, new_title.clone(), author.clone())?;
        changes.push(format!("Title: \"{}\" → \"{}\"", original.title, new_title));
    }

    // Check description change
    if original.description != edited.description {
        store.update_description(issue_id, edited.description.clone(), author.clone())?;
        let desc_change = if edited.description.is_empty() {
            "Description: cleared".to_string()
        } else if original.description.is_empty() {
            format!("Description: added ({} chars)", edited.description.len())
        } else {
            let diff = edited.description.len() as i32 - original.description.len() as i32;
            format!("Description: updated ({:+} chars)", diff)
        };
        changes.push(desc_change);
    }

    // Check status change
    let new_status = edited.status.parse::<IssueStatus>()?;
    if original.status != new_status {
        store.update_issue_status(issue_id, new_status, author.clone())?;
        changes.push(format!("Status: {} → {}", original.status, new_status));
    }

    // Check assignee change
    let new_assignee = edited.assignee.as_ref().map(|email| {
        Identity::new("".to_string(), email.clone()) // We don't have name, just email
    });
    if original.assignee != new_assignee {
        store.update_assignee(issue_id, new_assignee, author.clone())?;
        let assignee_change = match (&original.assignee, &edited.assignee) {
            (None, Some(email)) => format!("Assignee: assigned to {}", email),
            (Some(old), None) => format!("Assignee: unassigned from {}", old.email),
            (Some(old), Some(new)) => format!("Assignee: {} → {}", old.email, new),
            (None, None) => unreachable!(),
        };
        changes.push(assignee_change);
    }

    // Check label changes
    let original_labels: HashSet<String> = original.labels.iter().cloned().collect();
    let new_labels: HashSet<String> = edited.labels.iter().cloned().collect();

    let added_labels: Vec<_> = new_labels.difference(&original_labels).collect();
    let removed_labels: Vec<_> = original_labels.difference(&new_labels).collect();

    for label in &added_labels {
        store.add_label(issue_id, (*label).clone(), author.clone())?;
    }
    for label in &removed_labels {
        store.remove_label(issue_id, (*label).clone(), author.clone())?;
    }

    if !added_labels.is_empty() || !removed_labels.is_empty() {
        let mut label_parts = Vec::new();
        if !added_labels.is_empty() {
            let added_str = added_labels
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", +");
            label_parts.push(format!("+{}", added_str));
        }
        if !removed_labels.is_empty() {
            let removed_str = removed_labels
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", -");
            label_parts.push(format!("-{}", removed_str));
        }
        changes.push(format!("Labels: {}", label_parts.join(", ")));
    }

    // Show results
    if changes.is_empty() {
        println!("No changes made to issue #{}", issue_id);
    } else {
        println!("✓ Updated issue #{}:", issue_id);
        for change in changes {
            println!("  • {}", change);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::IssueEvent;
    use crate::storage::test_helpers::*;
    use tempfile::TempDir;

    // Helper functions for event verification

    /// Get all events for an issue in chronological order (oldest first)
    fn get_issue_events(store: &IssueStore, issue_id: IssueId) -> Vec<IssueEvent> {
        store.get_issue_events(issue_id).unwrap_or_default()
    }

    /// Find the last event of a specific type in the issue's event history
    fn find_last_event_of_type<F>(events: &[IssueEvent], predicate: F) -> Option<&IssueEvent>
    where
        F: Fn(&IssueEvent) -> bool,
    {
        events.iter().rev().find(|event| predicate(event))
    }

    /// Assert that a TitleChanged event exists with the specified values
    fn assert_title_changed_event(
        events: &[IssueEvent],
        old_title: &str,
        new_title: &str,
        author: &Identity,
    ) {
        let event =
            find_last_event_of_type(events, |e| matches!(e, IssueEvent::TitleChanged { .. }))
                .expect("Should have TitleChanged event");

        if let IssueEvent::TitleChanged {
            old_title: old,
            new_title: new,
            author: auth,
            ..
        } = event
        {
            assert_eq!(old, old_title, "Old title should match");
            assert_eq!(new, new_title, "New title should match");
            assert_eq!(auth, author, "Author should match");
        } else {
            panic!("Expected TitleChanged event");
        }
    }

    /// Assert that a DescriptionChanged event exists with the specified values
    fn assert_description_changed_event(
        events: &[IssueEvent],
        old_desc: &str,
        new_desc: &str,
        author: &Identity,
    ) {
        let event = find_last_event_of_type(events, |e| {
            matches!(e, IssueEvent::DescriptionChanged { .. })
        })
        .expect("Should have DescriptionChanged event");

        if let IssueEvent::DescriptionChanged {
            old_description: old,
            new_description: new,
            author: auth,
            ..
        } = event
        {
            assert_eq!(old, old_desc, "Old description should match");
            assert_eq!(new, new_desc, "New description should match");
            assert_eq!(auth, author, "Author should match");
        } else {
            panic!("Expected DescriptionChanged event");
        }
    }

    /// Assert that a StatusChanged event exists with the specified values
    fn assert_status_changed_event(
        events: &[IssueEvent],
        old_status: IssueStatus,
        new_status: IssueStatus,
        author: &Identity,
    ) {
        let event =
            find_last_event_of_type(events, |e| matches!(e, IssueEvent::StatusChanged { .. }))
                .expect("Should have StatusChanged event");

        if let IssueEvent::StatusChanged {
            from,
            to,
            author: auth,
            ..
        } = event
        {
            assert_eq!(*from, old_status, "Old status should match");
            assert_eq!(*to, new_status, "New status should match");
            assert_eq!(auth, author, "Author should match");
        } else {
            panic!("Expected StatusChanged event");
        }
    }

    /// Assert that a LabelAdded event exists with the specified values
    fn assert_label_added_event(events: &[IssueEvent], label: &str, author: &Identity) {
        let event = find_last_event_of_type(events, |e| {
            if let IssueEvent::LabelAdded { label: l, .. } = e {
                l == label
            } else {
                false
            }
        })
        .expect(&format!("Should have LabelAdded event for '{}'", label));

        if let IssueEvent::LabelAdded {
            label: l,
            author: auth,
            ..
        } = event
        {
            assert_eq!(l, label, "Label should match");
            assert_eq!(auth, author, "Author should match");
        } else {
            panic!("Expected LabelAdded event");
        }
    }

    /// Assert that a LabelRemoved event exists with the specified values
    fn assert_label_removed_event(events: &[IssueEvent], label: &str, author: &Identity) {
        let event = find_last_event_of_type(events, |e| {
            if let IssueEvent::LabelRemoved { label: l, .. } = e {
                l == label
            } else {
                false
            }
        })
        .expect(&format!("Should have LabelRemoved event for '{}'", label));

        if let IssueEvent::LabelRemoved {
            label: l,
            author: auth,
            ..
        } = event
        {
            assert_eq!(l, label, "Label should match");
            assert_eq!(auth, author, "Author should match");
        } else {
            panic!("Expected LabelRemoved event");
        }
    }

    /// Assert that an AssigneeChanged event exists with the specified values
    fn assert_assignee_changed_event(
        events: &[IssueEvent],
        old_assignee: Option<&Identity>,
        new_assignee: Option<&Identity>,
        author: &Identity,
    ) {
        let event =
            find_last_event_of_type(events, |e| matches!(e, IssueEvent::AssigneeChanged { .. }))
                .expect("Should have AssigneeChanged event");

        if let IssueEvent::AssigneeChanged {
            old_assignee: old,
            new_assignee: new,
            author: auth,
            ..
        } = event
        {
            assert_eq!(old.as_ref(), old_assignee, "Old assignee should match");
            assert_eq!(new.as_ref(), new_assignee, "New assignee should match");
            assert_eq!(auth, author, "Author should match");
        } else {
            panic!("Expected AssigneeChanged event");
        }
    }

    /// Count events of a specific type
    fn count_events<F>(events: &[IssueEvent], predicate: F) -> usize
    where
        F: Fn(&IssueEvent) -> bool,
    {
        events.iter().filter(|event| predicate(event)).count()
    }

    fn setup_temp_edit_repo() -> (TempDir, std::path::PathBuf, IssueId) {
        let temp_dir = TempDir::new().expect("Failed to create temporary directory");
        let repo_path = temp_dir.path().to_path_buf();

        // Create a test issue to edit
        let mut store = IssueStore::init(&repo_path).expect("Failed to initialize store");
        let author = create_test_identity();
        let issue_id = store
            .create_issue(
                "Original Title".to_string(),
                "Original description".to_string(),
                author,
            )
            .expect("Failed to create test issue");

        (temp_dir, repo_path, issue_id)
    }

    #[test]
    fn test_edit_title_change() {
        let (_temp_dir, repo_path, issue_id) = setup_temp_edit_repo();
        let author = create_test_identity();

        let args = EditArgs {
            id: issue_id,
            title: Some("Updated Title".to_string()),
            description: None,
            status: None,
            add_label: Vec::new(),
            remove_label: Vec::new(),
            assignee: None,
            no_editor: true,
            author_name: Some(author.name.clone()),
            author_email: Some(author.email.clone()),
        };

        let result = handle_edit(repo_path.clone(), args);
        assert!(result.is_ok(), "Edit title should succeed");

        // Verify the change was applied
        let store = IssueStore::open(&repo_path).expect("Should open store");
        let issue = store.get_issue(issue_id).expect("Should get issue");
        assert_eq!(issue.title, "Updated Title");
        assert_eq!(issue.description, "Original description"); // Should be unchanged

        // Verify the correct event was created
        let events = get_issue_events(&store, issue_id);
        assert_eq!(events.len(), 2, "Should have Created + TitleChanged events");
        assert_title_changed_event(&events, "Original Title", "Updated Title", &author);
    }

    #[test]
    fn test_edit_description_change() {
        let (_temp_dir, repo_path, issue_id) = setup_temp_edit_repo();
        let author = create_test_identity();

        let args = EditArgs {
            id: issue_id,
            title: None,
            description: Some("Updated description".to_string()),
            status: None,
            add_label: Vec::new(),
            remove_label: Vec::new(),
            assignee: None,
            no_editor: true,
            author_name: Some(author.name.clone()),
            author_email: Some(author.email.clone()),
        };

        let result = handle_edit(repo_path.clone(), args);
        assert!(result.is_ok(), "Edit description should succeed");

        // Verify the change was applied
        let store = IssueStore::open(&repo_path).expect("Should open store");
        let issue = store.get_issue(issue_id).expect("Should get issue");
        assert_eq!(issue.description, "Updated description");
        assert_eq!(issue.title, "Original Title"); // Should be unchanged

        // Verify the correct event was created
        let events = get_issue_events(&store, issue_id);
        assert_eq!(
            events.len(),
            2,
            "Should have Created + DescriptionChanged events"
        );
        assert_description_changed_event(
            &events,
            "Original description",
            "Updated description",
            &author,
        );
    }

    #[test]
    fn test_edit_status_change() {
        let (_temp_dir, repo_path, issue_id) = setup_temp_edit_repo();
        let author = create_test_identity();

        let args = EditArgs {
            id: issue_id,
            title: None,
            description: None,
            status: Some("in-progress".to_string()),
            add_label: Vec::new(),
            remove_label: Vec::new(),
            assignee: None,
            no_editor: true,
            author_name: Some(author.name.clone()),
            author_email: Some(author.email.clone()),
        };

        let result = handle_edit(repo_path.clone(), args);
        assert!(result.is_ok(), "Edit status should succeed");

        // Verify the change was applied
        let store = IssueStore::open(&repo_path).expect("Should open store");
        let issue = store.get_issue(issue_id).expect("Should get issue");
        assert_eq!(issue.status, IssueStatus::InProgress);

        // Verify the correct event was created
        let events = get_issue_events(&store, issue_id);
        assert_eq!(
            events.len(),
            2,
            "Should have Created + StatusChanged events"
        );
        assert_status_changed_event(&events, IssueStatus::Todo, IssueStatus::InProgress, &author);
    }

    #[test]
    fn test_edit_add_single_label() {
        let (_temp_dir, repo_path, issue_id) = setup_temp_edit_repo();
        let author = create_test_identity();

        let args = EditArgs {
            id: issue_id,
            title: None,
            description: None,
            status: None,
            add_label: vec!["bug".to_string()],
            remove_label: Vec::new(),
            assignee: None,
            no_editor: true,
            author_name: Some(author.name.clone()),
            author_email: Some(author.email.clone()),
        };

        let result = handle_edit(repo_path.clone(), args);
        assert!(result.is_ok(), "Edit add label should succeed");

        // Verify the change was applied
        let store = IssueStore::open(&repo_path).expect("Should open store");
        let issue = store.get_issue(issue_id).expect("Should get issue");
        assert!(issue.labels.contains(&"bug".to_string()));
        assert_eq!(issue.labels.len(), 1);

        // Verify the correct event was created
        let events = get_issue_events(&store, issue_id);
        assert_eq!(events.len(), 2, "Should have Created + LabelAdded events");
        assert_label_added_event(&events, "bug", &author);
    }

    #[test]
    fn test_edit_add_multiple_labels() {
        let (_temp_dir, repo_path, issue_id) = setup_temp_edit_repo();
        let author = create_test_identity();

        let args = EditArgs {
            id: issue_id,
            title: None,
            description: None,
            status: None,
            add_label: vec!["bug".to_string(), "feature".to_string()],
            remove_label: Vec::new(),
            assignee: None,
            no_editor: true,
            author_name: Some(author.name.clone()),
            author_email: Some(author.email.clone()),
        };

        let result = handle_edit(repo_path.clone(), args);
        assert!(result.is_ok(), "Edit add multiple labels should succeed");

        // Verify the changes were applied
        let store = IssueStore::open(&repo_path).expect("Should open store");
        let issue = store.get_issue(issue_id).expect("Should get issue");
        assert!(issue.labels.contains(&"bug".to_string()));
        assert!(issue.labels.contains(&"feature".to_string()));
        assert_eq!(issue.labels.len(), 2);
    }

    #[test]
    fn test_edit_remove_label() {
        let (_temp_dir, repo_path, issue_id) = setup_temp_edit_repo();
        let author = create_test_identity();

        // First add some labels
        let mut store = IssueStore::open(&repo_path).expect("Should open store");
        store
            .add_label(issue_id, "bug".to_string(), author.clone())
            .expect("Should add label");
        store
            .add_label(issue_id, "feature".to_string(), author.clone())
            .expect("Should add label");

        let args = EditArgs {
            id: issue_id,
            title: None,
            description: None,
            status: None,
            add_label: Vec::new(),
            remove_label: vec!["bug".to_string()],
            assignee: None,
            no_editor: true,
            author_name: Some(author.name.clone()),
            author_email: Some(author.email.clone()),
        };

        let result = handle_edit(repo_path.clone(), args);
        assert!(result.is_ok(), "Edit remove label should succeed");

        // Verify the change was applied
        let store = IssueStore::open(&repo_path).expect("Should open store");
        let issue = store.get_issue(issue_id).expect("Should get issue");
        assert!(!issue.labels.contains(&"bug".to_string()));
        assert!(issue.labels.contains(&"feature".to_string()));
        assert_eq!(issue.labels.len(), 1);

        // Verify the correct events were created
        let events = get_issue_events(&store, issue_id);
        assert_eq!(
            events.len(),
            4,
            "Should have Created + 2 LabelAdded + 1 LabelRemoved events"
        );

        // Count specific event types to verify all operations
        let label_added_count =
            count_events(&events, |e| matches!(e, IssueEvent::LabelAdded { .. }));
        let label_removed_count =
            count_events(&events, |e| matches!(e, IssueEvent::LabelRemoved { .. }));
        assert_eq!(label_added_count, 2, "Should have 2 LabelAdded events");
        assert_eq!(label_removed_count, 1, "Should have 1 LabelRemoved event");

        assert_label_removed_event(&events, "bug", &author);
    }

    #[test]
    fn test_edit_add_and_remove_labels() {
        let (_temp_dir, repo_path, issue_id) = setup_temp_edit_repo();
        let author = create_test_identity();

        // First add some labels
        let mut store = IssueStore::open(&repo_path).expect("Should open store");
        store
            .add_label(issue_id, "old-label".to_string(), author.clone())
            .expect("Should add label");

        let args = EditArgs {
            id: issue_id,
            title: None,
            description: None,
            status: None,
            add_label: vec!["new-label".to_string()],
            remove_label: vec!["old-label".to_string()],
            assignee: None,
            no_editor: true,
            author_name: Some(author.name.clone()),
            author_email: Some(author.email.clone()),
        };

        let result = handle_edit(repo_path.clone(), args);
        assert!(result.is_ok(), "Edit add and remove labels should succeed");

        // Verify the changes were applied
        let store = IssueStore::open(&repo_path).expect("Should open store");
        let issue = store.get_issue(issue_id).expect("Should get issue");
        assert!(!issue.labels.contains(&"old-label".to_string()));
        assert!(issue.labels.contains(&"new-label".to_string()));
        assert_eq!(issue.labels.len(), 1);
    }

    #[test]
    fn test_edit_assign_user() {
        let (_temp_dir, repo_path, issue_id) = setup_temp_edit_repo();
        let author = create_test_identity();

        let args = EditArgs {
            id: issue_id,
            title: None,
            description: None,
            status: None,
            add_label: Vec::new(),
            remove_label: Vec::new(),
            assignee: Some("assignee@example.com".to_string()),
            no_editor: true,
            author_name: Some(author.name.clone()),
            author_email: Some(author.email.clone()),
        };

        let result = handle_edit(repo_path.clone(), args);
        assert!(result.is_ok(), "Edit assign user should succeed");

        // Verify the change was applied
        let store = IssueStore::open(&repo_path).expect("Should open store");
        let issue = store.get_issue(issue_id).expect("Should get issue");
        assert!(issue.assignee.is_some());
        assert_eq!(issue.assignee.unwrap().email, "assignee@example.com");

        // Verify the correct event was created
        let events = get_issue_events(&store, issue_id);
        assert_eq!(
            events.len(),
            2,
            "Should have Created + AssigneeChanged events"
        );
        let new_assignee = Identity::new("".to_string(), "assignee@example.com".to_string());
        assert_assignee_changed_event(&events, None, Some(&new_assignee), &author);
    }

    #[test]
    fn test_edit_multiple_changes() {
        let (_temp_dir, repo_path, issue_id) = setup_temp_edit_repo();
        let author = create_test_identity();

        let args = EditArgs {
            id: issue_id,
            title: Some("New Title".to_string()),
            description: Some("New description".to_string()),
            status: Some("done".to_string()),
            add_label: vec!["enhancement".to_string()],
            remove_label: Vec::new(),
            assignee: Some("developer@example.com".to_string()),
            no_editor: true,
            author_name: Some(author.name.clone()),
            author_email: Some(author.email.clone()),
        };

        let result = handle_edit(repo_path.clone(), args);
        assert!(result.is_ok(), "Edit multiple changes should succeed");

        // Verify all changes were applied
        let store = IssueStore::open(&repo_path).expect("Should open store");
        let issue = store.get_issue(issue_id).expect("Should get issue");
        assert_eq!(issue.title, "New Title");
        assert_eq!(issue.description, "New description");
        assert_eq!(issue.status, IssueStatus::Done);
        assert!(issue.labels.contains(&"enhancement".to_string()));
        assert!(issue.assignee.is_some());
        assert_eq!(issue.assignee.unwrap().email, "developer@example.com");

        // Verify all the correct events were created
        let events = get_issue_events(&store, issue_id);
        assert_eq!(events.len(), 6, "Should have Created + 5 change events");

        // Verify each type of event was created with correct values
        assert_title_changed_event(&events, "Original Title", "New Title", &author);
        assert_description_changed_event(
            &events,
            "Original description",
            "New description",
            &author,
        );
        assert_status_changed_event(&events, IssueStatus::Todo, IssueStatus::Done, &author);
        assert_label_added_event(&events, "enhancement", &author);

        let new_assignee = Identity::new("".to_string(), "developer@example.com".to_string());
        assert_assignee_changed_event(&events, None, Some(&new_assignee), &author);
    }

    #[test]
    fn test_edit_no_changes() {
        let (_temp_dir, repo_path, issue_id) = setup_temp_edit_repo();
        let author = create_test_identity();

        let args = EditArgs {
            id: issue_id,
            title: None,
            description: None,
            status: None,
            add_label: Vec::new(),
            remove_label: Vec::new(),
            assignee: None,
            no_editor: true,
            author_name: Some(author.name.clone()),
            author_email: Some(author.email.clone()),
        };

        let result = handle_edit(repo_path.clone(), args);
        assert!(result.is_ok(), "Edit with no changes should succeed");

        // Verify issue remains unchanged
        let store = IssueStore::open(&repo_path).expect("Should open store");
        let issue = store.get_issue(issue_id).expect("Should get issue");
        assert_eq!(issue.title, "Original Title");
        assert_eq!(issue.description, "Original description");
        assert_eq!(issue.status, IssueStatus::Todo);
        assert!(issue.labels.is_empty());
        assert!(issue.assignee.is_none());

        // Verify no additional events were created (only the original Created event)
        let events = get_issue_events(&store, issue_id);
        assert_eq!(
            events.len(),
            1,
            "Should only have the original Created event"
        );
        assert!(matches!(events[0], IssueEvent::Created { .. }));
    }

    #[test]
    fn test_edit_same_title() {
        let (_temp_dir, repo_path, issue_id) = setup_temp_edit_repo();
        let author = create_test_identity();

        let args = EditArgs {
            id: issue_id,
            title: Some("Original Title".to_string()), // Same as current
            description: None,
            status: None,
            add_label: Vec::new(),
            remove_label: Vec::new(),
            assignee: None,
            no_editor: true,
            author_name: Some(author.name.clone()),
            author_email: Some(author.email.clone()),
        };

        let result = handle_edit(repo_path.clone(), args);
        assert!(result.is_ok(), "Edit same title should succeed (no-op)");

        // Verify no change detection works
        let store = IssueStore::open(&repo_path).expect("Should open store");
        let issue = store.get_issue(issue_id).expect("Should get issue");
        assert_eq!(issue.title, "Original Title");
    }

    #[test]
    fn test_edit_nonexistent_issue() {
        let (_temp_dir, repo_path, _issue_id) = setup_temp_edit_repo();
        let author = create_test_identity();

        let args = EditArgs {
            id: 9999, // Non-existent issue
            title: Some("Should Fail".to_string()),
            description: None,
            status: None,
            add_label: Vec::new(),
            remove_label: Vec::new(),
            assignee: None,
            no_editor: true,
            author_name: Some(author.name.clone()),
            author_email: Some(author.email.clone()),
        };

        let result = handle_edit(repo_path, args);
        assert!(result.is_err(), "Edit nonexistent issue should fail");
    }

    #[test]
    fn test_validate_editable_issue_empty_title() {
        let editable = EditableIssue {
            title: "".to_string(),
            status: "todo".to_string(),
            labels: Vec::new(),
            assignee: None,
            description: "Description".to_string(),
        };

        let result = validate_editable_issue(&editable);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Title cannot be empty")
        );
    }

    #[test]
    fn test_validate_editable_issue_label_with_spaces() {
        let editable = EditableIssue {
            title: "Valid Title".to_string(),
            status: "todo".to_string(),
            labels: vec!["label with spaces".to_string()],
            assignee: None,
            description: "Description".to_string(),
        };

        let result = validate_editable_issue(&editable);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("contains spaces"));
    }

    #[test]
    fn test_validate_editable_issue_invalid_status() {
        let editable = EditableIssue {
            title: "Valid Title".to_string(),
            status: "invalid-status".to_string(),
            labels: Vec::new(),
            assignee: None,
            description: "Description".to_string(),
        };

        let result = validate_editable_issue(&editable);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid status"));
    }

    #[test]
    fn test_validate_editable_issue_invalid_email() {
        let editable = EditableIssue {
            title: "Valid Title".to_string(),
            status: "todo".to_string(),
            labels: Vec::new(),
            assignee: Some("not-an-email".to_string()),
            description: "Description".to_string(),
        };

        let result = validate_editable_issue(&editable);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid email format")
        );
    }

    #[test]
    fn test_validate_editable_issue_valid() {
        let editable = EditableIssue {
            title: "Valid Title".to_string(),
            status: "in-progress".to_string(),
            labels: vec!["bug".to_string(), "urgent".to_string()],
            assignee: Some("user@example.com".to_string()),
            description: "Valid description".to_string(),
        };

        let result = validate_editable_issue(&editable);
        assert!(result.is_ok());
    }

    #[test]
    fn test_edit_comprehensive_event_verification() {
        let (_temp_dir, repo_path, issue_id) = setup_temp_edit_repo();
        let author = create_test_identity();

        // Step 1: Add initial labels and assign initial user
        let mut store = IssueStore::open(&repo_path).expect("Should open store");
        store
            .add_label(issue_id, "old-label".to_string(), author.clone())
            .expect("Should add label");
        store
            .update_assignee(
                issue_id,
                Some(Identity::new("".to_string(), "old@example.com".to_string())),
                author.clone(),
            )
            .expect("Should assign");

        // Step 2: Perform comprehensive edit with multiple changes
        let args = EditArgs {
            id: issue_id,
            title: Some("Comprehensive Test Title".to_string()),
            description: Some("Comprehensive test description with detailed info".to_string()),
            status: Some("in-progress".to_string()),
            add_label: vec!["new-feature".to_string(), "tested".to_string()],
            remove_label: vec!["old-label".to_string()],
            assignee: Some("new@example.com".to_string()),
            no_editor: true,
            author_name: Some(author.name.clone()),
            author_email: Some(author.email.clone()),
        };

        let result = handle_edit(repo_path.clone(), args);
        assert!(result.is_ok(), "Comprehensive edit should succeed");

        // Step 3: Verify all changes were applied to issue state
        let store = IssueStore::open(&repo_path).expect("Should open store");
        let issue = store.get_issue(issue_id).expect("Should get issue");

        assert_eq!(issue.title, "Comprehensive Test Title");
        assert_eq!(
            issue.description,
            "Comprehensive test description with detailed info"
        );
        assert_eq!(issue.status, IssueStatus::InProgress);
        assert!(issue.labels.contains(&"new-feature".to_string()));
        assert!(issue.labels.contains(&"tested".to_string()));
        assert!(!issue.labels.contains(&"old-label".to_string()));
        assert_eq!(issue.assignee.unwrap().email, "new@example.com");

        // Step 4: Comprehensive event verification using all helper functions
        let events = get_issue_events(&store, issue_id);

        // Should have: Created + LabelAdded + AssigneeChanged + TitleChanged + DescriptionChanged + StatusChanged + 2×LabelAdded + LabelRemoved + AssigneeChanged
        assert_eq!(events.len(), 10, "Should have all expected events");

        // Verify specific event types using helper functions
        assert_title_changed_event(
            &events,
            "Original Title",
            "Comprehensive Test Title",
            &author,
        );
        assert_description_changed_event(
            &events,
            "Original description",
            "Comprehensive test description with detailed info",
            &author,
        );
        assert_status_changed_event(&events, IssueStatus::Todo, IssueStatus::InProgress, &author);
        assert_label_added_event(&events, "new-feature", &author);
        assert_label_added_event(&events, "tested", &author);
        assert_label_removed_event(&events, "old-label", &author);

        let old_assignee = Identity::new("".to_string(), "old@example.com".to_string());
        let new_assignee = Identity::new("".to_string(), "new@example.com".to_string());
        assert_assignee_changed_event(&events, Some(&old_assignee), Some(&new_assignee), &author);

        // Use counting helper to verify event type counts
        let created_count = count_events(&events, |e| matches!(e, IssueEvent::Created { .. }));
        let title_changed_count =
            count_events(&events, |e| matches!(e, IssueEvent::TitleChanged { .. }));
        let description_changed_count = count_events(&events, |e| {
            matches!(e, IssueEvent::DescriptionChanged { .. })
        });
        let status_changed_count =
            count_events(&events, |e| matches!(e, IssueEvent::StatusChanged { .. }));
        let label_added_count =
            count_events(&events, |e| matches!(e, IssueEvent::LabelAdded { .. }));
        let label_removed_count =
            count_events(&events, |e| matches!(e, IssueEvent::LabelRemoved { .. }));
        let assignee_changed_count =
            count_events(&events, |e| matches!(e, IssueEvent::AssigneeChanged { .. }));

        assert_eq!(created_count, 1, "Should have 1 Created event");
        assert_eq!(title_changed_count, 1, "Should have 1 TitleChanged event");
        assert_eq!(
            description_changed_count, 1,
            "Should have 1 DescriptionChanged event"
        );
        assert_eq!(status_changed_count, 1, "Should have 1 StatusChanged event");
        assert_eq!(label_added_count, 3, "Should have 3 LabelAdded events"); // initial + 2 from edit
        assert_eq!(label_removed_count, 1, "Should have 1 LabelRemoved event");
        assert_eq!(
            assignee_changed_count, 2,
            "Should have 2 AssigneeChanged events"
        ); // initial + edit
    }
}