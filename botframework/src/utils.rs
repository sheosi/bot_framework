use async_openai::types::chat::{ChatCompletionTools, FunctionObject};

use crate::telegram::{Property, props_to_json};

/// Run health check
pub async fn run_health_check() -> Result<(), anyhow::Error> {
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

pub fn create_ai_fun(
    name: String,
    description: String,
    params_org: Vec<Property>,
) -> ChatCompletionTools {
    let strict = Some(params_org.len() > 0);
    ChatCompletionTools::Function(async_openai::types::chat::ChatCompletionTool {
        function: FunctionObject {
            name,
            description: Some(description),
            parameters: if params_org.len() > 0 {
                Some(props_to_json(params_org))
            } else {
                None
            },
            strict,
        },
    })
}
