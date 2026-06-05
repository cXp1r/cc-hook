use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use serde_json::{json, Value};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use toml_edit::{value, DocumentMut};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AgentChoice {
    Both,
    Claude,
    Codex,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScopeChoice {
    Global,
    Workspace,
}

#[derive(Clone, Debug, Default)]
struct CliOptions {
    agent: Option<AgentChoice>,
    scope: Option<ScopeChoice>,
    ipc_helper: Option<PathBuf>,
    claude_settings: Option<PathBuf>,
    codex_dir: Option<PathBuf>,
    auto_create: bool,
}

#[derive(Clone, Debug)]
enum LocationSelection {
    Global,
    Workspace,
    Custom(PathBuf),
}

enum FileEnsureResult {
    Ready,
    Retry,
    Abort,
}

fn print_usage() {
    println!(
        "Usage:\n  cc-hook.exe --ipc-helper PATH [--agent both|claude|codex] [--scope global|workspace] [--claude-settings PATH] [--codex-dir PATH] [--yes]\n\nExamples:\n  cc-hook.exe --ipc-helper D:\\\\project\\\\Rust\\\\cc-hook-core\\\\target\\\\debug\\\\agent-hooks-ipc.exe --agent claude --scope workspace\n  cc-hook.exe --ipc-helper D:\\\\project\\\\Rust\\\\cc-hook-core\\\\target\\\\debug\\\\agent-hooks-ipc.exe --agent codex --codex-dir D:\\\\work\\\\.codex --yes\n  cc-hook.exe --ipc-helper D:\\\\project\\\\Rust\\\\cc-hook-core\\\\target\\\\debug\\\\agent-hooks-ipc.exe --agent both"
    );
}

fn parse_agent(value: &str) -> Option<AgentChoice> {
    match value {
        "both" => Some(AgentChoice::Both),
        "claude" => Some(AgentChoice::Claude),
        "codex" => Some(AgentChoice::Codex),
        _ => None,
    }
}

fn parse_scope(value: &str) -> Option<ScopeChoice> {
    match value {
        "global" => Some(ScopeChoice::Global),
        "workspace" => Some(ScopeChoice::Workspace),
        _ => None,
    }
}

fn parse_args() -> CliOptions {
    let mut options = CliOptions::default();
    let args: Vec<String> = env::args().skip(1).collect();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];

        if arg == "--" {
            // Allow a bare `--` for callers that forward extra arguments.
        } else if let Some(value) = arg.strip_prefix("--agent=") {
            options.agent = Some(parse_agent(value).unwrap_or_else(|| {
                eprintln!("Unknown agent: {}", value);
                print_usage();
                process::exit(1);
            }));
        } else if arg == "--agent" || arg == "-a" {
            i += 1;
            let value = args.get(i).unwrap_or_else(|| {
                eprintln!("Missing value for --agent");
                print_usage();
                process::exit(1);
            });
            options.agent = parse_agent(value).or_else(|| {
                eprintln!("Unknown agent: {}", value);
                print_usage();
                process::exit(1);
            });
        } else if let Some(value) = arg.strip_prefix("--scope=") {
            options.scope = Some(parse_scope(value).unwrap_or_else(|| {
                eprintln!("Unknown scope: {}", value);
                print_usage();
                process::exit(1);
            }));
        } else if arg == "--scope" || arg == "-s" {
            i += 1;
            let value = args.get(i).unwrap_or_else(|| {
                eprintln!("Missing value for --scope");
                print_usage();
                process::exit(1);
            });
            options.scope = parse_scope(value).or_else(|| {
                eprintln!("Unknown scope: {}", value);
                print_usage();
                process::exit(1);
            });
        } else if let Some(value) = arg.strip_prefix("--ipc-helper=") {
            options.ipc_helper = Some(PathBuf::from(value));
        } else if arg == "--ipc-helper" {
            i += 1;
            let value = args.get(i).unwrap_or_else(|| {
                eprintln!("Missing value for --ipc-helper");
                print_usage();
                process::exit(1);
            });
            options.ipc_helper = Some(PathBuf::from(value));
        } else if let Some(value) = arg.strip_prefix("--claude-settings=") {
            options.claude_settings = Some(PathBuf::from(value));
        } else if arg == "--claude-settings" {
            i += 1;
            let value = args.get(i).unwrap_or_else(|| {
                eprintln!("Missing value for --claude-settings");
                print_usage();
                process::exit(1);
            });
            options.claude_settings = Some(PathBuf::from(value));
        } else if let Some(value) = arg.strip_prefix("--codex-dir=") {
            options.codex_dir = Some(PathBuf::from(value));
        } else if arg == "--codex-dir" {
            i += 1;
            let value = args.get(i).unwrap_or_else(|| {
                eprintln!("Missing value for --codex-dir");
                print_usage();
                process::exit(1);
            });
            options.codex_dir = Some(PathBuf::from(value));
        } else if arg == "--yes" || arg == "-y" {
            options.auto_create = true;
        } else if arg == "--help" || arg == "-h" {
            print_usage();
            process::exit(0);
        } else {
            eprintln!("Unknown argument: {}", arg);
            print_usage();
            process::exit(1);
        }

        i += 1;
    }

    options
}

