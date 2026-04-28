use std::path::Path;

use codex_hooks::HookToolAction;
use codex_hooks::HookToolActionItem;
use codex_hooks::HookToolActionKind;
use codex_protocol::parse_command::ParsedCommand;
use codex_utils_absolute_path::AbsolutePathBuf;

#[derive(Clone, Copy)]
pub(crate) enum ShellHookActionPhase {
    Pre,
    Post,
}

pub(crate) fn shell_tool_action(
    command: &str,
    cwd: &AbsolutePathBuf,
    phase: ShellHookActionPhase,
) -> Option<HookToolAction> {
    let tokens = shlex::split(command)
        .unwrap_or_else(|| vec!["bash".to_string(), "-lc".to_string(), command.to_string()]);
    let parsed = codex_shell_command::parse_command::parse_command(&tokens);
    if parsed.is_empty() {
        return None;
    }

    let mut items = Vec::with_capacity(parsed.len());
    for parsed in parsed {
        items.push(shell_action_item(parsed, cwd, phase, command));
    }
    Some(HookToolAction {
        display_label: shell_display_label(&items, phase),
        actions: items,
    })
}

fn shell_action_item(
    parsed: ParsedCommand,
    cwd: &AbsolutePathBuf,
    phase: ShellHookActionPhase,
    original_command: &str,
) -> HookToolActionItem {
    match parsed {
        ParsedCommand::Read { cmd, name, path } => HookToolActionItem {
            kind: HookToolActionKind::Read,
            label: "Read".to_string(),
            command: Some(cmd),
            name: Some(name),
            path: Some(display_action_path(cwd, &path)),
            query: None,
        },
        ParsedCommand::ListFiles { cmd, path } => HookToolActionItem {
            kind: HookToolActionKind::List,
            label: "List".to_string(),
            command: Some(cmd),
            name: None,
            path,
            query: None,
        },
        ParsedCommand::Search { cmd, query, path } => HookToolActionItem {
            kind: HookToolActionKind::Search,
            label: "Search".to_string(),
            command: Some(cmd),
            name: None,
            path,
            query,
        },
        ParsedCommand::Unknown { .. } => HookToolActionItem {
            kind: HookToolActionKind::Run,
            label: match phase {
                ShellHookActionPhase::Pre => "Run",
                ShellHookActionPhase::Post => "Ran",
            }
            .to_string(),
            command: Some(original_command.to_string()),
            name: None,
            path: None,
            query: None,
        },
    }
}

fn shell_display_label(items: &[HookToolActionItem], phase: ShellHookActionPhase) -> String {
    if items
        .iter()
        .all(|item| item.kind == HookToolActionKind::Read)
    {
        return "Read".to_string();
    }
    if let [item] = items {
        return item.label.clone();
    }
    if items
        .iter()
        .any(|item| item.kind == HookToolActionKind::Run)
    {
        return match phase {
            ShellHookActionPhase::Pre => "Run",
            ShellHookActionPhase::Post => "Ran",
        }
        .to_string();
    }
    items
        .first()
        .map(|item| item.label.clone())
        .unwrap_or_else(|| "Run".to_string())
}

pub(crate) fn apply_patch_tool_action(
    patch: &str,
    cwd: &AbsolutePathBuf,
) -> Option<HookToolAction> {
    let parsed = codex_apply_patch::parse_patch(patch).ok()?;
    if parsed.hunks.is_empty() {
        return None;
    }

    let actions: Vec<HookToolActionItem> = parsed
        .hunks
        .iter()
        .map(|hunk| {
            let (kind, label) = match hunk {
                codex_apply_patch::Hunk::AddFile { .. } => (HookToolActionKind::Added, "Added"),
                codex_apply_patch::Hunk::DeleteFile { .. } => {
                    (HookToolActionKind::Deleted, "Deleted")
                }
                codex_apply_patch::Hunk::UpdateFile { .. } => {
                    (HookToolActionKind::Edited, "Edited")
                }
            };
            let path = hunk.resolve_path(cwd);
            HookToolActionItem {
                kind,
                label: label.to_string(),
                command: None,
                name: path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string()),
                path: Some(path.display().to_string()),
                query: None,
            }
        })
        .collect();

    let display_label = if let [action] = actions.as_slice() {
        action.label.clone()
    } else {
        "Edited".to_string()
    };

    Some(HookToolAction {
        display_label,
        actions,
    })
}

fn display_action_path(cwd: &AbsolutePathBuf, path: &Path) -> String {
    if path.is_absolute() {
        path.display().to_string()
    } else {
        cwd.join(path).display().to_string()
    }
}
