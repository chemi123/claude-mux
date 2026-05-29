# Known Issues & Future Considerations

## Executor trait のシグネチャが外部コマンド実行に特化している

現在の `Executor` trait は `execute(program, args, cwd)` というシグネチャで、外部コマンド実行にしか対応していない。
テスト時のモック差し替えやボイラープレート削減には有効だが、git2 等のライブラリに切り替える場合はこのインターフェースでは対応できない。

**対応案:** `Worktree` 自体を trait 化し、CLI 実装とライブラリ実装を差し替え可能にする。`Executor` はコマンド実行専用（tmux 等）のまま維持する。

## clean 時に hooks が書き込んだファイルで dirty 判定される

`launch` 時に hooks が worktree 内の `.claude/settings.json` を作成するため、`clean` 時に `is_dirty` チェックで常に引っかかり `--force` が必要になる。

**対応案:** `clean` の処理順序を変更し、`hooks::unregister` → `is_dirty` チェック → `worktree::remove` の順にする。自分が書いたファイルを先に消せば dirty にならない。

## state.json の排他制御

複数プロセスからの同時アクセスに備え、ファイルロック（`flock` 等）が必要。現在は未実装。
