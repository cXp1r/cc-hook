use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use serde_json::{json, Value};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::vec;
use toml_edit::{DocumentMut, value};

fn parse_args() -> (String, u16) {
    let mut ip = String::from("127.0.0.1");
    let mut port: u16 = 2221;

    let args: Vec<String> = env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--ip" => {
                if i + 1 < args.len() {
                    ip = args[i + 1].clone();
                    i += 1;
                }
            }
            "--port" => {
                if i + 1 < args.len() {
                    port = args[i + 1].parse().unwrap_or(2221);
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    (ip, port)
}

fn make_curl_hook(base_url: &str, path: &str, agent: &str) -> Value {
    json!({
        "type": "command",
        "command": format!("curl -s http://{}/{}?a1={}", base_url, path, agent)
    })
}


fn build_hooks(base_url: &str, agent: &str) -> Value {
    json!({
        "PreToolUse": [{
            "matcher": ".*",
            "hooks": [make_curl_hook(base_url, "PreToolUse", agent)]
        }],

        "PostToolUse": [{
            "matcher": ".*",
            "hooks": [make_curl_hook(base_url, "PostToolUse", agent)]
        }],

        "PostToolUseFailure": [{
            "matcher": ".*",
            "hooks": [make_curl_hook(base_url, "PostToolUseFailure", agent)]
        }],

        "PermissionRequest": [{
            "matcher": ".*",
            "hooks": [make_curl_hook(base_url, "PermissionRequest", agent)]
        }],

        "Stop": [{
            "hooks": [make_curl_hook(base_url, "Stop", agent)]
        }],

        "SubagentStop": [{
            "hooks": [make_curl_hook(base_url, "SubagentStop", agent)]
        }],

        "Notification": [{
            "hooks": [make_curl_hook(base_url, "Notification", agent)]
        }],

        "UserPromptSubmit": [{
            "hooks": [make_curl_hook(base_url, "UserPromptSubmit", agent)]
        }],

        "SessionStart": [{
            "hooks": [make_curl_hook(base_url, "SessionStart", agent)]
        }],
        "SessionEnd": [{
            "hooks": [make_curl_hook(base_url, "SessionEnd", agent)]
        }],

        "PreCompact": [{
            "hooks": [make_curl_hook(base_url, "PreCompact", agent)]
        }]
    })
}

fn hook_cc(base_url: &str) {
    let items = vec![
        "global",
        "this workspace",
        "select a file",
        "exit",
    ];

    let config_path: PathBuf = loop {
        println!("=============================================\n======        Claude Code Hooks        ======\n=============================================");
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("where do you want to add Claude Code hooks?")
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
        }
    };

    let mut config: Value = {
        let content = fs::read_to_string(&config_path).expect("Failed to read config file");
        serde_json::from_str(&content).unwrap_or(json!({}))
    };

    let backup = config_path.with_extension("json.bak");
    fs::copy(&config_path, &backup).expect("Failed to create backup");
    println!("Backup created at: {}", backup.display());

    config["hooks"] = build_hooks(&base_url,"CC");
    let output = serde_json::to_string_pretty(&config).expect("Failed to serialize config");
    fs::write(&config_path, output).expect("Failed to write config file");

    println!("✓ Write to hooks successfully, all events reported to http://{}/[EventName]", base_url);
    println!("  PreToolUse / PostToolUse will append the tool name to the path, e.g., /PreToolUse/Bash");
}

fn hook_codex(base_url: &str) {
    let items = vec![
        "global",
        "this workspace",
        "select a file",
        "exit",
    ];

    let config_path: [PathBuf; 2] = loop {
        println!("=============================================\n======        Codex Hooks        ======\n=============================================");
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("where do you want to add Codex hooks?")
            .items(&items)
            .default(0)
            .interact()
            .unwrap();

        let path: [PathBuf; 2] = match selection {
            0 => {
                println!("You chose to add hooks globally.");
                #[cfg(target_os = "windows")]
                {
                    let userprofile = std::env::var("USERPROFILE").expect("USERPROFILE not set");
                    [PathBuf::from(&userprofile).join(".codex").join("hooks.json"),PathBuf::from(&userprofile).join(".codex").join("config.toml")]
                }
            }
            1 => {
                println!("You chose to add hooks to this workspace.");
                [env::current_dir().unwrap().join(".codex").join("hooks.json"), env::current_dir().unwrap().join(".codex").join("config.toml")]
            }
            2 => {
                println!("You chose to select a file.");
                let file_path: String = Input::new()
                    .with_prompt("Enter the path without extension (e.g., /path/to/hooks or /path/to/config)")
                    .interact_text()
                    .unwrap();
                [PathBuf::from(&file_path).join("hooks.json"), PathBuf::from(&file_path).join("config.toml")]
            }
            _ => {
                println!("Exiting.");
                return;
            }
        };

        if path[0].exists() && path[1].exists() {
            println!("hooks.json exist at: {}", path[0].display());
            println!("config.toml exist at: {}", path[1].display());
            break path;
        } else {
            let create = Confirm::new()
                .with_prompt("File does not exist. Do you want to create it?")
                .default(true)
                .interact()
                .unwrap();
            if create {
                fs::create_dir_all(path[0].parent().unwrap()).expect("Failed to create directories");
                fs::write(&path[0], "{}").expect("Failed to create file");
                fs::write(&path[1], "").expect("Failed to create file");
                println!("Created files at: {}", path[0].display());
                break path;
            }
        }
    };

    let mut config: Value = {
        let content = fs::read_to_string(&config_path[0]).expect("Failed to read config file");
        serde_json::from_str(&content).unwrap_or(json!({}))
    };

    let backup = config_path[0].with_extension("json.bak");
    fs::copy(&config_path[0], &backup).expect("Failed to create backup");
    println!("Backup created at: {}", backup.display());

    let backup = config_path[1].with_extension("toml.bak");
    fs::copy(&config_path[1], &backup).expect("Failed to create backup");
    println!("Backup created at: {}", backup.display());

    let content = fs::read_to_string(&config_path[1]).expect("Failed to read config file");
    let mut doc = content.parse::<DocumentMut>().map_err(|e| e.to_string()).expect("Failed to parse config.toml");

    if doc.get("features").is_none() {
        doc["features"] = toml_edit::table();
    }

    doc["features"]["codex_hooks"] = value(true);
    
    fs::write(&config_path[1], doc.to_string()).expect("Failed to write config.toml");

    config["hooks"] = build_hooks(&base_url, "Codex");
    let output = serde_json::to_string_pretty(&config).expect("Failed to serialize config");
    fs::write(&config_path[0], output).expect("Failed to write config file");

    println!("✓ Write to hooks successfully, all events reported to http://{}/[EventName]", base_url);
    println!("  PreToolUse / PostToolUse will append the tool name to the path, e.g., /PreToolUse/Bash");
}

fn main() {
    let (ip, port) = parse_args();
    let base_url = format!("{}:{}", ip, port);
    let agent = vec![
        "both Claude Code and Codex",
        "Claude Code",
        "Codex",
        "exit",
    ];
    let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("which agent do you want to add hooks for?")
            .items(&agent)
            .default(0)
            .interact()
            .unwrap();
    match selection {
        0 => {
            hook_cc(&base_url);
            hook_codex(&base_url);
        }
        1 => {
            hook_cc(&base_url);
        }
        2 => {
            hook_codex(&base_url);
        }
        3 => {
            println!("Exiting.");
            return;
        }
        _ => unreachable!(),
    }
    
}
