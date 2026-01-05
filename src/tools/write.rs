use crate::tools::types::ToolImpl;
use crate::tools::types::{ToolDefinition, FunctionDefinition, ParametersSchema};
use serde_json::Value;
use tokio::fs;

/// Write tool for writing content to files
pub struct WriteTool;

impl ToolImpl for WriteTool {
    fn definition(&self) -> ToolDefinition {
        let mut properties = serde_json::Map::new();
        properties.insert(
            "file_path".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "The absolute path to the file to write (e.g., '/home/user/document.txt')"
            }),
        );
        properties.insert(
            "content".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "The content to write to the file"
            }),
        );

        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "write".to_string(),
                description: "Write content to a file. Creates the file if it doesn't exist, overwrites if it does.".to_string(),
                parameters: ParametersSchema {
                    r#type: "object".to_string(),
                    properties,
                    required: vec!["file_path".to_string(), "content".to_string()],
                },
            },
        }
    }

    async fn execute(&self, arguments: &Value) -> Result<String, String> {
        let file_path = arguments
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'file_path' argument".to_string())?;

        let content = arguments
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'content' argument".to_string())?;

        // Write to the file asynchronously
        fs::write(file_path, content)
            .await
            .map_err(|e| format!("Failed to write to file '{}': {}", file_path, e))?;

        Ok(format!("Successfully wrote to file: {}", file_path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::fs;

    #[tokio::test]
    async fn test_write_new_file() {
        let tool = WriteTool;

        // Write to a new file
        let test_file = "/tmp/test_write_new.txt";
        let args = serde_json::json!({
            "file_path": test_file,
            "content": "Hello, World!"
        });

        let result = tool.execute(&args).await;
        assert_eq!(
            result,
            Ok(format!("Successfully wrote to file: {}", test_file))
        );

        // Verify the content was written
        let read_content = fs::read_to_string(test_file).await.unwrap();
        assert_eq!(read_content, "Hello, World!");

        // Clean up
        fs::remove_file(test_file).await.ok();
    }

    #[tokio::test]
    async fn test_write_overwrite_existing() {
        let tool = WriteTool;

        // Create a file with initial content
        let test_file = "/tmp/test_write_overwrite.txt";
        fs::write(test_file, "Initial content")
            .await
            .expect("Failed to create test file");

        // Overwrite with new content
        let args = serde_json::json!({
            "file_path": test_file,
            "content": "New content"
        });

        let result = tool.execute(&args).await;
        assert!(result.is_ok());

        // Verify the content was overwritten
        let read_content = fs::read_to_string(test_file).await.unwrap();
        assert_eq!(read_content, "New content");

        // Clean up
        fs::remove_file(test_file).await.ok();
    }

    #[tokio::test]
    async fn test_write_empty_content() {
        let tool = WriteTool;

        let test_file = "/tmp/test_write_empty.txt";
        let args = serde_json::json!({
            "file_path": test_file,
            "content": ""
        });

        let result = tool.execute(&args).await;
        assert!(result.is_ok());

        // Verify the file is empty
        let read_content = fs::read_to_string(test_file).await.unwrap();
        assert_eq!(read_content, "");

        // Clean up
        fs::remove_file(test_file).await.ok();
    }

    #[tokio::test]
    async fn test_write_multiline_content() {
        let tool = WriteTool;

        let test_file = "/tmp/test_write_multiline.txt";
        let content = "Line 1\nLine 2\nLine 3";
        let args = serde_json::json!({
            "file_path": test_file,
            "content": content
        });

        let result = tool.execute(&args).await;
        assert!(result.is_ok());

        // Verify the content
        let read_content = fs::read_to_string(test_file).await.unwrap();
        assert_eq!(read_content, content);

        // Clean up
        fs::remove_file(test_file).await.ok();
    }

    #[tokio::test]
    async fn test_write_missing_file_path() {
        let tool = WriteTool;
        let args = serde_json::json!({"content": "Hello"});
        assert_eq!(
            tool.execute(&args).await,
            Err("Missing 'file_path' argument".to_string())
        );
    }

    #[tokio::test]
    async fn test_write_missing_content() {
        let tool = WriteTool;
        let args = serde_json::json!({"file_path": "/tmp/test.txt"});
        assert_eq!(
            tool.execute(&args).await,
            Err("Missing 'content' argument".to_string())
        );
    }
}
