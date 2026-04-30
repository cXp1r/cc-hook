use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use serde_json::{json, Value};
use std::env;
use std::fs;
use std::path::PathBuf;

const BASE_URL: &str = "127.0.0.1:2221";

fn make_curl_hook(path: &str) -> Value {
    json!({
        "type": "command",
        "command": format!("curl -s http://{}/{}", BASE_URL, path)
    })
}

fn make_curl_hook_with_tool(path: &str) -> Value {
    // 读 stdin 拿 tool_name，拼到路径上
    json!({
        "type": "command",
        "command": format!(
            "input=$(cat); tool=$(echo $input | jq -r '.tool_name // \"unknown\"'); curl -s http://{}/{}/$(echo $tool | tr '/' '_')",
            BASE_URL, path
        )
    })
}

fn build_hooks() -> Value {
    json!({
        // ── 每次工具调用前 ──
        "PreToolUse": [{
            "matcher": ".*",
            "hooks": [make_curl_hook_with_tool("PreToolUse")]
        }],

        // ── 每次工具调用后 ──
        "PostToolUse": [{
            "matcher": ".*",
            "hooks": [make_curl_hook_with_tool("PostToolUse")]
        }],

        // ── 工具调用失败 ──
        "PostToolUseFailure": [{
            "matcher": ".*",
            "hooks": [make_curl_hook_with_tool("PostToolUseFailure")]
        }],

        // ── 权限请求 ──
        "PermissionRequest": [{
            "matcher": ".*",
            "hooks": [make_curl_hook("PermissionRequest")]
        }],

        // ── Claude 完成整轮回答 ──
        "Stop": [{
            "hooks": [make_curl_hook("Stop")]
        }],

        // ── 子 agent 完成 ──
        "SubagentStop": [{
            "hooks": [make_curl_hook("SubagentStop")]
        }],

        // ── 需要用户操作/通知 ──
        "Notification": [{
            "hooks": [make_curl_hook("Notification")]
        }],

        // ── 用户提交 prompt ──
        "UserPromptSubmit": [{
            "hooks": [make_curl_hook("UserPromptSubmit")]
        }],

        // ── Session 开始/结束 ──
        "SessionStart": [{
            "hooks": [make_curl_hook("SessionStart")]
        }],
        "SessionEnd": [{
            "hooks": [make_curl_hook("SessionEnd")]
        }],

        // ── 上下文压缩前 ──
        "PreCompact": [{
            "hooks": [make_curl_hook("PreCompact")]
        }]
    })
}

fn main() {
    let items = vec![
        "global",
        "this workspace",
        "select a file",
        "exit",
    ];

    // 把 config_path 定义在 loop 外，loop 用 break value 返回
    let config_path: PathBuf = loop {
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("where do you want to add claude code hooks?")
            .items(&items)
            .default(0)
            .interact()
            .unwrap();

        let path = match selection {
            0 => {
                println!("You chose to add hooks globally.");
                #[cfg(target_os = "windows")]
                {
                    let userprofile = std::env::var("USERPROFILE").expect("USERPROFILE not set");
                    PathBuf::from(userprofile).join(".claude").join("settings.json")
                }
                #[cfg(not(target_os = "windows"))]
                {
                    let home = std::env::var("HOME").expect("HOME not set");
                    PathBuf::from(home).join(".claude").join("settings.json")
                }
            }
            1 => {
                println!("You chose to add hooks to this workspace.");
                env::current_dir().unwrap().join(".claude").join("settings.json")
            }
            2 => {
                println!("You chose to select a file.");
                let file_path: String = Input::new()
                    .with_prompt("Enter the path to the settings.json file")
                    .interact_text()
                    .unwrap();
                PathBuf::from(file_path)
            }
            _ => {
                println!("Exiting.");
                return;
            }
        };

        if path.exists() {
            println!("File exists at: {}", path.display());
            break path;
        } else {
            println!("File does not exist at: {}", path.display());
            let create = Confirm::new()
                .with_prompt("File does not exist. Do you want to create it?")
                .default(true)
                .interact()
                .unwrap();
            if create {
                fs::create_dir_all(path.parent().unwrap()).expect("Failed to create directories");
                fs::write(&path, "{}").expect("Failed to create file");
                println!("Created file at: {}", path.display());
                break path;
            }
            // false 则继续 loop 重新选
        }
    };

    // 读取现有配置
    let mut config: Value = {
        let content = fs::read_to_string(&config_path).expect("Failed to read config file");
        serde_json::from_str(&content).unwrap_or(json!({}))
    };

    // 备份原文件
    let backup = config_path.with_extension("json.bak");
    fs::copy(&config_path, &backup).expect("Failed to create backup");
    println!("Backup created at: {}", backup.display());

    // 写入 hooks
    config["hooks"] = build_hooks();
    let output = serde_json::to_string_pretty(&config).expect("Failed to serialize config");
    fs::write(&config_path, output).expect("Failed to write config file");

    println!("✓ Write to hooks successfully, all events reported to http://{}/[EventName]", BASE_URL);
    println!("  PreToolUse / PostToolUse will append the tool name to the path, e.g., /PreToolUse/Bash");
}
