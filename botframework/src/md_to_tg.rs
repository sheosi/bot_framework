//! Convert traditional Markdown to Telegram MarkdownV2.
//!
//! Handles bold, italic, strikethrough, inline code and links, escaping all
//! other characters that are special in Telegram's MarkdownV2 parse mode.

use pulldown_cmark::{Event, Options, Parser, Tag, TagEnd};

const TG_SPECIAL: &[char] = &[
    '_', '*', '[', ']', '(', ')', '~', '`', '>', '#', '+', '-', '=', '|', '{', '}', '.', '!',
];

fn escape_tg(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        if TG_SPECIAL.contains(&ch) {
            out.push('\\');
        }
        out.push(ch);
    }
    out
}

fn wrap_code(code: &str) -> String {
    let mut max_run = 0;
    let mut current = 0;
    for ch in code.chars() {
        if ch == '`' {
            current += 1;
            max_run = max_run.max(current);
        } else {
            current = 0;
        }
    }
    let delim_len = max_run + 1;
    let delim = "`".repeat(delim_len);
    format!("{delim} {code} {delim}")
}

pub fn markdown_to_telegram(input: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(input, options);
    let mut out = String::new();
    let mut link_stack: Vec<String> = Vec::new();

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Strong => out.push('*'),
                Tag::Emphasis => out.push('_'),
                Tag::Strikethrough => out.push('~'),
                Tag::Link { dest_url, .. } => {
                    out.push('[');
                    link_stack.push(dest_url.to_string());
                }
                Tag::Paragraph
                | Tag::Heading { .. }
                | Tag::BlockQuote(_)
                | Tag::CodeBlock(_)
                | Tag::List(_)
                | Tag::Item
                | Tag::FootnoteDefinition(_)
                | Tag::Table(_)
                | Tag::TableHead
                | Tag::TableRow
                | Tag::TableCell
                | Tag::HtmlBlock
                | Tag::DefinitionList
                | Tag::DefinitionListTitle
                | Tag::DefinitionListDefinition
                | Tag::Image { .. }
                | Tag::MetadataBlock(_) => {}
            },
            Event::End(tag_end) => match tag_end {
                TagEnd::Strong => out.push('*'),
                TagEnd::Emphasis => out.push('_'),
                TagEnd::Strikethrough => out.push('~'),
                TagEnd::Link => {
                    if let Some(url) = link_stack.pop() {
                        out.push_str("](");
                        out.push_str(&escape_tg(&url));
                        out.push(')');
                    }
                }
                _ => {}
            },
            Event::Text(text) => out.push_str(&escape_tg(&text)),
            Event::Code(code) => out.push_str(&wrap_code(&code)),
            Event::Html(html) => out.push_str(&escape_tg(&html)),
            Event::SoftBreak | Event::HardBreak => out.push('\n'),
            Event::Rule => out.push_str("---"),
            _ => {}
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bold_and_italic() {
        assert_eq!(
            markdown_to_telegram("**bold** and *italic*"),
            "*bold* and _italic_"
        );
    }

    #[test]
    fn escape_plain_text_specials() {
        assert_eq!(
            markdown_to_telegram("Price: $5.00 + tax = $5.50!"),
            "Price: $5\\.00 \\+ tax \\= $5\\.50\\!"
        );
    }

    #[test]
    fn link() {
        assert_eq!(
            markdown_to_telegram("[Scryfall](https://scryfall.com)"),
            r"[Scryfall](https://scryfall\.com)"
        );
    }

    #[test]
    fn inline_code() {
        assert_eq!(
            markdown_to_telegram("use `rm -rf` carefully"),
            "use ` rm -rf ` carefully"
        );
    }

    #[test]
    fn code_with_backticks() {
        assert_eq!(
            markdown_to_telegram("``code `inside` code``"),
            "`` code `inside` code ``"
        );
    }

    #[test]
    fn strikethrough() {
        assert_eq!(markdown_to_telegram("~~deleted~~"), "~deleted~");
    }
}
