use anyhow::{bail, Result};

use crate::executor::Executor;

pub struct Tmux<E: Executor> {
    executor: E,
}

impl<E: Executor> Tmux<E> {
    pub fn new(executor: E) -> Self {
        Self { executor }
    }

    pub fn session_exists(&self, session: &str) -> bool {
        self.executor
            .execute("tmux", &["has-session", "-t", session], None)
            .is_ok()
    }

    pub fn resolve_session_name(&self, requested: Option<&str>) -> Result<String> {
        match requested {
            Some(name) => {
                if self.session_exists(name) {
                    bail!("session already exists: {name}. Use `add` to add a window to it.");
                }
                Ok(name.to_string())
            }
            None => {
                for i in 0.. {
                    let name = format!("claude-mux-{i}");
                    if !self.session_exists(&name) {
                        return Ok(name);
                    }
                }
                unreachable!()
            }
        }
    }

    pub fn create_session(&self, session: &str, window_name: &str, working_dir: &str) -> Result<()> {
        self.executor.execute("tmux", &[
            "new-session", "-d", "-s", session, "-n", window_name, "-c", working_dir,
        ], None)?;
        self.split_window(session, window_name, working_dir)?;
        Ok(())
    }

    pub fn create_window(&self, session: &str, window_name: &str, working_dir: &str) -> Result<()> {
        self.executor.execute("tmux", &[
            "new-window", "-t", session, "-n", window_name, "-c", working_dir,
        ], None)?;
        self.split_window(session, window_name, working_dir)?;
        Ok(())
    }

    pub fn send_keys(&self, session: &str, window_name: &str, pane: u32, keys: &str) -> Result<()> {
        let target = format!("{session}:{window_name}.{pane}");
        self.executor.execute("tmux", &["send-keys", "-t", &target, keys, "Enter"], None)?;
        Ok(())
    }

    pub fn kill_session(&self, session: &str) -> Result<()> {
        self.executor.execute("tmux", &["kill-session", "-t", session], None)?;
        Ok(())
    }

    pub fn list_sessions(&self) -> Result<Vec<String>> {
        match self.executor.execute("tmux", &["list-sessions", "-F", "#{session_name}"], None) {
            Ok(output) => {
                let sessions = output.stdout
                    .lines()
                    .map(|s| s.to_string())
                    .collect();
                Ok(sessions)
            }
            Err(_) => Ok(vec![]),
        }
    }

    fn split_window(&self, session: &str, window_name: &str, working_dir: &str) -> Result<()> {
        let target = format!("{session}:{window_name}");
        self.executor.execute("tmux", &[
            "split-window", "-h", "-t", &target, "-c", working_dir,
        ], None)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::MockExecutor;

    #[test]
    fn test_session_exists_true() {
        let tmux = Tmux::new(MockExecutor::new(vec![MockExecutor::ok("")]));
        assert!(tmux.session_exists("test"));
    }

    #[test]
    fn test_session_exists_false() {
        let tmux = Tmux::new(MockExecutor::new(vec![MockExecutor::err("no session")]));
        assert!(!tmux.session_exists("test"));
    }

    #[test]
    fn test_resolve_session_name_specified_new() {
        let tmux = Tmux::new(MockExecutor::new(vec![MockExecutor::err("no session")]));
        let name = tmux.resolve_session_name(Some("my-session")).unwrap();
        assert_eq!(name, "my-session");
    }

    #[test]
    fn test_resolve_session_name_specified_exists() {
        let tmux = Tmux::new(MockExecutor::new(vec![MockExecutor::ok("")]));
        let err = tmux.resolve_session_name(Some("existing")).unwrap_err();
        assert!(err.to_string().contains("session already exists"));
    }

    #[test]
    fn test_resolve_session_name_auto() {
        let tmux = Tmux::new(MockExecutor::new(vec![
            MockExecutor::ok(""),
            MockExecutor::ok(""),
            MockExecutor::err("no session"),
        ]));
        let name = tmux.resolve_session_name(None).unwrap();
        assert_eq!(name, "claude-mux-2");
    }

    #[test]
    fn test_create_session() {
        let tmux = Tmux::new(MockExecutor::new(vec![
            MockExecutor::ok(""),
            MockExecutor::ok(""),
        ]));
        tmux.create_session("s", "w", "/tmp").unwrap();
    }

    #[test]
    fn test_list_sessions() {
        let tmux = Tmux::new(MockExecutor::new(vec![MockExecutor::ok("s1\ns2\ns3")]));
        let sessions = tmux.list_sessions().unwrap();
        assert_eq!(sessions, vec!["s1", "s2", "s3"]);
    }

    #[test]
    fn test_list_sessions_empty() {
        let tmux = Tmux::new(MockExecutor::new(vec![MockExecutor::err("no server running")]));
        let sessions = tmux.list_sessions().unwrap();
        assert!(sessions.is_empty());
    }
}
