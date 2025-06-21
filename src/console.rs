use anyhow::Result;
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

pub enum Message {
    USER(ChatCompletionMessage),
    SYSTEM(ChatCompletionMessage),
}

pub fn extract_message_text(msg: &ChatCompletionMessage) -> &str {
    match msg.content {
        Content::Text(ref text) => text,
        Content::ImageUrl(_) => "An image",
    }
}

pub fn update_terminal(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    input: &mut String,
    history: &Vec<Message>,
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
            match msg {
                &Message::USER(ref user_msg) => {
                    history_text.push_str(&format!("You: {}\n", extract_message_text(user_msg)));
                }
                &Message::SYSTEM(ref system_msg) => {
                    history_text.push_str(&format!("Bot: {}\n", extract_message_text(system_msg)));
                }
            }
        }

        let history_para = Paragraph::new(history_text)
            .block(Block::default().borders(Borders::ALL).title("History"));
        f.render_widget(history_para, chunks[0]);

        let input_para = Paragraph::new(input.clone())
            .block(Block::default().borders(Borders::ALL).title("Input"))
            .wrap(Wrap { trim: false });
        f.render_widget(input_para, chunks[1]);

        f.set_cursor_position(ratatui::layout::Position::new(
            chunks[1].x + input.len() as u16 + 1,
            chunks[1].y + 1,
        ));

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

    loop {
        update_terminal(&mut terminal, &mut input, &msg_history);
        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    let prompt = input.drain(..).collect::<String>();
                    history.push(format!("You: {}", prompt));
                    let chat_msg = ChatCompletionMessage {
                        role: MessageRole::user,
                        content: Content::Text(prompt.clone()),
                        name: None,
                        tool_calls: None,
                        tool_call_id: None,
                    };
                    msg_history.push(Message::USER(chat_msg.clone()));

                    update_terminal(&mut terminal, &mut input, &mut msg_history)?;
                    let req = ChatCompletionRequest::new(model.to_string(), vec![chat_msg.clone()]);
                    if let Ok(res) = client.chat_completion(req.clone()).await {
                        let reply = res.choices[0].message.content.clone().unwrap_or_default();

                        let system_msg = ChatCompletionMessage {
                            role: MessageRole::system,
                            content: Content::Text(reply),
                            name: None,
                            tool_calls: None,
                            tool_call_id: None,
                        };
                        msg_history.push(Message::SYSTEM(system_msg));

                        // history.push(format!("Bot: {}", reply));
                    }
                }
                KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    msg_history.clear();
                }
                KeyCode::Backspace => {
                    input.pop();
                }
                KeyCode::Char(c) => {
                    input.push(c);
                }
                _ => {}
            }
        }
    }

    ratatui::restore();
    Ok(())
}
