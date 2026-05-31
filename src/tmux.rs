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

    pub fn send_bell(&self, session: &str, window: &str) -> Result<()> {
        let target = format!("{session}:{window}");
        self.executor.execute("tmux", &["send-keys", "-t", &target, "\x07"], None)?;
        Ok(())
    }

    pub fn rename_window(&self, session: &str, window: &str, new_name: &str) -> Result<()> {
        let target = format!("{session}:{window}");
        self.executor.execute("tmux", &["rename-window", "-t", &target, new_name], None)?;
        Ok(())
    }

    pub fn notify_complete(&self, session: &str, window: &str) -> Result<()> {
        self.rename_window(session, window, &format!("[done] {window}"))?;
        self.send_bell(session, &format!("[done] {window}"))?;
        Ok(())
    }

    pub fn notify_question(&self, session: &str, window: &str) -> Result<()> {
        self.rename_window(session, window, &format!("[wait] {window}"))?;
        self.send_bell(session, &format!("[wait] {window}"))?;
        Ok(())
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
    fn test_send_bell() {
        let tmux = Tmux::new(MockExecutor::new(vec![MockExecutor::ok("")]));
        tmux.send_bell("s", "w").unwrap();
    }

    #[test]
    fn test_rename_window() {
        let tmux = Tmux::new(MockExecutor::new(vec![MockExecutor::ok("")]));
        tmux.rename_window("s", "w", "[done] w").unwrap();
    }

    #[test]
    fn test_notify_complete() {
        let tmux = Tmux::new(MockExecutor::new(vec![
            MockExecutor::ok(""),
            MockExecutor::ok(""),
        ]));
        tmux.notify_complete("s", "main").unwrap();
    }

    #[test]
    fn test_notify_question() {
        let tmux = Tmux::new(MockExecutor::new(vec![
            MockExecutor::ok(""),
            MockExecutor::ok(""),
        ]));
        tmux.notify_question("s", "main").unwrap();
    }
}