fn require_ipc_helper(options: &CliOptions) -> PathBuf {
    let Some(path) = options.ipc_helper.clone() else {
        eprintln!("Missing required argument: --ipc-helper");
        print_usage();
        process::exit(1);
    };

    if !path.exists() {
        eprintln!("IPC helper does not exist: {}", path.display());
        process::exit(1);
    }

    path
}

fn resolve_agent(options: &CliOptions) -> Option<AgentChoice> {
    if let Some(agent) = options.agent {
        return Some(agent);
    }

    match (options.claude_settings.is_some(), options.codex_dir.is_some()) {
        (true, true) => Some(AgentChoice::Both),
        (true, false) => Some(AgentChoice::Claude),
        (false, true) => Some(AgentChoice::Codex),
        (false, false) => None,
    }
}

fn select_agent_interactive() -> Option<AgentChoice> {
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
        0 => Some(AgentChoice::Both),
        1 => Some(AgentChoice::Claude),
        2 => Some(AgentChoice::Codex),
        3 => None,
        _ => unreachable!(),
    }
}

fn select_location(prompt: &str, custom_item_label: &str, custom_prompt: &str) -> Option<LocationSelection> {
    let items = vec!["global", "this workspace", custom_item_label, "exit"];

    println!(
        "=============================================\n======        {}        ======\n=============================================",
        prompt
    );
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .items(&items)
        .default(0)
        .interact()
        .unwrap();

    match selection {
        0 => Some(LocationSelection::Global),
        1 => Some(LocationSelection::Workspace),
        2 => {
            let file_path: String = Input::new()
                .with_prompt(custom_prompt)
                .interact_text()
                .unwrap();
            Some(LocationSelection::Custom(PathBuf::from(file_path)))
        }
        3 => None,
        _ => unreachable!(),
    }
}

fn windows_home_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        PathBuf::from(env::var("USERPROFILE").expect("USERPROFILE not set"))
    }

    #[cfg(not(target_os = "windows"))]
    {
        PathBuf::from(env::var("HOME").expect("HOME not set"))
    }
}

fn claude_global_settings_path() -> PathBuf {
    windows_home_dir().join(".claude").join("settings.json")
}

fn claude_workspace_settings_path() -> PathBuf {
    env::current_dir()
        .expect("Failed to get current directory")
        .join(".claude")
        .join("settings.json")
}

fn codex_global_paths() -> [PathBuf; 2] {
    let base = windows_home_dir().join(".codex");
    [base.join("hooks.json"), base.join("config.toml")]
}

fn codex_workspace_paths() -> [PathBuf; 2] {
    let base = env::current_dir()
        .expect("Failed to get current directory")
        .join(".codex");
    [base.join("hooks.json"), base.join("config.toml")]
}

