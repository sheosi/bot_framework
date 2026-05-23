use async_openai::types::chat::{ChatCompletionTools, FunctionObject};
use teloxide::types::ChatId;

use crate::telegram::{Property, TgBot, props_to_json};

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

pub trait ErrMsg {
    fn err_msg(
        self,
        chat_id: ChatId,
        msg: &str,
        ctx: &TgBot,
    ) -> impl std::future::Future<Output = Self>;
}

impl<T, E> ErrMsg for Result<T, E> {
    async fn err_msg(self, chat_id: ChatId, msg: &str, bot: &TgBot) -> Self {
        if let Err(_) = self {
            if let Err(e) = bot.send_raw(chat_id, msg).await {
                tracing::error!("Failed to send error msg: {e}")
            }
        }

        self
    }
}
