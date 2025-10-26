use crate::agent::{Agent, AgentOptions, AgentStep};
use crate::client::{ChatRequest, McpClient};
use crate::model::ModelProvider;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{debug, error, info};

#[derive(Debug, Error)]
pub enum StdioError {
    #[error("stdin/stdout I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to serialize stdio response: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Debug, Deserialize)]
struct StdioChatRequest {
    prompt: String,
    model: Option<String>,
    system_prompt: Option<String>,
    session_id: Option<String>,
    #[serde(default)]
    agent: bool,
    #[serde(default)]
    max_tool_steps: Option<usize>,
}

#[derive(Debug, Serialize)]
struct StdioChatResponse {
    session_id: Option<String>,
    content: Option<String>,
    error: Option<String>,
    tool_steps: Vec<AgentStep>,
}

impl StdioChatResponse {
    fn success(session_id: String, content: String, tool_steps: Vec<AgentStep>) -> Self {
        Self {
            session_id: Some(session_id),
            content: Some(content),
            error: None,
            tool_steps,
        }
    }

    fn error(message: impl Into<String>) -> Self {
        Self {
            session_id: None,
            content: None,
            error: Some(message.into()),
            tool_steps: Vec::new(),
        }
    }
}

pub async fn run<P>(client: Arc<McpClient<P>>) -> Result<(), StdioError>
where
    P: ModelProvider + 'static,
{
    let stdin = BufReader::new(io::stdin());
    let mut lines = stdin.lines();
    let mut stdout = io::stdout();

    while let Some(line) = lines.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }
        debug!("Received STDIO line");

        match serde_json::from_str::<StdioChatRequest>(&line) {
            Ok(request) => {
                if request.prompt.trim().is_empty() {
                    write_response(
                        &mut stdout,
                        StdioChatResponse::error("prompt cannot be empty"),
                    )
                    .await?;
                    continue;
                }

                if request.agent {
                    info!("Processing STDIO agent request");
                    let mut options = AgentOptions::default();
                    options.model = request.model;
                    options.system_prompt = request.system_prompt;
                    options.session_id = request.session_id;
                    if let Some(max_steps) = request.max_tool_steps {
                        options.max_steps = max_steps;
                    }
                    let agent = Agent::new(client.clone());
                    match agent.run(request.prompt, options).await {
                        Ok(outcome) => {
                            write_response(
                                &mut stdout,
                                StdioChatResponse::success(
                                    outcome.session_id,
                                    outcome.response,
                                    outcome.steps,
                                ),
                            )
                            .await?;
                        }
                        Err(error) => {
                            error!(%error, "Agent processing failed via STDIO");
                            let message = error.user_message();
                            write_response(&mut stdout, StdioChatResponse::error(message)).await?;
                        }
                    }
                } else {
                    info!("Processing STDIO direct chat request");
                    match client
                        .chat(ChatRequest {
                            prompt: request.prompt,
                            model: request.model,
                            system_prompt: request.system_prompt,
                            session_id: request.session_id,
                        })
                        .await
                    {
                        Ok(result) => {
                            write_response(
                                &mut stdout,
                                StdioChatResponse::success(
                                    result.session_id,
                                    result.content,
                                    Vec::new(),
                                ),
                            )
                            .await?;
                        }
                        Err(error) => {
                            error!(%error, "STDIO chat request failed");
                            let message = error.user_message();
                            write_response(&mut stdout, StdioChatResponse::error(message)).await?;
                        }
                    }
                }
            }
            Err(error) => {
                error!(%error, "Failed to parse STDIO input line");
                write_response(
                    &mut stdout,
                    StdioChatResponse::error(format!("Format input JSON tidak valid: {error}")),
                )
                .await?;
            }
        }
    }

    stdout.flush().await?;
    Ok(())
}

async fn write_response(
    stdout: &mut io::Stdout,
    response: StdioChatResponse,
) -> Result<(), StdioError> {
    let mut payload = serde_json::to_vec(&response)?;
    payload.push(b'\n');
    stdout.write_all(&payload).await?;
    stdout.flush().await?;
    Ok(())
}
