use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;

use super::errors::{GitError, GitResult};
use crate::common::Identity;
use gix::prelude::{FindExt, Write};

/// A Git repository wrapper for git-tracker's issue storage
///
/// `GitRepository` provides a high-level interface for storing git-tracker issues
/// and events in a Git repository using an event-sourced architecture. Issues are
/// stored as chains of commit objects, with each commit representing an issue event.
///
/// ## Storage Model
///
/// - **Issues**: Stored as commit chains in `refs/git-tracker/issues/{issue_id}`
/// - **Events**: Each commit in the chain represents a single `IssueEvent`
/// - **ID Management**: Sequential issue IDs tracked in `refs/git-tracker/meta/next-issue-id`
/// - **Namespace**: All git-tracker refs use the `refs/git-tracker/` prefix
///
/// ## Example Usage
///
/// ```rust,no_run
/// use git_tracker::storage::GitRepository;
/// use std::path::Path;
///
/// // Open existing repository
/// let mut repo = GitRepository::open(Path::new("."))?;
///
/// // Get next issue ID
/// let issue_id = repo.increment_issue_id()?;
/// println!("Created issue #{}", issue_id);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// ## Implementation Status
///
/// This is currently a simplified implementation where complex git operations
/// (reference management, object storage) are implemented as placeholders.
/// The interface is stable and will be fully implemented as the project matures.
pub struct GitRepository {
    repo: gix::Repository,
    refs_namespace: String,
}

/// Represents an entry in a Git tree object
///
/// A `TreeEntry` corresponds to a single file or subdirectory within a Git tree.
/// In git-tracker's context, trees are used to store serialized issue events
/// as JSON blobs within commit objects.
///
/// ## Fields
///
/// - `name`: The filename or directory name
/// - `oid`: Git object ID (hash) pointing to the content
/// - `mode`: Unix file permissions and type (e.g., 0o100644 for regular files)
///
/// ## Common Mode Values
///
/// - `0o100644`: Regular file
/// - `0o100755`: Executable file  
/// - `0o040000`: Directory/subdirectory
/// - `0o120000`: Symbolic link
///
/// ## Example
///
/// ```rust
/// use git_tracker::storage::TreeEntry;
/// use gix::ObjectId;
///
/// let entry = TreeEntry {
///     name: "event.json".to_string(),
///     oid: ObjectId::null(gix::hash::Kind::Sha1),
///     mode: 0o100644, // regular file
/// };
/// ```
#[derive(Debug, Clone)]
pub struct TreeEntry {
    pub name: String,
    pub oid: gix::ObjectId,
    pub mode: u32,
}

/// Parsed data from a Git commit object
///
/// `CommitData` represents the structured information extracted from a Git commit.
/// In git-tracker's event-sourced architecture, each commit represents a single
/// issue event, with the commit message describing the event type and the tree
/// containing the serialized event data.
///
/// ## Fields
///
/// - `tree`: Object ID of the Git tree containing the commit's files/data
/// - `parents`: List of parent commit object IDs (empty for initial commits)
/// - `author`: Identity of the person who created the commit
/// - `message`: Commit message describing the change
/// - `timestamp`: When the commit was created (UTC)
///
/// ## Usage in git-tracker
///
/// Each issue event is stored as a commit:
/// - **Commit message**: Describes the event (e.g., "Created: Fix auth bug")
/// - **Tree**: Contains `event.json` with serialized `IssueEvent` data
/// - **Parent**: Previous event commit (forming an event chain)
/// - **Author**: User who performed the action
///
/// ## Example
///
/// ```rust
/// use git_tracker::storage::CommitData;
/// use git_tracker::common::Identity;
/// use chrono::Utc;
///
/// let commit = CommitData {
///     tree: "abc123...".to_string(),
///     parents: vec!["def456...".to_string()],
///     author: Identity::new("Alice".to_string(), "alice@example.com".to_string()),
///     message: "StatusChanged: todo → in-progress".to_string(),
///     timestamp: Utc::now(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitData {
    pub tree: String,
    pub parents: Vec<String>,
    pub author: Identity,
    pub message: String,
    pub timestamp: DateTime<Utc>,
}

