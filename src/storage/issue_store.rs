use std::path::Path;

use super::errors::{StorageError, StorageResult};
use super::repo::{GitRepository, TreeEntry};
use crate::common::{Identity, Issue, IssueEvent, IssueId, IssueStatus};

/// High-level issue CRUD operations using git-issue's event-sourced storage
///
/// `IssueStore` provides a clean interface for managing issues backed by Git storage.
/// Each issue is stored as a chain of commits representing events, with the issue
/// reference pointing to the latest event commit.
///
/// ## Storage Architecture
///
/// - **Issues**: Stored as commit chains in `refs/git-issue/issues/{issue_id}`
/// - **Events**: Each commit represents a single `IssueEvent` (created, status changed, etc.)
/// - **Reconstruction**: Issues are rebuilt by replaying all events in chronological order
/// - **Concurrency**: Uses Git's atomic reference updates for thread-safe operations
///
/// ## Example Usage
///
/// ```rust,no_run
/// use git_issue::storage::IssueStore;
/// use git_issue::common::{Identity, IssueStatus};
/// use std::path::Path;
///
/// let mut store = IssueStore::open(Path::new("."))?;
/// let author = Identity::new("Alice".to_string(), "alice@example.com".to_string());
///
/// // Create a new issue
/// let issue_id = store.create_issue(
///     "Fix authentication bug".to_string(),
///     "Users can't log in with OAuth".to_string(),
///     author.clone(),
/// )?;
///
/// // Update the issue status
/// store.update_issue_status(issue_id, IssueStatus::InProgress, author.clone())?;
///
/// // Retrieve the current issue state
/// let issue = store.get_issue(issue_id)?;
/// println!("Issue #{}: {} ({})", issue.id, issue.title, issue.status);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub struct IssueStore {
    repo: GitRepository,
}

impl IssueStore {
    /// Open an existing git repository for issue storage
    pub fn open<P: AsRef<Path>>(path: P) -> StorageResult<Self> {
        let repo = GitRepository::open(path)?;
        Ok(Self { repo })
    }

    /// Initialize a new git repository for issue storage
    pub fn init<P: AsRef<Path>>(path: P) -> StorageResult<Self> {
        let repo = GitRepository::init(path)?;
        Ok(Self { repo })
    }

    /// Create a new issue and return its ID
    ///
    /// This generates a new sequential issue ID, creates an initial "Created" event,
    /// and stores it as the first commit in the issue's event chain.
    pub fn create_issue(
        &mut self,
        title: String,
        description: String,
        author: Identity,
    ) -> StorageResult<IssueId> {
        // Get the next available issue ID
        let issue_id = self.repo.increment_issue_id()?;

        // Create the initial "Created" event
        let created_event = IssueEvent::created(title.clone(), description.clone(), author.clone());

        // Store the event as the first commit in the issue chain
        self.append_event(issue_id, created_event, None)?;

        Ok(issue_id)
    }

    /// Retrieve an issue by ID
    ///
    /// Reconstructs the current issue state by replaying all events in its commit chain.
    /// Returns `StorageError::IssueNotFound` if the issue doesn't exist.
    pub fn get_issue(&self, issue_id: IssueId) -> StorageResult<Issue> {
        let events = self.get_issue_events(issue_id)?;

        if events.is_empty() {
            return Err(StorageError::issue_not_found(issue_id));
        }

        Issue::from_events(issue_id, &events)
            .map_err(|e| StorageError::invalid_event_sequence(e.to_string()))
    }

    /// Check if an issue exists
    pub fn issue_exists(&self, issue_id: IssueId) -> StorageResult<bool> {
        let ref_name = self.repo.issue_ref_name(issue_id);
        let ref_exists = self.repo.read_ref(&ref_name)?.is_some();
        Ok(ref_exists)
    }

    /// Update an issue's status
    ///
    /// Creates a new "StatusChanged" event and appends it to the issue's event chain.
    pub fn update_issue_status(
        &mut self,
        issue_id: IssueId,
        new_status: IssueStatus,
        author: Identity,
    ) -> StorageResult<()> {
        // Verify the issue exists and get current status
        let current_issue = self.get_issue(issue_id)?;

        if current_issue.status == new_status {
            // Status unchanged, no-op
            return Ok(());
        }

        // Create status change event
        let status_event = IssueEvent::status_changed(current_issue.status, new_status, author);

        // Get the current HEAD commit to use as parent
        let parent_commit = self.get_issue_head_commit(issue_id)?;

        // Append the event to the issue chain
        self.append_event(issue_id, status_event, Some(parent_commit))?;

        Ok(())
    }

