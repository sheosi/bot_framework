/// Run health check
async fn run_health_check() -> Result<()> {
    use reqwest::Client;

    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8079u16);

    let client = Client::new();
    let url = format!("http://localhost:{}/health", port);

    match client.get(&url).send().await {
        Ok(response) if response.status().is_success() => {
            println!("Health check passed");
            Ok(())
        }
        Ok(response) => {
            anyhow::bail!("Health check failed with status: {}", response.status())
        }
        Err(e) => {
            anyhow::bail!("Health check failed: {}", e)
        }
    }
}
