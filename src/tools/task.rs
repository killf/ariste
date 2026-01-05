use crate::agent::Message;
use crate::config::AgentConfig;
use crate::llm::Ollama;
use crate::tools::types::ToolImpl;
use crate::tools::types::{ToolDefinition, FunctionDefinition, ParametersSchema};
use serde_json::Value;

/// Task tool for spawning subagents to handle complex tasks
pub struct TaskTool;

#[derive(Debug, Clone, PartialEq)]
enum SubAgentType {
    GeneralPurpose,
    Explore,
    Plan,
    #[allow(dead_code)]
    ClaudeCodeGuide,
    #[allow(dead_code)]
    GlmPlanUsage,
}

impl SubAgentType {
    fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "general-purpose" => Ok(SubAgentType::GeneralPurpose),
            "explore" | "Explore" => Ok(SubAgentType::Explore),
            "plan" | "Plan" => Ok(SubAgentType::Plan),
            "claude-code-guide" => Ok(SubAgentType::ClaudeCodeGuide),
            "glm-plan-usage:usage-query-agent" => Ok(SubAgentType::GlmPlanUsage),
            _ => Err(format!(
                "Unknown subagent type '{}'. Valid types are: general-purpose, explore, plan",
                s
            )),
        }
    }

    fn description(&self) -> &str {
        match self {
            SubAgentType::GeneralPurpose => "General-purpose agent for complex tasks",
            SubAgentType::Explore => "Fast agent for exploring codebases",
            SubAgentType::Plan => "Software architect agent for designing implementation plans",
            SubAgentType::ClaudeCodeGuide => "Guide for Claude Code documentation",
            SubAgentType::GlmPlanUsage => "Query GLM Coding Plan usage statistics",
        }
    }

    fn system_prompt(&self) -> Option<&str> {
        match self {
            SubAgentType::Explore => Some(
                "You are a codebase exploration agent. Your goal is to quickly find files, \
                 search code, and answer questions about the codebase structure. \
                 Be thorough but efficient in your exploration.",
            ),
            SubAgentType::Plan => Some(
                "You are a software architect agent. Your goal is to design implementation plans \
                 by exploring the codebase and providing step-by-step plans. Focus on: \
                 1) Understanding existing patterns, 2) Identifying critical files, \
                 3) Considering architectural trade-offs.",
            ),
            _ => None,
        }
    }
}