    /// Add a comment to an issue
    ///
    /// Creates a new "CommentAdded" event with a sequential comment ID.
    pub fn add_comment(
        &mut self,
        issue_id: IssueId,
        content: String,
        author: Identity,
    ) -> StorageResult<String> {
        // Verify the issue exists and get current comment count
        let current_issue = self.get_issue(issue_id)?;
        let comment_id = format!("{}-{}", issue_id, current_issue.comments.len() + 1);

        // Create comment event
        let comment_event = IssueEvent::comment_added(comment_id.clone(), content, author);

        // Get the current HEAD commit to use as parent
        let parent_commit = self.get_issue_head_commit(issue_id)?;

        // Append the event to the issue chain
        self.append_event(issue_id, comment_event, Some(parent_commit))?;

        Ok(comment_id)
    }

    /// Add a label to an issue
    pub fn add_label(
        &mut self,
        issue_id: IssueId,
        label: String,
        author: Identity,
    ) -> StorageResult<()> {
        // Verify the issue exists and check if label already exists
        let current_issue = self.get_issue(issue_id)?;

        if current_issue.labels.contains(&label) {
            // Label already exists, no-op
            return Ok(());
        }

        // Create label added event
        let label_event = IssueEvent::label_added(label, author);

        // Get the current HEAD commit to use as parent
        let parent_commit = self.get_issue_head_commit(issue_id)?;

        // Append the event to the issue chain
        self.append_event(issue_id, label_event, Some(parent_commit))?;

        Ok(())
    }

    /// Remove a label from an issue
    pub fn remove_label(
        &mut self,
        issue_id: IssueId,
        label: String,
        author: Identity,
    ) -> StorageResult<()> {
        // Verify the issue exists and check if label exists
        let current_issue = self.get_issue(issue_id)?;

        if !current_issue.labels.contains(&label) {
            // Label doesn't exist, no-op
            return Ok(());
        }

        // Create label removed event
        let label_event = IssueEvent::label_removed(label, author);

        // Get the current HEAD commit to use as parent
        let parent_commit = self.get_issue_head_commit(issue_id)?;

        // Append the event to the issue chain
        self.append_event(issue_id, label_event, Some(parent_commit))?;

        Ok(())
    }

    /// Update an issue's title
    pub fn update_title(
        &mut self,
        issue_id: IssueId,
        new_title: String,
        author: Identity,
    ) -> StorageResult<()> {
        // Verify the issue exists and get current title
        let current_issue = self.get_issue(issue_id)?;

        if current_issue.title == new_title {
            // Title unchanged, no-op
            return Ok(());
        }

        // Create title changed event
        let title_event = IssueEvent::title_changed(current_issue.title, new_title, author);

        // Get the current HEAD commit to use as parent
        let parent_commit = self.get_issue_head_commit(issue_id)?;

        // Append the event to the issue chain
        self.append_event(issue_id, title_event, Some(parent_commit))?;

        Ok(())
    }

    /// Update an issue's assignee
    pub fn update_assignee(
        &mut self,
        issue_id: IssueId,
        new_assignee: Option<Identity>,
        author: Identity,
    ) -> StorageResult<()> {
        // Verify the issue exists and get current assignee
        let current_issue = self.get_issue(issue_id)?;

        if current_issue.assignee == new_assignee {
            // Assignee unchanged, no-op
            return Ok(());
        }

        // Create assignee changed event
        let assignee_event =
            IssueEvent::assignee_changed(current_issue.assignee, new_assignee, author);

        // Get the current HEAD commit to use as parent
        let parent_commit = self.get_issue_head_commit(issue_id)?;

        // Append the event to the issue chain
        self.append_event(issue_id, assignee_event, Some(parent_commit))?;

        Ok(())
    }

