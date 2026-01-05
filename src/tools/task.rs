use crate::tools::types::ToolImpl;
use crate::tools::types::{ToolDefinition, FunctionDefinition, ParametersSchema};
use serde_json::Value;

/// Task tool for spawning subagent tasks
pub struct TaskTool;

impl ToolImpl for TaskTool {
    fn definition(&self) -> ToolDefinition {
        let mut properties = serde_json::Map::new();
        properties.insert(
            "subagent_type".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "The type of subagent to spawn",
                "enum": [
                    "general-purpose",
                    "explore",
                    "plan",
                    "code-review",
                    "test-runner"
                ]
            }),
        );
        properties.insert(
            "prompt".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "The task description for the subagent (3-5 words summary)"
            }),
        );
        properties.insert(
            "description".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "Detailed prompt explaining what the subagent should do"
            }),
        );
        properties.insert(
            "include_context".to_string(),
            serde_json::json!({
                "type": "boolean",
                "description": "Whether to include conversation context (default: false)"
            }),
        );
        properties.insert(
            "include_tools".to_string(),
            serde_json::json!({
                "type": "boolean",
                "description": "Whether the subagent should have access to tools (default: false)"
            }),
        );

        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "task".to_string(),
                description: "Launch a subagent to handle complex, multi-step tasks. Use this for: exploring codebases, planning implementations, code reviews, testing, or any task that requires specialized autonomous handling.".to_string(),
                parameters: ParametersSchema {
                    r#type: "object".to_string(),
                    properties,
                    required: vec!["subagent_type".to_string(), "prompt".to_string(), "description".to_string()],
                },
            },
        }
    }

    async fn execute(&self, _arguments: &Value) -> Result<String, String> {
        // This should never be called directly
        // Agent::execute_tool handles Task specially by calling spawn_task
        Err("Task tool must be executed through Agent::execute_tool".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_definition() {
        let tool = TaskTool;
        let def = tool.definition();

        assert_eq!(def.function.name, "task");
        assert!(def.function.description.contains("subagent"));

        let params = def.function.parameters;
        assert_eq!(params.r#type, "object");

        // Check required fields
        assert!(params.required.contains(&"subagent_type".to_string()));
        assert!(params.required.contains(&"prompt".to_string()));
        assert!(params.required.contains(&"description".to_string()));

        // Check subagent_type enum values
        let subagent_type = params.properties.get("subagent_type").unwrap();
        let enum_values = subagent_type.get("enum").unwrap().as_array().unwrap();
        assert_eq!(enum_values.len(), 5);
    }
}
