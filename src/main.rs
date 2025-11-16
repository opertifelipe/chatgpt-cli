use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::env;
use std::error::Error;
use std::io::{self, Write};

// Basic ANSI styling helpers for nicer terminal output
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const ITALIC: &str = "\x1b[3m";
const FG_RED: &str = "\x1b[31m";
const FG_GREEN: &str = "\x1b[32m";
const FG_CYAN: &str = "\x1b[36m";
const FG_MAGENTA: &str = "\x1b[35m";
const FG_BRIGHT_BLACK: &str = "\x1b[90m";

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ResponseRequest<'a> {
    model: &'a str,
    input: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning: Option<ReasoningConfig>,
}

#[derive(Debug, Serialize)]
struct ReasoningConfig {
    effort: String,
}

#[derive(Debug, Deserialize)]
struct ResponsesApiResponse {
    #[serde(default)]
    output: Vec<ResponseOutputBlock>,
}

#[derive(Debug, Deserialize)]
struct ResponseOutputBlock {
    #[serde(default)]
    content: Vec<ResponseContentPiece>,
}

#[derive(Debug, Deserialize)]
struct ResponseContentPiece {
    #[serde(rename = "type")]
    r#type: String,
    text: Option<String>,
}

#[derive(Debug, Clone, Copy)]
struct CliConfig {
    reasoning: ReasoningLevel,
}

#[derive(Debug, Clone, Copy)]
enum ReasoningLevel {
    None,
    Low,
    Medium,
    High,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = parse_cli_args().unwrap_or_else(|err| {
        eprintln!("{FG_RED}{BOLD}✗ Invalid arguments{RESET}: {}", err);
        print_usage();
        std::process::exit(2);
    });

    let api_key = match env::var("OPENAI_API_KEY") {
        Ok(val) if !val.trim().is_empty() => val,
        _ => {
            eprintln!("{FG_RED}{BOLD}✗ ERROR{RESET}: OPENAI_API_KEY is not set or is empty.",);
            eprintln!(
                "{DIM}Hint:{RESET} export OPENAI_API_KEY=\"your_api_key_here\" and run again.",
            );
            std::process::exit(1);
        }
    };

    let client = reqwest::Client::new();

    println!("{BOLD}{FG_CYAN}╔══════════════════════════════════════╗{RESET}");
    println!(
        "{BOLD}{FG_CYAN}║ ChatGPT CLI {RESET}{DIM}(gpt-5.1){RESET}{BOLD}{FG_CYAN}                ║{RESET}"
    );
    println!("{BOLD}{FG_CYAN}╚══════════════════════════════════════╝{RESET}");
    println!("{DIM}Hint:{RESET} type a message and press Enter.");
    println!("{DIM}Commands:{RESET}");
    println!("  {FG_CYAN}/new{RESET} start a new conversation, {FG_CYAN}/exit{RESET} quit.");
    println!(
        "  {FG_CYAN}/reasoning <none|low|medium|high>{RESET} set reasoning effort (default none)."
    );
    println!();

    let mut conversation: Vec<ChatMessage> = Vec::new();
    let mut current_reasoning = cli.reasoning;

    loop {
        print!("{BOLD}{FG_GREEN}You{RESET}{DIM}:{RESET} ");
        io::stdout().flush()?;

        let mut input = String::new();
        let bytes_read = io::stdin().read_line(&mut input)?;
        if bytes_read == 0 {
            println!("\n{DIM}Session closed.{RESET}");
            break;
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        if input.eq_ignore_ascii_case("/exit") {
            println!("{DIM}See you next time!{RESET}");
            break;
        }

        if input.eq_ignore_ascii_case("/new") {
            conversation.clear();
            println!("{DIM}────────────────────────────────────────────{RESET}");
            println!("{BOLD}{FG_CYAN}Started a new conversation.{RESET}");
            println!("{DIM}────────────────────────────────────────────{RESET}");
            continue;
        }

        if let Some(rest) = input.strip_prefix("/reasoning") {
            let desired = rest.trim();
            if desired.is_empty() {
                println!(
                    "{DIM}Current reasoning:{RESET} {}",
                    current_reasoning.as_str()
                );
            } else {
                match ReasoningLevel::from_str(desired) {
                    Ok(level) => {
                        current_reasoning = level;
                        println!(
                            "{DIM}Reasoning set to{RESET} {}",
                            current_reasoning.as_str()
                        );
                    }
                    Err(err) => {
                        eprintln!("{FG_RED}{BOLD}✗ Invalid reasoning value{RESET}: {}", err)
                    }
                }
            }
            continue;
        }

        conversation.push(ChatMessage {
            role: "user".to_string(),
            content: input.to_string(),
        });

        match send_to_openai(&client, &api_key, &conversation, current_reasoning).await {
            Ok(reply) => {
                println!();
                println!("{BOLD}{FG_MAGENTA}Assistant{RESET}{DIM} ▸{RESET}");
                println!("{DIM}────────────────────────────────────────────{RESET}");
                render_markdown(&reply);
                println!("{DIM}────────────────────────────────────────────{RESET}");
                conversation.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: reply,
                });
            }
            Err(err) => {
                eprintln!(
                    "{FG_RED}{BOLD}✗ Error{RESET} while communicating with OpenAI: {}",
                    err
                );
                break;
            }
        }
    }

    Ok(())
}

