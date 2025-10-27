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
    provider: Option<String>,
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
    logs: Vec<String>,
}

impl StdioChatResponse {
    fn success(
        session_id: String,
        content: String,
        tool_steps: Vec<AgentStep>,
        logs: Vec<String>,
    ) -> Self {
        Self {
            session_id: Some(session_id),
            content: Some(content),
            error: None,
            tool_steps,
            logs,
        }
    }

    fn error(message: impl Into<String>) -> Self {
        Self {
            session_id: None,
            content: None,
            error: Some(message.into()),
            tool_steps: Vec::new(),
            logs: Vec::new(),
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
                let StdioChatRequest {
                    prompt,
                    provider,
                    model,
                    system_prompt,
                    session_id,
                    agent,
                    max_tool_steps,
                } = request;

                if prompt.trim().is_empty() {
                    write_response(
                        &mut stdout,
                        StdioChatResponse::error("prompt cannot be empty"),
                    )
                    .await?;
                    continue;
                }

                if agent {
                    info!("Processing STDIO agent request");
                    let mut options = AgentOptions::default();
                    options.provider = provider.clone();
                    options.model = model.clone();
                    options.system_prompt = system_prompt.clone();
                    options.session_id = session_id.clone();
                    if let Some(max_steps) = max_tool_steps {
                        options.max_steps = max_steps;
                    }
                    let agent = Agent::new(client.clone());
                    match agent.run(prompt, options).await {
                        Ok(outcome) => {
                            write_response(
                                &mut stdout,
                                StdioChatResponse::success(
                                    outcome.session_id,
                                    outcome.response,
                                    outcome.steps,
                                    outcome.logs,
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
                            prompt,
                            provider,
                            model,
                            system_prompt,
                            session_id,
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
                                    result.logs,
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
