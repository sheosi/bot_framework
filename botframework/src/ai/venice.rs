use async_openai::types::chat::{
    ChatCompletionRequestMessage, ChatCompletionTools, CreateChatCompletionResponse,
};

use super::AiProvider;

#[derive(Clone)]
pub struct VeniceProvider;

impl AiProvider for VeniceProvider {
    type CreateChatCompletionRequest = VeniceCreateChatCompletionRequest;
    type CreateChatCompletionResponse<'de> = CreateChatCompletionResponse;

    fn tts_model() -> &'static str {
        "whisper-large-v3-turbo"
    }

    fn base_url() -> &'static str {
        "https://api.venice.ai/api/v1"
    }

    fn create_chat_completion_request(
        messages: Vec<async_openai::types::chat::ChatCompletionRequestMessage>,
        tools: Option<Vec<async_openai::types::chat::ChatCompletionTools>>,
    ) -> Self::CreateChatCompletionRequest {
        VeniceCreateChatCompletionRequest {
            model: "".to_string(),
            messages,
            tools,
            venice_parameters: VeniceCreateChatCompletionRequestVeniceParams {
                enable_web_search: EnablingState::Off,
            },
        }
    }
}

#[derive(serde::Serialize)]
pub struct VeniceCreateChatCompletionRequest {
    model: String,
    messages: Vec<ChatCompletionRequestMessage>,
    tools: Option<Vec<ChatCompletionTools>>,
    venice_parameters: VeniceCreateChatCompletionRequestVeniceParams,
}

#[derive(serde::Serialize)]
struct VeniceCreateChatCompletionRequestVeniceParams {
    enable_web_search: EnablingState,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "lowercase")]
enum EnablingState {
    Auto,
    Off,
    On,
}
