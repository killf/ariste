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

/// Enum representing all available tools
pub enum Tool {
    Bash(BashTool),
    Read(ReadTool),
    Write(WriteTool),
    Glob(GlobTool),
    Grep(GrepTool),
    Edit(EditTool),
    WebFetch(WebFetchTool),
    TodoWrite(TodoWriteTool),
    Task(TaskTool),
}

impl Tool {
    /// Get the tool definition
    pub fn definition(&self) -> ToolDefinition {
        match self {
            Tool::Bash(tool) => tool.definition(),
            Tool::Read(tool) => tool.definition(),
            Tool::Write(tool) => tool.definition(),
            Tool::Glob(tool) => tool.definition(),
            Tool::Grep(tool) => tool.definition(),
            Tool::Edit(tool) => tool.definition(),
            Tool::WebFetch(tool) => tool.definition(),
            Tool::TodoWrite(tool) => tool.definition(),
            Tool::Task(tool) => tool.definition(),
        }
    }

    /// Execute the tool
    pub async fn execute(&self, arguments: &Value) -> Result<String, String> {
        match self {
            Tool::Bash(tool) => tool.execute(arguments).await,
            Tool::Read(tool) => tool.execute(arguments).await,
            Tool::Write(tool) => tool.execute(arguments).await,
            Tool::Glob(tool) => tool.execute(arguments).await,
            Tool::Grep(tool) => tool.execute(arguments).await,
            Tool::Edit(tool) => tool.execute(arguments).await,
            Tool::WebFetch(tool) => tool.execute(arguments).await,
            Tool::TodoWrite(tool) => tool.execute(arguments).await,
            Tool::Task(tool) => tool.execute(arguments).await,
        }
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
pub use crate::tools::bash::BashTool;
pub use crate::tools::read::ReadTool;
pub use crate::tools::write::WriteTool;
pub use crate::tools::glob::GlobTool;
pub use crate::tools::grep::GrepTool;
pub use crate::tools::edit::EditTool;
pub use crate::tools::web_fetch::WebFetchTool;
pub use crate::tools::todo_write::TodoWriteTool;
pub use crate::tools::task::TaskTool;
