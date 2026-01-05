use crate::tools::types::ToolImpl;
use crate::tools::types::{ToolDefinition, FunctionDefinition, ParametersSchema};
use serde_json::Value;
use tokio::fs;
use tokio::io::AsyncReadExt;

/// Read tool for reading file contents
pub struct ReadTool;

impl ToolImpl for ReadTool {
    fn definition(&self) -> ToolDefinition {
        let mut properties = serde_json::Map::new();
        properties.insert(
            "file_path".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "The absolute path to the file to read (e.g., '/home/user/document.txt')"
            }),
        );

        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "read".to_string(),
                description: "Read the contents of a file from the file system".to_string(),
                parameters: ParametersSchema {
                    r#type: "object".to_string(),
                    properties,
                    required: vec!["file_path".to_string()],
                },
            },
        }
    }

    async fn execute(&self, arguments: &Value) -> Result<String, String> {
        let file_path = arguments
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'file_path' argument".to_string())?;

        // Read the file asynchronously
        let mut file = fs::File::open(file_path)
            .await
            .map_err(|e| format!("Failed to open file '{}': {}", file_path, e))?;

        let mut contents = Vec::new();
        file.read_to_end(&mut contents)
            .await
            .map_err(|e| format!("Failed to read file '{}': {}", file_path, e))?;

        // Convert to string, replacing any invalid UTF-8 sequences
        let result = String::from_utf8_lossy(&contents).to_string();
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::fs;

    #[tokio::test]
    async fn test_read_text_file() {
        let tool = ReadTool;

        // Create a temporary test file
        let test_file = "/tmp/test_read.txt";
        fs::write(test_file, "Hello, World!")
            .await
            .expect("Failed to create test file");

        let args = serde_json::json!({"file_path": test_file});
        let result = tool.execute(&args).await;
        assert_eq!(result, Ok("Hello, World!".to_string()));

        // Clean up
        fs::remove_file(test_file).await.ok();
    }

    #[tokio::test]
    async fn test_read_empty_file() {
        let tool = ReadTool;

        // Create an empty test file
        let test_file = "/tmp/test_read_empty.txt";
        fs::write(test_file, "")
            .await
            .expect("Failed to create test file");

        let args = serde_json::json!({"file_path": test_file});
        let result = tool.execute(&args).await;
        assert_eq!(result, Ok("".to_string()));

        // Clean up
        fs::remove_file(test_file).await.ok();
    }

    #[tokio::test]
    async fn test_read_multiline_file() {
        let tool = ReadTool;

        // Create a test file with multiple lines
        let test_file = "/tmp/test_read_multiline.txt";
        let content = "Line 1\nLine 2\nLine 3";
        fs::write(test_file, content)
            .await
            .expect("Failed to create test file");

        let args = serde_json::json!({"file_path": test_file});
        let result = tool.execute(&args).await;
        assert_eq!(result, Ok(content.to_string()));

        // Clean up
        fs::remove_file(test_file).await.ok();
    }

    #[tokio::test]
    async fn test_read_nonexistent_file() {
        let tool = ReadTool;
        let args = serde_json::json!({"file_path": "/nonexistent/file.txt"});
        assert!(tool.execute(&args).await.is_err());
    }

    #[tokio::test]
    async fn test_read_missing_file_path() {
        let tool = ReadTool;
        let args = serde_json::json!({});
        assert_eq!(
            tool.execute(&args).await,
            Err("Missing 'file_path' argument".to_string())
        );
    }
}