impl ToolImpl for TaskTool {
    fn definition(&self) -> ToolDefinition {
        let mut properties = serde_json::Map::new();
        properties.insert(
            "subagent_type".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "The type of subagent to launch",
                "enum": ["general-purpose", "explore", "plan", "claude-code-guide", "glm-plan-usage:usage-query-agent"]
            }),
        );
        properties.insert(
            "prompt".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "The detailed task for the agent to perform"
            }),
        );
        properties.insert(
            "description".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "A short description (3-5 words) of what the agent will do"
            }),
        );
        properties.insert(
            "model".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "Optional model to use (defaults to the configured model)"
            }),
        );
        properties.insert(
            "run_in_background".to_string(),
            serde_json::json!({
                "type": "boolean",
                "description": "Whether to run the agent in background (not fully supported yet)"
            }),
        );

        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "task".to_string(),
                description: "Launch a specialized subagent to handle complex, multi-step tasks autonomously".to_string(),
                parameters: ParametersSchema {
                    r#type: "object".to_string(),
                    properties,
                    required: vec!["subagent_type".to_string(), "prompt".to_string(), "description".to_string()],
                },
            },
        }
    }

    async fn execute(&self, arguments: &Value) -> Result<String, String> {
        let subagent_type_str = arguments
            .get("subagent_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'subagent_type' argument".to_string())?;

        let prompt = arguments
            .get("prompt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'prompt' argument".to_string())?;

        let description = arguments
            .get("description")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'description' argument".to_string())?;

        let model = arguments.get("model").and_then(|v| v.as_str());

        let _run_in_background = arguments
            .get("run_in_background")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Parse subagent type
        let subagent_type = SubAgentType::from_str(subagent_type_str)?;

        // Load config to get base URL
        let config_file = ".ariste/settings.json";
        let config = if !tokio::fs::try_exists(&config_file)
            .await
            .map_err(|e| format!("Failed to check config file: {}", e))?
        {
            AgentConfig::default()
        } else {
            let buf = tokio::fs::read(&config_file)
                .await
                .map_err(|e| format!("Failed to read config file: {}", e))?;
            serde_json::from_slice(&buf)
                .map_err(|e| format!("Failed to parse config file: {}", e))?
        };

        // Build the URL
        let url = if let Some(base) = &config.base {
            format!("{}/api/chat", base)
        } else {
            "http://localhost:11434/api/chat".to_string()
        };

        // Get model to use
        let default_model = config.model.as_deref().unwrap_or("qwen3-vl:32b");
        let model_to_use = model.unwrap_or(default_model);

        // Build the messages
        let mut messages = Vec::new();

        // Add system prompt if applicable
        if let Some(system_prompt) = subagent_type.system_prompt() {
            messages.push(Message {
                role: "system".to_string(),
                content: system_prompt.to_string(),
                tool_calls: None,
                tool_call_id: None,
            });
        }

        // Build the full prompt
        let full_prompt = format!("Task: {}\n\nDetails:\n{}", description, prompt);

        // Add user message
        messages.push(Message {
            role: "user".to_string(),
            content: full_prompt,
            tool_calls: None,
            tool_call_id: None,
        });

        // Create Ollama instance without tools (to avoid recursion)
        let ollama = Ollama::new().url(url).stream(false).think(false);

        // Execute the task
        let response = ollama
            .execute_with_messages(model_to_use, &messages)
            .await
            .map_err(|e| format!("Ollama execution failed: {}", e))?;

        // Add metadata header
        let output = format!(
            "=== Subagent Task: {} ===\nType: {}\nModel: {}\n\n{}\n\n=== Task Complete ===",
            description,
            subagent_type.description(),
            model_to_use,
            response.content
        );

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subagent_type_parsing() {
        assert_eq!(
            SubAgentType::from_str("general-purpose").unwrap(),
            SubAgentType::GeneralPurpose
        );
        assert_eq!(
            SubAgentType::from_str("explore").unwrap(),
            SubAgentType::Explore
        );
        assert_eq!(
            SubAgentType::from_str("plan").unwrap(),
            SubAgentType::Plan
        );
        assert!(SubAgentType::from_str("invalid").is_err());
    }

    #[test]
    fn test_subagent_type_descriptions() {
        assert!(SubAgentType::GeneralPurpose.description().contains("General-purpose"));
        assert!(SubAgentType::Explore.description().contains("explor"));
        assert!(SubAgentType::Plan.description().contains("architect"));
    }

    #[test]
    fn test_explore_description() {
        let desc = SubAgentType::Explore.description();
        assert!(desc.contains("explor") || desc.contains("Fast agent"));
    }

    #[tokio::test]
    async fn test_task_missing_subagent_type() {
        let tool = TaskTool;
        let args = serde_json::json!({
            "prompt": "Test task",
            "description": "Test"
        });
        assert!(tool.execute(&args).await.is_err());
    }

    #[tokio::test]
    async fn test_task_missing_prompt() {
        let tool = TaskTool;
        let args = serde_json::json!({
            "subagent_type": "general-purpose",
            "description": "Test"
        });
        assert!(tool.execute(&args).await.is_err());
    }

    #[tokio::test]
    async fn test_task_missing_description() {
        let tool = TaskTool;
        let args = serde_json::json!({
            "subagent_type": "general-purpose",
            "prompt": "Test task"
        });
        assert!(tool.execute(&args).await.is_err());
    }

    #[tokio::test]
    async fn test_task_invalid_subagent_type() {
        let tool = TaskTool;
        let args = serde_json::json!({
            "subagent_type": "invalid-type",
            "prompt": "Test task",
            "description": "Test"
        });
        let result = tool.execute(&args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown subagent type"));
    }
}
