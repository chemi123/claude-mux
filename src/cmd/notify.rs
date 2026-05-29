use crate::executor::Executor;
use anyhow::Result;
use crate::notify::Notifier;

pub fn run<E: Executor>(executor: &E, session: &str, window: &str, event: &str) -> Result<()> {
    let notifier = Notifier::new(executor);

    match event {
        "complete" => notifier.notify_complete(session, window),
        "question" => notifier.notify_question(session, window),
        other => anyhow::bail!("unknown event: {other}"),
    }
}
