use anyhow::{Context, Result};

use crate::executor::Executor;
use crate::hooks;
use crate::state;
use crate::tmux::Tmux;
use crate::worktree::Worktree;

pub fn run<E: Executor>(executor: &E, session: &str, force: bool) -> Result<()> {
    let st = state::load()?;

    let entry = state::get_session(&st, session)
        .with_context(|| format!("session not found: {session}"))?;

    let wt = Worktree::new(executor);

    for window in &entry.windows {
        hooks::unregister(&window.worktree)?;
        if force {
            wt.force_remove(&window.repo, &window.branch)?;
        } else {
            wt.remove(&window.repo, &window.branch)?;
        }
    }

    let tmux = Tmux::new(executor);
    if tmux.session_exists(session) {
        tmux.kill_session(session)?;
    }

    state::with_state(|st| {
        let new_st = state::remove_session(st, session)?;
        Ok((new_st, ()))
    })?;

    println!("Cleaned session: {session}");
    Ok(())
}

pub fn run_all<E: Executor>(executor: &E, force: bool) -> Result<()> {
    let st = state::load()?;
    let sessions: Vec<String> = st.sessions.keys().cloned().collect();

    if sessions.is_empty() {
        println!("No sessions to clean.");
        return Ok(());
    }

    for session in &sessions {
        run(executor, session, force)?;
    }

    Ok(())
}
