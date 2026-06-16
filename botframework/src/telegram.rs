use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Context;
use chrono::{DateTime, Utc};
use teloxide::{
    Bot,
    dispatching::UpdateFilterExt,
    net::Download,
    payloads::{AnswerCallbackQuerySetters, SendMessageSetters},
    requests::Requester,
    types::{
        CallbackQuery, ChatId, FileId, InlineKeyboardButton, InlineKeyboardMarkup, InputFile,
        MaybeInaccessibleMessage, MessageId, ParseMode, Update, User, UserId,
    },
};
use tokio::sync::RwLock;

// Re-export derive macros when the 'derive' feature is enabled
#[cfg(feature = "derive")]
pub use botframework_derive::ToolParameters;

pub fn props_to_json(props: Properties) -> serde_json::Value {
    let required: Vec<&'static str> = props.iter().map(|p| p.name).collect();
    let mut prop_map = HashMap::new();

    for prop in props.iter() {
        let prop_content = if let Some(enum_values) = prop.kind.enum_values() {
            // Include enum values in the property
            serde_json::json!({
                "type": prop.kind.as_str(),
                "description": prop.description,
                "enum": enum_values
            })
        } else {
            serde_json::json!({
                "type": prop.kind.as_str(),
                "description": prop.description
            })
        };
        prop_map.insert(prop.name, prop_content);
    }

    serde_json::json!({
        "type": "object",
        "properties": prop_map,
        "required": required,
        "additionalProperties": false
    })
}
pub struct Property {
    pub kind: PropertyKind,
    pub name: &'static str,
    pub description: &'static str,
}

impl Property {
    pub fn string(name: &'static str, description: &'static str) -> Self {
        Property {
            kind: PropertyKind::String,
            name,
            description,
        }
    }

    pub fn integer(name: &'static str, description: &'static str) -> Self {
        Property {
            kind: PropertyKind::Integer,
            name,
            description,
        }
    }

    pub fn number(name: &'static str, description: &'static str) -> Self {
        Property {
            kind: PropertyKind::Number,
            name,
            description,
        }
    }

    pub fn string_enum(
        name: &'static str,
        description: &'static str,
        values: &'static [&'static str],
    ) -> Self {
        Property {
            kind: PropertyKind::Enum(values),
            name,
            description,
        }
    }

    pub fn boolean(name: &'static str, description: &'static str) -> Self {
        Property {
            kind: PropertyKind::Boolean,
            name,
            description,
        }
    }
}

/// Trait for types that can provide tool parameter metadata.
/// This is automatically implemented by the `ToolParameters` derive macro.
pub trait ToolParameters {
    /// Returns the properties vector describing the parameters
    fn parameters() -> Properties;
}

pub type Properties = Vec<Property>;

pub enum PropertyKind {
    String,
    Integer,
    Number,
    Boolean,
    Enum(&'static [&'static str]),
}

impl PropertyKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            PropertyKind::String => "string",
            PropertyKind::Integer => "integer",
            PropertyKind::Number => "number",
            PropertyKind::Boolean => "boolean",
            PropertyKind::Enum(_) => "string",
        }
    }

    /// Returns the enum values if this is an Enum kind, None otherwise
    pub fn enum_values(&self) -> Option<&'static [&'static str]> {
        match self {
            PropertyKind::Enum(values) => Some(values),
            _ => None,
        }
    }
}

/// Delayed action for callback handling
#[derive(Debug, Clone)]
pub struct DelayedAction {
    pub action: String,
    pub target: String,
    pub expires: DateTime<Utc>,
}

pub enum ToolCallAction {
    Message(String),
    MarkDown(String),
    Confirm(String, String),
    List { msg: String, items: Vec<String> },
}

impl From<String> for ToolCallAction {
    fn from(value: String) -> Self {
        ToolCallAction::Message(value)
    }
}

impl From<&str> for ToolCallAction {
    fn from(value: &str) -> Self {
        ToolCallAction::Message(value.to_string())
    }
}

/// Represents a message from Telegram
#[derive(Debug, Clone)]
pub struct Message {
    pub chat_id: ChatId,
    pub text: String,
    pub username: String,
}

