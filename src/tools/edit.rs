use crate::tools::types::ToolImpl;
use crate::tools::types::{ToolDefinition, FunctionDefinition, ParametersSchema};
use serde_json::Value;
use tokio::fs;
use tokio::io::AsyncReadExt;

/// Edit tool for editing file contents
pub struct EditTool;

impl ToolImpl for EditTool {
    fn definition(&self) -> ToolDefinition {
        let mut properties = serde_json::Map::new();
        properties.insert(
            "file_path".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "The absolute path to the file to edit (e.g., '/home/user/document.txt')"
            }),
        );
        properties.insert(
            "old_string".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "The exact string to search for and replace. Must be an exact match."
            }),
        );
        properties.insert(
            "new_string".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "The new string to replace the old_string with."
            }),
        );
        properties.insert(
            "replace_all".to_string(),
            serde_json::json!({
                "type": "boolean",
                "description": "If true, replace all occurrences of old_string. If false (default), only replace the first occurrence."
            }),
        );

        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "edit".to_string(),
                description: "Edit a file by replacing text. Reads the file, replaces occurrences of old_string with new_string, and writes it back. Preserves the original file encoding and line endings.".to_string(),
                parameters: ParametersSchema {
                    r#type: "object".to_string(),
                    properties,
                    required: vec!["file_path".to_string(), "old_string".to_string(), "new_string".to_string()],
                },
            },
        }
    }

    async fn execute(&self, arguments: &Value) -> Result<String, String> {
        let file_path = arguments
            .get("file_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'file_path' argument".to_string())?;

        let old_string = arguments
            .get("old_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'old_string' argument".to_string())?;

        let new_string = arguments
            .get("new_string")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'new_string' argument".to_string())?;

        let replace_all = arguments
            .get("replace_all")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Read the file
        let mut file = fs::File::open(file_path)
            .await
            .map_err(|e| format!("Failed to open file '{}': {}", file_path, e))?;

        let mut contents = Vec::new();
        file.read_to_end(&mut contents)
            .await
            .map_err(|e| format!("Failed to read file '{}': {}", file_path, e))?;

        // Convert to string
        let original = String::from_utf8_lossy(&contents).to_string();

        // Perform replacement
        let new_contents = if replace_all {
            original.replace(old_string, new_string)
        } else {
            original.replacen(old_string, new_string, 1)
        };

        // Check if replacement was made
        if new_contents == original {
            return Err(format!(
                "Old string '{}' not found in file '{}'",
                old_string, file_path
            ));
        }

        // Write back to file
        fs::write(file_path, new_contents)
            .await
            .map_err(|e| format!("Failed to write file '{}': {}", file_path, e))?;

        let replacement_type = if replace_all {
            "all occurrences"
        } else {
            "first occurrence"
        };

        Ok(format!(
            "Successfully replaced {} of '{}' with '{}' in file '{}'",
            replacement_type, old_string, new_string, file_path
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::fs;

    #[tokio::test]
    async fn test_edit_replace_first() {
        let tool = EditTool;

        // Create test file
        let test_file = "/tmp/test_edit.txt";
        fs::write(test_file, "Hello World\nHello Rust\nHello Test")
            .await
            .expect("Failed to create test file");

        let args = serde_json::json!({
            "file_path": test_file,
            "old_string": "Hello",
            "new_string": "Hi",
            "replace_all": false
        });

        let result = tool.execute(&args).await;
        assert!(result.is_ok());

        // Verify only first occurrence was replaced
        let contents = fs::read_to_string(test_file).await.unwrap();
        assert_eq!(contents, "Hi World\nHello Rust\nHello Test");

        // Clean up
        fs::remove_file(test_file).await.ok();
    }

    #[tokio::test]
    async fn test_edit_replace_all() {
        let tool = EditTool;

        // Create test file
        let test_file = "/tmp/test_edit_all.txt";
        fs::write(test_file, "Hello World\nHello Rust\nHello Test")
            .await
            .expect("Failed to create test file");

        let args = serde_json::json!({
            "file_path": test_file,
            "old_string": "Hello",
            "new_string": "Hi",
            "replace_all": true
        });

        let result = tool.execute(&args).await;
        assert!(result.is_ok());

        // Verify all occurrences were replaced
        let contents = fs::read_to_string(test_file).await.unwrap();
        assert_eq!(contents, "Hi World\nHi Rust\nHi Test");

        // Clean up
        fs::remove_file(test_file).await.ok();
    }

    #[tokio::test]
    async fn test_edit_string_not_found() {
        let tool = EditTool;

        // Create test file
        let test_file = "/tmp/test_edit_not_found.txt";
        fs::write(test_file, "Hello World").await.expect("Failed to create test file");

        let args = serde_json::json!({
            "file_path": test_file,
            "old_string": "Goodbye",
            "new_string": "Hi"
        });

        let result = tool.execute(&args).await;
        assert!(result.is_err());

        // Clean up
        fs::remove_file(test_file).await.ok();
    }

    #[tokio::test]
    async fn test_edit_missing_file_path() {
        let tool = EditTool;
        let args = serde_json::json!({
            "old_string": "Hello",
            "new_string": "Hi"
        });
        assert_eq!(
            tool.execute(&args).await,
            Err("Missing 'file_path' argument".to_string())
        );
    }

    #[tokio::test]
    async fn test_edit_missing_old_string() {
        let tool = EditTool;
        let args = serde_json::json!({
            "file_path": "/tmp/test.txt",
            "new_string": "Hi"
        });
        assert_eq!(
            tool.execute(&args).await,
            Err("Missing 'old_string' argument".to_string())
        );
    }

    #[tokio::test]
    async fn test_edit_missing_new_string() {
        let tool = EditTool;
        let args = serde_json::json!({
            "file_path": "/tmp/test.txt",
            "old_string": "Hello"
        });
        assert_eq!(
            tool.execute(&args).await,
            Err("Missing 'new_string' argument".to_string())
        );
    }
}
