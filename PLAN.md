# claude-mux Implementation Plan

## Phase 1: Foundation ✅

- [x] `error.rs` — anyhow Result alias
- [x] `state.rs` — `~/.claude-mux/var/state.json` read/write
- [x] `worktree.rs` — git worktree create/remove, branch resolution, path: `~/.claude-mux/worktrees/<repo>/<repo>-<branch>`

## Phase 2: tmux + Notification ✅

- [x] `cmd.rs` — `Command` trait + `CommandExecutor` (static dispatch via generics)
- [x] `tmux.rs` — session/window/pane operations, session name resolution (claude-mux-{N})
- [x] `notify.rs` — tmux bell + window rename
- [x] `hooks.rs` — Claude Code hooks register/unregister into `<worktree>/.claude/settings.json`

## Phase 3: CLI Subcommands ✅

- [x] `cmd/mod.rs` — clap 定義 + サブコマンドルーティング
- [x] `cmd/launch.rs` — launch: セッション新規作成
- [x] `cmd/add.rs` — add: 既存セッションにウィンドウ追加
- [x] `cmd/clean.rs` — clean: セッション削除 (--all, --force)
- [x] `cmd/list.rs` — list: セッション一覧表示
- [x] `cmd/notify.rs` — notify: hooks から呼ばれる通知 (内部コマンド)
- [x] `command.rs` — Command trait を cmd.rs から分離、blanket impl for &T 追加
- [x] `main.rs` — cmd::run() を呼ぶだけのエントリポイント
