//! Ported from: packages/opencode/src/tool/shell/prompt.ts

use crate::truncate;

const PS_SHELLS: &[&str] = &["powershell", "pwsh"];
const CMD_SHELLS: &[&str] = &["cmd"];

/// Ported from: prompt.ts renderPrompt — substitusi ${key}.
fn render_prompt(template: &str, values: &[(&str, String)]) -> String {
    let mut out = template.to_string();
    // proses berulang sederhana: ganti ${word} bila ada di values
    let mut result = String::with_capacity(template.len());
    let chars: Vec<char> = out.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] == '$' && i + 1 < chars.len() && chars[i + 1] == '{' {
            let mut j = i + 2;
            while j < chars.len() && chars[j] != '}' {
                j += 1;
            }
            if j < chars.len() {
                let key: String = chars[i + 2..j].iter().collect();
                if let Some((_, value)) = values.iter().find(|(k, _)| *k == key) {
                    result.push_str(value);
                    i = j + 1;
                    continue;
                }
            }
        }
        result.push(chars[i]);
        i += 1;
    }
    out.clear();
    out.push_str(&result);
    out
}

/// Ported from: prompt.ts shellDisplayName
fn shell_display_name(name: &str) -> String {
    match name {
        "pwsh" => "PowerShell (7+)".to_string(),
        "powershell" => "Windows PowerShell (5.1)".to_string(),
        "cmd" => "cmd.exe".to_string(),
        other => other.to_string(),
    }
}

/// Ported from: prompt.ts powershellNotes
fn powershell_notes(name: &str) -> String {
    if name == "pwsh" {
        return "# PowerShell (7+) shell notes\n- This cross-platform shell supports pipeline chain operators (`&&` and `||`).\n- Use double quotes for interpolated strings (`\"Hello $name\"`), single quotes for verbatim strings.\n- Prefer full cmdlet names like `Get-ChildItem`, `Set-Content`, `Remove-Item`, and `New-Item` over aliases.\n- Use `$(...)` for subexpressions. Use `@(...)` for array expressions.\n- To call a native executable whose path contains spaces, use the call operator: `& \"path/to/exe\" args`.\n- Escape special characters with the PowerShell backtick character.".to_string();
    }
    if name == "powershell" {
        return "# Windows PowerShell (5.1) shell notes\n- Use `cmd1; if ($?) { cmd2 }` to chain dependent commands.\n- Use double quotes for interpolated strings (`\"Hello $name\"`), single quotes for verbatim strings.\n- Prefer full cmdlet names like `Get-ChildItem`, `Set-Content`, `Remove-Item`, and `New-Item` over aliases.\n- Use `$(...)` for subexpressions. Use `@(...)` for array expressions.\n- To call a native executable whose path contains spaces, use the call operator: `& \"path/to/exe\" args`.\n- Escape special characters with the PowerShell backtick character.".to_string();
    }
    String::new()
}

/// Ported from: prompt.ts chainGuidance
fn chain_guidance(name: &str) -> String {
    if name == "powershell" {
        return "If the commands depend on each other and must run sequentially, avoid '&&' in this shell because Windows PowerShell (5.1) does not support it. Use PowerShell conditionals such as `cmd1; if ($?) { cmd2 }` when later commands must depend on earlier success.".to_string();
    }
    if PS_SHELLS.contains(&name) {
        return "If the commands depend on each other and must run sequentially, use a single bash tool call with '&&' to chain them together (e.g., `git add . && git commit -m \"message\" && git push`). For instance, if one operation must complete before another starts (like New-Item before Copy-Item, Write before bash for git operations, or git add before git commit), run these operations sequentially instead.".to_string();
    }
    if CMD_SHELLS.contains(&name) {
        return "If the commands depend on each other and must run sequentially, use a single bash tool call with `&&` to chain them together (e.g., `mkdir out && dir out`). For instance, if one operation must complete before another starts, run these operations sequentially instead.".to_string();
    }
    "If the commands depend on each other and must run sequentially, use a single Bash call with '&&' to chain them together (e.g., `git add . && git commit -m \"message\" && git push`). For instance, if one operation must complete before another starts (like mkdir before cp, Write before Bash for git operations, or git add before git commit), run these operations sequentially instead.".to_string()
}

