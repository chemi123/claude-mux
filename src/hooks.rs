use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde_json::{json, Value};


const MARKER: &str = "claude-mux-managed";

pub fn register(worktree_dir: &Path, session: &str, window: &str) -> Result<()> {
    let claude_dir = worktree_dir.join(".claude");
    fs::create_dir_all(&claude_dir)
        .with_context(|| format!("failed to create {}", claude_dir.display()))?;

    let settings_path = claude_dir.join("settings.json");
    let mut settings = load_settings(&settings_path)?;

    let hooks = settings
        .as_object_mut()
        .context("settings.json is not an object")?
        .entry("hooks")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .context("hooks is not an object")?;

    let notify_cmd = format!("claude-mux notify --session {session} --window {window}");

    let pre_tool_use = hooks
        .entry("PreToolUse")
        .or_insert_with(|| json!([]))
        .as_array_mut()
        .context("PreToolUse is not an array")?;

    pre_tool_use.push(json!({
        "_marker": MARKER,
        "matcher": "AskUserQuestion",
        "hooks": [{
            "type": "command",
            "command": format!("{notify_cmd} --event question")
        }]
    }));

    let stop = hooks
        .entry("Stop")
        .or_insert_with(|| json!([]))
        .as_array_mut()
        .context("Stop is not an array")?;

    stop.push(json!({
        "_marker": MARKER,
        "matcher": "",
        "hooks": [{
            "type": "command",
            "command": format!("{notify_cmd} --event complete")
        }]
    }));

    save_settings(&settings_path, &settings)?;
    Ok(())
}

pub fn unregister(worktree_dir: &Path) -> Result<()> {
    let claude_dir = worktree_dir.join(".claude");
    let settings_path = claude_dir.join("settings.json");

    if !settings_path.exists() {
        return Ok(());
    }

    let mut settings = load_settings(&settings_path)?;

    if let Some(hooks) = settings.get_mut("hooks").and_then(|h| h.as_object_mut()) {
        for (_key, entries) in hooks.iter_mut() {
            if let Some(arr) = entries.as_array_mut() {
                arr.retain(|entry| {
                    entry.get("_marker").and_then(|m| m.as_str()) != Some(MARKER)
                });
            }
        }
    }

    if is_settings_empty(&settings) {
        fs::remove_file(&settings_path)
            .with_context(|| format!("failed to remove {}", settings_path.display()))?;
        if claude_dir.read_dir()?.next().is_none() {
            fs::remove_dir(&claude_dir)
                .with_context(|| format!("failed to remove {}", claude_dir.display()))?;
        }
    } else {
        save_settings(&settings_path, &settings)?;
    }

    Ok(())
}

fn is_settings_empty(settings: &Value) -> bool {
    let obj = match settings.as_object() {
        Some(obj) => obj,
        None => return false,
    };
    if obj.is_empty() {
        return true;
    }
    if obj.len() != 1 {
        return false;
    }
    let hooks = match obj.get("hooks").and_then(|h| h.as_object()) {
        Some(hooks) => hooks,
        None => return false,
    };
    hooks.values().all(|v| v.as_array().map_or(false, |a| a.is_empty()))
}

fn load_settings(path: &Path) -> Result<Value> {
    if !path.exists() {
        return Ok(json!({}));
    }
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let value: Value = serde_json::from_str(&content)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(value)
}

fn save_settings(path: &Path, value: &Value) -> Result<()> {
    let content = serde_json::to_string_pretty(value)?;
    fs::write(path, content)
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inject_creates_settings() {
        let dir = tempfile::tempdir().unwrap();
        register(dir.path(), "claude-mux-0", "main").unwrap();

        let path = dir.path().join(".claude/settings.json");
        assert!(path.exists());

        let settings: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let hooks = settings["hooks"].as_object().unwrap();

        let pre = hooks["PreToolUse"].as_array().unwrap();
        assert_eq!(pre.len(), 1);
        assert_eq!(pre[0]["_marker"], MARKER);
        assert_eq!(pre[0]["matcher"], "AskUserQuestion");

        let stop = hooks["Stop"].as_array().unwrap();
        assert_eq!(stop.len(), 1);
        assert_eq!(stop[0]["_marker"], MARKER);
    }

    #[test]
    fn test_inject_preserves_existing() {
        let dir = tempfile::tempdir().unwrap();
        let claude_dir = dir.path().join(".claude");
        fs::create_dir_all(&claude_dir).unwrap();
        fs::write(
            claude_dir.join("settings.json"),
            json!({"hooks": {"PreToolUse": [{"matcher": "Write", "hooks": []}]}}).to_string(),
        ).unwrap();

        register(dir.path(), "s", "w").unwrap();

        let settings: Value = serde_json::from_str(
            &fs::read_to_string(claude_dir.join("settings.json")).unwrap(),
        ).unwrap();

        let pre = settings["hooks"]["PreToolUse"].as_array().unwrap();
        assert_eq!(pre.len(), 2);
        assert_eq!(pre[0]["matcher"], "Write");
        assert_eq!(pre[1]["_marker"], MARKER);
    }

    #[test]
    fn test_remove_deletes_empty_settings() {
        let dir = tempfile::tempdir().unwrap();
        register(dir.path(), "s", "w").unwrap();
        assert!(dir.path().join(".claude/settings.json").exists());

        unregister(dir.path()).unwrap();

        assert!(!dir.path().join(".claude/settings.json").exists());
        assert!(!dir.path().join(".claude").exists());
    }

    #[test]
    fn test_remove_preserves_non_managed() {
        let dir = tempfile::tempdir().unwrap();
        let claude_dir = dir.path().join(".claude");
        fs::create_dir_all(&claude_dir).unwrap();
        fs::write(
            claude_dir.join("settings.json"),
            json!({"hooks": {"PreToolUse": [
                {"matcher": "Write", "hooks": []},
                {"_marker": MARKER, "matcher": "AskUserQuestion", "hooks": []}
            ]}}).to_string(),
        ).unwrap();

        unregister(dir.path()).unwrap();

        let settings: Value = serde_json::from_str(
            &fs::read_to_string(claude_dir.join("settings.json")).unwrap(),
        ).unwrap();

        let pre = settings["hooks"]["PreToolUse"].as_array().unwrap();
        assert_eq!(pre.len(), 1);
        assert_eq!(pre[0]["matcher"], "Write");
    }

    #[test]
    fn test_remove_missing_file_is_ok() {
        let dir = tempfile::tempdir().unwrap();
        unregister(dir.path()).unwrap();
    }

    #[test]
    fn test_remove_keeps_claude_dir_if_not_empty() {
        let dir = tempfile::tempdir().unwrap();
        register(dir.path(), "s", "w").unwrap();
        fs::write(dir.path().join(".claude/CLAUDE.md"), "notes").unwrap();

        unregister(dir.path()).unwrap();

        assert!(!dir.path().join(".claude/settings.json").exists());
        assert!(dir.path().join(".claude").exists());
        assert!(dir.path().join(".claude/CLAUDE.md").exists());
    }

    #[test]
    fn test_is_settings_empty() {
        assert!(is_settings_empty(&json!({})));
        assert!(is_settings_empty(&json!({"hooks": {"PreToolUse": [], "Stop": []}})));
        assert!(!is_settings_empty(&json!({"hooks": {"PreToolUse": [{"matcher": "Write"}]}})));
        assert!(!is_settings_empty(&json!({"other_key": "value"})));
        assert!(!is_settings_empty(&json!({"hooks": {}, "other": "value"})));
    }
}
