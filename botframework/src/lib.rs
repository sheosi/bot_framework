use clap::Parser;
use dotenvy::dotenv;

pub mod ai;
pub mod api;
pub mod infisical;
pub mod md_to_tg;
pub mod telegram;
pub mod utils;

#[cfg(feature = "health_check")]
fn try_run_health_check_api(cli: &Cli) {
    if !cli.migrate && !cli.check {
        api::run_healthcheck_api();
    }
}

#[cfg(not(feature = "health_check"))]
fn try_run_health_check_api(_cli: &Cli) {}

/// Loads env vars and tracing
pub fn init() -> Cli {
    // Load environment variables
    dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    try_run_health_check_api(&cli);

    cli
}

pub fn get_port() -> u16 {
    std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8079)
}

#[cfg(feature = "health_check")]
#[derive(Parser)]
#[command(name = "botframework")]
#[command(about = "A bot from botframework")]
pub struct Cli {
    /// Run health check
    #[arg(long = "check")]
    pub check: bool,

    /// Run database migrations
    #[arg(long = "migrate")]
    pub migrate: bool,
}

#[cfg(not(feature = "health_check"))]
#[derive(Parser)]
#[command(name = "botframework")]
#[command(about = "A bot from botframework")]
pub struct Cli {
    /// Run database migrations
    #[arg(long = "migrate")]
    pub migrate: bool,
}

#[derive(serde::Deserialize)]
pub struct ToolData<'a> {
    pub name: &'a str,
    pub arguments: &'a str,
}
