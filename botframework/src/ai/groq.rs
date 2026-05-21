use async_openai::types::chat::{
    ChatChoice, ChatCompletionRequestMessage, ChatCompletionTools, CompletionUsage,
    CreateChatCompletionRequest,
};

use super::{AiProvider, ChatCompletionResponseTrait};

pub struct GroqProvider;

#[derive(Debug, serde::Deserialize, Clone, PartialEq, serde::Serialize)]
pub struct GroqCreateChatCompletionResponse {
    /// A unique identifier for the chat completion.
    pub id: String,
    /// A list of chat completion choices. Can be more than one if `n` is greater than 1.
    pub choices: Vec<ChatChoice>,
    /// The Unix timestamp (in seconds) of when the chat completion was created.
    pub created: u32,
    /// The model used for the chat completion.
    pub model: String,

    /// The object type, which is always `chat.completion`.
    pub object: String,
    pub usage: Option<CompletionUsage>,
}

impl ChatCompletionResponseTrait for GroqCreateChatCompletionResponse {
    fn choices(self) -> Vec<ChatChoice> {
        self.choices
    }
}

impl AiProvider for GroqProvider {
    type CreateChatCompletionRequest = CreateChatCompletionRequest;
    type CreateChatCompletionResponse<'de> = GroqCreateChatCompletionResponse;

    fn base_url() -> &'static str {
        "https://api.groq.com/openai/v1"
    }

    fn create_chat_completion_request(
        messages: Vec<ChatCompletionRequestMessage>,
        tools: Option<Vec<ChatCompletionTools>>,
    ) -> Self::CreateChatCompletionRequest {
        CreateChatCompletionRequest {
            model: "openai/gpt-oss-120b".to_string(),
            messages,
            tools,
            ..Default::default()
        }
    }
}