/// Ported from: prompt.ts bashCommandSection
fn bash_command_section(chain: &str, limits: (usize, usize), default_timeout_ms: u64) -> String {
    format!(
        "Before executing the command, please follow these steps:\n\n1. Directory Verification:\n   - If the command will create new directories or files, first use `ls` to verify the parent directory exists and is the correct location\n   - For example, before running \"mkdir foo/bar\", first use `ls foo` to check that \"foo\" exists and is the intended parent directory\n\n2. Command Execution:\n   - Always quote file paths that contain spaces with double quotes (e.g., rm \"path with spaces/file.txt\")\n   - Examples of proper quoting:\n     - mkdir \"/Users/name/My Documents\" (correct)\n     - mkdir /Users/name/My Documents (incorrect - will fail)\n     - python \"/path/with spaces/script.py\" (correct)\n     - python /path/with spaces/script.py (incorrect - will fail)\n   - After ensuring proper quoting, execute the command.\n   - Capture the output of the command.\n\nUsage notes:\n  - The command argument is required.\n  - You can specify an optional timeout in milliseconds. If not specified, commands will time out after {default_timeout_ms}ms.\n  - If the output exceeds {} lines or {} bytes, it will be truncated and the full output will be written to a file. You can use Read with offset/limit to read specific sections or Grep to search the full content. Do NOT use `head`, `tail`, or other truncation commands to limit output; the full output will already be captured to a file for more precise searching.\n\n  - Avoid using Bash with the `find`, `grep`, `cat`, `head`, `tail`, `sed`, `awk`, or `echo` commands, unless explicitly instructed or when these commands are truly necessary for the task. Instead, always prefer using the dedicated tools for these commands:\n    - File search: Use Glob (NOT find or ls)\n    - Content search: Use Grep (NOT grep or rg)\n    - Read files: Use Read (NOT cat/head/tail)\n    - Edit files: Use Edit (NOT sed/awk)\n    - Write files: Use Write (NOT echo >/cat <<EOF)\n    - Communication: Output text directly (NOT echo/printf)\n  - When issuing multiple commands:\n    - If the commands are independent and can run in parallel, make multiple bash tool calls in a single message. For example, if you need to run \"git status\" and \"git diff\", send a single message with two bash tool calls in parallel.\n    - {chain}\n    - Use ';' only when you need to run commands sequentially but don't care if earlier commands fail\n    - DO NOT use newlines to separate commands (newlines are ok in quoted strings)\n  - AVOID using `cd <directory> && <command>`. Use the `workdir` parameter to change directories instead.\n    <good-example>\n    Use workdir=\"/foo/bar\" with command: pytest tests\n    </good-example>\n    <bad-example>\n    cd /foo/bar && pytest tests\n    </bad-example>",
        limits.0, limits.1
    )
}

/// Ported from: prompt.ts powershellCommandSection
fn powershell_command_section(
    name: &str,
    chain: &str,
    path_sep: &str,
    limits: (usize, usize),
    default_timeout_ms: u64,
) -> String {
    let bad_example = if name == "powershell" {
        format!("Set-Location -LiteralPath \"project{path_sep}subdir\"; if ($?) {{ pytest tests }}")
    } else {
        format!("Set-Location -LiteralPath \"project{path_sep}subdir\" && pytest tests")
    };
    format!(
        "{}\n\nBefore executing the command, please follow these steps:\n\n1. Directory Verification:\n   - If the command will create new directories or files, first use `Test-Path -LiteralPath <parent>` to verify the parent directory exists and is the correct location\n   - For example, before creating `foo{path_sep}bar`, first use `Test-Path -LiteralPath \"foo\"` to check that `foo` exists and is the intended parent directory\n\n2. Command Execution:\n   - Always quote file paths that contain spaces with double quotes (e.g., Remove-Item -LiteralPath \"path with spaces{path_sep}file.txt\")\n   - Examples of proper quoting:\n     - New-Item -ItemType Directory -Path \"My Documents\" (correct)\n     - New-Item -ItemType Directory -Path My Documents (incorrect - path is split)\n     - & \"path with spaces{path_sep}script.ps1\" (correct)\n     - path with spaces{path_sep}script.ps1 (incorrect - path is split and not invoked)\n   - After ensuring proper quoting, execute the command.\n   - Capture the output of the command.\n\nUsage notes:\n  - The command argument is required.\n  - You can specify an optional timeout in milliseconds. If not specified, commands will time out after {default_timeout_ms}ms.\n  - If the output exceeds {} lines or {} bytes, it will be truncated and the full output will be written to a file. You can use Read with offset/limit to read specific sections or Grep to search the full content. Do NOT use `Select-Object -First`, `Select-Object -Last`, or other truncation commands to limit output; the full output will already be captured to a file for more precise searching.\n\n  - Avoid using Shell with PowerShell file/content cmdlets unless explicitly instructed or when these cmdlets are truly necessary for the task. Instead, always prefer using the dedicated tools for these commands:\n    - File search: Use Glob (NOT Get-ChildItem)\n    - Content search: Use Grep (NOT Select-String)\n    - Read files: Use Read (NOT Get-Content)\n    - Edit files: Use Edit (NOT Set-Content)\n    - Write files: Use Write (NOT Set-Content/Out-File or here-strings)\n    - Communication: Output text directly (NOT Write-Output/Write-Host)\n  - When issuing multiple commands:\n    - If the commands are independent and can run in parallel, make multiple bash tool calls in a single message. For example, if you need to run \"git status\" and \"git diff\", send a single message with two bash tool calls in parallel.\n    - {chain}\n    - Use `;` only when you need to run commands sequentially but don't care if earlier commands fail\n    - DO NOT use newlines to separate commands (newlines are ok in quoted strings)\n  - AVOID changing directories inside the command. Use the `workdir` parameter to change directories instead.\n    <good-example>\n    Use workdir=\"project{path_sep}subdir\" with command: pytest tests\n    </good-example>\n    <bad-example>\n    {bad_example}\n    </bad-example>",
        powershell_notes(name),
        limits.0,
        limits.1
    )
}

