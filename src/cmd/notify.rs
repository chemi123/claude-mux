use crate::executor::Executor;
use crate::tmux::Tmux;
use anyhow::Result;

pub fn run<E: Executor>(executor: &E, session: &str, window: &str, event: &str) -> Result<()> {
    let tmux = Tmux::new(executor);

    match event {
        "complete" => tmux.notify_complete(session, window),
        "question" => tmux.notify_question(session, window),
        other => anyhow::bail!("unknown event: {other}"),
    }
}
