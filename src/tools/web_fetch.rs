use crate::tools::types::ToolImpl;
use crate::tools::types::{ToolDefinition, FunctionDefinition, ParametersSchema};
use serde_json::Value;

/// WebFetch tool for fetching web content
pub struct WebFetchTool;

impl ToolImpl for WebFetchTool {
    fn definition(&self) -> ToolDefinition {
        let mut properties = serde_json::Map::new();
        properties.insert(
            "url".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "The URL to fetch content from (e.g., 'https://example.com', 'https://api.github.com/repos')"
            }),
        );
        properties.insert(
            "timeout".to_string(),
            serde_json::json!({
                "type": "integer",
                "description": "Request timeout in seconds. Default is 30 seconds."
            }),
        );
        properties.insert(
            "method".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "HTTP method to use. Default is GET."
            }),
        );
        properties.insert(
            "headers".to_string(),
            serde_json::json!({
                "type": "object",
                "description": "Optional HTTP headers to include in the request (e.g., {'Authorization': 'Bearer token', 'Content-Type': 'application/json'})"
            }),
        );
        properties.insert(
            "body".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "Optional request body for POST/PUT requests."
            }),
        );

        ToolDefinition {
            r#type: "function".to_string(),
            function: FunctionDefinition {
                name: "web_fetch".to_string(),
                description: "Fetch content from a URL. Supports various HTTP methods, custom headers, and timeouts. Returns the response body as text. Useful for retrieving web pages, API responses, or online resources.".to_string(),
                parameters: ParametersSchema {
                    r#type: "object".to_string(),
                    properties,
                    required: vec!["url".to_string()],
                },
            },
        }
    }

    async fn execute(&self, arguments: &Value) -> Result<String, String> {
        let url = arguments
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Missing 'url' argument".to_string())?;

        let timeout_secs = arguments
            .get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(30);

        let method = arguments
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("GET");

        let timeout_duration = std::time::Duration::from_secs(timeout_secs);

        // Build HTTP client
        let client = reqwest::Client::builder()
            .timeout(timeout_duration)
            .build()
            .map_err(|e| format!("Failed to build HTTP client: {}", e))?;

        // Build request
        let mut request = match method.to_uppercase().as_str() {
            "GET" => client.get(url),
            "POST" => client.post(url),
            "PUT" => client.put(url),
            "DELETE" => client.delete(url),
            "PATCH" => client.patch(url),
            "HEAD" => client.head(url),
            _ => {
                return Err(format!("Unsupported HTTP method: {}", method));
            }
        };

        // Add headers if provided
        if let Some(headers) = arguments.get("headers").and_then(|v| v.as_object()) {
            for (key, value) in headers {
                if let Some(header_value) = value.as_str() {
                    request = request.header(key, header_value);
                }
            }
        }

        // Add body if provided
        if let Some(body) = arguments.get("body").and_then(|v| v.as_str()) {
            request = request.body(body.to_string());
        }

        // Execute request
        let response = request
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        let status = response.status();
        let url_final = response.url().clone();

        let body = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response body: {}", e))?;

        // Return formatted result
        Ok(format!(
            "Status: {}\nURL: {}\n\n{}",
            status, url_final, body
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_web_fetch_simple() {
        let tool = WebFetchTool;

        // Use a reliable test endpoint
        let args = serde_json::json!({
            "url": "https://httpbin.org/get",
            "timeout": 10
        });

        let result = tool.execute(&args).await;
        assert!(result.is_ok());
        let result_str = result.unwrap();
        assert!(result_str.contains("Status:"));
        assert!(result_str.contains("URL:"));
    }

    #[tokio::test]
    async fn test_web_fetch_missing_url() {
        let tool = WebFetchTool;
        let args = serde_json::json!({});
        assert_eq!(
            tool.execute(&args).await,
            Err("Missing 'url' argument".to_string())
        );
    }

    #[tokio::test]
    async fn test_web_fetch_invalid_url() {
        let tool = WebFetchTool;
        let args = serde_json::json!({"url": "not-a-valid-url"});
        assert!(tool.execute(&args).await.is_err());
    }

    #[tokio::test]
    async fn test_web_fetch_with_headers() {
        let tool = WebFetchTool;

        let args = serde_json::json!({
            "url": "https://httpbin.org/headers",
            "headers": {
                "X-Custom-Header": "test-value"
            }
        });

        let result = tool.execute(&args).await;
        assert!(result.is_ok());
    }
}
