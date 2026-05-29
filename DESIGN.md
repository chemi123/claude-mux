# claude-mux 設計書

## 概要

tmux + git worktree + Claude CLI の並列実行を管理するCLIツール。
複数タスクの起動・分離・通知を自動化し、並列エージェント作業を効率化する。

## 背景・課題

Claude CLIを複数並列で動かす際、以下を手動で行う必要がある：

- tmuxウィンドウ作成
- git worktreeの払い出し（ブランチ分離）
- 各ペインでclaude cliを起動してタスク投入
- 進捗確認のためのウィンドウ切り替え

これを1コマンドで自動化する。

## ユースケース

1. **同一リポジトリ内で複数タスク並列**（例：リファクタ + テスト追加）
   → git worktreeでブランチ分離して競合回避
2. **別リポジトリで並列**（例：フロントエンドrepo + バックエンドrepo）
   → 各repoからworktreeを作成し、同一セッションで管理
3. **混在**（同一repoの複数タスク + 別repoのタスク）
   → 全てworktreeで分離、1セッションに集約

## 技術スタック

| 項目 | 選定 | 理由 |
|------|------|------|
| 言語 | Rust | single binary、型安全 |
| CLIパース | clap (derive) | 定番、derive macroで簡潔 |
| エラー型 | anyhow | CLIアプリ向き。`?` + context() で十分 |
| git操作 | git CLI (`Command`) | git2のworktree APIが不安定なため |
| tmux操作 | tmux CLI (`Command`) | 直叩きが最もシンプル |
| JSON操作 | serde_json | hooks注入時のsettings.json読み書き |

### 意図的に入れないもの（MVP）

| クレート | 理由 |
|---------|------|
| git2 | worktree周りのAPIが不安定。CLIで十分 |
| thiserror | CLIアプリなのでanyhowで十分。エラー型の細分化は不要 |
| tokio | MVPは同期で完結。v2のTUI時に導入 |
| ratatui | v2スコープ |
| serde + toml | TOMLベースの設定ファイルは廃止。CLI引数ベースに変更 |

## CLIインターフェース

### `launch` — セッション起動

新規セッションを作成し、最初のウィンドウを追加する。

```
claude-mux launch [OPTIONS]
```

| 引数 | 短縮 | 必須 | デフォルト | 説明 |
|------|------|------|-----------|------|
| `--session` | `-s` | No | `claude-mux-{連番}` | tmuxセッション名 |
| `--repo` | `-r` | No | カレントディレクトリ | 対象リポジトリのパス（git管理必須） |
| `--branch` | `-b` | No | 現在のブランチ | ブランチ名 |

**セッション解決:**

| 条件 | 動作 |
|------|------|
| `--session` 指定なし | `claude-mux-0` から連番で空きを探して新規作成 |
| `--session` 指定あり + 存在しない | その名前で新規作成 |
| `--session` 指定あり + 存在する | エラー（追加は `add` を使う） |

**バリデーション:**

- `--repo` がgitリポジトリでない場合はエラー

**ブランチ解決:**

常にworktreeを作成する（元repoを汚さないため）。

| 条件 | 動作 |
|------|------|
| `--branch` なし | 現在のブランチ (`git rev-parse --abbrev-ref HEAD`) でworktree作成 |
| `--branch` 指定あり | そのブランチでworktree作成 |

作業ディレクトリ: `~/.claude-mux/worktrees/<repo>/<repo>-<branch>`

### `add` — 既存セッションにウィンドウ追加

既存セッションに新しいウィンドウ（worktree + Claude CLI）を追加する。

```
claude-mux add <SESSION_NAME> [OPTIONS]
```

| 引数 | 短縮 | 必須 | デフォルト | 説明 |
|------|------|------|-----------|------|
| `SESSION_NAME` | — | Yes | — | 追加先セッション名（位置引数） |
| `--repo` | `-r` | No | カレントディレクトリ | 対象リポジトリのパス（git管理必須） |
| `--branch` | `-b` | No | 現在のブランチ | ブランチ名 |

**バリデーション:**

- `SESSION_NAME` が存在しない場合はエラー
- `--repo` がgitリポジトリでない場合はエラー

**ブランチ解決:** `launch` と同じ。

### `clean` — セッション削除

worktree削除 + tmux終了 + state除去。

```
claude-mux clean <SESSION_NAME>
claude-mux clean --all
```