/// Ported from: prompt.ts cmdCommandSection
fn cmd_command_section(chain: &str, limits: (usize, usize), default_timeout_ms: u64) -> String {
    format!(
        "# cmd.exe shell notes\n- Use double quotes for paths with spaces.\n- Use %VAR% for environment variables.\n- Use `if exist` for existence checks.\n- Use `call` when invoking batch files from another batch-style command.\n\nBefore executing the command, please follow these steps:\n\n1. Directory Verification:\n   - If the command will create new directories or files, first use `if exist` to verify the parent directory exists and is the correct location\n   - For example, before creating `foo\\\\bar`, first use `if exist \"foo\\\\\" dir \"foo\"` to check that `foo` exists and is the intended parent directory\n\n2. Command Execution:\n   - Always quote file paths that contain spaces with double quotes (e.g., del \"path with spaces\\\\file.txt\")\n   - Examples of proper quoting:\n     - mkdir \"My Documents\" (correct)\n     - mkdir My Documents (incorrect - path is split)\n     - call \"path with spaces\\\\script.bat\" (correct)\n     - path with spaces\\\\script.bat (incorrect - path is split and not invoked correctly)\n   - After ensuring proper quoting, execute the command.\n   - Capture the output of the command.\n\nUsage notes:\n  - The command argument is required.\n  - You can specify an optional timeout in milliseconds. If not specified, commands will time out after {default_timeout_ms}ms.\n  - If the output exceeds {} lines or {} bytes, it will be truncated and the full output will be written to a file. You can use Read with offset/limit to read specific sections or Grep to search the full content. Do NOT use `more` or other pagination commands to limit output; the full output will already be captured to a file for more precise searching.\n\n  - Avoid using Shell with cmd.exe file/content commands unless explicitly instructed or when these commands are truly necessary for the task. Instead, always prefer using the dedicated tools for these commands:\n    - File search: Use Glob (NOT dir /s)\n    - Content search: Use Grep (NOT findstr)\n    - Read files: Use Read (NOT type)\n    - Edit files: Use Edit (NOT copy)\n    - Write files: Use Write (NOT echo > file)\n    - Communication: Output text directly (NOT echo)\n  - When issuing multiple commands:\n    - If the commands are independent and can run in parallel, make multiple bash tool calls in a single message. For example, if you need to run \"dir\" and \"where cmd\", send a single message with two bash tool calls in parallel.\n    - {chain}\n    - Use `&` only when you need to run commands sequentially but don't care if earlier commands fail\n    - DO NOT use newlines to separate commands (newlines are ok in quoted strings)\n  - AVOID changing directories inside the command. Use the `workdir` parameter to change directories instead.\n    <good-example>\n    Use workdir=\"project\\\\subdir\" with command: dir\n    </good-example>\n    <bad-example>\n    cd /d \"project\\\\subdir\" && dir\n    </bad-example>",
        limits.0, limits.1
    )
}

