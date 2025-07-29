# Git Storage Layer Implementation Plan

## Educational Overview: Git's Storage Model

### Git Object Database Fundamentals

Git stores all data in an **object database** using a content-addressable storage system. Every piece of data is stored as an "object" identified by a SHA-1 hash of its content. This means identical content always has the same hash, enabling deduplication and integrity verification.

Git has four fundamental object types:
- **Blob**: Raw file content (like issue event JSON)
- **Tree**: Directory structure (maps names to hashes) 
- **Commit**: Snapshot with metadata (author, timestamp, parent commits)
- **Tag**: Named reference to another object

### References (Refs) System

Git uses **references** to create human-readable names that point to objects:
- `refs/heads/main` → points to latest commit on main branch
- `refs/tags/v1.0` → points to a specific commit or tag object
- `refs/remotes/origin/main` → tracks remote branch state

We'll use a custom ref namespace: `refs/git-issue/issues/{issue-id}`

## Git-Tracker Storage Strategy

### Issue Identification System

Issues will use **sequential u64 identifiers** starting from 1:
- Issue IDs: 1, 2, 3, 4, ...
- Stored in `refs/git-issue/issues/1`, `refs/git-issue/issues/2`, etc.
- Next issue ID tracked in `refs/git-issue/meta/next-issue-id`
- Comments use format: `{issue-id}-{sequence}` (e.g., "1-1", "1-2" for issue 1's comments)

### Issue Storage as Event Chains

Each issue will be stored as a **chain of commit objects**, where:
1. **First commit** contains the "Created" event (issue birth)
2. **Subsequent commits** contain additional events (status changes, comments, etc.)
3. **Each commit** points to its parent, forming an immutable event log
4. **Issue ref** (`refs/git-issue/issues/{issue-id}`) points to the latest commit

### Object Structure Design

```
refs/git-issue/issues/1
    ↓
Commit Object (latest event)
├── Tree Object
│   └── event.json (blob with IssueEvent JSON)
├── Parent: Previous event commit (or none for first)
├── Author: Event author identity  
└── Message: "StatusChanged: todo → in-progress"

Previous Event Commit
├── Tree Object  
│   └── event.json (blob with previous IssueEvent JSON)
├── Parent: Even earlier commit
├── Author: Previous event author
└── Message: "Created: Fix authentication bug"
```

## Implementation Components

### 1. GitRepository (`src/storage/git_repo.rs`)

**Core Responsibilities:**
- Low-level git operations abstraction
- Repository lifecycle management  
- Object storage and retrieval
- Reference management

**Detailed Interface:**
```rust
pub struct GitRepository {
    repo: gix::Repository,
    refs_namespace: String, // "refs/git-issue"
}

impl GitRepository {
    // Repository Management
    pub fn open(path: &Path) -> Result<Self, GitError>;
    pub fn init(path: &Path) -> Result<Self, GitError>;
    pub fn ensure_namespace(&mut self) -> Result<(), GitError>;
    
    // Object Operations  
    pub fn write_blob(&mut self, content: &[u8]) -> Result<gix::ObjectId, GitError>;
    pub fn read_blob(&self, oid: gix::ObjectId) -> Result<Vec<u8>, GitError>;
    pub fn write_tree(&mut self, entries: Vec<TreeEntry>) -> Result<gix::ObjectId, GitError>;
    pub fn read_tree(&self, oid: gix::ObjectId) -> Result<Vec<TreeEntry>, GitError>;
    
    // Commit Operations
    pub fn write_commit(&mut self, 
        tree: gix::ObjectId,
        parents: Vec<gix::ObjectId>,
        author: &Identity,
        message: &str
    ) -> Result<gix::ObjectId, GitError>;
    pub fn read_commit(&self, oid: gix::ObjectId) -> Result<CommitData, GitError>;
    
    // Reference Management
    pub fn create_ref(&mut self, name: &str, oid: gix::ObjectId) -> Result<(), GitError>;
    pub fn update_ref(&mut self, name: &str, oid: gix::ObjectId, expected: Option<gix::ObjectId>) -> Result<(), GitError>;
    pub fn read_ref(&self, name: &str) -> Result<Option<gix::ObjectId>, GitError>;
    pub fn delete_ref(&mut self, name: &str) -> Result<(), GitError>;
    pub fn list_refs(&self, prefix: &str) -> Result<Vec<(String, gix::ObjectId)>, GitError>;
    
    // ID Management
    pub fn get_next_issue_id(&self) -> Result<u64, GitError>;
    pub fn increment_issue_id(&mut self) -> Result<u64, GitError>;
}
```

### 2. IssueStore (`src/storage/issue_store.rs`)

**Core Responsibilities:**
- High-level issue CRUD operations
- Event chain to Issue object conversion
- Issue metadata management
- Concurrent access coordination

### 3. EventLog (`src/storage/event_log.rs`)

**Core Responsibilities:**
- Event serialization and commit storage
- Event chain traversal and reconstruction
- Concurrent update detection and merging

### File Structure
```
src/storage/
├── mod.rs              # Public API exports, IssueRepository trait impl
├── git_repo.rs         # Low-level git operations wrapper
├── issue_store.rs      # High-level issue CRUD operations  
├── event_log.rs        # Event chain management
├── errors.rs           # Storage error types
└── tests/              # Integration tests
    ├── git_repo_test.rs
    ├── issue_store_test.rs
    └── event_log_test.rs
```

### Data Model Updates Required

The existing `Issue` and `Comment` structs need modification:
- Change `IssueId` from `String` to `u64`
- Change `CommentId` from `String` to `String` (format: "{issue_id}-{seq}")
- Remove UUID generation from `Issue::new()` and `Comment::new()`
- Add sequential ID parameters to constructors

### Success Criteria
- ✅ Issues use sequential u64 IDs starting from 1
- ✅ Can create/read/update issues via storage API
- ✅ Events persist as immutable git commits with proper parent chains
- ✅ Issue refs point to correct latest commits  
- ✅ Event history can be reconstructed from git objects
- ✅ Next issue ID properly tracked and incremented
- ✅ All operations work with standard git repository
- ✅ Comprehensive test coverage for all storage components