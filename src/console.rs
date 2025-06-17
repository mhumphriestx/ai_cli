use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use openai_api_rs::v1::api::OpenAIClient;
use openai_api_rs::v1::chat_completion::{
    ChatCompletionMessage, ChatCompletionRequest, Content, MessageRole,
};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io;

pub async fn run_console(client: &mut OpenAIClient, model: &str) -> Result<()> {
    let mut stdout = io::stdout();
    crossterm::terminal::enable_raw_mode()?;
    let backend = CrosstermBackend::new(&mut stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut input = String::new();
    let mut history: Vec<String> = Vec::new();

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Length(3)].as_ref())
                .split(f.size());

            let history_text = history.join("\n");
            let history_para = Paragraph::new(history_text)
                .block(Block::default().borders(Borders::ALL).title("History"));
            f.render_widget(history_para, chunks[0]);

            let input_para = Paragraph::new(input.as_ref())
                .block(Block::default().borders(Borders::ALL).title("Input"));
            f.render_widget(input_para, chunks[1]);
            f.set_cursor(chunks[1].x + input.len() as u16 + 1, chunks[1].y + 1);
        })?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => break,
                KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    let prompt = input.drain(..).collect::<String>();
                    history.push(format!("You: {}", prompt));
                    let req = ChatCompletionRequest::new(
                        model.to_string(),
                        vec![ChatCompletionMessage {
                            role: MessageRole::user,
                            content: Content::Text(prompt),
                            name: None,
                            tool_calls: None,
                            tool_call_id: None,
                        }],
                    );
                    if let Ok(res) = client.chat_completion(req).await {
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

    crossterm::terminal::disable_raw_mode()?;
    terminal.show_cursor()?;
    Ok(())
}
