use crate::tools::types::ToolImpl;
use crate::tools::types::{ToolDefinition, FunctionDefinition, ParametersSchema};
use serde_json::Value;
use std::process::Command;
use tokio::task;

/// Bash tool for executing shell commands
pub struct BashTool;

impl ToolImpl for BashTool {
    fn definition(&self) -> ToolDefinition {
        let mut properties = serde_json::Map::new();
        properties.insert(
            "command".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "The bash command to execute (e.g., 'ls -la', 'pwd', 'echo hello')"
            }),
        );

        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "bash".to_string(),
                description: "Execute bash commands in the shell".to_string(),
                parameters: ParametersSchema {
                    r#type: "object".to_string(),
                    properties,
                    required: vec!["command".to_string()],
                },
            },
        }
    }

    async fn execute(&self, arguments: &Value) -> Result<String, String> {
        let command = arguments
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'command' argument".to_string())?
            .to_string(); // Clone the command string to own it

        // Execute the command in a blocking task
        let result = task::spawn_blocking(move || {
            // Use sh -c to execute the command, which supports pipes, redirects, etc.
            let output = Command::new("sh")
                .arg("-c")
                .arg(&command)
                .output();

            match output {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                    if output.status.success() {
                        Ok(stdout)
                    } else {
                        let error_msg = if !stderr.is_empty() {
                            stderr
                        } else {
                            format!("Command failed with exit code: {:?}", output.status.code())
                        };
                        Err(error_msg)
                    }
                }
                Err(e) => Err(format!("Failed to execute command: {}", e)),
            }
        })
        .await
        .map_err(|e| format!("Task join error: {}", e))?;

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bash_echo() {
        let tool = BashTool;
        let args = serde_json::json!({"command": "echo hello"});
        assert_eq!(tool.execute(&args).await, Ok("hello\n".to_string()));
    }

    #[tokio::test]
    async fn test_bash_pwd() {
        let tool = BashTool;
        let args = serde_json::json!({"command": "pwd"});
        assert!(tool.execute(&args).await.is_ok());
    }

    #[tokio::test]
    async fn test_bash_pipe() {
        let tool = BashTool;
        let args = serde_json::json!({"command": "echo hello | wc -c"});
        assert!(tool.execute(&args).await.is_ok());
    }

    #[tokio::test]
    async fn test_bash_invalid_command() {
        let tool = BashTool;
        let args = serde_json::json!({"command": "nonexistentcommand123"});
        assert!(tool.execute(&args).await.is_err());
    }

    #[tokio::test]
    async fn test_bash_empty_command() {
        let tool = BashTool;
        let args = serde_json::json!({"command": ""});
        // Empty command is valid in sh -c "", just returns empty output
        assert_eq!(tool.execute(&args).await, Ok("".to_string()));
    }
}
