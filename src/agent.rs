use crate::config::AgentConfig;
use crate::error::Error;
use crate::ollama::Ollama;
use std::path::PathBuf;

pub(crate) struct Agent {
    pub config: AgentConfig,
    pub ollama: Ollama,
    pub workdir: PathBuf,
}

impl Agent {
    pub async fn load_from_config(workdir: PathBuf) -> Result<Self, Error> {
        let config_file = ".ariste/config.json";
        let config = if !tokio::fs::try_exists(&config_file).await? {
            AgentConfig::default()
        } else {
            let buf = tokio::fs::read(".ariste/config.json").await?;
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
        })
    }

    pub async fn invoke(&mut self, prompt: &str) -> Result<String, Error> {
        self.ollama.execute("qwen3-vl:32b", prompt).await
    }

    pub async fn quit(&mut self) -> Result<(), Error> {
        Ok(())
    }
}
