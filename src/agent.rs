use crate::config::AgentConfig;
use crate::error::Error;
use crate::ollama::Ollama;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

pub(crate) struct Agent {
    pub config: AgentConfig,
    pub ollama: Ollama,
    pub workdir: PathBuf,
    pub messages: Vec<Message>,
}

impl Agent {
    pub async fn load_from_config(workdir: PathBuf) -> Result<Self, Error> {
        let config_file = ".ariste/settings.json";
        let config = if !tokio::fs::try_exists(&config_file).await? {
            AgentConfig::default()
        } else {
            let buf = tokio::fs::read(&config_file).await?;
            serde_json::from_slice(&buf)?
        };

        let url = if let Some(ollama) = &config.ollama
            && let Some(base) = ollama.base.as_deref()
        {
            format!("{}/api/chat", base)
        } else {
            "http://localhost:11434/api/chat".to_string()
        };

        let ollama = Ollama::new().url(url).think(false);
        Ok(Self {
            config,
            workdir,
            ollama,
            messages: Vec::new(),
        })
    }

    pub async fn invoke(&mut self, prompt: &str) -> Result<String, Error> {
        // 添加用户消息到历史
        self.messages.push(Message {
            role: "user".to_string(),
            content: prompt.to_string(),
        });

        // 使用完整的消息历史调用 Ollama
        let response = self.ollama.execute_with_messages("qwen3-vl:32b", &self.messages).await?;

        // 添加助手回复到历史
        self.messages.push(Message {
            role: "assistant".to_string(),
            content: response.clone(),
        });

        Ok(response)
    }

    pub fn clear_history(&mut self) {
        self.messages.clear();
    }

    pub async fn quit(&mut self) -> Result<(), Error> {
        Ok(())
    }
}
