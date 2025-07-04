use anyhow::Result;
use colored::Colorize;
use openai_api_rs::v1::api::OpenAIClient;
use openai_api_rs::v1::chat_completion::{
    ChatCompletionMessage, ChatCompletionRequest, ChatCompletionResponse, Content, MessageRole,
};
use ratatui::crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal,
};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::mem;
use std::{fmt, fmt::Display};
use tui_textarea::{Input, Key, TextArea};

pub enum Message {
    USER(ChatCompletionMessage),
    SYSTEM(ChatCompletionMessage),
    ASSISTANT(ChatCompletionMessage),
}

impl Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Message::USER(msg) => format!("You: {}", Self::extract_message_text(msg)),
            Message::ASSISTANT(msg) => format!("Bot: {}", Self::extract_message_text(msg)),
            Message::SYSTEM(msg) => format!("System: {}", Self::extract_message_text(msg)),
        };

        write!(f, "{}", text)
    }
}

impl Message {
    fn extract_message_text(msg: &ChatCompletionMessage) -> &str {
        match msg.content {
            Content::Text(ref text) => text,
            Content::ImageUrl(_) => "An image",
        }
    }
}

pub fn update_terminal(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    history: &Vec<Message>,
    input_area: &TextArea,
) -> Result<()> {
    terminal.draw(|f| {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Min(1),
                    Constraint::Length(3),
                    Constraint::Length(1),
                ]
                .as_ref(),
            )
            .split(f.area());

        let mut history_text = String::new();
        for msg in history {
            let text = match msg {
                Message::USER(_) => msg.to_string().red().to_string(),
                _ => msg.to_string().green().to_string(),
            };
            history_text.push_str(&format!("{}\n", text));
        }

        let history_para = Paragraph::new(history_text)
            .block(Block::default().borders(Borders::ALL).title("History"))
            .wrap(ratatui::widgets::Wrap { trim: false });
        f.render_widget(history_para, chunks[0]);

        f.render_widget(input_area, chunks[1]);

        f.render_widget(
            Paragraph::new("Ctrl+S:send|Ctrl+Q:quit|Ctrl+N:clear history"),
            chunks[2],
        );
    })?;
    Ok(())
}

pub async fn run_console(client: &mut OpenAIClient, model: &str) -> Result<()> {
    let mut terminal = ratatui::init();
    terminal::enable_raw_mode()?;

    let mut msg_history: Vec<Message> = Vec::new();
    let mut input_area = TextArea::default();
    input_area.set_block(Block::default().borders(Borders::ALL).title("Input"));

    loop {
        update_terminal(&mut terminal, &msg_history, &input_area)?;
        let event = event::read()?;
        if let Event::Key(key) = event {
            match key.code {
                KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    let prompt = input_area.lines().join("\n");
                    let chat_msg = ChatCompletionMessage {
                        role: MessageRole::user,
                        content: Content::Text(prompt.clone()),
                        name: None,
                        tool_calls: None,
                        tool_call_id: None,
                    };
                    msg_history.push(Message::USER(chat_msg.clone()));

                    update_terminal(&mut terminal, &msg_history, &input_area)?;
                    let req = ChatCompletionRequest::new(
                        model.to_string(),
                        msg_history
                            .iter()
                            .map(|msg| match msg {
                                Message::USER(user_msg) => user_msg.clone(),
                                Message::ASSISTANT(assistant_msg) => assistant_msg.clone(),

                                Message::SYSTEM(system_msg) => system_msg.clone(),
                            })
                            .collect(),
                    );
                    if let Ok(res) = client.chat_completion(req.clone()).await {
                        let reply = res.choices[0].message.content.clone().unwrap_or_default();

                        let system_msg = ChatCompletionMessage {
                            role: MessageRole::assistant,
                            content: Content::Text(reply),
                            name: None,
                            tool_calls: None,
                            tool_call_id: None,
                        };
                        msg_history.push(Message::ASSISTANT(system_msg));
                    }
                    let block = input_area.block().unwrap_or(&Block::default()).clone();
                    input_area = TextArea::default();
                    input_area.set_block(block);
                    continue;
                }
                KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    msg_history.clear();
                    continue;
                }
                _ => {}
            }
        }

        input_area.input(event);
    }

    ratatui::restore();
    Ok(())
}