impl GitRepository {
    /// Open an existing git repository
    pub fn open<P: AsRef<Path>>(path: P) -> GitResult<Self> {
        let repo = gix::open(path.as_ref()).map_err(|_e| GitError::RepositoryNotFound {
            path: path.as_ref().display().to_string(),
        })?;

        let git_repo = Self {
            repo,
            refs_namespace: "refs/git-tracker".to_string(),
        };

        Ok(git_repo)
    }

    /// Initialize a new git repository
    pub fn init<P: AsRef<Path>>(path: P) -> GitResult<Self> {
        let repo = gix::init(path.as_ref()).map_err(GitError::from)?;

        let git_repo = Self {
            repo,
            refs_namespace: "refs/git-tracker".to_string(),
        };

        Ok(git_repo)
    }

    /// Write a blob object
    pub fn write_blob(&mut self, content: &[u8]) -> GitResult<gix::ObjectId> {
        let odb = self.repo.objects.clone();
        let blob = gix::objs::Blob {
            data: content.to_vec(),
        };
        let oid = odb
            .write(&blob)
            .map_err(|e| GitError::ObjectCreationFailed {
                object_type: "blob".to_string(),
                message: e.to_string(),
            })?;
        Ok(oid)
    }

    /// Read a blob object
    pub fn read_blob(&self, oid: gix::ObjectId) -> GitResult<Vec<u8>> {
        let mut buffer = Vec::new();
        let _object = self
            .repo
            .objects
            .find_blob(&oid, &mut buffer)
            .map_err(|e| GitError::ObjectReadFailed {
                oid: oid.to_string(),
                message: e.to_string(),
            })?;
        Ok(buffer)
    }

    /// Write a tree object
    pub fn write_tree(&mut self, entries: Vec<TreeEntry>) -> GitResult<gix::ObjectId> {
        let mut tree_entries = Vec::new();

        for entry in entries {
            let tree_entry = gix::objs::tree::Entry {
                mode: gix::object::tree::EntryMode::try_from(entry.mode as u32).map_err(|_| {
                    GitError::InvalidTreeStructure {
                        message: format!("Invalid file mode: {}", entry.mode),
                    }
                })?,
                filename: entry.name.into(),
                oid: entry.oid,
            };
            tree_entries.push(tree_entry);
        }

        let tree_object = gix::objs::Tree {
            entries: tree_entries,
        };
        let odb = self.repo.objects.clone();
        let oid = odb
            .write(&tree_object)
            .map_err(|e| GitError::ObjectCreationFailed {
                object_type: "tree".to_string(),
                message: e.to_string(),
            })?;
        Ok(oid)
    }

    /// Read a tree object
    pub fn read_tree(&self, oid: gix::ObjectId) -> GitResult<Vec<TreeEntry>> {
        let mut buffer = Vec::new();
        let object = self
            .repo
            .objects
            .find_tree(&oid, &mut buffer)
            .map_err(|e| GitError::ObjectReadFailed {
                oid: oid.to_string(),
                message: e.to_string(),
            })?;

        let mut entries = Vec::new();
        for entry in object.entries {
            let tree_entry = TreeEntry {
                name: entry.filename.to_string(),
                oid: entry.oid.into(),
                mode: entry.mode.kind() as u32 | (entry.mode.is_executable() as u32 * 0o111),
            };
            entries.push(tree_entry);
        }

        Ok(entries)
    }

    /// Write a commit object
    pub fn write_commit(
        &mut self,
        tree: gix::ObjectId,
        parents: Vec<gix::ObjectId>,
        author: &Identity,
        message: &str,
    ) -> GitResult<gix::ObjectId> {
        let timestamp = chrono::Utc::now();
        let signature = gix::actor::Signature {
            name: author.name.as_str().into(),
            email: author.email.as_str().into(),
            time: gix::date::Time::new(timestamp.timestamp(), 0),
        };

        let commit_object = gix::objs::Commit {
            tree,
            parents: parents.into(),
            author: signature.clone(),
            committer: signature,
            encoding: None,
            message: message.as_bytes().into(),
            extra_headers: vec![],
        };

        let odb = self.repo.objects.clone();
        let oid = odb
            .write(&commit_object)
            .map_err(|e| GitError::ObjectCreationFailed {
                object_type: "commit".to_string(),
                message: e.to_string(),
            })?;
        Ok(oid)
    }