fn render_markdown(markdown: &str) {
    let mut in_code_block = false;

    for line in markdown.lines() {
        let trimmed = line.trim();

        // toggle code block on ``` fences
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            if in_code_block {
                println!("{FG_BRIGHT_BLACK}```{RESET}");
            } else {
                println!("```{RESET}");
            }
            continue;
        }

        if in_code_block {
            // gray, indented code lines
            println!("{FG_BRIGHT_BLACK}    {}{RESET}", line);
            continue;
        }

        // headings
        if let Some(rest) = trimmed.strip_prefix("# ") {
            println!("{BOLD}{FG_CYAN}{}{RESET}", rest.to_uppercase());
        } else if let Some(rest) = trimmed.strip_prefix("## ") {
            println!("{BOLD}{FG_CYAN}{}{RESET}", rest);
        } else if let Some(rest) = trimmed.strip_prefix("### ") {
            println!("{BOLD}{FG_CYAN}{}{RESET}", rest);
        } else if let Some(rest) = trimmed.strip_prefix("- ") {
            println!("  {FG_CYAN}•{RESET} {}", apply_inline_styles(rest));
        } else if let Some(rest) = trimmed.strip_prefix("* ") {
            println!("  {FG_CYAN}•{RESET} {}", apply_inline_styles(rest));
        } else if trimmed.is_empty() {
            println!();
        } else {
            println!("{}", apply_inline_styles(trimmed));
        }
    }
}

// Very small inline markdown handling: **bold** and *italic*
fn apply_inline_styles(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars().peekable();
    let mut bold_on = false;
    let mut italic_on = false;

    while let Some(c) = chars.next() {
        if c == '*' {
            if let Some('*') = chars.peek() {
                // consume the second '*'
                chars.next();
                bold_on = !bold_on;
                refresh_inline_styles(&mut result, bold_on, italic_on);
                continue;
            } else {
                italic_on = !italic_on;
                refresh_inline_styles(&mut result, bold_on, italic_on);
                continue;
            }
        }
        if c == '_' {
            italic_on = !italic_on;
            refresh_inline_styles(&mut result, bold_on, italic_on);
            continue;
        }
        result.push(c);
    }

    if bold_on || italic_on {
        // close any unbalanced bold
        result.push_str(RESET);
    }

    result
}

fn refresh_inline_styles(result: &mut String, bold_on: bool, italic_on: bool) {
    result.push_str(RESET);
    if bold_on {
        result.push_str(BOLD);
    }
    if italic_on {
        result.push_str(ITALIC);
    }
}

async fn send_to_openai(
    client: &reqwest::Client,
    api_key: &str,
    messages: &[ChatMessage],
    reasoning: ReasoningLevel,
) -> Result<String, Box<dyn Error>> {
    let request_body = ResponseRequest {
        model: "gpt-5.1",
        input: messages.to_vec(),
        reasoning: reasoning.to_api_value().map(|effort| ReasoningConfig {
            effort: effort.to_string(),
        }),
    };

    let response = client
        .post("https://api.openai.com/v1/responses")
        .bearer_auth(api_key)
        .json(&request_body)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        if status == StatusCode::UNAUTHORIZED {
            return Err("Unauthorized: check your OPENAI_API_KEY".into());
        }

        return Err(format!("API error {}: {}", status, body).into());
    }

    let parsed: ResponsesApiResponse = response.json().await?;
    let reply =
        extract_text_from_response(&parsed).ok_or("No textual content returned by the API")?;

    Ok(reply.trim().to_string())
}

fn extract_text_from_response(response: &ResponsesApiResponse) -> Option<String> {
    let mut collected = String::new();

    for block in &response.output {
        for piece in &block.content {
            if (piece.r#type == "output_text" || piece.r#type == "text") && piece.text.is_some() {
                if !collected.is_empty() {
                    collected.push('\n');
                }
                collected.push_str(piece.text.as_ref().unwrap());
            }
        }
    }

    if collected.trim().is_empty() {
        None
    } else {
        Some(collected)
    }
}

fn parse_cli_args() -> Result<CliConfig, String> {
    let mut args = env::args().skip(1);
    let mut reasoning = ReasoningLevel::None;

    while let Some(arg) = args.next() {
        if arg == "--help" || arg == "-h" {
            print_usage();
            std::process::exit(0);
        } else if let Some(rest) = arg.strip_prefix("--reasoning=") {
            reasoning = ReasoningLevel::from_str(rest)?;
        } else if arg == "--reasoning" {
            let value = args
                .next()
                .ok_or("`--reasoning` requires a value (none, low, medium, high)")?;
            reasoning = ReasoningLevel::from_str(&value)?;
        } else {
            return Err(format!("Unknown argument: {}", arg));
        }
    }

    Ok(CliConfig { reasoning })
}

impl ReasoningLevel {
    fn from_str(value: &str) -> Result<Self, String> {
        match value.to_ascii_lowercase().as_str() {
            "none" => Ok(Self::None),
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            other => Err(format!(
                "Unsupported reasoning value `{}` (use none, low, medium, or high)",
                other
            )),
        }
    }

    fn to_api_value(self) -> Option<&'static str> {
        match self {
            ReasoningLevel::None => None,
            ReasoningLevel::Low => Some("low"),
            ReasoningLevel::Medium => Some("medium"),
            ReasoningLevel::High => Some("high"),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            ReasoningLevel::None => "none",
            ReasoningLevel::Low => "low",
            ReasoningLevel::Medium => "medium",
            ReasoningLevel::High => "high",
        }
    }
}

fn print_usage() {
    println!("Usage: chatgpt-cli [--reasoning <none|low|medium|high>]");
    println!("Default reasoning: none (disabled).");
    println!("During a session use /reasoning <value> to change it on the fly.");
}
