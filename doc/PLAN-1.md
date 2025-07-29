# Phase 1: Core Infrastructure Implementation Plan

## Current State Analysis
- Project exists with planning documents and nix shell setup
- No Rust code exists yet - starting from scratch
- Nix shell configured with rustup, just, git-bug, and git
- Architecture plan complete with event-sourced design

## Phase 1 Implementation Steps

### 1. Project Setup
- Initialize Rust project with `cargo init`
- Create workspace structure: `storage/`, `cli/`, `common/`
- Set up `Cargo.toml` with required dependencies:
  - `gix` for git operations (instead of git2)
  - `serde` with `serde_derive` for serialization
  - `uuid` for issue ID generation
  - `chrono` for timestamps
  - `anyhow` for error handling
  - `thiserror` for custom errors
- Create `justfile` for common development tasks

### 2. Core Data Structures (`src/common/`)
- Implement `Issue`, `IssueEvent`, `IssueStatus`, `Identity`, `Comment` structs from PLAN.md
- Add serde derives for serialization
- Implement basic validation and constructors
- Add unit tests for data structures

### 3. Git Storage Layer (`src/storage/`)
- Implement `GitRepository` wrapper around gix
- Create `EventLog` for event persistence in git objects
- Implement `IssueStore` with basic CRUD operations:
  - `create_issue()` - create new issue with initial event
  - `apply_event()` - append event to issue history  
  - `get_issue()` - reconstruct issue from event log
  - `list_issues()` - list all issues with basic filtering
- Set up refs structure: `refs/git-issue/issues/{issue-id}`
- Add comprehensive tests for storage operations

### 4. Basic CLI (`src/cli/`)
- Set up `clap` command structure with subcommands:
  - `new <title>` - create new issue
  - `list` - list issues  
  - `show <id>` - show issue details
  - `status <id> <status>` - change issue status
- Implement basic command handlers calling storage layer
- Add colored output and basic formatting
- Create integration tests for CLI commands

### 5. Project Infrastructure
- Set up proper error handling with custom error types
- Add logging with `env_logger`
- Create development justfile with build, test, lint commands
- Set up basic CI structure (if needed)
- Add README with basic usage instructions

## Deliverables
- Working Rust project with core event-sourced storage
- Functional CLI for basic issue operations (create, list, show, status change)
- Comprehensive test suite covering storage and CLI
- Documentation for development and basic usage

## Success Criteria
- Can create, list, and update issues via CLI
- Issues persist in git objects and survive repository operations
- All tests pass and code is well-documented
- Foundation ready for Phase 2 kanban workflow features

## Dependencies Used

### Core Git Operations
- `gix` - Pure Rust git implementation with better performance and safety than git2
- `gix-ref` - Git reference handling
- `gix-object` - Git object manipulation

### Serialization & Data
- `serde` + `serde_derive` - Serialization framework
- `serde_json` - JSON serialization for events
- `uuid` - Unique ID generation
- `chrono` - Date/time handling with serde support

### CLI & User Interface
- `clap` - Command line argument parsing
- `console` - Terminal formatting and colors
- `indicatif` - Progress bars (for future use)

### Error Handling & Logging
- `anyhow` - Flexible error handling
- `thiserror` - Custom error types
- `env_logger` - Environment-based logging

### Development & Testing
- `tokio` - Async runtime (for future WebUI)
- Standard test framework for unit tests