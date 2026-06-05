# cc-hook

一个用于安装 [Claude Code / Codex] hooks 的 CLI 工具，把会话中的所有事件转发到 HTTP endpoint。

## 它做什么

它和另一个项目配合使用，用来实现 CC 消息展示。

`cc-hook` 会把一整套 [Claude Code hooks](https://docs.anthropic.com/en/docs/claude-code/hooks) 写入你的 `settings.json`，每个 hook 都会调用一个本地 IPC 助手。这样你就可以实时观察并记录 Claude Code 会话中的每个事件。

`cc-hook` 也可以为 Codex 写入 hooks。

### 支持的事件

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

工具级 hook 会把工具名追加到 URL 路径后面，例如 `/PreToolUse/Bash`、`/PostToolUse/Read`。

## 用法

```bash
cc-hook.exe --ipc-helper D:\project\Rust\cc-hook-core\target\debug\agent-hooks-ipc.exe
cc-hook.exe --ipc-helper D:\project\Rust\cc-hook-core\target\debug\agent-hooks-ipc.exe --agent claude --scope workspace
cc-hook.exe --ipc-helper D:\project\Rust\cc-hook-core\target\debug\agent-hooks-ipc.exe --agent codex --codex-dir D:\work\.codex --yes
cc-hook.exe --ipc-helper D:\project\Rust\cc-hook-core\target\debug\agent-hooks-ipc.exe --agent both
```

你也可以继续使用交互式模式，或者通过参数跳过菜单：
- `--ipc-helper PATH` 是必填项，指向 `agent-hooks-ipc.exe`
- `--agent both|claude|codex` 指定要安装哪一组 hooks
- `--scope global|workspace` 为所选 agent 指定默认配置位置
- `--claude-settings PATH` 直接指定 Claude Code 的 `settings.json`
- `--codex-dir PATH` 直接指定包含 `hooks.json` 和 `config.toml` 的目录
- `--yes` 在缺少文件时自动创建，不再询问

如果不传参数，会提示你选择安装位置：
- `global` - `~/.claude/settings.json` 或 `~/.codex/{hooks.json,config.toml}`
- `this workspace` - 当前目录下的 `.claude/settings.json` 或 `.codex/{hooks.json,config.toml}`
- `select a file / directory` - 任意路径

写入前会自动创建 `.bak` 备份。如果目标文件不存在，程序会询问是否创建它。
