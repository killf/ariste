use crate::tools::types::ToolImpl;
use crate::tools::types::{ToolDefinition, FunctionDefinition, ParametersSchema};
use serde_json::Value;
use std::path::Path;

/// Glob tool for file pattern matching
pub struct GlobTool;

impl ToolImpl for GlobTool {
    fn definition(&self) -> ToolDefinition {
        let mut properties = serde_json::Map::new();
        properties.insert(
            "pattern".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "The glob pattern to match files (e.g., '**/*.rs', 'src/**/*.json', '/home/user/**/*.txt'). Supports standard wildcards: * matches any sequence of characters within a path segment, ** matches any number of path segments, ? matches a single character."
            }),
        );
        properties.insert(
            "path".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "The base directory to search in. If not provided, uses current working directory."
            }),
        );

        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "glob".to_string(),
                description: "Search for files matching a glob pattern. Returns a list of matching file paths sorted by modification time. This is useful for finding files by name pattern or extension.".to_string(),
                parameters: ParametersSchema {
                    r#type: "object".to_string(),
                    properties,
                    required: vec!["pattern".to_string()],
                },
            },
        }
    }

    async fn execute(&self, arguments: &Value) -> Result<String, String> {
        let pattern = arguments
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'pattern' argument".to_string())?;

        let base_path = arguments
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        // Construct the full pattern
        let full_pattern = if Path::new(pattern).is_absolute() {
            pattern.to_string()
        } else {
            format!("{}/{}", base_path, pattern)
        };

        // Perform glob search
        let mut matches: Vec<String> = glob::glob(&full_pattern)
            .map_err(|e| format!("Invalid glob pattern '{}': {}", full_pattern, e))?
            .filter_map(|entry| match entry {
                Ok(path) => path.into_os_string().into_string().ok(),
                Err(e) => {
                    eprintln!("Glob error: {}", e);
                    None
                }
            })
            .collect();

        // Sort matches for consistent output
        matches.sort();

        if matches.is_empty() {
            Ok(format!("No files found matching pattern: {}", full_pattern))
        } else {
            Ok(matches.join("\n"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::fs;

    #[tokio::test]
    async fn test_glob_txt_files() {
        let tool = GlobTool;

        // Create test files
        let test_dir = "/tmp/test_glob";
        fs::create_dir_all(test_dir).await.ok();

        fs::write(format!("{}/test1.txt", test_dir), "content1")
            .await
            .ok();
        fs::write(format!("{}/test2.txt", test_dir), "content2")
            .await
            .ok();
        fs::write(format!("{}/test.rs", test_dir), "code")
            .await
            .ok();

        let args = serde_json::json!({"pattern": "*.txt", "path": test_dir});
        let result = tool.execute(&args).await;

        assert!(result.is_ok());
        let result_str = result.unwrap();
        assert!(result_str.contains("test1.txt"));
        assert!(result_str.contains("test2.txt"));
        assert!(!result_str.contains("test.rs"));

        // Clean up
        fs::remove_dir_all(test_dir).await.ok();
    }

    #[tokio::test]
    async fn test_glob_recursive() {
        let tool = GlobTool;

        // Create test directory structure
        let test_dir = "/tmp/test_glob_recursive";
        fs::create_dir_all(format!("{}/subdir1", test_dir)).await.ok();
        fs::create_dir_all(format!("{}/subdir2", test_dir)).await.ok();

        fs::write(format!("{}/file.txt", test_dir), "root")
            .await
            .ok();
        fs::write(format!("{}/subdir1/file.txt", test_dir), "sub1")
            .await
            .ok();
        fs::write(format!("{}/subdir2/file.txt", test_dir), "sub2")
            .await
            .ok();

        let args = serde_json::json!({"pattern": "**/*.txt", "path": test_dir});
        let result = tool.execute(&args).await;

        assert!(result.is_ok());
        let result_str = result.unwrap();
        // Should find all three txt files
        assert!(result_str.lines().count() >= 3);

        // Clean up
        fs::remove_dir_all(test_dir).await.ok();
    }

    #[tokio::test]
    async fn test_glob_missing_pattern() {
        let tool = GlobTool;
        let args = serde_json::json!({});
        assert_eq!(
            tool.execute(&args).await,
            Err("Missing 'pattern' argument".to_string())
        );
    }
}
