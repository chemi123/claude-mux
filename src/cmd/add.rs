use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::executor::Executor;
use crate::hooks;
use crate::state::{self, WindowEntry};
use crate::tmux::Tmux;
use crate::worktree::Worktree;

pub fn run<E: Executor>(
    executor: &E,
    session: &str,
    repo: &Path,
    branch: Option<&str>,
) -> Result<()> {
    let tmux = Tmux::new(executor);
    if !tmux.session_exists(session) {
        bail!("session not found: {session}. Use `launch` to create a new session.");
    }

    let wt = Worktree::new(executor);

    if !wt.is_git_repo(repo) {
        bail!("not a git repository: {}", repo.display());
    }

    let branch = match branch {
        Some(b) => b.to_string(),
        None => wt.resolve_branch(repo)?,
    };

    let repo = std::fs::canonicalize(repo)
        .with_context(|| format!("failed to canonicalize {}", repo.display()))?;

    let wt_path = wt.create(&repo, &branch)?;
    let wt_path_str = wt_path.to_string_lossy().to_string();
    let window_name = branch.replace('/', "-");

    hooks::register(&wt_path, session, &window_name)?;

    tmux.create_window(session, &window_name, &wt_path_str)?;
    tmux.send_keys(session, &window_name, 0, "claude")?;

    state::with_state(|st| {
        let new_st = state::add_window(st, session, WindowEntry {
            repo: repo.clone(),
            branch: branch.clone(),
            worktree: wt_path,
        })?;
        Ok((new_st, ()))
    })?;

    println!("Added window: {window_name} to session: {session}");
    Ok(())
}
