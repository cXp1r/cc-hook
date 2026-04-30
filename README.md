# cc-hook

A CLI tool that installs [Claude Code / Codex] hooks to stream all session events to an HTTP endpoint.

## What it does

Work together with another project to implement CC message display.

`cc-hook` writes a complete set of [Claude Code hooks](https://docs.anthropic.com/en/docs/claude-code/hooks) into your `settings.json`, each configured to report events via `curl` to `127.0.0.1:2221`. This lets you observe and log every event in a Claude Code session in real time.

`cc-hook` also can write hooks for codex
### Hooked events

| Hook | Path |
|---|---|
| PreToolUse | `/PreToolUse/<tool_name>` |
| PostToolUse | `/PostToolUse/<tool_name>` |
| PostToolUseFailure | `/PostToolUseFailure/<tool_name>` |
| PermissionRequest | `/PermissionRequest` |
| Stop | `/Stop` |
| SubagentStop | `/SubagentStop` |
| UserPromptSubmit | `/UserPromptSubmit` |
| Notification | `/Notification` |
| SessionStart | `/SessionStart` |
| SessionEnd | `/SessionEnd` |
| PreCompact | `/PreCompact` |

Tool-level hooks append the tool name to the URL path (e.g., `/PreToolUse/Bash`, `/PostToolUse/Read`).

## Usage

```bash
cargo run
```

You'll be prompted to choose where to install the hooks:
- **global** — `~/.claude/settings.json`
- **this workspace** — `.claude/settings.json` in the current directory
- **select a file** — arbitrary path

The tool creates a `.bak` backup before writing. If the target file doesn't exist, you'll be asked whether to create it.

## Building

```bash
cargo build --release
```

The binary will be at `target/release/cc-hook`.
