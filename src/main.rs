use clap::{Args, Parser};

use serde::Serialize;
use std::env;
mod console;
use anyhow::Result;

#[derive(Args, Debug, Serialize, Default)]
#[group(required = false, multiple = false)]
struct Mode {
    /// Use fast mode (default)
    #[arg(long, short, group = "mode", help = "Use fast mode (default)")]
    fast: bool,

    /// Use normal mode
    #[arg(long, short, group = "mode", help = "Use normal model")]
    normal: bool,

    /// Use code mode
    #[arg(long, short, group = "mode", help = "Use a coding model")]
    code: bool,

    /// Use code mode
    #[arg(long, short, group = "mode", help = "Use a thinking model")]
    think: bool,
}
#[derive(Serialize, Parser, Default)]
struct CLIInput {
    #[arg(short, long)]
    authorization: Option<String>,
    #[arg()]
    prompt: String,
    #[arg(short, long, help = "Provide a custom system prompt")]
    system_prompt: Option<String>,
    #[arg(short = 'C', long, help = "Launch interactive console UI")]
    Console: bool,
    #[arg(short, long, help = "Print response headers (debug)")]
    debug: bool,
    #[command(flatten, next_help_heading = "Mode Selection (select one)")]
    mode: Mode,
}

use openai_api_rs::v1::api::OpenAIClient;
use openai_api_rs::v1::chat_completion::{self, ChatCompletionRequest};
use openai_api_rs::v1::common::GPT4_O_MINI;

const fast_model: &str = GPT4_O_MINI;
const normal_model: &str = "openai/gpt-4.1";

const code_model: &str = "anthropic/claude-3.7-sonnet";
const thinking_model: &str = "google/gemini-2.5-pro-preview";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = env::var("OPENROUTER_API_KEY").unwrap().to_string();
    let mut client = OpenAIClient::builder()
        .with_endpoint("https://openrouter.ai/api/v1")
        .with_api_key(api_key)
        .build()?;

    let input = CLIInput::parse();
    let mode = input.mode;
    let model = if mode.fast {
        fast_model
    } else if mode.normal {
        normal_model
    } else if mode.code {
        code_model
    } else if mode.think {
        thinking_model
    } else {
        fast_model // Default to fast model if no mode is selected
    };

    if input.Console {
        console::run_console(&mut client, model).await?;
        return Ok(());
    }

    let req = ChatCompletionRequest::new(
        model.to_string(),
        vec![chat_completion::ChatCompletionMessage {
            role: chat_completion::MessageRole::user,
            content: chat_completion::Content::Text(input.prompt),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }],
    );

    let result = client.chat_completion(req).await?;
    let message = result.choices[0]
        .message
        .content
        .clone()
        .unwrap_or_else(|| "No results".to_string());

    println!("{message}");

    if input.debug {
        for (key, value) in client.response_headers.unwrap().iter() {
            println!("{}: {:?}", key, value);
        }
    }

    Ok(())
}

#[test]
fn print_env() {
    for (key, var) in env::vars() {
        println!("{key}: {var}");
    }
}
