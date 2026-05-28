# claude-mux Implementation Plan

## Phase 1: Foundation

- [ ] `error.rs` — error types
- [ ] `state.rs` — `~/.claude-mux/var/state.json` read/write
- [ ] `worktree.rs` — git worktree create/remove, branch resolution, path: `~/.claude-mux/worktrees/<repo>/<repo>-<branch>`

## Phase 2: tmux + Notification

- [ ] `tmux.rs` — session/window/pane operations, session name resolution (claude-mux-{N})
- [ ] `notify.rs` — tmux bell + window rename
- [ ] `hooks.rs` — Claude Code hooks inject/remove into `<worktree>/.claude/settings.json`

## Phase 3: Launcher + CLI

- [ ] `launcher.rs` — launch / add / clean logic (worktree → hooks → tmux → claude)
- [ ] `main.rs` — clap subcommands: launch / add / clean / list