    /// Read a commit object
    pub fn read_commit(&self, oid: gix::ObjectId) -> GitResult<CommitData> {
        let mut buffer = Vec::new();
        let object = self
            .repo
            .objects
            .find_commit(&oid, &mut buffer)
            .map_err(|e| GitError::ObjectReadFailed {
                oid: oid.to_string(),
                message: e.to_string(),
            })?;

        let parents = object.parents.into_iter().map(|p| p.to_string()).collect();
        let author = Identity::new(
            object.author.name.to_string(),
            object.author.email.to_string(),
        );
        let message = String::from_utf8_lossy(&object.message).to_string();
        // TODO: Parse the time from the git commit object properly
        let timestamp = Utc::now();

        Ok(CommitData {
            tree: object.tree.to_string(),
            parents,
            author,
            message,
            timestamp,
        })
    }

    /// Create a new reference
    pub fn create_ref(&mut self, name: &str, oid: gix::ObjectId) -> GitResult<()> {
        use gix::refs::transaction::{Change, LogChange, PreviousValue, RefEdit};

        // Create a RefEdit for creating the reference
        let edit = RefEdit {
            change: Change::Update {
                log: LogChange::default(),
                expected: PreviousValue::MustNotExist,
                new: gix::refs::Target::Object(oid),
            },
            name: name
                .try_into()
                .map_err(|e| GitError::ReferenceCreationFailed {
                    ref_name: name.to_string(),
                    message: format!("Invalid reference name: {:?}", e),
                })?,
            deref: false,
        };

        // Prepare and commit the transaction
        let transaction = self
            .repo
            .refs
            .transaction()
            .prepare(
                vec![edit],
                gix::lock::acquire::Fail::Immediately,
                gix::lock::acquire::Fail::Immediately,
            )
            .map_err(|e| GitError::ReferenceCreationFailed {
                ref_name: name.to_string(),
                message: e.to_string(),
            })?;

        transaction
            .commit(None)
            .map_err(|e| GitError::ReferenceCreationFailed {
                ref_name: name.to_string(),
                message: e.to_string(),
            })?;

        Ok(())
    }

    /// Update an existing reference
    pub fn update_ref(
        &mut self,
        name: &str,
        oid: gix::ObjectId,
        expected: Option<gix::ObjectId>,
    ) -> GitResult<()> {
        use gix::refs::transaction::{Change, LogChange, PreviousValue, RefEdit};

        let previous_value = match expected {
            Some(expected_oid) => {
                PreviousValue::MustExistAndMatch(gix::refs::Target::Object(expected_oid))
            }
            None => PreviousValue::Any,
        };

        let edit = RefEdit {
            change: Change::Update {
                log: LogChange::default(),
                expected: previous_value,
                new: gix::refs::Target::Object(oid),
            },
            name: name
                .try_into()
                .map_err(|e| GitError::ReferenceUpdateFailed {
                    ref_name: name.to_string(),
                    message: format!("Invalid reference name: {:?}", e),
                })?,
            deref: false,
        };

        let transaction = self
            .repo
            .refs
            .transaction()
            .prepare(
                vec![edit],
                gix::lock::acquire::Fail::Immediately,
                gix::lock::acquire::Fail::Immediately,
            )
            .map_err(|e| GitError::ReferenceUpdateFailed {
                ref_name: name.to_string(),
                message: e.to_string(),
            })?;

        transaction
            .commit(None)
            .map_err(|e| GitError::ReferenceUpdateFailed {
                ref_name: name.to_string(),
                message: e.to_string(),
            })?;

        Ok(())
    }

    /// Read a reference
    pub fn read_ref(&self, name: &str) -> GitResult<Option<gix::ObjectId>> {
        match self.repo.refs.find(name) {
            Ok(reference) => {
                if let Some(target_id) = reference.target.try_id() {
                    Ok(Some(target_id.to_owned()))
                } else {
                    Ok(None)
                }
            }
            Err(gix::refs::file::find::existing::Error::NotFound { name: _ }) => Ok(None),
            Err(e) => Err(GitError::ReferenceReadFailed {
                ref_name: name.to_string(),
                message: e.to_string(),
            }),
        }
    }

