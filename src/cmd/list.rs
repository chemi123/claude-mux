use anyhow::Result;

use crate::executor::Executor;
use crate::state;
use crate::tmux::Tmux;

pub fn run<E: Executor>(executor: &E) -> Result<()> {
    let st = state::load()?;

    if st.sessions.is_empty() {
        println!("No active sessions.");
        return Ok(());
    }

    let tmux = Tmux::new(executor);

    for (name, entry) in &st.sessions {
        let alive = tmux.session_exists(name);
        let marker = if alive { "" } else { " [stale]" };
        println!("{name}{marker}");
        for window in &entry.windows {
            println!("  {} ({})", window.branch, window.worktree.display());
        }
    }

    Ok(())
}
