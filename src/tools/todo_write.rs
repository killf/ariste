use crate::tools::types::ToolImpl;
use crate::tools::types::{ToolDefinition, FunctionDefinition, ParametersSchema};
use serde_json::Value;
use serde::{Deserialize, Serialize};

/// TodoWrite tool for managing todo lists
pub struct TodoWriteTool;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TodoItem {
    content: String,
    status: String,
    #[serde(rename = "activeForm")]
    active_form: String,
}

impl ToolImpl for TodoWriteTool {
    fn definition(&self) -> ToolDefinition {
        let mut properties = serde_json::Map::new();
        properties.insert(
            "todos".to_string(),
            serde_json::json!({
                "type": "array",
                "description": "The updated todo list with all current tasks",
                "items": {
                    "type": "object",
                    "properties": {
                        "content": {
                            "type": "string",
                            "description": "The task description in imperative form (e.g., 'Run tests')"
                        },
                        "status": {
                            "type": "string",
                            "enum": ["pending", "in_progress", "completed"],
                            "description": "The current status of the task"
                        },
                        "activeForm": {
                            "type": "string",
                            "description": "The task description in present continuous form (e.g., 'Running tests')"
                        }
                    },
                    "required": ["content", "status", "activeForm"]
                }
            }),
        );

        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "todo_write".to_string(),
                description: "Update the todo list to track progress and organize tasks".to_string(),
                parameters: ParametersSchema {
                    r#type: "object".to_string(),
                    properties,
                    required: vec!["todos".to_string()],
                },
            },
        }
    }

    async fn execute(&self, arguments: &Value) -> Result<String, String> {
        let todos = arguments
            .get("todos")
            .and_then(|v| v.as_array())
            .ok_or_else(|| "Missing 'todos' argument or it's not an array".to_string())?;

        // Parse todos
        let parsed_todos: Result<Vec<TodoItem>, String> = todos
            .iter()
            .map(|item| {
                let content = item
                    .get("content")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing 'content' field in todo item".to_string())?;

                let status = item
                    .get("status")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing 'status' field in todo item".to_string())?;

                let active_form = item
                    .get("activeForm")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| "Missing 'activeForm' field in todo item".to_string())?;

                // Validate status
                if !matches!(status, "pending" | "in_progress" | "completed") {
                    return Err(format!(
                        "Invalid status '{}': must be one of 'pending', 'in_progress', or 'completed'",
                        status
                    ));
                }

                Ok(TodoItem {
                    content: content.to_string(),
                    status: status.to_string(),
                    active_form: active_form.to_string(),
                })
            })
            .collect();

        let parsed_todos = parsed_todos?;

        // Count tasks by status
        let pending_count = parsed_todos
            .iter()
            .filter(|t| t.status == "pending")
            .count();
        let in_progress_count = parsed_todos
            .iter()
            .filter(|t| t.status == "in_progress")
            .count();
        let completed_count = parsed_todos
            .iter()
            .filter(|t| t.status == "completed")
            .count();

        // Format output
        let mut output = String::new();
        output.push_str("Todo list updated:\n");

        for todo in &parsed_todos {
            let status_icon = match todo.status.as_str() {
                "pending" => "○",
                "in_progress" => "◐",
                "completed" => "●",
                _ => "?",
            };
            output.push_str(&format!("  {} {}\n", status_icon, todo.active_form));
        }

        output.push_str(&format!(
            "\nTotal: {} tasks ({} pending, {} in progress, {} completed)",
            parsed_todos.len(),
            pending_count,
            in_progress_count,
            completed_count
        ));

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_todo_write_basic() {
        let tool = TodoWriteTool;

        let args = serde_json::json!({
            "todos": [
                {"content": "Task 1", "status": "pending", "activeForm": "Working on task 1"},
                {"content": "Task 2", "status": "in_progress", "activeForm": "Working on task 2"},
                {"content": "Task 3", "status": "completed", "activeForm": "Working on task 3"}
            ]
        });

        let result = tool.execute(&args).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("Todo list updated:"));
        assert!(output.contains("Working on task 1"));
        assert!(output.contains("Working on task 2"));
        assert!(output.contains("Working on task 3"));
        assert!(output.contains("Total: 3 tasks"));
    }

    #[tokio::test]
    async fn test_todo_write_empty() {
        let tool = TodoWriteTool;

        let args = serde_json::json!({"todos": []});
        let result = tool.execute(&args).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("Total: 0 tasks"));
    }

    #[tokio::test]
    async fn test_todo_write_missing_todos() {
        let tool = TodoWriteTool;
        let args = serde_json::json!({});
        assert!(tool.execute(&args).await.is_err());
    }

    #[tokio::test]
    async fn test_todo_write_invalid_status() {
        let tool = TodoWriteTool;

        let args = serde_json::json!({
            "todos": [
                {"content": "Task 1", "status": "invalid", "activeForm": "Working on task 1"}
            ]
        });

        let result = tool.execute(&args).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_todo_write_missing_fields() {
        let tool = TodoWriteTool;

        let args = serde_json::json!({
            "todos": [
                {"content": "Task 1"}
            ]
        });

        let result = tool.execute(&args).await;
        assert!(result.is_err());
    }
}
