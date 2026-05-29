use crate::executor::Executor;
use anyhow::Result;

pub struct Notifier<E: Executor> {
    executor: E,
}

impl<E: Executor> Notifier<E> {
    pub fn new(executor: E) -> Self {
        Self { executor }
    }

    pub fn send_bell(&self, session: &str, window: &str) -> Result<()> {
        let target = format!("{session}:{window}");
        self.executor.execute(
            "tmux",
            &["send-keys", "-t", &target, "\x07"],
            None,
        )?;
        Ok(())
    }

    pub fn rename_window(&self, session: &str, window: &str, new_name: &str) -> Result<()> {
        let target = format!("{session}:{window}");
        self.executor.execute(
            "tmux",
            &["rename-window", "-t", &target, new_name],
            None,
        )?;
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::MockExecutor;

    #[test]
    fn test_send_bell() {
        let notifier = Notifier::new(MockExecutor::new(vec![MockExecutor::ok("")]));
        notifier.send_bell("s", "w").unwrap();
    }

    #[test]
    fn test_rename_window() {
        let notifier = Notifier::new(MockExecutor::new(vec![MockExecutor::ok("")]));
        notifier.rename_window("s", "w", "[done] w").unwrap();
    }

    #[test]
    fn test_notify_complete() {
        let notifier = Notifier::new(MockExecutor::new(vec![
            MockExecutor::ok(""),
            MockExecutor::ok(""),
        ]));
        notifier.notify_complete("s", "main").unwrap();
    }

    #[test]
    fn test_notify_question() {
        let notifier = Notifier::new(MockExecutor::new(vec![
            MockExecutor::ok(""),
            MockExecutor::ok(""),
        ]));
        notifier.notify_question("s", "main").unwrap();
    }
}