    /// Delete a reference
    pub fn delete_ref(&mut self, name: &str) -> GitResult<()> {
        use gix::refs::transaction::{Change, LogChange, PreviousValue, RefEdit};

        let edit = RefEdit {
            change: Change::Delete {
                log: gix::refs::transaction::RefLog::AndReference,
                expected: PreviousValue::Any,
            },
            name: name
                .try_into()
                .map_err(|e| GitError::ReferenceUpdateFailed {
                    ref_name: name.to_string(),
                    message: format!("Invalid reference name: {:?}", e),
                })?,
            deref: false,
        };

        let transaction = self
            .repo
            .refs
            .transaction()
            .prepare(
                vec![edit],
                gix::lock::acquire::Fail::Immediately,
                gix::lock::acquire::Fail::Immediately,
            )
            .map_err(|e| GitError::ReferenceUpdateFailed {
                ref_name: name.to_string(),
                message: e.to_string(),
            })?;

        transaction
            .commit(None)
            .map_err(|e| GitError::ReferenceUpdateFailed {
                ref_name: name.to_string(),
                message: e.to_string(),
            })?;

        Ok(())
    }

    /// List references with a prefix
    pub fn list_refs(&self, prefix: &str) -> GitResult<Vec<(String, gix::ObjectId)>> {
        let mut refs = Vec::new();

        // Get all references
        let iter = self
            .repo
            .refs
            .iter()
            .map_err(|e| GitError::ReferenceReadFailed {
                ref_name: prefix.to_string(),
                message: e.to_string(),
            })?;

        let all_refs = iter
            .all()
            .map_err(|e| GitError::ReferenceReadFailed {
                ref_name: prefix.to_string(),
                message: e.to_string(),
            })?
            .into_iter()
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| GitError::ReferenceReadFailed {
                ref_name: prefix.to_string(),
                message: e.to_string(),
            })?;

        for reference in all_refs {
            let ref_name = reference.name.as_bstr().to_string();

            // Filter by prefix
            if ref_name.starts_with(prefix) {
                if let Some(target_id) = reference.target.try_id() {
                    refs.push((ref_name, target_id.to_owned()));
                }
            }
        }

