use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct AgentConfig {
    pub provider: Option<String>,
    pub base: Option<String>,
    pub model: Option<String>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            provider: Some("ollama".to_string()),
            base: Some("http://127.0.0.1:11434".to_string()),
            model: Some("qwen3".to_string()),
        }
    }
}
