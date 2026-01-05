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

/// Enum representing all available tools
pub enum Tool {
    Calculator(CalculatorTool),
    Bash(BashTool),
}

impl Tool {
    /// Get the tool definition
    pub fn definition(&self) -> ToolDefinition {
        match self {
            Tool::Calculator(tool) => tool.definition(),
            Tool::Bash(tool) => tool.definition(),
        }
    }

    /// Execute the tool
    pub async fn execute(&self, arguments: &Value) -> Result<String, String> {
        match self {
            Tool::Calculator(tool) => tool.execute(arguments).await,
            Tool::Bash(tool) => tool.execute(arguments).await,
        }
    }

    /// Get the tool name
    pub fn name(&self) -> String {
        self.definition().function.name
    }
}

/// Trait that all tools must implement
pub trait ToolImpl: Send + Sync {
    /// Returns the tool definition for the AI model
    fn definition(&self) -> ToolDefinition;

    /// Executes the tool with the given arguments
    async fn execute(&self, arguments: &Value) -> Result<String, String>;
}

// Import the actual tool implementations
pub use crate::tools::calculator::CalculatorTool;
pub use crate::tools::bash::BashTool;
