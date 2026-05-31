use std::path::Path;

use anyhow::{bail, Context, Result};

use crate::executor::Executor;
use crate::hooks;
use crate::state::{self, WindowEntry};
use crate::tmux::Tmux;
use crate::worktree::Worktree;

pub fn run<E: Executor>(
    executor: &E,
    session: Option<&str>,
    repo: &Path,
    branch: Option<&str>,
) -> Result<()> {
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

    let tmux = Tmux::new(executor);
    let session_name = tmux.resolve_session_name(session)?;

    hooks::register(&wt_path, &session_name, &window_name)?;

    tmux.create_session(&session_name, &window_name, &wt_path_str)?;
    tmux.send_keys(&session_name, &window_name, 0, "claude")?;

    state::with_state(|st| {
        let new_st = state::add_window(st, &session_name, WindowEntry {
            repo: repo.clone(),
            branch: branch.clone(),
            worktree: wt_path,
        })?;
        Ok((new_st, ()))
    })?;

    println!("Session: {session_name}, Window: {window_name}");
    Ok(())
}