mod md_replace {
    v_escape::escape! {
        b'[' -> "\\[",  b']' -> "\\]", b'(' -> "\\(",  b')' -> "\\)",
        b'~' -> "\\~", b'`' -> "\\`", b'>' -> "\\>", b'#' -> "\\#",
        b'+' -> "\\+", b'-' -> "\\-", b'=' -> "\\=", b'|' -> "\\|", b'{' -> "\\{",
        b'}' -> "\\}", b'.' -> "\\.", b'!' -> "\\!"
    }

    pub fn escape_md(input: &str) -> String {
        escape_fmt(input).to_string()
    }
}

pub use md_replace::escape_md;
use tracing::error;

use crate::ai::{AiProvider, AiService};

#[derive(Clone)]
pub struct TgBot {
    bot: Bot,
    username: String,
    id: UserId,
}

type Result<T> = core::result::Result<T, anyhow::Error>;

impl TgBot {
    pub async fn new(key: String) -> Self {
        let bot = Bot::new(key);

        let this_bot = bot.get_me().await.expect("Failed to get self bot");
        let username = this_bot
            .username
            .clone()
            .expect("Me returned but had no name");

        Self {
            bot,
            username,
            id: this_bot.id,
        }
    }

    pub fn get_inner(&self) -> Bot {
        self.bot.clone()
    }

    pub fn get_inner_ref(&self) -> &Bot {
        &self.bot
    }

    /// Send raw message
    pub async fn send_raw(&self, chat_id: ChatId, text: &str) -> Result<MessageId> {
        let msg = self
            .bot
            .send_message(chat_id, text)
            .await
            .context("Failed to send message")?;
        Ok(msg.id)
    }

    /// Send markdown message
    pub async fn send_md(&self, chat_id: ChatId, text: &str) -> Result<MessageId> {
        let text = escape_md(text);
        let msg = self
            .bot
            .send_message(chat_id, &text)
            .parse_mode(ParseMode::MarkdownV2)
            .await
            .context("Failed to send markdown message")?;
        Ok(msg.id)
    }

    /// Send confirmation message with inline keyboard
    pub async fn send_confirm(
        &self,
        chat_id: ChatId,
        text: &str,
        callback_data: &str,
    ) -> Result<MessageId> {
        let keyboard = InlineKeyboardMarkup::new(vec![vec![
            InlineKeyboardButton::callback("✅ Confirmar", callback_data.to_string()),
            InlineKeyboardButton::callback("❌ Cancelar", "{}".to_string()),
        ]]);

        let msg = self
            .bot
            .send_message(chat_id, text)
            .reply_markup(keyboard)
            .await
            .context("Failed to send confirmation message")?;
        Ok(msg.id)
    }

    /// Replace a message with confirmation
    pub async fn replace_confirm(
        &self,
        chat_id: ChatId,
        message_id: MessageId,
        text: &str,
    ) -> Result<()> {
        self.bot
            .edit_message_text(chat_id, message_id, text)
            .await
            .context("Failed to replace message")?;
        Ok(())
    }

    /// Send custom list with inline keyboard
    pub async fn send_custom_list(
        &self,
        chat_id: ChatId,
        text: String,
        items: Vec<String>,
    ) -> Result<MessageId> {
        let mut keyboard_rows: Vec<Vec<InlineKeyboardButton>> = Vec::new();

        for item in &items {
            keyboard_rows.push(vec![InlineKeyboardButton::callback(
                item.clone(),
                "Delete".to_string(),
            )]);
        }

        // Add cancel button
        keyboard_rows.push(vec![InlineKeyboardButton::callback(
            "❌ Cancelar",
            "{}".to_string(),
        )]);

        let keyboard = InlineKeyboardMarkup::new(keyboard_rows);

        let msg = self
            .bot
            .send_message(chat_id, &text)
            .reply_markup(keyboard)
            .await
            .context("Failed to send list message")?;
        Ok(msg.id)
    }

    /// Replace message with a new list
    pub async fn replace_list(
        &self,
        chat_id: ChatId,
        message_id: MessageId,
        text: &str,
    ) -> Result<()> {
        self.bot
            .edit_message_text(chat_id, message_id, text)
            .await
            .context("Failed to replace list message")?;
        Ok(())
    }

    /// Download file from Telegram
    pub async fn get_file(&self, file_id: FileId, path: &Path) -> Result<()> {
        let file = self.bot.get_file(file_id).await?;

        if let Some(parent_dir) = path.parent() {
            tokio::fs::create_dir_all(parent_dir).await?;
        }

        self.bot
            .download_file(&file.path, &mut tokio::fs::File::create(&path).await?)
            .await?;

        Ok(())
    }

