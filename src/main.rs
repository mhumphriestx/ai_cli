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

enum ProcessStatus {
    Starting,
    Processing,
    Succeeded,
    Failed,
    Canceled,
}

fn process_status(status: &str) -> ProcessStatus {
    match status {
        "starting" => ProcessStatus::Starting,
        "processing" => ProcessStatus::Processing,
        "succeeded" => ProcessStatus::Succeeded,
        "failed" => ProcessStatus::Failed,
        "canceled" => ProcessStatus::Canceled,
        _ => ProcessStatus::Failed, // Default case
    }
}

async fn get_status(client: &Client, get_url: &str) -> Result<ProcessStatus> {
    let resp = client.get(get_url).send().await?;
    let status_text = resp.text().await?;
    let status_json: Value = serde_json::from_str(&status_text)?;

    let status = status_json
        .get("status")
        .and_then(|s| s.as_str())
        .unwrap_or("unknown");
    Ok(process_status(status))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = CLIInput::parse();
    let mut auth_header: String = String::new();

    if let Some(auth) = input.Authorization {
        auth_header = format!("{auth}");
    } else if let Ok(API_KEY) = env::var(&"REPLICATE_API_TOKEN") {
        auth_header = API_KEY;
    } else {
        eprintln!("Authorization header is required.");
        return Ok(());
    }

    // let mut client = Client::new();
    let mut default_headers = HeaderMap::new();
    default_headers.insert(
        "Authorization",
        HeaderValue::from_str(&format!("Bearer {auth_header}")).unwrap(),
    );
    default_headers.insert("Content-Type", HeaderValue::from_static("application/json"));
    default_headers.insert("Prefer", HeaderValue::from_static("wait"));

    let mut client = Client::builder().default_headers(default_headers).build()?;

    let llm_input = Input {
        prompt: input.prompt,
        system_prompt: input
            .system_prompt
            .unwrap_or_else(|| "You are a helpful assistant.".to_string()),
    };

    let req_body = RequestBody { input: llm_input };

    let resp = client.post(LLM_ENDPOINT).json(&req_body).send().await?;

    let json: serde_json::Value = resp.json().await?;

    let get_url = json
        .get("urls")
        .and_then(|urls| urls.get("get"))
        .and_then(|v| v.as_str())
        .unwrap_or_default();

    println!("URL: {}", get_url);
    loop {
        match get_status(&client, &get_url).await.unwrap() {
            ProcessStatus::Starting | ProcessStatus::Processing => (),
            ProcessStatus::Succeeded => {
                println!("Process succeeded.");
                break;
            }
            _ => break,
        }

        sleep(Duration::from_millis(100));
    }
    let results = client.get(get_url).send().await?;
    if let Ok(res_text) = results.text().await {
        // println!("Response Text: {res_text}");

        let output: serde_json::Value = serde_json::from_str(&res_text)
            .unwrap_or_else(|_| serde_json::json!({"error": "Failed to parse response"}));
        // println!("Output: {:#?}", &output);
        let res = &output["output"];
        println!("The LLM output\n");
        match &res {
            serde_json::Value::String(s) => println!("{}", s),
            serde_json::Value::Array(arr) => {
                for item in arr {
                    if let Some(s) = item.as_str() {
                        print!("{}", s);
                    } else {
                        print!("{:?}", item);
                    }
                }
            }
            _ => println!("Unexpected response format: {:?}", res),
        }
    } else {
        println!("Failed to get response text.");
    }

    Ok(())
}

#[test]
fn print_env() {
    for (key, var) in env::vars() {
        println!("{key}: {var}");
    }
}
