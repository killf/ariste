use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Tool definition that describes available tools to the AI model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub r#type: String,
    pub function: FunctionDefinition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: ParametersSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParametersSchema {
    pub r#type: String,
    pub properties: serde_json::Map<String, Value>,
    pub required: Vec<String>,
}

/// Tool call request from the AI model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: Value,
}

/// Result of a tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub name: String,
    pub result: String,
}

/// Trait that all tools must implement
pub trait Tool: Send + Sync {
    /// Returns the tool definition for the AI model
    fn definition(&self) -> ToolDefinition;

    /// Executes the tool with the given arguments
    fn execute(&self, arguments: &Value) -> Result<String, String>;
}