fn resolve_claude_settings_path(options: &CliOptions) -> Option<PathBuf> {
    if let Some(path) = &options.claude_settings {
        return Some(path.clone());
    }

    match options.scope {
        Some(ScopeChoice::Global) => Some(claude_global_settings_path()),
        Some(ScopeChoice::Workspace) => Some(claude_workspace_settings_path()),
        None => loop {
            let selection = select_location(
                "Claude Code Hooks",
                "select a file",
                "Enter the path to the settings.json file",
            )?;

            let path = match selection {
                LocationSelection::Global => claude_global_settings_path(),
                LocationSelection::Workspace => claude_workspace_settings_path(),
                LocationSelection::Custom(path) => path,
            };

            return Some(path);
        },
    }
}

fn resolve_codex_paths(options: &CliOptions) -> Option<[PathBuf; 2]> {
    if let Some(dir) = &options.codex_dir {
        return Some([dir.join("hooks.json"), dir.join("config.toml")]);
    }

    match options.scope {
        Some(ScopeChoice::Global) => Some(codex_global_paths()),
        Some(ScopeChoice::Workspace) => Some(codex_workspace_paths()),
        None => loop {
            let selection = select_location(
                "Codex Hooks",
                "select a directory",
                "Enter the path without extension (e.g., /path/to/hooks or /path/to/config)",
            )?;

            let paths = match selection {
                LocationSelection::Global => codex_global_paths(),
                LocationSelection::Workspace => codex_workspace_paths(),
                LocationSelection::Custom(path) => [path.join("hooks.json"), path.join("config.toml")],
            };

            return Some(paths);
        },
    }
}

fn create_parent_dirs(path: &Path) {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).expect("Failed to create directories");
        }
    }
}

fn ensure_file(
    path: &Path,
    default_content: &str,
    auto_create: bool,
    prompt: &str,
    allow_retry: bool,
) -> FileEnsureResult {
    if path.exists() {
        println!("File exists at: {}", path.display());
        return FileEnsureResult::Ready;
    }

    println!("File does not exist at: {}", path.display());
    let create = if auto_create {
        true
    } else {
        Confirm::new()
            .with_prompt(prompt)
            .default(true)
            .interact()
            .unwrap()
    };

    if !create {
        return if allow_retry {
            FileEnsureResult::Retry
        } else {
            FileEnsureResult::Abort
        };
    }

    create_parent_dirs(path);
    fs::write(path, default_content).expect("Failed to create file");
    println!("Created file at: {}", path.display());
    FileEnsureResult::Ready
}

fn make_ipc_hook(ipc_helper: &Path, agent: &str) -> Value {
    json!({
        "type": "command",
        "command": format!("\"{}\" {}", ipc_helper.display(), agent)
    })
}

fn build_hooks(ipc_helper: &Path, agent: &str) -> Value {
    json!({
        "PreToolUse": [{
            "matcher": ".*",
            "hooks": make_ipc_hook(ipc_helper, agent)
        }],

        "PostToolUse": [{
            "matcher": ".*",
            "hooks": make_ipc_hook(ipc_helper, agent)
        }],

        "PostToolUseFailure": [{
            "matcher": ".*",
            "hooks": make_ipc_hook(ipc_helper, agent)
        }],

        "PermissionRequest": [{
            "matcher": ".*",
            "hooks": make_ipc_hook(ipc_helper, agent)
        }],

        "Stop": [{
            "hooks": make_ipc_hook(ipc_helper, agent)
        }],

        "SubagentStop": [{
            "hooks": make_ipc_hook(ipc_helper, agent)
        }],

        "Notification": [{
            "hooks": make_ipc_hook(ipc_helper, agent)
        }],

        "UserPromptSubmit": [{
            "hooks": make_ipc_hook(ipc_helper, agent)
        }],

        "SessionStart": [{
            "hooks": make_ipc_hook(ipc_helper, agent)
        }],
        "SessionEnd": [{
            "hooks": make_ipc_hook(ipc_helper, agent)
        }],

        "PreCompact": [{
            "hooks": make_ipc_hook(ipc_helper, agent)
        }]
    })
}

