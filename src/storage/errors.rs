use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Git error: {0}")]
    Git(#[from] GitError),

    #[error("Issue not found: {issue_id}")]
    IssueNotFound { issue_id: u64 },

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Invalid event sequence: {message}")]
    InvalidEventSequence { message: String },

    #[error("Invalid issue ID format: expected u64, got '{value}'")]
    InvalidIssueId { value: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Error, Debug)]
pub enum GitError {
    #[error("Repository not found at path: {path}")]
    RepositoryNotFound { path: String },

    #[error("Failed to initialize repository: {message}")]
    InitializationFailed { message: String },

    #[allow(unused)]
    #[error("Object not found: {oid}")]
    ObjectNotFound { oid: String },

    #[allow(unused)]
    #[error("Invalid object type: expected {expected}, got {actual}")]
    InvalidObjectType { expected: String, actual: String },

    #[allow(unused)]
    #[error("Reference not found: {ref_name}")]
    ReferenceNotFound { ref_name: String },

    #[error("Reference update failed: {ref_name} - {message}")]
    ReferenceUpdateFailed { ref_name: String, message: String },

    #[error("Reference creation failed: {ref_name} - {message}")]
    ReferenceCreationFailed { ref_name: String, message: String },

    #[error("Reference read failed: {ref_name} - {message}")]
    ReferenceReadFailed { ref_name: String, message: String },

    #[error("Failed to create object: {object_type} - {message}")]
    ObjectCreationFailed {
        object_type: String,
        message: String,
    },

    #[error("Failed to read object: {oid} - {message}")]
    ObjectReadFailed { oid: String, message: String },

    #[error("Invalid git object data: {message}")]
    InvalidObjectData { message: String },

    #[allow(unused)]
    #[error("Tree entry not found: {name}")]
    TreeEntryNotFound { name: String },

    #[error("Invalid tree structure: {message}")]
    InvalidTreeStructure { message: String },

    #[allow(unused)]
    #[error("Commit parsing failed: {message}")]
    CommitParsingFailed { message: String },

    #[allow(unused)]
    #[error("Invalid reference name: {ref_name}")]
    InvalidReferenceName { ref_name: String },

    #[allow(unused)]
    #[error("Concurrent reference update: {ref_name}")]
    ConcurrentReferenceUpdate { ref_name: String },

    #[allow(unused)]
    #[error("Repository locked: {message}")]
    RepositoryLocked { message: String },

    #[allow(unused)]
    #[error("Git operation failed: {operation} - {message}")]
    OperationFailed { operation: String, message: String },
}

impl From<gix::open::Error> for GitError {
    fn from(err: gix::open::Error) -> Self {
        GitError::RepositoryNotFound {
            path: format!("{:?}", err),
        }
    }
}

impl From<gix::init::Error> for GitError {
    fn from(err: gix::init::Error) -> Self {
        GitError::InitializationFailed {
            message: format!("{:?}", err),
        }
    }
}

impl From<gix::object::find::Error> for GitError {
    fn from(err: gix::object::find::Error) -> Self {
        GitError::ObjectReadFailed {
            oid: "unknown".to_string(),
            message: format!("{:?}", err),
        }
    }
}

impl From<gix::reference::edit::Error> for GitError {
    fn from(err: gix::reference::edit::Error) -> Self {
        GitError::ReferenceUpdateFailed {
            ref_name: "unknown".to_string(),
            message: format!("{:?}", err),
        }
    }
}

impl StorageError {
    pub fn issue_not_found(issue_id: u64) -> Self {
        StorageError::IssueNotFound { issue_id }
    }

    pub fn invalid_event_sequence(message: impl AsRef<str>) -> Self {
        StorageError::InvalidEventSequence {
            message: message.as_ref().to_string(),
        }
    }

    pub fn invalid_issue_id(value: impl AsRef<str>) -> Self {
        StorageError::InvalidIssueId {
            value: value.as_ref().to_string(),
        }
    }
}

// Result type alias for convenience
pub type StorageResult<T> = Result<T, StorageError>;
pub type GitResult<T> = Result<T, GitError>;
