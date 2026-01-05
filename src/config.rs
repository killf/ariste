use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct AgentConfig {
    pub ollama: Option<OllamaConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct OllamaConfig {
    pub base: Option<String>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            ollama: Some(OllamaConfig {
                base: Some("http://127.0.0.1:11434".to_string()),
            }),
        }
    }
}
