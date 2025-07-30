use anyhow::Result;
use clap::{Parser, Subcommand};

mod create;
mod edit;
mod list;
mod show;
mod status;

pub use create::{handle_create, CreateArgs};
#[cfg(test)]
pub use create::handle_create_with_env;
pub use edit::{handle_edit, EditArgs};
pub use list::{handle_list, ListArgs};
pub use show::{handle_show, ShowArgs};
pub use status::{handle_status, StatusArgs};

use crate::common::{EnvProvider, Identity, IssueStatus};
use crate::storage::IssueStore;

#[derive(Parser)]
#[command(name = "git-issue")]
#[command(about = "An offline-first issue tracker with git backend")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Repository path (defaults to current directory)
    #[arg(short, long, global = true)]
    pub repo: Option<std::path::PathBuf>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Create a new issue
    Create(CreateArgs),
    /// List issues
    List(ListArgs),
    /// Show issue details
    Show(ShowArgs),
    /// Change issue status
    Status(StatusArgs),
    /// Edit an issue
    Edit(EditArgs),
}

pub fn run_command(cli: Cli) -> Result<()> {
    let repo_path = cli.repo.unwrap_or_else(|| std::env::current_dir().unwrap());

    match cli.command {
        Commands::Create(args) => handle_create(repo_path, args),
        Commands::List(args) => handle_list(repo_path, args),
        Commands::Show(args) => handle_show(repo_path, args),
        Commands::Status(args) => handle_status(repo_path, args),
        Commands::Edit(args) => handle_edit(repo_path, args),
    }
}

/// Get author identity from provided arguments or environment variables
pub(crate) fn get_author_identity(
    name: Option<String>,
    email: Option<String>,
    store: &IssueStore,
    env_provider: impl EnvProvider,
) -> Result<Identity> {
    let name = name.unwrap_or_else(|| {
        env_provider
            .get_var("GIT_AUTHOR_NAME")
            .or_else(|| store.get_config("user.name"))
            .or_else(|| env_provider.get_var("USER"))
            .unwrap_or_else(|| "Unknown".to_string())
    });

    let email = email.unwrap_or_else(|| {
        env_provider
            .get_var("GIT_AUTHOR_EMAIL")
            .or_else(|| store.get_config("user.email"))
            .unwrap_or_else(|| "unknown@localhost".to_string())
    });

    Ok(Identity::new(name, email))
}

/// Parse status string into IssueStatus enum
pub(crate) fn parse_status(status_str: &str) -> Result<IssueStatus> {
    match status_str.to_lowercase().as_str() {
        "todo" | "open" => Ok(IssueStatus::Todo),
        "in-progress" | "inprogress" | "progress" => Ok(IssueStatus::InProgress),
        "done" | "closed" | "complete" => Ok(IssueStatus::Done),
        _ => anyhow::bail!(
            "Invalid status '{}'. Valid options: todo, in-progress, done",
            status_str
        ),
    }
}