| 引数 | 短縮 | 必須 | 説明 |
|------|------|------|------|
| `SESSION_NAME` | — | No | 削除するセッション名（位置引数） |
| `--all` | `-a` | No | 全セッションを削除 |

| 条件 | 動作 |
|------|------|
| `claude-mux clean claude-mux-0` | 指定セッションを削除 |
| `claude-mux clean --all` | 全セッション削除 |
| `claude-mux clean` | エラー。セッション一覧を表示し、セッション名の指定を促す |

### `list` — セッション一覧表示

claude-muxで作成したセッションのみ表示（state.jsonから取得）。

```
claude-mux list
```

引数なし。

### `notify` — 通知送信（内部コマンド）

Claude Code hooksから呼ばれる。ユーザーが直接使うことは想定しない。

```
claude-mux notify --session <name> --window <name> --event <type>
```

| 引数 | 短縮 | 必須 | 説明 |
|------|------|------|------|
| `--session` | `-s` | Yes | 対象セッション名 |
| `--window` | `-w` | Yes | 対象ウィンドウ名 |
| `--event` | `-e` | Yes | イベント種別: `complete` / `question` |

## 状態管理

`~/.claude-mux/var/state.json` でセッションとworktreeの紐付けを管理。

```json
{
  "sessions": {
    "claude-mux-0": {
      "windows": [
        {
          "repo": "/home/user/github/nes-rust",
          "branch": "feature/auth",
          "worktree": "/home/user/.claude-mux/worktrees/nes-rust/nes-rust-feature-auth"
        },
        {
          "repo": "/home/user/github/frontend",
          "branch": "main",
          "worktree": "/home/user/.claude-mux/worktrees/frontend/frontend-main"
        }
      ]
    }
  }
}
```

- `launch` 時にエントリ追加
- `clean` 時にエントリを参照してworktreeを削除し、エントリも除去

## worktree管理

- パス規則: `~/.claude-mux/worktrees/<repo-name>/<repo-name>-<branch-name>`
  - 例: `~/.claude-mux/worktrees/nes-rust/nes-rust-feature-auth`
  - `/` は `-` に変換（`feature/auth` → `feature-auth`）
- 同一ブランチのworktreeは1つだけ（重複指定はエラー）
- `clean` コマンドで state.json を参照して対象worktreeを削除
- worktree内で `git worktree add` / `git worktree remove` を使用

## tmuxレイアウト

各タスクは独立したtmuxウィンドウ。ウィンドウ名はブランチ名。ウィンドウ内はvertical 2分割。

```
tmux session: "claude-mux-0"
┌──────────────────────────────────────────────┐
│ window: main                   [Ctrl-b n/p]  │
│ ┌─────────────────────┬────────────────────┐ │
│ │                     │                    │ │
│ │  Claude CLI         │  フリーペイン      │ │
│ │  (自動起動)         │  (シェル)          │ │
│ │                     │                    │ │
│ └─────────────────────┴────────────────────┘ │
├──────────────────────────────────────────────┤
│ window: feature-auth           [Ctrl-b n/p]  │
│ ┌─────────────────────┬────────────────────┐ │
│ │                     │                    │ │
│ │  Claude CLI         │  フリーペイン      │ │
│ │  (自動起動)         │  (シェル)          │ │
│ │                     │                    │ │
│ └─────────────────────┴────────────────────┘ │
└──────────────────────────────────────────────┘
```

- **左ペイン**: Claude CLIが自動起動（prompt指定時は投入済み）
- **右ペイン**: 自由に使えるシェル（nvim、コマンド実行等）
- **ウィンドウ切替**: `Ctrl-b n`（次）/ `Ctrl-b p`（前）
- **通知**: タスク完了・質問待ち時にtmuxウィンドウがベルでハイライト

## ディレクトリ構成

```
claude-mux/
├── Cargo.toml
├── DESIGN.md
├── src/
│   ├── main.rs             # エントリポイント (cmd::run() を呼ぶだけ)
│   ├── cmd/
│   │   ├── mod.rs          # clap 定義 + サブコマンドルーティング
│   │   ├── launch.rs       # launch: セッション新規作成
│   │   ├── add.rs          # add: 既存セッションにウィンドウ追加
│   │   ├── clean.rs        # clean: セッション削除
│   │   ├── list.rs         # list: セッション一覧表示
│   │   └── notify.rs       # notify: hooks から呼ばれる通知 (内部コマンド)
│   ├── command.rs          # Command trait + CommandExecutor (infra層)
│   ├── worktree.rs         # git worktree 生成・削除 + ブランチ解決
│   ├── tmux.rs             # tmux session/window/pane 操作
│   ├── state.rs            # ~/.claude-mux/var/state.json 読み書き
│   ├── notify.rs           # tmux bell + window rename による通知
│   ├── hooks.rs            # Claude Code hooks の register/unregister
│   └── error.rs            # anyhow Result エイリアス
```

