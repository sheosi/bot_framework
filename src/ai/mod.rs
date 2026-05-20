mod groq;
mod venice;

pub use groq::GroqProvider;
pub use venice::VeniceProvider;

use std::{marker::PhantomData, path::Path};

use async_openai::types::{
    audio::{AudioInput, CreateTranscriptionRequest},
    chat::{
        ChatChoice, ChatCompletionRequestAssistantMessage,
        ChatCompletionRequestAssistantMessageContent, ChatCompletionRequestMessage,
        ChatCompletionRequestSystemMessage, ChatCompletionRequestSystemMessageContent,
        ChatCompletionRequestUserMessage, ChatCompletionRequestUserMessageContent,
        ChatCompletionResponseMessage, ChatCompletionTools, CreateChatCompletionResponse,
    },
};

type Result<T> = core::result::Result<T, anyhow::Error>;

// Some providers have some unique and special features, so we need to specialize for it
pub trait AiProvider {
    type CreateChatCompletionResponse<'de>: serde::de::DeserializeOwned
        + Clone
        + serde::Serialize
        + ChatCompletionResponseTrait;

    type CreateChatCompletionRequest: serde::Serialize;

    fn base_url() -> &'static str;
    fn create_chat_completion_request(
        messages: Vec<ChatCompletionRequestMessage>,
        tools: Option<Vec<ChatCompletionTools>>,
    ) -> Self::CreateChatCompletionRequest;
}

pub trait ChatCompletionResponseTrait {
    fn choices(self) -> Vec<ChatChoice>;
}

impl ChatCompletionResponseTrait for CreateChatCompletionResponse {
    fn choices(self) -> Vec<ChatChoice> {
        self.choices
    }
}

pub struct AiService<A: AiProvider> {
    client: async_openai::Client<async_openai::config::OpenAIConfig>,
    prompt: String,
    ai_tools: Vec<ChatCompletionTools>,
    _phantom: PhantomData<A>,
}

impl<A: AiProvider> AiService<A> {
    pub fn new<'a>(
        groq_key: &'a str,
        ai_tools: Vec<ChatCompletionTools>,
        ai_prompt: String,
    ) -> Self {
        let client = async_openai::Client::with_config(
            async_openai::config::OpenAIConfig::new()
                .with_api_key(groq_key)
                .with_api_base(A::base_url()),
        );
        Self {
            ai_tools,
            client,
            prompt: ai_prompt,
            _phantom: PhantomData,
        }
    }

    pub async fn input(
        &self,
        input: String,
        history: Vec<(String, String)>,
        with_tools: bool,
    ) -> Result<ChatCompletionResponseMessage> {
        fn user_msg(msg: String) -> ChatCompletionRequestMessage {
            ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
                content: ChatCompletionRequestUserMessageContent::Text(msg),
                name: None,
            })
        }

        fn assistant_msg(msg: String) -> ChatCompletionRequestMessage {
            ChatCompletionRequestMessage::Assistant(ChatCompletionRequestAssistantMessage {
                content: Some(ChatCompletionRequestAssistantMessageContent::Text(msg)),
                name: None,
                audio: None,
                refusal: None,
                tool_calls: None,
                function_call: None,
            })
        }

        let tools = if with_tools {
            Some(self.ai_tools.clone())
        } else {
            None
        };

        let mut transformed = Vec::with_capacity((history.len() + 1) * 2);
        transformed.push(ChatCompletionRequestMessage::System(
            ChatCompletionRequestSystemMessage {
                content: ChatCompletionRequestSystemMessageContent::Text(self.prompt.clone()),
                name: None,
            },
        ));

        for val in history.into_iter() {
            transformed.push(user_msg(val.0));
            transformed.push(assistant_msg(val.1));
        }

        transformed.push(user_msg(input));

        let req = A::create_chat_completion_request(transformed, tools);
        let resp: A::CreateChatCompletionResponse<'_> = self.client.chat().create_byot(req).await?;

        Ok(resp.choices().into_iter().next().unwrap().message)
    }

    pub async fn transcribe_audio(&self, input: &Path) -> Result<String> {
        let file_name = input.file_name().unwrap().to_str().unwrap().to_string();
        let contents = std::fs::read(input).unwrap();
        let res = self
            .client
            .audio()
            .transcription()
            .create(CreateTranscriptionRequest {
                file: AudioInput::from_vec_u8(file_name, contents),
                model: "whisper-large-v3-turbo".into(),
                ..Default::default()
            })
            .await?;

        Ok(res.text)
    }
}
