# Known Issues & Future Considerations

## Executor trait のシグネチャが外部コマンド実行に特化している

現在の `Executor` trait は `execute(program, args, cwd)` というシグネチャで、外部コマンド実行にしか対応していない。
テスト時のモック差し替えやボイラープレート削減には有効だが、git2 等のライブラリに切り替える場合はこのインターフェースでは対応できない。

**対応案:** `Worktree` 自体を trait 化し、CLI 実装とライブラリ実装を差し替え可能にする。`Executor` はコマンド実行専用（tmux 等）のまま維持する。

## ~~clean 時に hooks が書き込んだファイルで dirty 判定される~~ (解決済み)

`clean` の処理順序を `hooks::unregister` → `wt.remove` に変更。`unregister` は managed entries 削除後に settings.json が実質空なら ファイル/`.claude/` ディレクトリごと削除するようにした。

## ~~state.json の排他制御~~ (解決済み)

`fs2` crate の `flock` ベースロックを導入。`state::with_state(|st| ...)` で exclusive lock + atomic な read-modify-write を提供。`state::load()` は shared lock で安全な読み取り。公開 `save()` は廃止し、書き込みは `with_state` 経由のみに統一。
