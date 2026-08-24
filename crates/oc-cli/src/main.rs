//! Ported dari packages/opencode/src/cli/index.ts (subset core commands).

use clap::{Parser, Subcommand};
use oc_session::store::SessionStore;

#[derive(Parser)]
#[command(name = "rust-opencode", version, about = "Rust port of sst/opencode")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start interactive session
    Run {
        /// Working directory
        #[arg(short, long)]
        directory: Option<String>,
    },
    /// List sessions
    Sessions,
    /// Show server status
    Serve {
        #[arg(short, long, default_value = "4096")]
        port: u16,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Run { directory }) => {
            let dir = directory.unwrap_or_else(|| ".".to_string());
            println!("Starting opencode session in {dir}...");
            // TODO: full TUI menyusul
            println!("Interactive TUI not yet implemented.");
        }
        Some(Commands::Sessions) => {
            let store = SessionStore::new().unwrap_or_else(|e| {
                eprintln!("Failed to init storage: {e}");
                std::process::exit(1);
            });
            let sessions = store.list_sessions().unwrap_or_default();
            if sessions.is_empty() {
                println!("No sessions found.");
                return;
            }
            for session in &sessions {
                println!("{}  {}", session.id, session.title);
            }
        }
        Some(Commands::Serve { port }) => {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async move {
                let store = SessionStore::new().unwrap_or_else(|e| {
                    eprintln!("Failed to init storage: {e}");
                    std::process::exit(1);
                });
                let app = oc_server::router(store);
                let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}"))
                    .await
                    .unwrap();
                println!("Listening on http://127.0.0.1:{port}");
                axum::serve(listener, app).await.unwrap();
            });
        }
        None => {
            println!("rust-opencode: not yet implemented — use --help for commands");
        }
    }
}