    /// Get bot user info
    pub fn get_bot_username(&self) -> &str {
        self.username.as_str()
    }

    pub fn get_bot_id(&self) -> UserId {
        self.id
    }

    pub async fn send_document<P: Into<PathBuf>>(&self, chat_id: ChatId, file: P) -> Result<()> {
        let path: PathBuf = file.into();
        self.bot
            .send_document(chat_id, InputFile::file(path))
            .await?;
        Ok(())
    }

    /// Extract command text, removing @botname in groups, if it says command@bot
    /// returns true on the second return
    pub fn extract_command_text<'a>(&self, text: &'a str) -> (&'a str, bool) {
        let bot_handle = self.get_bot_username();

        if let Some(idx) = text.find(&format!("@{}", bot_handle)) {
            let after_handle = &text[idx + bot_handle.len() + 1..];
            let before_handle = &text[..idx + bot_handle.len()];
            (after_handle.trim(), before_handle == "command")
        } else {
            (text, false)
        }
    }

    /// Handle new members joining a group
    /// Bot id y bot username
    async fn handle_new_members(
        &self,
        chat_id: ChatId,
        members: &[User],
    ) -> Result<Option<Message>> {
        let bot_id = self.get_bot_id();

        // Check if bot is in the new members
        let bot_joining = members.iter().any(|m| m.id == bot_id);

        if !bot_joining {
            let bot_handle = self.get_bot_username();
            let welcome = format!(
                "Hola! Estoy aquí para ayudarte, _woof_! Si quieres que conteste a un mensaje escribe '/info@{} tu pregunta...'",
                bot_handle
            );
            self.send_md(chat_id, &welcome).await?;
        }

        Ok(None)
    }

    pub async fn process_msg<A: AiProvider + Sync>(
        &self,
        msg: teloxide::types::Message,
        ai: &AiService<A>,
    ) -> Result<Option<Message>> {
        let chat_id = msg.chat.id;
        let username = msg
            .from
            .as_ref()
            .map(|u| u.username.clone())
            .flatten()
            .unwrap_or_default();

        // Check for new chat members (group join)
        if let Some(new_members) = msg.new_chat_members() {
            return self.handle_new_members(chat_id, new_members).await;
        }

        // Extract message text
        let text = if let Some(voice) = msg.voice() {
            // Handle voice message
            return self
                .handle_voice_message(chat_id, voice, &username, ai)
                .await;
        } else if let Some(text) = msg.text() {
            text.to_string()
        } else {
            return Ok(None);
        };

        // Handle group commands with @botname
        let (text, is_group_command) = self.extract_command_text(&text);

        if text.is_empty() && is_group_command {
            let bot_handle = self.get_bot_username();
            let response = format!(
                "Si quieres hablar conmigo escribe '/info@{} lo que quieres decir'",
                bot_handle
            );
            self.send_raw(chat_id, &response).await?;
            return Ok(None);
        }

        // Process as regular message
        Ok(Some(Message {
            chat_id,
            text: text.to_string(),
            username,
        }))
    }

    /// Handle voice message - download and transcribe
    async fn handle_voice_message<A: AiProvider + Sync>(
        &self,
        chat_id: ChatId,
        voice: &teloxide::types::Voice,
        username: &str,
        ai: &AiService<A>,
    ) -> Result<Option<Message>> {
        // Check duration as a proxy for file size (max ~5 min = ~5MB at typical compression)
        if voice.duration.seconds() > 300 {
            self.send_raw(chat_id, "El mensaje de voz es demasiado grande")
                .await?;
            return Ok(None);
        }
        let voice_path = Path::new("voice_note.ogg");

        // Download voice file
        self.get_file(voice.file.id.clone(), voice_path).await?;

        // Transcribe audio
        match ai.transcribe_audio(voice_path).await {
            Ok(transcription) => Ok(Some(Message {
                chat_id,
                text: transcription,
                username: username.to_string(),
            })),
            Err(e) => {
                tracing::warn!("Failed to transcribe voice: {}", e);
                self.send_raw(chat_id, "Problemas al transcribir").await?;
                Ok(None)
            }
        }
    }

    /// Handle callback query from inline keyboard
    pub async fn handle_callback(
        &self,
        query: CallbackQuery,
        check_allowed: impl AsyncFnOnce(String) -> bool + Send,
    ) -> Result<Option<(String, Option<MaybeInaccessibleMessage>)>> {
        let username = query.from.username.clone().unwrap_or_default();

        // Acknowledge the callback
        let id = query.id.clone();
        self.bot.answer_callback_query(id).text("").await?;

        if !check_allowed(username).await {
            return Ok(None);
        }

        let data = query.data.clone().unwrap_or_default();

        match data.as_str() {
            "{}" => {
                // Cancel operation
                if let Some(msg) = query.message {
                    self.replace_confirm(msg.chat().id, msg.id(), "Operación cancelada")
                        .await?;
                }
                Ok(None)
            }
            "Delete" => {
                // Delete operation - remove from list
                if let Some(msg) = query.message {
                    // Update keyboard to remove items
                    self.replace_confirm(msg.chat().id, msg.id(), "Elemento eliminado")
                        .await?;
                }
                Ok(None)
            }
            "Cancel" => {
                if let Some(msg) = query.message {
                    self.replace_confirm(msg.chat().id, msg.id(), "Lista terminada")
                        .await?;
                }
                Ok(None)
            }
            _ => {
                // Check for pending operation
                Ok(Some((data, query.message)))
            }
        }
    }
}