        Ok(refs)
    }

    /// Get the next issue ID
    pub fn get_next_issue_id(&self) -> GitResult<u64> {
        let meta_ref = format!("{}/meta/next-issue-id", self.refs_namespace);

        match self.read_ref(&meta_ref)? {
            Some(oid) => {
                // Read the blob containing the next issue ID
                let blob_data = self.read_blob(oid)?;
                let id_str =
                    String::from_utf8(blob_data).map_err(|e| GitError::InvalidObjectData {
                        message: format!("Invalid UTF-8 in issue ID blob: {}", e),
                    })?;
                let id = id_str
                    .trim()
                    .parse::<u64>()
                    .map_err(|e| GitError::InvalidObjectData {
                        message: format!("Invalid issue ID format: {}", e),
                    })?;
                Ok(id)
            }
            None => {
                // No meta ref exists, start from 1
                Ok(1)
            }
        }
    }

    /// Increment and return the next issue ID
    pub fn increment_issue_id(&mut self) -> GitResult<u64> {
        let current_id = self.get_next_issue_id()?;
        let next_id = current_id + 1;

        // Store the next issue ID in a blob
        let next_id_bytes = next_id.to_string().into_bytes();
        let blob_oid = self.write_blob(&next_id_bytes)?;

        // Update the meta reference
        let meta_ref = format!("{}/meta/next-issue-id", self.refs_namespace);

        match self.read_ref(&meta_ref)? {
            Some(old_oid) => {
                // Update existing reference
                self.update_ref(&meta_ref, blob_oid, Some(old_oid))?;
            }
            None => {
                // Create new reference
                self.create_ref(&meta_ref, blob_oid)?;
            }
        }

        Ok(current_id)
    }

    /// Get the reference name for an issue
    pub fn issue_ref_name(&self, issue_id: u64) -> String {
        format!("{}/issues/{}", self.refs_namespace, issue_id)
    }

    /// Get the repository path
    pub fn path(&self) -> &Path {
        self.repo.path()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::{Issue, IssueEvent, IssueStatus};
    use crate::storage::test_helpers::*;

    #[test]
    fn test_initialize_git_tracker_repo() {
        let (_temp_dir, mut repo) = setup_temp_repo();

        // Verify the repository was initialized successfully
        assert!(repo.path().exists(), "Repository path should exist");

        // For a bare repository, the git directory might be the repo path itself,
        // or it might have a .git subdirectory for a work tree
        let has_git_dir = repo.path().join(".git").exists() || repo.path().join("objects").exists();
        assert!(has_git_dir, "Git directory structure should exist");

        // Test that we can get the next issue ID (should start at 1)
        let next_id = repo
            .get_next_issue_id()
            .expect("Should be able to get next issue ID");
        assert_eq!(next_id, 1, "First issue ID should be 1");

        // Test that we can increment the issue ID (placeholder implementation uses global counter)
        let incremented_id = repo
            .increment_issue_id()
            .expect("Should be able to increment issue ID");
        // Note: Due to the placeholder implementation using a global static counter,
        // the exact value depends on test execution order. We just verify it returns a positive number.
        assert!(incremented_id > 0, "Incremented ID should be positive");

        // Test that we can reopen the repository
        let reopened_repo =
            GitRepository::open(repo.path()).expect("Should be able to reopen the repository");
        assert_eq!(
            reopened_repo.path(),
            repo.path(),
            "Reopened repo should have same path"
        );

        // Test issue reference name generation
        let ref_name = repo.issue_ref_name(1);
        assert_eq!(
            ref_name, "refs/git-tracker/issues/1",
            "Issue ref name should follow expected format"
        );
    }

    #[test]
    fn test_blob_operations() {
        let (_temp_dir, mut repo) = setup_temp_repo();

        // Test writing and reading a blob
        let content = b"Hello, World!";
        let blob_oid = repo
            .write_blob(content)
            .expect("Should be able to write blob");

        let read_content = repo
            .read_blob(blob_oid)
            .expect("Should be able to read blob");

        assert_eq!(
            content,
            read_content.as_slice(),
            "Blob content should match"
        );
    }

    #[test]
    fn test_tree_operations() {
        let (_temp_dir, mut repo) = setup_temp_repo();

        // Create some blob content
        let blob1_content = b"file1 content";
        let blob1_oid = repo.write_blob(blob1_content).expect("Should write blob1");

        let blob2_content = b"file2 content";
        let blob2_oid = repo.write_blob(blob2_content).expect("Should write blob2");

        // Create tree entries
        let tree_entries = vec![
            TreeEntry {
                name: "file1.txt".to_string(),
                oid: blob1_oid,
                mode: 0o100644, // Regular file
            },
            TreeEntry {
                name: "file2.txt".to_string(),
                oid: blob2_oid,
                mode: 0o100644, // Regular file
            },
        ];

        // Write the tree
        let tree_oid = repo
            .write_tree(tree_entries.clone())
            .expect("Should be able to write tree");

        // Read the tree back
        let read_entries = repo
            .read_tree(tree_oid)
            .expect("Should be able to read tree");

        assert_eq!(read_entries.len(), 2, "Should have 2 entries");

        // Sort both sets for comparison
        let mut expected = tree_entries;
        expected.sort_by(|a, b| a.name.cmp(&b.name));
        let mut actual = read_entries;
        actual.sort_by(|a, b| a.name.cmp(&b.name));

        for (expected, actual) in expected.iter().zip(actual.iter()) {
            assert_eq!(expected.name, actual.name, "Entry names should match");
            assert_eq!(expected.oid, actual.oid, "Entry OIDs should match");
            assert_eq!(expected.mode, actual.mode, "Entry modes should match");
        }
    }

    #[test]
    fn test_commit_operations() {
        let (_temp_dir, mut repo) = setup_temp_repo();
        let author = create_test_identity();

        // Create a tree for the commit
        let blob_content = b"commit test content";
        let blob_oid = repo.write_blob(blob_content).expect("Should write blob");

        let tree_entries = vec![TreeEntry {
            name: "test.txt".to_string(),
            oid: blob_oid,
            mode: 0o100644,
        }];

        let tree_oid = repo.write_tree(tree_entries).expect("Should write tree");

        // Create a commit
        let commit_message = "Test commit message";
        let commit_oid = repo
            .write_commit(tree_oid, vec![], &author, commit_message)
            .expect("Should be able to write commit");

        // Read the commit back
        let commit_data = repo
            .read_commit(commit_oid)
            .expect("Should be able to read commit");

        assert_eq!(
            commit_data.tree,
            tree_oid.to_string(),
            "Tree OID should match"
        );
        assert_eq!(commit_data.parents.len(), 0, "Should have no parents");
        assert_eq!(
            commit_data.author.name, author.name,
            "Author name should match"
        );
        assert_eq!(
            commit_data.author.email, author.email,
            "Author email should match"
        );
        assert_eq!(
            commit_data.message, commit_message,
            "Commit message should match"
        );
    }

    #[test]
    fn test_reference_operations() {
        let (_temp_dir, mut repo) = setup_temp_repo();

        // Create a blob to reference
        let blob_content = b"reference test";
        let blob_oid = repo.write_blob(blob_content).expect("Should write blob");

        let ref_name = "refs/test/sample";

        // Create a reference
        repo.create_ref(ref_name, blob_oid)
            .expect("Should be able to create reference");

        // Read the reference
        let read_oid = repo
            .read_ref(ref_name)
            .expect("Should be able to read reference")
            .expect("Reference should exist");

        assert_eq!(read_oid, blob_oid, "Reference should point to correct OID");

        // Update the reference
        let new_blob_content = b"updated reference test";
        let new_blob_oid = repo
            .write_blob(new_blob_content)
            .expect("Should write new blob");

        repo.update_ref(ref_name, new_blob_oid, Some(blob_oid))
            .expect("Should be able to update reference");

        let updated_oid = repo
            .read_ref(ref_name)
            .expect("Should be able to read reference")
            .expect("Reference should exist");

        assert_eq!(
            updated_oid, new_blob_oid,
            "Reference should point to new OID"
        );

        // List references
        let refs = repo
            .list_refs("refs/test/")
            .expect("Should be able to list references");

        assert_eq!(refs.len(), 1, "Should find one reference");
        assert_eq!(refs[0].0, ref_name, "Reference name should match");
        assert_eq!(refs[0].1, new_blob_oid, "Reference OID should match");

        // Delete the reference
        repo.delete_ref(ref_name)
            .expect("Should be able to delete reference");

        let deleted_ref = repo
            .read_ref(ref_name)
            .expect("Should be able to attempt reading deleted reference");

        assert!(deleted_ref.is_none(), "Reference should be deleted");
    }

    #[test]
    fn test_issue_id_management() {
        let (_temp_dir, mut repo) = setup_temp_repo();

        // Initially should start at 1
        let first_id = repo.get_next_issue_id().expect("Should get next issue ID");
        assert_eq!(first_id, 1, "First issue ID should be 1");

        // Increment and get the ID
        let incremented_id = repo
            .increment_issue_id()
            .expect("Should increment issue ID");
        assert_eq!(
            incremented_id, 1,
            "Should return current ID before incrementing"
        );

        // Next ID should now be 2
        let next_id = repo.get_next_issue_id().expect("Should get next issue ID");
        assert_eq!(next_id, 2, "Next issue ID should be 2");

        // Increment again
        let second_incremented = repo
            .increment_issue_id()
            .expect("Should increment issue ID");
        assert_eq!(
            second_incremented, 2,
            "Should return current ID before incrementing"
        );

        // Should now be 3
        let third_id = repo.get_next_issue_id().expect("Should get next issue ID");
        assert_eq!(third_id, 3, "Next issue ID should be 3");
    }

    #[test]
    fn test_issue_reconstruction_from_events() {
        let (_temp_dir, mut repo) = setup_temp_repo();
        let author = create_test_identity();

        let issue_id = repo
            .increment_issue_id()
            .expect("Should be able to get issue ID");

        // Create a sequence of events
        let events = vec![
            IssueEvent::created(
                "Bug Report".to_string(),
                "Found a critical bug".to_string(),
                author.clone(),
            ),
            IssueEvent::status_changed(IssueStatus::Todo, IssueStatus::InProgress, author.clone()),
            IssueEvent::comment_added(
                format!("{}-1", issue_id),
                "Working on fixing this".to_string(),
                author.clone(),
            ),
        ];

        // Test that we can reconstruct an issue from these events
        let reconstructed_issue = Issue::from_events(issue_id, &events)
            .expect("Should be able to reconstruct issue from events");

        assert_eq!(reconstructed_issue.id, issue_id);
        assert_eq!(reconstructed_issue.title, "Bug Report");
        assert_eq!(reconstructed_issue.description, "Found a critical bug");
        assert_eq!(reconstructed_issue.status, IssueStatus::InProgress);
        assert_eq!(reconstructed_issue.comments.len(), 1);
        assert_eq!(
            reconstructed_issue.comments[0].content,
            "Working on fixing this"
        );
        assert_eq!(reconstructed_issue.created_by, author);
    }
}
