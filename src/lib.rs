use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use anyhow::Context;
use chrono::{DateTime, Utc};
use teloxide::{
    Bot,
    net::Download,
    payloads::SendMessageSetters,
    requests::Requester,
    types::{
        ChatId, FileId, InlineKeyboardButton, InlineKeyboardMarkup, InputFile, MessageId,
        ParseMode, User,
    },
};

pub mod ai;
pub mod infisical;

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

/// Tool trait for AI function calling
#[enum_dispatch::enum_dispatch]
pub trait Tool<T>: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn parameters(&self) -> Properties;
    async fn tool_call(
        &self,
        ctx: &mut T,
        chat_id: ChatId,
        arguments: &str,
    ) -> anyhow::Result<ToolCallAction>;
    async fn handle_callback(
        &self,
        _ctx: &mut T,
        _callback_data: &str,
        _delayed_action: DelayedAction,
    ) -> anyhow::Result<String> {
        Ok("Done".to_string())
    }
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

#[derive(Clone)]
pub struct TgBot {
    bot: Bot,
}

type Result<T> = core::result::Result<T, anyhow::Error>;

impl TgBot {
    pub fn new(key: String) -> Self {
        Self { bot: Bot::new(key) }
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
    pub async fn get_bot_user(&self) -> Result<User> {
        let me = self.bot.get_me().await?;
        Ok(me.user)
    }

    pub async fn send_document<P: Into<PathBuf>>(&self, chat_id: ChatId, file: P) -> Result<()> {
        let path: PathBuf = file.into();
        self.bot
            .send_document(chat_id, InputFile::file(path))
            .await?;
        Ok(())
    }
}