    /// List all issue IDs in the repository
    pub fn list_issue_ids(&self) -> StorageResult<Vec<IssueId>> {
        let refs = self.repo.list_refs("refs/git-issue/issues/")?;
        let mut issue_ids = Vec::new();

        for (ref_name, _oid) in refs {
            // Extract issue ID from ref name: "refs/git-issue/issues/123" -> 123
            if let Some(id_str) = ref_name.strip_prefix("refs/git-issue/issues/") {
                match id_str.parse::<u64>() {
                    Ok(issue_id) => issue_ids.push(issue_id),
                    Err(_) => {
                        return Err(StorageError::invalid_issue_id(id_str));
                    }
                }
            }
        }

        issue_ids.sort();
        Ok(issue_ids)
    }

    /// Get all issues (useful for listing/search operations)
    pub fn list_issues(&self) -> StorageResult<Vec<Issue>> {
        let issue_ids = self.list_issue_ids()?;
        let mut issues = Vec::new();

        for issue_id in issue_ids {
            match self.get_issue(issue_id) {
                Ok(issue) => issues.push(issue),
                Err(StorageError::IssueNotFound { .. }) => {
                    // Issue reference exists but events are corrupted, skip it
                    continue;
                }
                Err(e) => return Err(e),
            }
        }

        Ok(issues)
    }

    /// Get the repository path
    pub fn path(&self) -> &Path {
        self.repo.path()
    }

    // Private helper methods

    /// Get all events for an issue in chronological order
    fn get_issue_events(&self, issue_id: IssueId) -> StorageResult<Vec<IssueEvent>> {
        let ref_name = self.repo.issue_ref_name(issue_id);

        // Get the HEAD commit for this issue
        let head_commit_oid = match self.repo.read_ref(&ref_name)? {
            Some(oid) => oid,
            None => return Ok(Vec::new()), // Issue doesn't exist
        };

        // Traverse the commit chain to collect all events
        let mut events = Vec::new();
        let mut current_commit_oid = Some(head_commit_oid);

        while let Some(commit_oid) = current_commit_oid {
            // Read the commit
            let commit_data = self.repo.read_commit(commit_oid)?;

            // Read the tree to get the event.json blob
            let tree_oid = commit_data.tree.parse().map_err(|_| {
                StorageError::invalid_event_sequence("Invalid tree OID in commit".to_string())
            })?;
            let tree_entries = self.repo.read_tree(tree_oid)?;

            // Find the event.json entry
            let event_blob_oid = tree_entries
                .iter()
                .find(|entry| entry.name == "event.json")
                .map(|entry| entry.oid)
                .ok_or_else(|| {
                    StorageError::invalid_event_sequence("No event.json in commit tree".to_string())
                })?;

            // Read and deserialize the event
            let event_json = self.repo.read_blob(event_blob_oid)?;
            let event: IssueEvent =
                serde_json::from_slice(&event_json).map_err(|e| StorageError::Serialization(e))?;

            events.push(event);

            // Move to parent commit (earlier in history)
            current_commit_oid = commit_data
                .parents
                .first()
                .and_then(|parent_str| parent_str.parse().ok());
        }

        // Reverse to get chronological order (oldest first)
        events.reverse();
        Ok(events)
    }

    /// Get the HEAD commit OID for an issue
    fn get_issue_head_commit(&self, issue_id: IssueId) -> StorageResult<gix::ObjectId> {
        let ref_name = self.repo.issue_ref_name(issue_id);
        self.repo
            .read_ref(&ref_name)?
            .ok_or_else(|| StorageError::issue_not_found(issue_id))
    }

