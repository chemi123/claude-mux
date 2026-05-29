mod add;
mod clean;
mod launch;
mod list;
mod notify;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::executor::CommandExecutor;
use anyhow::Result;

#[derive(Parser)]
#[command(name = "claude-mux", about = "tmux + git worktree + Claude CLI parallel execution manager")]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new session with the first window
    Launch {
        #[arg(short, long)]
        session: Option<String>,

        #[arg(short, long, default_value = ".")]
        repo: PathBuf,

        #[arg(short, long)]
        branch: Option<String>,
    },

    /// Add a window to an existing session
    Add {
        /// Target session name
        session: String,

        #[arg(short, long, default_value = ".")]
        repo: PathBuf,

        #[arg(short, long)]
        branch: Option<String>,
    },

    /// Remove sessions and their worktrees
    Clean {
        /// Session name to clean
        session: Option<String>,

        #[arg(short, long)]
        all: bool,

        #[arg(short, long)]
        force: bool,
    },

    /// List active sessions
    List,

    /// Send notification (internal, called by Claude Code hooks)
    Notify {
        #[arg(short, long)]
        session: String,

        #[arg(short, long)]
        window: String,

        #[arg(short, long)]
        event: String,
    },
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    let executor = CommandExecutor;

    match cli.command {
        Commands::Launch { session, repo, branch } => {
            launch::run(&executor, session.as_deref(), &repo, branch.as_deref())
        }
        Commands::Add { session, repo, branch } => {
            add::run(&executor, &session, &repo, branch.as_deref())
        }
        Commands::Clean { session, all, force } => {
            if all {
                clean::run_all(&executor, force)
            } else if let Some(session) = session {
                clean::run(&executor, &session, force)
            } else {
                list::run()?;
                anyhow::bail!("specify a session name or use --all");
            }
        }
        Commands::List => list::run(),
        Commands::Notify { session, window, event } => {
            notify::run(&executor, &session, &window, &event)
        }
    }
}
