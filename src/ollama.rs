#![allow(unused)]
use crate::error::Error;
use crate::image::load_image_as_base64;
use colored::Colorize;
use futures_util::StreamExt;
use serde_json::json;
use std::io::{stdout, Write};

#[derive(Debug)]
pub struct Ollama {
    pub url: Option<String>,
    pub stream: bool,
    pub verbose: bool,
    pub think: bool,
}

impl Ollama {
    pub fn new() -> Self {
        Ollama {
            url: None,
            stream: true,
            verbose: true,
            think: true,
        }
    }

    pub fn url(mut self, url: String) -> Self {
        self.url = Some(url);
        self
    }

    pub fn stream(mut self, stream: bool) -> Self {
        self.stream = stream;
        self
    }

    pub fn verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    pub fn think(mut self, think: bool) -> Self {
        self.think = think;
        self
    }

    pub async fn execute(&self, model: &str, prompt: &str) -> Result<String, Error> {
        self.execute_impl(&json!({
            "model": model,
            "messages": [{
                "role": "user",
                "content": prompt
            }],
            "stream": self.stream,
            "think": self.think
        }))
        .await
    }

    pub async fn execute_with_image<I, E>(&self, model: &str, prompt: &str, images: I) -> Result<String, Error>
    where
        I: IntoIterator<Item = E>,
        E: AsRef<str>,
    {
        let mut image_list = Vec::new();
        for image_url in images {
            let image_url = image_url.as_ref();
            if image_url.starts_with("http://") || image_url.starts_with("https://") {
                image_list.push(load_image_as_base64(image_url).await?);
            } else {
                image_list.push(image_url.to_string());
            }
        }

        self.execute_impl(&json!({
            "model": model,
            "prompt": prompt,
            "images": image_list,
            "stream": self.stream,
            "think": self.think
        }))
        .await
    }

    async fn execute_impl(&self, payload: &serde_json::Value) -> Result<String, Error> {
        let client = reqwest::Client::new();

        let url = self.url.as_deref().unwrap_or("http://localhost:11434/api/chat");
        let resp = client.post(url).json(payload).send().await?;

        let mut status = 0;
        let mut response = String::new();
        let mut stream = resp.bytes_stream();

        while let Some(chunk) = stream.next().await {
            if let Ok(bytes) = chunk
                && let Ok(text) = std::str::from_utf8(&bytes)
                && let Ok(resp) = serde_json::from_str::<serde_json::Value>(text)
            {
                if let Some(done) = resp.get("done")
                    && let Some(done) = done.as_bool()
                    && done
                {
                    break;
                }

                if let Some(message) = resp.get("message") {
                    if let Some(fragment) = message.get("thinking")
                        && let Some(fragment) = fragment.as_str()
                    {
                        if self.verbose {
                            if status == 0 {
                                println!("{}", "<thinking>".cyan());
                                status = 1;
                            }

                            print!("{}", fragment.cyan());
                            drop(stdout().flush());
                        }

                        continue;
                    }

                    if let Some(fragment) = message.get("content")
                        && let Some(fragment) = fragment.as_str()
                    {
                        if self.verbose {
                            if status == 1 {
                                println!("{}", "\n</thinking>".cyan());
                                status = 2;
                            }

                            print!("{}", fragment);
                            drop(stdout().flush());
                        }

                        response.push_str(fragment);
                        continue;
                    }
                }
            }
        }

        if self.verbose {
            print!("\n");
            drop(stdout().flush());
        }

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_ollama() {
        let result = Ollama::new().execute("qwen3-vl:32b", "1+2=").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_ollama_with_image() {
        let images = ["http://172.16.200.202:9000/api/view?filename=ComfyUI_00811_.png&subfolder=&type=output"];
        let result = Ollama::new().execute_with_image("qwen3-vl:32b", "描述一下这张图片", &images).await;
        assert!(result.is_ok());
    }
}
