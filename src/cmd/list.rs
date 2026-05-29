use anyhow::Result;
use crate::state;

pub fn run() -> Result<()> {
    let st = state::load()?;

    if st.sessions.is_empty() {
        println!("No active sessions.");
        return Ok(());
    }

    for (name, entry) in &st.sessions {
        println!("{name}");
        for window in &entry.windows {
            println!("  {} ({})", window.branch, window.worktree.display());
        }
    }

    Ok(())
}