pub trait SimpleBotDispatch<A: AiProvider + Sync + Send> {
    fn process_message(&self, msg: Message)
    -> impl std::future::Future<Output = Result<()>> + Send;

    fn handle_callback(
        &self,
        _data: &str,
        _query_msg: Option<MaybeInaccessibleMessage>,
    ) -> impl std::future::Future<Output = Result<()>> + Send {
        tracing::info!("Received a callback, not yet implemented");
        std::future::ready(Ok(()))
    }

    fn is_allowed(&self, username: &str) -> impl std::future::Future<Output = Result<bool>> + Send;

    fn get_ai_service(&self) -> &AiService<A>;

    fn get_bot(&self) -> &TgBot;
}

/// Start the bot dispatcher
pub async fn start_bot<
    D: SimpleBotDispatch<A> + Sync + Send + 'static,
    A: AiProvider + Sync + Send + Clone + 'static,
>(
    context: Arc<RwLock<D>>,
) -> Result<()> {
    use teloxide::dispatching::Dispatcher;

    let bot = {
        let ctx = context.read().await;
        ctx.get_bot().clone()
    };

    let message_handler = Update::filter_message().endpoint({
        let ctx = Arc::clone(&context);
        let bot = bot.clone();
        let ai = ctx.read().await.get_ai_service().clone();

        move |_bot: Bot, msg: teloxide::prelude::Message| {
            let ctx = Arc::clone(&ctx);
            let bot = bot.clone();
            let ai = ai.clone();

            async move {
                match bot.process_msg(msg, &ai).await {
                    Ok(Some(app_msg)) => {
                        if let Err(e) = ctx.read().await.process_message(app_msg).await {
                            error!("Error handling message: {}", e);
                        }
                    }
                    Ok(None) => {}
                    Err(e) => {
                        error!("Error handling message: {}", e);
                    }
                }
                Ok::<(), anyhow::Error>(())
            }
        }
    });

    let callback_handler = Update::filter_callback_query().endpoint({
        let ctx = Arc::clone(&context);
        move |_bot: Bot, query: CallbackQuery| {
            let ctx = Arc::clone(&ctx);
            async move {
                // Clone ctx for the permission callback
                let ctx_for_cb = Arc::clone(&ctx);
                let is_allowed = move |username: String| async move {
                    ctx_for_cb
                        .read()
                        .await
                        .is_allowed(&username)
                        .await
                        .unwrap_or(false)
                };
                let bot = ctx.read().await.get_bot().clone();
                match bot.handle_callback(query, is_allowed).await {
                    Ok(Some((c, query_msg))) => {
                        if let Err(e) = ctx.read().await.handle_callback(&c, query_msg).await {
                            error!("Error handling callback: {}", e)
                        }
                    }
                    Ok(None) => {}
                    Err(e) => error!("Error handling callback: {}", e),
                }
                Ok::<(), anyhow::Error>(())
            }
        }
    });

    let handler = message_handler.branch(callback_handler);

    Dispatcher::builder(bot.get_inner(), handler)
        .build()
        .dispatch()
        .await;

    Ok(())
}
