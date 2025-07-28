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

    #[error("Concurrent modification detected for issue {issue_id}")]
    ConcurrentModification { issue_id: u64 },

    #[error("Repository initialization error: {message}")]
    RepositoryInit { message: String },

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

    #[error("Object not found: {oid}")]
    ObjectNotFound { oid: String },

    #[error("Invalid object type: expected {expected}, got {actual}")]
    InvalidObjectType { expected: String, actual: String },

    #[error("Reference not found: {ref_name}")]
    ReferenceNotFound { ref_name: String },

    #[error("Reference update failed: {ref_name} - {message}")]
    ReferenceUpdateFailed { ref_name: String, message: String },

    #[error("Failed to create object: {object_type} - {message}")]
    ObjectCreationFailed {
        object_type: String,
        message: String,
    },

    #[error("Failed to read object: {oid} - {message}")]
    ObjectReadFailed { oid: String, message: String },

    #[error("Invalid git object data: {message}")]
    InvalidObjectData { message: String },

    #[error("Tree entry not found: {name}")]
    TreeEntryNotFound { name: String },

    #[error("Invalid tree structure: {message}")]
    InvalidTreeStructure { message: String },

    #[error("Commit parsing failed: {message}")]
    CommitParsingFailed { message: String },

    #[error("Invalid reference name: {ref_name}")]
    InvalidReferenceName { ref_name: String },

    #[error("Concurrent reference update: {ref_name}")]
    ConcurrentReferenceUpdate { ref_name: String },

    #[error("Repository locked: {message}")]
    RepositoryLocked { message: String },

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

// Helper functions for creating common errors
impl GitError {
    pub fn object_not_found(oid: impl AsRef<str>) -> Self {
        GitError::ObjectNotFound {
            oid: oid.as_ref().to_string(),
        }
    }

    pub fn reference_not_found(ref_name: impl AsRef<str>) -> Self {
        GitError::ReferenceNotFound {
            ref_name: ref_name.as_ref().to_string(),
        }
    }

    pub fn invalid_object_type(expected: impl AsRef<str>, actual: impl AsRef<str>) -> Self {
        GitError::InvalidObjectType {
            expected: expected.as_ref().to_string(),
            actual: actual.as_ref().to_string(),
        }
    }

    pub fn operation_failed(operation: impl AsRef<str>, message: impl AsRef<str>) -> Self {
        GitError::OperationFailed {
            operation: operation.as_ref().to_string(),
            message: message.as_ref().to_string(),
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

    pub fn concurrent_modification(issue_id: u64) -> Self {
        StorageError::ConcurrentModification { issue_id }
    }

    pub fn repository_init(message: impl AsRef<str>) -> Self {
        StorageError::RepositoryInit {
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
