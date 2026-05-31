mod cmd;
mod executor;
mod hooks;
mod state;
mod tmux;
mod worktree;

fn main() {
    if let Err(e) = cmd::run() {
        eprintln!("Error: {e:#}");
        std::process::exit(1);
    }
}
