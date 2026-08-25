//! TUI interaktif — ported dari packages/opencode/src/cli/index.ts.
//! Loop input → provider → response → tampilkan.

use std::io::{self, Write};

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    style::{Color, Print, SetForegroundColor},
    terminal::{self, ClearType},
};
use oc_session::model::*;
use oc_session::store::SessionStore;

/// Banner yang ditampilkan saat startup.
fn print_banner() {
    let mut stdout = io::stdout();
    let _ = execute!(
        stdout,
        cursor::MoveTo(0, 0),
        terminal::Clear(ClearType::All),
        SetForegroundColor(Color::Cyan),
        Print("╔══════════════════════════════════════════╗\n"),
        Print("║          rust-opencode v0.1.0            ║\n"),
        Print("║   Type your message and press Enter.     ║\n"),
        Print("║   Commands:                              ║\n"),
        Print("║     /quit    — exit                      ║\n"),
        Print("║     /new     — new session               ║\n"),
        Print("║     /list    — list sessions             ║\n"),
        Print("║     /clear   — clear screen              ║\n"),
        Print("╚══════════════════════════════════════════╝\n"),
        SetForegroundColor(Color::Reset),
        Print("\n"),
    );
}

/// Prompt input dari user.
fn read_input() -> Option<String> {
    let mut stdout = io::stdout();
    let _ = execute!(
        stdout,
        SetForegroundColor(Color::Green),
        Print("you > "),
        SetForegroundColor(Color::Reset),
    );
    let _ = stdout.flush();

    let mut input = String::new();
    loop {
        match event::read() {
            Ok(Event::Key(key)) if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Char(c) => {
                    input.push(c);
                    print!("{}", c);
                    let _ = stdout.flush();
                }
                KeyCode::Backspace => {
                    input.pop();
                    let _ = execute!(stdout, cursor::MoveLeft(1), Print(" "), cursor::MoveLeft(1),);
                    let _ = stdout.flush();
                }
                KeyCode::Enter => {
                    println!();
                    let _ = stdout.flush();
                    return Some(input);
                }
                KeyCode::Esc => return None,
                _ => {}
            },
            Ok(_) => {}
            Err(_) => return None,
        }
    }
}

/// Tampilkan response dari assistant.
fn print_assistant(text: &str) {
    let mut stdout = io::stdout();
    let _ = execute!(
        stdout,
        SetForegroundColor(Color::Blue),
        Print("assistant > "),
        SetForegroundColor(Color::Reset),
    );
    for line in text.lines() {
        println!("  {line}");
    }
    println!();
    let _ = stdout.flush();
}

/// Tampilkan error.
fn print_error(msg: &str) {
    let mut stdout = io::stdout();
    let _ = execute!(
        stdout,
        SetForegroundColor(Color::Red),
        Print("error: "),
        SetForegroundColor(Color::Reset),
        Print(format!("{msg}\n")),
    );
    let _ = stdout.flush();
}

/// Tampilkan info.
fn print_info(msg: &str) {
    let mut stdout = io::stdout();
    let _ = execute!(
        stdout,
        SetForegroundColor(Color::Yellow),
        Print("info: "),
        SetForegroundColor(Color::Reset),
        Print(format!("{msg}\n")),
    );
    let _ = stdout.flush();
}

/// Main TUI loop — ported dari cli/index.ts interactive mode.
pub fn run_interactive(store: &SessionStore, directory: &str) {
    print_banner();

    // Buat atau load session
    let session_id = format!(
        "ses_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    );
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let session = SessionRow {
        id: session_id.clone(),
        title: "Interactive Session".into(),
        directory: directory.into(),
        version: "1.18.21".into(),
        slug: "interactive".into(),
        project_id: "default".into(),
        time_created: now,
        time_updated: now,
        ..Default::default()
    };
    if let Err(e) = store.upsert_session(&session) {
        print_error(&format!("Failed to create session: {e}"));
        return;
    }
    print_info(&format!("Session: {} in {}", session_id, directory));

    // Message history untuk provider
    let mut messages: Vec<serde_json::Value> = Vec::new();

    while let Some(input) = read_input() {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Handle commands
        match trimmed {
            "/quit" | "/exit" | "/q" => break,
            "/new" => {
                let new_id = format!(
                    "ses_{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis())
                        .unwrap_or(0)
                );
                let now2 = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let new_session = SessionRow {
                    id: new_id.clone(),
                    title: "New Session".into(),
                    directory: directory.into(),
                    version: "1.18.21".into(),
                    slug: "new".into(),
                    project_id: "default".into(),
                    time_created: now2,
                    time_updated: now2,
                    ..Default::default()
                };
                if store.upsert_session(&new_session).is_ok() {
                    messages.clear();
                    print_info(&format!("New session: {}", new_id));
                } else {
                    print_error("Failed to create new session");
                }
                continue;
            }
            "/list" => {
                match store.list_sessions() {
                    Ok(sessions) => {
                        if sessions.is_empty() {
                            print_info("No sessions found.");
                        } else {
                            for s in &sessions {
                                println!("  {}  {}", s.id, s.title);
                            }
                        }
                    }
                    Err(e) => print_error(&format!("Failed to list sessions: {e}")),
                }
                continue;
            }
            "/clear" => {
                let _ = execute!(
                    io::stdout(),
                    terminal::Clear(ClearType::All),
                    cursor::MoveTo(0, 0)
                );
                continue;
            }
            _ => {}
        }

        // Save user message to store
        let msg_id = format!(
            "msg_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        let user_msg = UserOrAssistant::User(UserMessage {
            id: msg_id,
            session_id: session_id.clone(),
            time: TimeCreated { created: now },
            format: None,
            summary: None,
            agent: "build".into(),
            model: ModelRefJson {
                provider_id: "anthropic".into(),
                model_id: "claude-sonnet-4".into(),
            },
            system: None,
            tools: None,
        });
        let _ = store.append_message(&user_msg);

        messages.push(serde_json::json!({
            "role": "user",
            "content": trimmed
        }));

        // Placeholder response — full provider integration via prompt loop
        let response_text = format!(
            "[placeholder] Received: \"{}\"\n\
             Full LLM integration requires provider configuration.\n\
             Use `--help` or see README for setup instructions.",
            trimmed
        );

        // Save assistant response
        let asst_msg = UserOrAssistant::Assistant(AssistantMessage {
            id: format!(
                "msg_a{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or(0)
            ),
            session_id: session_id.clone(),
            time: TimeWithCompletion {
                created: now,
                completed: Some(now),
            },
            error: None,
            parent_id: "msg_p".into(),
            model_id: "claude-sonnet-4".into(),
            provider_id: "anthropic".into(),
            mode: "primary".into(),
            agent: "build".into(),
            path: SessionPath {
                cwd: directory.into(),
                root: directory.into(),
            },
            summary: None,
            cost: 0.0,
            tokens: TokenUsage::default(),
            structured: None,
            variant: None,
            finish: Some("stop".into()),
        });
        let _ = store.append_message(&asst_msg);

        print_assistant(&response_text);
    }

    print_info("Goodbye!");
}