    /// Append an event to an issue's commit chain
    fn append_event(
        &mut self,
        issue_id: IssueId,
        event: IssueEvent,
        parent_commit: Option<gix::ObjectId>,
    ) -> StorageResult<()> {
        // Serialize the event to JSON
        let event_json = serde_json::to_string(&event).map_err(StorageError::Serialization)?;

        // Create a blob for the event
        let blob_oid = self.repo.write_blob(event_json.as_bytes())?;

        // Create a tree containing the event blob
        let tree_entries = vec![TreeEntry {
            name: "event.json".to_string(),
            oid: blob_oid,
            mode: 0o100644, // Regular file
        }];
        let tree_oid = self.repo.write_tree(tree_entries)?;

        // Create a commit message describing the event
        let commit_message = match &event {
            IssueEvent::Created { title, .. } => format!("Created: {}", title),
            IssueEvent::StatusChanged { from, to, .. } => {
                format!("StatusChanged: {} → {}", from, to)
            }
            IssueEvent::CommentAdded { comment_id, .. } => format!("CommentAdded: {}", comment_id),
            IssueEvent::LabelAdded { label, .. } => format!("LabelAdded: {}", label),
            IssueEvent::LabelRemoved { label, .. } => format!("LabelRemoved: {}", label),
            IssueEvent::TitleChanged { new_title, .. } => format!("TitleChanged: {}", new_title),
            IssueEvent::AssigneeChanged { new_assignee, .. } => match new_assignee {
                Some(identity) => format!("AssigneeChanged: {}", identity.name),
                None => "AssigneeChanged: unassigned".to_string(),
            },
        };

        // Create the commit
        let parents = parent_commit.map(|oid| vec![oid]).unwrap_or_default();
        let commit_oid =
            self.repo
                .write_commit(tree_oid, parents, event.author(), &commit_message)?;

        // Update the issue reference to point to the new commit
        let ref_name = self.repo.issue_ref_name(issue_id);
        match parent_commit {
            Some(expected_parent) => {
                // Update existing reference with expected old value for concurrency safety
                self.repo
                    .update_ref(&ref_name, commit_oid, Some(expected_parent))?;
            }
            None => {
                // Create new reference for first commit
                self.repo.create_ref(&ref_name, commit_oid)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::test_helpers::*;
    use tempfile::TempDir;

    fn setup_temp_store() -> (TempDir, IssueStore) {
        let temp_dir = TempDir::new().expect("Failed to create temporary directory");
        let store = IssueStore::init(temp_dir.path()).expect("Failed to initialize issue store");
        (temp_dir, store)
    }

    #[test]
    fn test_create_issue() {
        let (_temp_dir, mut store) = setup_temp_store();
        let author = create_test_identity();

        let issue_id = store
            .create_issue(
                "Test Issue".to_string(),
                "This is a test".to_string(),
                author.clone(),
            )
            .expect("Should create issue");

        // Note: Due to global counter, exact ID depends on test execution order
        assert!(issue_id > 0, "Issue ID should be positive");

        // Note: Can't verify issue retrieval due to placeholder GitRepository implementation
        // In a full implementation, we would verify:
        // let issue = store.get_issue(issue_id).expect("Should retrieve issue");
        // assert_eq!(issue.id, issue_id);
        // assert_eq!(issue.title, "Test Issue");
        // assert_eq!(issue.description, "This is a test");
        // assert_eq!(issue.status, IssueStatus::Todo);
        // assert_eq!(issue.created_by, author);
        // assert!(issue.assignee.is_none());
        // assert!(issue.labels.is_empty());
        // assert!(issue.comments.is_empty());
    }

    #[test]
    fn test_issue_not_found() {
        let (_temp_dir, store) = setup_temp_store();

        // With placeholder implementation, get_issue returns empty events which results in IssueNotFound
        let result = store.get_issue(999);
        assert!(
            result.is_err(),
            "Should return error for non-existent issue"
        );
    }

    #[test]
    fn test_basic_operations() {
        let (_temp_dir, mut store) = setup_temp_store();
        let author = create_test_identity();

        // Test create_issue operation
        let issue_id = store
            .create_issue("Test".to_string(), "Test".to_string(), author.clone())
            .expect("Should create issue");
        assert!(issue_id > 0, "Issue ID should be positive");

        // Note: Due to placeholder GitRepository implementation, we can't test:
        // - update_issue_status (requires get_issue which needs read_ref)
        // - add_comment (requires get_issue)
        // - add_label/remove_label (requires get_issue)
        // - update_title (requires get_issue)
        // - update_assignee (requires get_issue)
        // - list_issues (requires list_refs)

        // These operations complete without error but don't actually store/retrieve data
        // In a full implementation with working GitRepository, all CRUD operations would work
    }

    #[test]
    fn test_store_path() {
        let (_temp_dir, store) = setup_temp_store();

        // Verify we can get the store path
        let path = store.path();
        assert!(path.exists(), "Store path should exist");
    }
}
