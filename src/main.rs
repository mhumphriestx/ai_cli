use anyhow::Result;
use clap::Parser;
use reqwest::{Client, header::HeaderMap, header::HeaderValue};
use serde::Serialize;
use serde_json::{self, Value};
use std::collections::HashMap;
use std::env;
use std::thread::sleep;

use std::time::Duration;

const LLM_ENDPOINT: &str =
    "https://api.replicate.com/v1/models/deepseek-ai/deepseek-r1/predictions";

// const LLM_ENDPOINT: &str = "https://api.replicate.com/v1/models/openai/gpt-4o-mini/predictions";

#[derive(Serialize, Parser, Default)]
struct CLIInput {
    // your fields here
    #[arg(short, long)]
    Authorization: Option<String>,
    #[arg()]
    prompt: String,
    #[arg(short, long)]
    system_prompt: Option<String>,
}

#[derive(Serialize, Default)]
struct RequestHeader {
    Authorization: String,
}

#[derive(Serialize)]
struct RequestBody {
    input: Input,
}

#[derive(Serialize)]
struct Input {
    prompt: String,
    system_prompt: String,
}

use openai_api_rs::v1::api::OpenAIClient;
use openai_api_rs::v1::chat_completion::{self, ChatCompletionRequest};
use openai_api_rs::v1::common::GPT4_O_MINI;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = env::var("OPENROUTER_API_KEY").unwrap().to_string();
    let mut client = OpenAIClient::builder()
        .with_endpoint("https://openrouter.ai/api/v1")
        .with_api_key(api_key)
        .build()?;

    let input = CLIInput::parse();

    let req = ChatCompletionRequest::new(
        GPT4_O_MINI.to_string(),
        vec![chat_completion::ChatCompletionMessage {
            role: chat_completion::MessageRole::user,
            content: chat_completion::Content::Text(input.prompt),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }],
    );

    let result = client.chat_completion(req).await?;
    // println!("Content: {:?}", result.choices[0].message.content);
    let message = result.choices[0]
        .message
        .content
        .clone()
        .unwrap_or_else(|| "Not result".to_string());

    println!("{message}");

    for (key, value) in client.response_headers.unwrap().iter() {
        println!("{}: {:?}", key, value);
    }

    Ok(())
}

#[test]
fn print_env() {
    for (key, var) in env::vars() {
        println!("{key}: {var}");
    }
}
