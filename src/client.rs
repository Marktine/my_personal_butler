use reqwest::Client;
use serde_json::json;
use std::process::Command;
use tokio::sync::{mpsc, oneshot};

use crate::app::UiEvent;
use crate::config::{save_config, ButlerConfig};
use crate::models::*;
use crate::utils::AnyError;

pub struct GeminiClient {
    client: Client,
    api_key: String,
    history: Vec<Content>,
    tools: Vec<Tool>,
    pub config: ButlerConfig,
    pub ui_tx: mpsc::Sender<UiEvent>,
}

impl GeminiClient {
    pub fn new(api_key: String, config: ButlerConfig, ui_tx: mpsc::Sender<UiEvent>) -> Self {
        let tools = vec![Tool {
            function_declarations: vec![FunctionDeclaration {
                name: "execute_shell_command".to_string(),
                description: "Execute a shell command on the user's system.".to_string(),
                parameters: json!({
                    "type": "OBJECT",
                    "properties": {
                        "command": {
                            "type": "STRING",
                            "description": "The exact shell command to execute."
                        }
                    },
                    "required": ["command"]
                }),
            }],
        }];

        Self {
            client: Client::new(),
            api_key,
            history: Vec::new(),
            tools,
            config,
            ui_tx,
        }
    }

    pub async fn send_message(&mut self, prompt: Option<&str>) -> Result<(), AnyError> {
        if self.config.tokens_used >= self.config.token_limit {
            self.ui_tx.send(UiEvent::AiError("Token limit exceeded!".into())).await?;
            return Ok(());
        }

        if self.config.requests_made >= self.config.request_limit {
            self.ui_tx.send(UiEvent::AiError("Request limit exceeded!".into())).await?;
            return Ok(());
        }

        if let Some(text) = prompt {
            self.history.push(Content {
                role: "user".to_string(),
                parts: vec![Part {
                    text: Some(text.to_string()),
                    function_call: None,
                    function_response: None,
                }],
            });
            self.ui_tx.send(UiEvent::AppendMessage(self.config.user_name.clone(), text.to_string())).await?;
        }

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash:generateContent?key={}",
            self.api_key
        );

        let system_prompt_text = format!(
            "Your name is {}. The user's name is {}. Your vibe and core instruction is: {}.",
            self.config.ai_name, self.config.user_name, self.config.vibe
        );

        let system_instruction = Content {
            role: "user".to_string(),
            parts: vec![Part {
                text: Some(system_prompt_text),
                function_call: None,
                function_response: None,
            }],
        };

        let request_body = GeminiRequest {
            system_instruction: Some(system_instruction),
            contents: self.history.clone(),
            tools: Some(self.tools.clone()),
        };

        let response = match self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await {
                Ok(resp) => resp,
                Err(e) => {
                    self.history.pop();
                    self.ui_tx.send(UiEvent::AiError(format!("Network Error: {}", e))).await?;
                    return Ok(());
                }
            };

        let status = response.status();
        let body_text = response.text().await?;

        if !status.is_success() {
            self.history.pop();
            self.ui_tx.send(UiEvent::AiError(format!("API Request failed: {} - {}", status, body_text))).await?;
            return Ok(());
        }

        let resp: GeminiResponse = serde_json::from_str(&body_text)?;

        self.config.requests_made += 1;
        if let Some(usage) = &resp.usage_metadata {
            if let Some(total) = usage.total_token_count {
                self.config.tokens_used += total;
                save_config(&self.config);
                self.ui_tx.send(UiEvent::UsageUpdate(self.config.tokens_used, self.config.token_limit, self.config.requests_made, self.config.request_limit)).await?;
            }
        }

        if let Some(candidates) = resp.candidates {
            if let Some(candidate) = candidates.first() {
                if let Some(content) = &candidate.content {
                    self.history.push(content.clone());

                    for part in &content.parts {
                        if let Some(text) = &part.text {
                            self.ui_tx.send(UiEvent::AppendMessage(self.config.ai_name.clone(), text.to_string())).await?;
                        }

                        if let Some(call) = &part.function_call {
                            if let Err(e) = self.handle_function_call(call).await {
                                self.ui_tx.send(UiEvent::AiError(format!("Function call error: {}", e))).await?;
                            }
                        }
                    }
                    return Ok(());
                }
            }
        }

        self.history.pop();
        self.ui_tx.send(UiEvent::AiError("Failed to parse response".into())).await?;
        Ok(())
    }

    async fn handle_function_call(
        &mut self,
        call: &FunctionCall,
    ) -> Result<(), AnyError> {
        if call.name == "execute_shell_command" {
            let cmd_str = call
                .args
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let (tx, rx) = oneshot::channel();
            self.ui_tx.send(UiEvent::ToolPrompt(self.config.ai_name.clone(), cmd_str.clone(), tx)).await?;

            let confirm = rx.await.unwrap_or(false);

            let result_json = if confirm {
                self.ui_tx.send(UiEvent::ToolExecuted(format!("Executing: {}", cmd_str))).await?;
                let output = Command::new("sh").arg("-c").arg(&cmd_str).output();

                match output {
                    Ok(out) => {
                        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                        json!({ "stdout": stdout, "stderr": stderr, "status": out.status.code() })
                    }
                    Err(e) => json!({ "error": e.to_string() }),
                }
            } else {
                self.ui_tx.send(UiEvent::ToolExecuted("Command execution denied.".into())).await?;
                json!({ "error": "User denied execution." })
            };

            self.history.push(Content {
                role: "user".to_string(),
                parts: vec![Part {
                    text: None,
                    function_call: None,
                    function_response: Some(FunctionResponse {
                        name: call.name.clone(),
                        response: result_json,
                    }),
                }],
            });

            Box::pin(self.send_message(None)).await?;
        }

        Ok(())
    }
}
