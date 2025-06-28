use anyhow::Result;
use colored::Colorize;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    terminal,
};
use openai_api_rs::v1::api::OpenAIClient;
use openai_api_rs::v1::chat_completion::{
    ChatCompletionMessage, ChatCompletionRequest, ChatCompletionResponse, Content, MessageRole,
};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::mem;
use tui_textarea::TextArea;

pub enum Message {
    USER(ChatCompletionMessage),
    SYSTEM(ChatCompletionMessage),
    ASSISTANT(ChatCompletionMessage),
}

pub fn extract_message_text(msg: &ChatCompletionMessage) -> &str {
    match msg.content {
        Content::Text(ref text) => text,
        Content::ImageUrl(_) => "An image",
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
            match *msg {
                Message::USER(ref user_msg) => {
                    let text = format!("You: {}\n", extract_message_text(user_msg))
                        .red()
                        .to_string();
                    history_text.push_str(&text);
                }
                Message::ASSISTANT(ref system_msg) => {
                    let text = format!("Bot: {}\n", extract_message_text(system_msg))
                        .green()
                        .to_string();
                    history_text.push_str(&text);
                }
                _ => (),
            }
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
    // terminal.clear()?;

    let mut input = String::new();
    let mut history: Vec<String> = Vec::new();
    let mut msg_history: Vec<Message> = Vec::new();
    let mut input_area = TextArea::default();
    input_area.set_block(Block::default().borders(Borders::ALL).title("Input"));

    loop {
        update_terminal(&mut terminal, &msg_history, &input_area);
        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    // let prompt = input.drain(..).collect::<String>();
                    let prompt = mem::take(&mut input);
                    history.push(format!("You: {}", prompt));
                    let chat_msg = ChatCompletionMessage {
                        role: MessageRole::user,
                        content: Content::Text(prompt.clone()),
                        name: None,
                        tool_calls: None,
                        tool_call_id: None,
                    };
                    msg_history.push(Message::USER(chat_msg.clone()));

                    update_terminal(&mut terminal, &mut msg_history, &input_area)?;
                    // let req = ChatCompletionRequest::new(model.to_string(), vec![chat_msg.clone()]);
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

                        // history.push(format!("Bot: {}", reply));
                    }
                    let block = input_area.block().unwrap_or(&Block::default()).clone();
                    input_area = TextArea::default();
                    input_area.set_block(block);
                }
                KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    //todo: clear the  history
                    msg_history.clear();
                }
                KeyCode::Char(c) => {
                    input_area.insert_char(c);
                }
                _ => {}
            }
        }
    }

    ratatui::restore();
    Ok(())
}