struct Profile {
    intro: String,
    workdir_section: &'static str,
    command_section: String,
    git_commands: &'static str,
    git_command_restriction: &'static str,
    create_pr_instruction: &'static str,
    create_pr_example: String,
}

/// Ported from: prompt.ts profile()
fn profile(name: &str, platform: &str, limits: (usize, usize), default_timeout_ms: u64) -> Profile {
    let is_powershell = PS_SHELLS.contains(&name);
    let chain = chain_guidance(name);
    if CMD_SHELLS.contains(&name) {
        return Profile {
            intro: format!(
                "Executes a given {} command with optional timeout, ensuring proper handling and security measures.",
                shell_display_name(name)
            ),
            workdir_section: "All commands run in the current working directory by default. Use the `workdir` parameter if you need to run a command in a different directory. AVOID changing directories inside the command - use `workdir` instead.",
            command_section: cmd_command_section(&chain, limits, default_timeout_ms),
            git_commands: "git commands",
            git_command_restriction: "git commands",
            create_pr_instruction: "Create PR using a temporary body file so cmd.exe quoting stays simple.",
            create_pr_example: "(\n  echo ## Summary\n  echo - ^<1-3 bullet points^>\n) > pr-body.txt\ngh pr create --title \"the pr title\" --body-file pr-body.txt".to_string(),
        };
    }
    if is_powershell {
        let path_sep = if platform == "win32" { "\\" } else { "/" };
        return Profile {
            intro: format!(
                "Executes a given {} command with optional timeout, ensuring proper handling and security measures.",
                shell_display_name(name)
            ),
            workdir_section: "All commands run in the current working directory by default. Use the `workdir` parameter if you need to run a command in a different directory. AVOID changing directories inside the command - use `workdir` instead.",
            command_section: powershell_command_section(name, &chain, path_sep, limits, default_timeout_ms),
            git_commands: "git commands",
            git_command_restriction: "git commands",
            create_pr_instruction: "Create PR using gh pr create with a PowerShell here-string to pass the body correctly.",
            create_pr_example: "gh pr create --title \"the pr title\" --body @'\n## Summary\n- <1-3 bullet points>\n'@".to_string(),
        };
    }
    Profile {
        intro: "Executes a given bash command in a persistent shell session with optional timeout, ensuring proper handling and security measures.".to_string(),
        workdir_section: "All commands run in the current working directory by default. Use the `workdir` parameter if you need to run a command in a different directory. AVOID using `cd <directory> && <command>` patterns - use `workdir` instead.",
        command_section: bash_command_section(&chain, limits, default_timeout_ms),
        git_commands: "bash commands",
        git_command_restriction: "git bash commands",
        create_pr_instruction: "Create PR using gh pr create with the format below. Use a HEREDOC to pass the body to ensure correct formatting.",
        create_pr_example: "gh pr create --title \"the pr title\" --body \"$(cat <<'EOF'\n## Summary\n<1-3 bullet points>".to_string(),
    }
}

pub struct RenderedPrompt {
    pub description: String,
}

/// Ported from: prompt.ts render() — substitusi ke template DESCRIPTION.
pub fn render(name: &str, platform: &str, limits: (usize, usize), default_timeout_ms: u64) -> RenderedPrompt {
    const TEMPLATE: &str = include_str!("../assets/shell.txt");
    let selected = profile(name, platform, limits, default_timeout_ms);
    let tmp = oc_global::global::path().tmp.to_string_lossy().into_owned();
    let description = render_prompt(
        TEMPLATE,
        &[
            ("intro", selected.intro),
            ("os", platform.to_string()),
            ("shell", name.to_string()),
            ("tmp", tmp),
            ("workdirSection", selected.workdir_section.to_string()),
            ("commandSection", selected.command_section),
            ("gitCommands", selected.git_commands.to_string()),
            ("toolName", "bash".to_string()),
            ("gitCommandRestriction", selected.git_command_restriction.to_string()),
            ("createPrInstruction", selected.create_pr_instruction.to_string()),
            ("createPrExample", selected.create_pr_example),
        ],
    );
    RenderedPrompt { description }
}

/// Default limits padanan Truncate.limits() subset (config tool_output menyusul).
pub fn default_limits() -> (usize, usize) {
    (truncate::MAX_LINES, truncate::MAX_BYTES)
}
