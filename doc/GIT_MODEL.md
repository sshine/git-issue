# Git Storage Model for git-issue

This document explains how git-issue stores issues and events in git objects using an event-sourcing architecture.

## Overview

Git-issue uses git's native object database to store issue data as an immutable event log. Each issue is represented as a chain of git commits, where each commit represents a single event in the issue's lifecycle.

## Storage Architecture

### Reference Structure

Git-issue uses a dedicated namespace within git references:

```
.git/refs/git-issue/
├── issues/
│   ├── 1              # Points to latest commit for issue #1
│   ├── 2              # Points to latest commit for issue #2
│   └── ...
└── meta/
    └── next-issue-id   # Contains next sequential issue ID
```

### Issue Identification

- **Sequential IDs**: Issues use sequential u64 identifiers (1, 2, 3, ...)
- **Issue References**: `refs/git-issue/issues/{issue_id}` points to the latest event commit
- **ID Management**: Next ID stored as a blob in `refs/git-issue/meta/next-issue-id`

## Event-Sourcing Implementation

### Event Chain Structure

Each issue is stored as a linear chain of git commits:

```
refs/git-issue/issues/1
    ↓
Latest Event Commit (e.g., StatusChanged)
├── Tree: event.json (blob containing event data)
├── Parent: Previous Event Commit
├── Author: Event author
└── Message: "StatusChanged: todo → in-progress"
    ↓
Previous Event Commit (e.g., Created)
├── Tree: event.json (blob containing creation event)
├── Parent: None (first event)
├── Author: Issue creator
└── Message: "Created: Fix authentication bug"
```

### Event Types

Events are stored as JSON objects in the `event.json` blob of each commit:

#### Created Event
```json
{
  "Created": {
    "title": "Fix authentication bug",
    "description": "Users cannot log in with OAuth",
    "author": {
      "name": "simonshine",
      "email": "simon@example.com"
    },
    "timestamp": "2025-07-29T22:19:17.128148Z"
  }
}
```

#### StatusChanged Event
```json
{
  "StatusChanged": {
    "from": "Todo",
    "to": "InProgress", 
    "author": {
      "name": "simonshine",
      "email": "simon@example.com"
    },
    "timestamp": "2025-07-29T22:20:35.445678Z"
  }
}
```

#### Other Event Types
- `CommentAdded`: New comment on issue
- `LabelAdded`/`LabelRemoved`: Label management
- `TitleChanged`: Title updates
- `AssigneeChanged`: Assignee modifications

## Git Object Inspection

### Examining Issue References

```bash
# List all issues
ls .git/refs/git-issue/issues/

# Get the latest commit hash for an issue
cat .git/refs/git-issue/issues/1

# Show the commit details
git show <commit-hash>
```

### Viewing Event History

```bash
# Show the complete event chain for an issue
ISSUE_HASH=$(cat .git/refs/git-issue/issues/1)
git log --oneline --graph $ISSUE_HASH

# Example output:
# * 9df278f StatusChanged: in-progress → done
# * f8f21d3 StatusChanged: todo → in-progress  
# * c340cfd Created: First issue
```

### Inspecting Event Data

```bash
# Show the event JSON for a specific commit
git show <commit-hash>:event.json

# Or examine the blob directly
git cat-file -p <commit-hash>^{tree}
git cat-file -p <blob-hash>
```

### Low-Level Object Analysis

```bash
# Show raw commit object
git cat-file -p <commit-hash>

# Show object type
git cat-file -t <commit-hash>

# List all git-tracker references
git for-each-ref refs/git-issue/
```

## Issue Reconstruction

Issues are reconstructed by:

1. **Reading the reference**: `refs/git-issue/issues/{id}` gives the latest commit
2. **Traversing the chain**: Follow parent commits back to the root
3. **Collecting events**: Extract and deserialize event JSON from each commit
4. **Replaying events**: Apply events chronologically to build current state

## Advantages of Git Storage

### Immutability
- Events cannot be modified once committed
- Complete audit trail preserved forever
- Git's content-addressable storage prevents tampering

### Branching & Merging
- Can create branches of issue history
- Merge concurrent modifications using git's merge algorithms
- Conflict resolution for simultaneous edits

### Distribution
- Issues sync with git remotes automatically
- Offline-first operation
- Decentralized collaboration

### Efficiency
- Git's delta compression reduces storage overhead
- Deduplication of identical content
- Fast traversal of commit chains

## Storage Overhead

For a typical issue with 10 events:
- **Blob storage**: ~200 bytes per event (JSON)
- **Commit overhead**: ~150 bytes per commit
- **Tree overhead**: ~50 bytes per commit
- **Total per issue**: ~4KB for 10 events

Git's delta compression significantly reduces actual disk usage for related events.

## Implementation Details

### Atomic Operations
- Reference updates use git's atomic compare-and-swap
- Prevents concurrent modification conflicts
- Ensures consistency during multi-step operations

### Sequential ID Management
- Next ID stored as blob content in `refs/git-issue/meta/next-issue-id`
- Incremented atomically using reference updates
- Ensures unique, sequential issue numbering

### Error Handling
- Invalid references handled gracefully
- Corrupted events skipped during reconstruction
- Provides degraded functionality rather than complete failure

## Examples

### Creating a New Issue
1. Get next ID from `refs/git-issue/meta/next-issue-id`
2. Create "Created" event JSON
3. Write event as blob
4. Create tree containing event blob
5. Create commit with tree and metadata
6. Update issue reference atomically
7. Increment next-issue-id counter

### Modifying an Issue
1. Read current issue state from event chain
2. Create new event (e.g., "StatusChanged")
3. Write event blob and create tree
4. Create commit with previous commit as parent
5. Update issue reference to new commit

This architecture provides a robust, distributed, and audit-friendly foundation for issue tracking while leveraging git's proven storage and synchronization capabilities.