fn hook_cc(options: &CliOptions, ipc_helper: &Path) {
    let interactive = options.claude_settings.is_none() && options.scope.is_none();

    loop {
        let config_path = match resolve_claude_settings_path(options) {
            Some(path) => path,
            None => {
                println!("Exiting.");
                return;
            }
        };

        match ensure_file(
            &config_path,
            "{}",
            options.auto_create,
            "File does not exist. Do you want to create it?",
            interactive,
        ) {
            FileEnsureResult::Ready => {
                let mut config: Value = {
                    let content =
                        fs::read_to_string(&config_path).expect("Failed to read config file");
                    serde_json::from_str(&content).unwrap_or(json!({}))
                };

                let backup = config_path.with_extension("json.bak");
                fs::copy(&config_path, &backup).expect("Failed to create backup");
                println!("Backup created at: {}", backup.display());

                config["hooks"] = build_hooks(&ipc_helper, "claude");
                let output =
                    serde_json::to_string_pretty(&config).expect("Failed to serialize config");
                fs::write(&config_path, output).expect("Failed to write config file");
                break;
            }
            FileEnsureResult::Retry => continue,
            FileEnsureResult::Abort => return,
        }
    }
}

fn hook_codex(options: &CliOptions, ipc_helper: &Path) {
    let interactive = options.codex_dir.is_none() && options.scope.is_none();

    loop {
        let config_path = match resolve_codex_paths(options) {
            Some(path) => path,
            None => {
                println!("Exiting.");
                return;
            }
        };

        match ensure_file(
            &config_path[0],
            "{}",
            options.auto_create,
            "hooks.json does not exist. Do you want to create it?",
            interactive,
        ) {
            FileEnsureResult::Ready => {}
            FileEnsureResult::Retry => continue,
            FileEnsureResult::Abort => return,
        }

        match ensure_file(
            &config_path[1],
            "",
            options.auto_create,
            "config.toml does not exist. Do you want to create it?",
            interactive,
        ) {
            FileEnsureResult::Ready => {}
            FileEnsureResult::Retry => continue,
            FileEnsureResult::Abort => return,
        }

        let mut config: Value = {
            let content =
                fs::read_to_string(&config_path[0]).expect("Failed to read config file");
            serde_json::from_str(&content).unwrap_or(json!({}))
        };

        let backup = config_path[0].with_extension("json.bak");
        fs::copy(&config_path[0], &backup).expect("Failed to create backup");
        println!("Backup created at: {}", backup.display());

        let backup = config_path[1].with_extension("toml.bak");
        fs::copy(&config_path[1], &backup).expect("Failed to create backup");
        println!("Backup created at: {}", backup.display());

        let content = fs::read_to_string(&config_path[1]).expect("Failed to read config file");
        let mut doc = if content.trim().is_empty() {
            DocumentMut::new()
        } else {
            content
                .parse::<DocumentMut>()
                .map_err(|e| e.to_string())
                .expect("Failed to parse config.toml")
        };

        if doc.get("features").is_none() {
            doc["features"] = toml_edit::table();
        }

        doc["features"]["codex_hooks"] = value(true);

        fs::write(&config_path[1], doc.to_string()).expect("Failed to write config.toml");

        config["hooks"] = build_hooks(&ipc_helper, "codex");
        let output = serde_json::to_string_pretty(&config).expect("Failed to serialize config");
        fs::write(&config_path[0], output).expect("Failed to write config file");

        println!("Write to hooks successfully, all events are reported through the IPC helper.");
        println!("PreToolUse / PostToolUse will append the tool name to the path, e.g. /PreToolUse/Bash");
        break;
    }
}

fn main() {
    let options = parse_args();
    let ipc_helper = require_ipc_helper(&options);

    let agent = resolve_agent(&options).or_else(select_agent_interactive);
    let Some(agent) = agent else {
        println!("Exiting.");
        return;
    };

    match agent {
        AgentChoice::Both => {
            hook_cc(&options, &ipc_helper);
            hook_codex(&options, &ipc_helper);
        }
        AgentChoice::Claude => hook_cc(&options, &ipc_helper),
        AgentChoice::Codex => hook_codex(&options, &ipc_helper),
    }
}
