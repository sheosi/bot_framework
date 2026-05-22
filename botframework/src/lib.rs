// Re-export derive macros when the 'derive' feature is enabled
#[cfg(feature = "derive")]
pub use botframework_derive::ToolParameters;

use clap::Parser;
use dotenvy::dotenv;

pub mod ai;
pub mod api;
pub mod infisical;
pub mod telegram;
pub mod utils;

/// Loads env vars and tracing
pub fn init() -> Cli {
    // Load environment variables
    dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    Cli::parse()
}

pub fn get_port() -> u16 {
    std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8079)
}

#[derive(Parser)]
#[command(name = "go7rs")]
#[command(about = "Go7 Telegram Bot - Rust rewrite")]
pub struct Cli {
    /// Run health check
    #[arg(long = "check")]
    pub check: bool,

    /// Run database migrations
    #[arg(long = "migrate")]
    pub migrate: bool,
}
