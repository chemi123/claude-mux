use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use fs2::FileExt;
use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowEntry {
    pub repo: PathBuf,
    pub branch: String,
    pub worktree: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct State {
    pub sessions: BTreeMap<String, SessionEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEntry {
    pub windows: Vec<WindowEntry>,
}

fn state_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("failed to resolve home directory")?;
    Ok(home.join(".claude-mux").join("var").join("state.json"))
}

fn lock_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("failed to resolve home directory")?;
    Ok(home.join(".claude-mux").join("var").join("state.lock"))
}

fn acquire_lock(exclusive: bool) -> Result<fs::File> {
    let path = lock_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(&path)
        .with_context(|| format!("failed to open lock file: {}", path.display()))?;
    if exclusive {
        file.lock_exclusive()
            .context("failed to acquire exclusive lock on state")?;
    } else {
        file.lock_shared()
            .context("failed to acquire shared lock on state")?;
    }
    Ok(file)
}

fn load_inner() -> Result<State> {
    let path = state_path()?;
    if !path.exists() {
        return Ok(State::default());
    }
    let content = fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let state: State = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(state)
}

fn save_inner(state: &State) -> Result<()> {
    let path = state_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let content = serde_json::to_string_pretty(state)?;
    fs::write(&path, content)
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

pub fn load() -> Result<State> {
    let _lock = acquire_lock(false)?;
    load_inner()
}

pub fn with_state<F, R>(f: F) -> Result<R>
where
    F: FnOnce(&State) -> Result<(State, R)>,
{
    let _lock = acquire_lock(true)?;
    let state = load_inner()?;
    let (new_state, result) = f(&state)?;
    save_inner(&new_state)?;
    Ok(result)
}

pub fn add_window(state: &State, session: &str, window: WindowEntry) -> Result<State> {
    let mut new_state = state.clone();
    new_state
        .sessions
        .entry(session.to_string())
        .or_insert_with(|| SessionEntry { windows: vec![] })
        .windows
        .push(window);
    Ok(new_state)
}

pub fn remove_session(state: &State, session: &str) -> Result<State> {
    let mut new_state = state.clone();
    if new_state.sessions.remove(session).is_none() {
        bail!("session not found: {session}");
    }
    Ok(new_state)
}

pub fn get_session<'a>(state: &'a State, session: &str) -> Option<&'a SessionEntry> {
    state.sessions.get(session)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_window_creates_session() {
        let state = State::default();
        let window = WindowEntry {
            repo: PathBuf::from("/repo"),
            branch: "main".to_string(),
            worktree: PathBuf::from("/worktree"),
        };

        let state = add_window(&state, "test-session", window).unwrap();

        assert_eq!(state.sessions.len(), 1);
        assert_eq!(state.sessions["test-session"].windows.len(), 1);
        assert_eq!(state.sessions["test-session"].windows[0].branch, "main");
    }

    #[test]
    fn test_add_window_appends_to_existing() {
        let state = State::default();
        let w1 = WindowEntry {
            repo: PathBuf::from("/repo"),
            branch: "main".to_string(),
            worktree: PathBuf::from("/wt1"),
        };
        let w2 = WindowEntry {
            repo: PathBuf::from("/repo"),
            branch: "dev".to_string(),
            worktree: PathBuf::from("/wt2"),
        };

        let state = add_window(&state, "s", w1).unwrap();
        let state = add_window(&state, "s", w2).unwrap();

        assert_eq!(state.sessions["s"].windows.len(), 2);
    }

    #[test]
    fn test_remove_session() {
        let state = State::default();
        let window = WindowEntry {
            repo: PathBuf::from("/repo"),
            branch: "main".to_string(),
            worktree: PathBuf::from("/wt"),
        };
        let state = add_window(&state, "s", window).unwrap();
        let state = remove_session(&state, "s").unwrap();

        assert!(state.sessions.is_empty());
    }

    #[test]
    fn test_remove_session_not_found() {
        let state = State::default();
        let err = remove_session(&state, "nonexistent").unwrap_err();
        assert!(err.to_string().contains("session not found"));
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");

        let mut state = State::default();
        let window = WindowEntry {
            repo: PathBuf::from("/repo"),
            branch: "main".to_string(),
            worktree: PathBuf::from("/wt"),
        };
        state = add_window(&state, "s", window).unwrap();

        let content = serde_json::to_string_pretty(&state).unwrap();
        fs::write(&path, &content).unwrap();

        let loaded: State = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(loaded.sessions.len(), 1);
        assert_eq!(loaded.sessions["s"].windows[0].branch, "main");
    }
}