## 処理フロー

### `claude-mux launch`

```
1. CLI引数パース
   └─ --session (default: 連番), --repo (default: cwd), --branch (default: current)

2. worktree作成（常に）:
   ├─ --branch なし → 現在のブランチ名を取得 (git rev-parse --abbrev-ref HEAD)
   └─ worktree::create(repo, branch)
      working_dir = ~/.claude-mux/worktrees/<repo>/<repo>-<branch>

3. hooks::inject(working_dir)
   └─ <worktree>/.claude/settings.json に通知用hookを追加

4. セッション解決:
   ├─ --session 指定なし → claude-mux-{連番} で新規作成
   ├─ --session 指定あり + 存在しない → 新規作成
   └─ --session 指定あり + 存在する → エラー（`add` を使う旨を表示）

5. tmux:
   ├─ create_session or create_window(session, branch名, working_dir)
   └─ split-window -h (vertical 2分割)

6. Claude CLI起動:
   └─ send_keys("claude")

7. state::save()
   └─ ~/.claude-mux/var/state.json にセッション・worktree情報を記録
```

### `claude-mux clean <session-name>`

```
1. state::load()
   └─ ~/.claude-mux/var/state.json からセッション情報を読み込み

2. 引数なし → セッション一覧を表示して終了
   --all    → 全セッションを対象に以下を実行

3. tmux::kill_session(session_name)

4. for each window in session:
   └─ worktree::remove(repo, branch)

5. state::remove_session(session_name)
   └─ state.json から該当セッションのエントリを除去
```

## 通知システム

### 検知方法

| イベント | 検知方法 |
|---------|---------|
| タスク完了 | tmuxペイン内プロセスの終了監視 |
| 質問待ち（パーミッション要求） | Claude Code hooks (`PreToolUse`) |
| 質問待ち（ユーザー入力待ち） | Claude Code hooks (`PostToolUse` for `AskUserQuestion`) |

### 通知方法

- **tmuxベル**: `\x07` をペインに送信 → ステータスバーでウィンドウがハイライト
- **ウィンドウ名変更**: `[done] branch-name` / `[wait] branch-name` に変更

### hooks登録イメージ

`claude-mux launch` 実行時にworktreeの `<worktree>/.claude/settings.json` へ登録:

```json
{
  "hooks": {
    "PreToolUse": [{
      "_marker": "claude-mux-managed",
      "matcher": "AskUserQuestion",
      "hooks": [{
        "type": "command",
        "command": "claude-mux notify --session claude-mux-0 --window feature-auth --event question"
      }]
    }],
    "Stop": [{
      "_marker": "claude-mux-managed",
      "matcher": "",
      "hooks": [{
        "type": "command",
        "command": "claude-mux notify --session claude-mux-0 --window feature-auth --event complete"
      }]
    }]
  }
}
```

`claude-mux clean` で `_marker: "claude-mux-managed"` のエントリを登録解除。

## エラー戦略

- worktree作成失敗 → エラーメッセージを表示
- tmuxセッションが既に存在 → エラー（`add` コマンドを案内）
- clean時にworktreeが存在しない → スキップ
- clean時にworktreeに未コミット変更あり → 警告して中断（`--force` で強制削除）

## 実装上の注意

- **state.json の排他制御**: 複数プロセスからの同時アクセスに備え、ファイルロック（`flock` 等）で排他制御する

## スコープ

### MVP (v1)

- [ ] CLI引数パース (clap)
- [ ] ブランチ解決ロジック
- [ ] git worktree自動生成・削除 (`~/.claude-mux/` 以下)
- [ ] tmuxウィンドウ作成（vertical 2分割）
- [ ] Claude CLI自動起動（prompt optional）
- [ ] タスク完了・質問待ち通知（tmux bell + window rename）
- [ ] Claude Code hooks注入・除去
- [ ] cleanコマンド

### v2（将来）

- [ ] ratatui TUIモニタリング画面
- [ ] subagent検知・表示
- [ ] タスク依存関係の定義
- [ ] プロンプトテンプレート
