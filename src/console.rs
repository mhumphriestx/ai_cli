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

enum History {
    USER(ChatCompletionMessage),
    SYSTEM(ChatCompletionMessage),
}

pub fn update_terminal(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    input: &mut String,
    history: &mut Vec<String>,
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

        let history_text = history.join("\n");
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
    let mut msg_history: Vec<History> = Vec::new();

    loop {
        update_terminal(&mut terminal, &mut input, &mut history);
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

                    update_terminal(&mut terminal, &mut input, &mut history)?;
                    let req = ChatCompletionRequest::new(model.to_string(), vec![chat_msg.clone()]);
                    if let Ok(res) = client.chat_completion(req.clone()).await {
                        msg_history.push(History::USER(chat_msg));
                        let system_msg = ChatCompletionMessage {
                            role: MessageRole::system,
                            content: Content::Text("".to_string()),
                            name: None,
                            tool_calls: None,
                            tool_call_id: None,
                        };
                        msg_history.push(History::SYSTEM(system_msg));
                        let reply = res.choices[0].message.content.clone().unwrap_or_default();
                        history.push(format!("Bot: {}", reply));
                    }
                }
                KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    history.clear();
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
