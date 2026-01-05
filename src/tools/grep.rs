use crate::tools::types::ToolImpl;
use crate::tools::types::{ToolDefinition, FunctionDefinition, ParametersSchema};
use regex::Regex;
use serde_json::Value;
use std::path::Path;
use tokio::fs;
use tokio::io::AsyncReadExt;

/// Grep tool for searching file contents
pub struct GrepTool;

impl ToolImpl for GrepTool {
    fn definition(&self) -> ToolDefinition {
        let mut properties = serde_json::Map::new();
        properties.insert(
            "pattern".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "The regular expression pattern to search for in file contents. Uses Rust regex syntax."
            }),
        );
        properties.insert(
            "path".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "The file or directory path to search in. If a directory, searches all files recursively."
            }),
        );
        properties.insert(
            "glob".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "Optional glob pattern to filter files when searching a directory (e.g., '*.rs', '**/*.json'). If not provided, searches all files."
            }),
        );
        properties.insert(
            "case_insensitive".to_string(),
            serde_json::json!({
                "type": "boolean",
                "description": "Whether to perform case-insensitive search. Default is false."
            }),
        );
        properties.insert(
            "output_mode".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "Output format: 'content' shows matching lines, 'files_with_matches' shows only file paths, 'count' shows match counts per file. Default is 'content'."
            }),
        );

        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "grep".to_string(),
                description: "Search for text patterns in files using regular expressions. Supports recursive directory searching and multiple output modes.".to_string(),
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

        let path = arguments
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or(".");

        let glob_pattern = arguments.get("glob").and_then(|v| v.as_str());

        let _case_insensitive = arguments
            .get("case_insensitive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let output_mode = arguments
            .get("output_mode")
            .and_then(|v| v.as_str())
            .unwrap_or("content");

        // Compile regex
        let regex = Regex::new(pattern)
            .map_err(|e| format!("Invalid regex pattern '{}': {}", pattern, e))?;

        // Check if path is a file or directory
        let search_path = Path::new(path);
        let mut results = Vec::new();

        if search_path.is_file() {
            // Search single file
            self.search_file(path, &regex, output_mode, &mut results)
                .await?;
        } else if search_path.is_dir() {
            // Search directory
            let files = self.find_files_to_search(path, glob_pattern)?;
            for file_path in files {
                self.search_file(&file_path, &regex, output_mode, &mut results)
                    .await?;
            }
        } else {
            return Err(format!("Path '{}' is not a valid file or directory", path));
        }

        if results.is_empty() {
            Ok(format!("No matches found for pattern: {}", pattern))
        } else {
            Ok(results.join("\n"))
        }
    }
}

impl GrepTool {
    async fn search_file(
        &self,
        file_path: &str,
        regex: &Regex,
        output_mode: &str,
        results: &mut Vec<String>,
    ) -> Result<(), String> {
        let mut file = fs::File::open(file_path)
            .await
            .map_err(|e| format!("Failed to open file '{}': {}", file_path, e))?;

        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .await
            .map_err(|e| format!("Failed to read file '{}': {}", file_path, e))?;

        let lines: Vec<&str> = contents.lines().collect();
        let mut matches = Vec::new();
        let mut match_count = 0;

        for (line_num, line) in lines.iter().enumerate() {
            if regex.is_match(line) {
                match output_mode {
                    "content" => {
                        matches.push(format!("{}:{}:{}", file_path, line_num + 1, line));
                    }
                    "count" => {
                        match_count += 1;
                    }
                    "files_with_matches" => {
                        results.push(file_path.to_string());
                        return Ok(());
                    }
                    _ => {
                        matches.push(format!("{}:{}:{}", file_path, line_num + 1, line));
                    }
                }
            }
        }

        match output_mode {
            "count" => {
                if match_count > 0 {
                    results.push(format!("{}:{}", file_path, match_count));
                }
            }
            "files_with_matches" => {
                // Already handled above
            }
            _ => {
                results.extend(matches);
            }
        }

        Ok(())
    }

    fn find_files_to_search(
        &self,
        path: &str,
        glob_pattern: Option<&str>,
    ) -> Result<Vec<String>, String> {
        let search_path = Path::new(path);

        let mut files = Vec::new();

        if let Some(glob_pat) = glob_pattern {
            let full_pattern = if Path::new(glob_pat).is_absolute() {
                glob_pat.to_string()
            } else {
                format!("{}/{}", path, glob_pat)
            };

            let matches =
                glob::glob(&full_pattern).map_err(|e| format!("Invalid glob pattern: {}", e))?;

            for entry in matches {
                match entry {
                    Ok(path) => {
                        if path.is_file() {
                            if let Some(path_str) = path.into_os_string().into_string().ok() {
                                files.push(path_str);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Glob error: {}", e);
                    }
                }
            }
        } else {
            // Recursively find all files
            self.find_all_files(search_path, &mut files)?;
        }

        Ok(files)
    }

    fn find_all_files(&self, dir: &Path, files: &mut Vec<String>) -> Result<(), String> {
        let entries = std::fs::read_dir(dir)
            .map_err(|e| format!("Failed to read directory '{:?}': {}", dir, e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();

            if path.is_file() {
                if let Some(path_str) = path.into_os_string().into_string().ok() {
                    files.push(path_str);
                }
            } else if path.is_dir() {
                self.find_all_files(&path, files)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::fs;

    #[tokio::test]
    async fn test_grep_search_file() {
        let tool = GrepTool;

        // Create test file
        let test_file = "/tmp/test_grep.txt";
        fs::write(test_file, "Hello World\nHello Rust\nGoodbye World")
            .await
            .expect("Failed to create test file");

        let args = serde_json::json!({"pattern": "Hello", "path": test_file});
        let result = tool.execute(&args).await;

        assert!(result.is_ok());
        let result_str = result.unwrap();
        assert!(result_str.contains("Hello World"));
        assert!(result_str.contains("Hello Rust"));

        // Clean up
        fs::remove_file(test_file).await.ok();
    }

    #[tokio::test]
    async fn test_grep_count_mode() {
        let tool = GrepTool;

        // Create test file
        let test_file = "/tmp/test_grep_count.txt";
        fs::write(test_file, "Hello World\nHello Rust\nHello Test")
            .await
            .expect("Failed to create test file");

        let args =
            serde_json::json!({"pattern": "Hello", "path": test_file, "output_mode": "count"});
        let result = tool.execute(&args).await;

        assert!(result.is_ok());
        let result_str = result.unwrap();
        assert!(result_str.contains(":3"));

        // Clean up
        fs::remove_file(test_file).await.ok();
    }

    #[tokio::test]
    async fn test_grep_missing_pattern() {
        let tool = GrepTool;
        let args = serde_json::json!({});
        assert_eq!(
            tool.execute(&args).await,
            Err("Missing 'pattern' argument".to_string())
        );
    }

    #[tokio::test]
    async fn test_grep_invalid_regex() {
        let tool = GrepTool;
        let args = serde_json::json!({"pattern": "[invalid", "path": "/tmp/test.txt"});
        assert!(tool.execute(&args).await.is_err());
    }
}
