# Git Issue Tracker Architecture

An offline-first issue tracker with git backend, designed for simplicity and kanban-style workflows.

## System Overview

This project implements a distributed issue tracking system that stores issue data directly in git
objects, similar to [git-bug](https://github.com/git-bug/git-bug) but with key architectural
differences:

- **Simplified CLI**: Streamlined command interface focused on essential operations
- **Kanban-First Design**: Built-in issue status workflow rather than label-based states only
- **Local multi-user**: Support for multiple concurrent local users for concurrent AI interaction

## Data Model

### Event-Sourced Architecture

Issues are stored as sequences of immutable events in git objects, following event sourcing patterns:

```rust
// Core event types
pub enum IssueEvent {
    Created { title: String, description: String, author: Identity },
    StatusChanged { from: IssueStatus, to: IssueStatus, author: Identity },
    CommentAdded { content: String, author: Identity },
    LabelAdded { label: String, author: Identity },
    LabelRemoved { label: String, author: Identity },
    TitleChanged { old_title: String, new_title: String, author: Identity },
}

// Kanban-first status model
pub enum IssueStatus {
    Todo,
    InProgress,
    Done,
}

// Core Issue aggregate
pub struct Issue {
    pub id: IssueId,
    pub title: String,
    pub description: String,
    pub status: IssueStatus,
    pub labels: Vec<String>,
    pub comments: Vec<Comment>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Identity,
    pub assignee: Option<Identity>,
}

pub struct Comment {
    pub id: CommentId,
    pub content: String,
    pub author: Identity,
    pub created_at: DateTime<Utc>,
}

pub type IssueId = String;  // UUID or git hash
pub type CommentId = String;
```

### Git Storage Strategy

- **Issue Objects**: Stored as git objects with unique hashes
- **Event Log**: Append-only sequence of events per issue
- **Refs Structure**: `refs/git-tracker/issues/{issue-id}` points to latest event
- **Merge Strategy**: Operational Transform for concurrent updates

### Identity Management

```rust
pub struct Identity {
    pub name: String,
    pub email: String,
    pub key: PublicKey,
}
```

## Component Architecture

### 1. Storage Layer (`storage/`)

**Git Operations**
- `GitRepository`: Core git operations wrapper
- `IssueStore`: High-level issue CRUD operations
- `EventLog`: Event persistence and retrieval
- `SyncEngine`: Multi-repository synchronization

**Key Traits**
```rust
pub trait IssueRepository {
    fn create_issue(&mut self, create_event: IssueCreated) -> Result<IssueId>;
    fn apply_event(&mut self, issue_id: IssueId, event: IssueEvent) -> Result<()>;
    fn get_issue(&self, issue_id: IssueId) -> Result<Issue>;
    fn list_issues(&self, filter: IssueFilter) -> Result<Vec<Issue>>;
}
```

### 2. CLI Interface (`cli/`)

**Command Structure**
```
git-tracker <command> [args]

Commands:
  new <title>              Create new issue
  list [--status=todo]     List issues with optional filter  
  show <id>                Show issue details
  status <id> <status>     Change issue status
  comment <id> <text>      Add comment
  sync                     Sync with remotes
```

**Implementation Strategy**
- Use `clap` for command parsing
- Async operations with `tokio`
- Progress indicators for long operations
- Colorized output with `console`

### 3. WebUI (`webui/`)

**Real-time Features**
- **Live Updates**: WebSocket connections for real-time issue changes
- **Multi-user Awareness**: Show who's currently viewing/editing
- **Conflict Resolution**: Visual merge tools for concurrent edits

**Technology Stack**
- `axum`: Web framework
- `tokio-tungstenite`: WebSocket support  
- `serde`: JSON serialization
- Frontend: Vanilla JS with minimal dependencies for performance

**API Design**
```rust
// REST API for basic operations
GET    /api/issues          - List issues
POST   /api/issues          - Create issue
GET    /api/issues/{id}     - Get issue
PATCH  /api/issues/{id}     - Update issue

// WebSocket for real-time updates
WS     /ws                  - Real-time event stream
```

### 4. Synchronization Engine (`sync/`)

**File Watching**
- Monitor `.git/refs/git-tracker/` for external changes
- Detect concurrent modifications
- Trigger UI updates via WebSocket

**Conflict Resolution**
- **Operational Transform**: Automatic resolution of non-conflicting changes
- **Manual Resolution**: UI for handling semantic conflicts
- **Merge Strategies**: Last-writer-wins vs. manual merge

**Multi-repository Sync**
```rust
pub struct SyncManager {
    repos: Vec<RemoteRepository>,
    conflict_resolver: Box<dyn ConflictResolver>,
    event_bus: EventBus,
}
```

## Technology Stack

### Core Dependencies

**Storage & Git**
- `git2`: Git operations
- `serde`: Serialization
- `uuid`: Issue ID generation
- `chrono`: Timestamps

**Event Sourcing**
- `eventually`: Event sourcing framework
- `async-trait`: Async trait support

**CLI**  
- `clap`: Command line parsing
- `console`: Terminal formatting
- `indicatif`: Progress bars
- `tokio`: Async runtime

**WebUI**
- `axum`: Web framework
- `tower`: Middleware
- `tokio-tungstenite`: WebSockets
- `serde_json`: JSON handling

**File Watching**
- `notify`: File system events
- `futures`: Stream processing

## Deployment Architecture

### Single Repository Mode
```
project-root/
├── .git/
│   └── refs/git-tracker/    # Issue storage
├── src/                     # Project source
└── git-tracker.toml         # Configuration
```

### Multi-repository Mode
```
workspace/
├── repo-a/                  # Git repository A
├── repo-b/                  # Git repository B  
└── .git-tracker/            # Shared tracker state
    ├── config.toml
    └── cache/
```

## Workflow Integration

### Kanban Workflow
1. **Issue Creation**: `git-tracker new "Fix auth bug"` → Todo status
2. **Start Work**: `git-tracker status {id} in-progress` + branch creation
3. **Complete Work**: `git-tracker status {id} done` + merge completion

### Branch Integration
```bash
# Automatic branch management
git-tracker start {issue-id}  # Creates branch, sets in-progress
git-tracker finish {issue-id} # Merges branch, sets done
```

### AI Agent Integration
- **Background Updates**: AI agents can modify issues via CLI
- **Live Observation**: WebUI shows real-time changes as agents work
- **Audit Trail**: Complete event log of all agent actions

## Security Considerations

### Cryptographic Signatures
- All events signed with user's private key
- Verification on event application
- Prevents tampering and ensures authenticity

### Access Control
- Repository-level permissions via git
- No additional authentication layer needed
- Relies on git's existing security model

## Performance Characteristics

### Scalability Targets
- **Issues**: 10,000+ issues per repository
- **Events**: 100,000+ events total
- **Concurrent Users**: 10+ simultaneous local users
- **Sync Latency**: <100ms for local changes

### Optimization Strategies
- **Lazy Loading**: Load issue details on demand
- **Incremental Sync**: Only sync changed events
- **Caching**: In-memory cache for frequently accessed issues
- **Indexing**: Custom indices for fast filtering/searching

## Future Extensions

### Plugin Architecture
- Custom status workflows
- External integrations (GitHub, Jira)
- Custom event types
- UI themes and layouts

### Advanced Features
- **Time Tracking**: Built-in time logging
- **Dependencies**: Issue relationships and blocking
- **Automation**: Rules-based status transitions
- **Reporting**: Analytics and metrics dashboard

## Implementation Phases

### Phase 1: Core Infrastructure
1. Git storage layer
2. Basic event sourcing
3. Simple CLI commands

### Phase 2: Kanban Workflow  
1. Status transitions
2. Branch integration
3. Workflow automation

### Phase 3: Multi-user Support
1. File watching
2. Conflict resolution
3. Real-time synchronization

### Phase 4: WebUI
1. Basic web interface
2. WebSocket integration
3. Live collaboration features

### Phase 5: Polish & Extensions
1. Performance optimization
2. Plugin system
3. Advanced workflow features
