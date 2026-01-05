#![allow(unused)]
use crate::agent::Message;
use crate::error::Error;
use crate::utils::load_image_as_base64;
use crate::tools::ToolDefinition;
use crate::ui::UI;
use colored::Colorize;
use futures_util::StreamExt;
use serde_json::{json, Value};
use std::io::{stdout, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

#[derive(Debug, Clone)]
pub struct OllamaResponse {
    pub content: String,
    pub tool_calls: Option<Vec<Value>>,
}

#[derive(Debug)]
pub struct Ollama {
    pub url: Option<String>,
    pub stream: bool,
    pub verbose: bool,
    pub think: bool,
    pub tools: Option<Vec<ToolDefinition>>,
}

impl Ollama {
    pub fn new() -> Self {
        Ollama {
            url: None,
            stream: true,
            verbose: true,
            think: true,
            tools: None,
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

    pub fn tools(mut self, tools: Vec<ToolDefinition>) -> Self {
        self.tools = Some(tools);
        self
    }

    pub async fn execute(&self, model: &str, prompt: &str) -> Result<OllamaResponse, Error> {
        let mut payload = json!({
            "model": model,
            "messages": [{
                "role": "user",
                "content": prompt
            }],
            "stream": self.stream,
            "think": self.think
        });

        // Add tools if available
        if let Some(tools) = &self.tools {
            payload["tools"] = serde_json::to_value(tools).unwrap();
        }

        self.execute_impl(&payload).await
    }

    pub async fn execute_with_messages(&self, model: &str, messages: &[Message]) -> Result<OllamaResponse, Error> {
        let mut payload = json!({
            "model": model,
            "messages": messages,
            "stream": self.stream,
            "think": self.think
        });

        // Add tools if available
        if let Some(tools) = &self.tools {
            payload["tools"] = serde_json::to_value(tools).unwrap();
        }

        self.execute_impl(&payload).await
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

        let response = self.execute_impl(&json!({
            "model": model,
            "prompt": prompt,
            "images": image_list,
            "stream": self.stream,
            "think": self.think
        }))
        .await?;

        Ok(response.content)
    }

    async fn execute_impl(&self, payload: &serde_json::Value) -> Result<OllamaResponse, Error> {
        let client = reqwest::Client::new();

        let url = self.url.as_deref().unwrap_or("http://localhost:11434/api/chat");
        let resp = client.post(url).json(payload).send().await?;

        let mut status = 0;
        let mut response = String::new();
        let mut thinking_buffer = String::new();
        let mut tool_calls_buffer: Vec<Value> = Vec::new();
        let mut stream = resp.bytes_stream();

        // 启动 spinner
        let spinner_running = Arc::new(AtomicBool::new(true));
        let spinner_running_clone = spinner_running.clone();

        // 在异步任务中运行 spinner
        tokio::spawn(async move {
            let mut ui = UI::new();
            while spinner_running_clone.load(Ordering::Relaxed) {
                ui.thinking_start();
                sleep(Duration::from_millis(150)).await;
            }
        });

        while let Some(chunk) = stream.next().await {
            if let Ok(bytes) = chunk
                && let Ok(text) = std::str::from_utf8(&bytes)
                && let Ok(resp) = serde_json::from_str::<serde_json::Value>(text)
            {
                // Check for tool_calls
                if let Some(message) = resp.get("message") {
                    if let Some(tool_calls) = message.get("tool_calls") {
                        if let Some(calls) = tool_calls.as_array() {
                            for call in calls {
                                tool_calls_buffer.push(call.clone());
                            }
                        }
                    }
                }

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
                                // 停止 spinner 并清除行
                                spinner_running.store(false, Ordering::Relaxed);
                                sleep(Duration::from_millis(50)).await;
                                UI::clear_line();

                                // 显示思考块开始
                                UI::thinking_block_start();
                                status = 1;
                            }

                            // 累积思考内容
                            thinking_buffer.push_str(fragment);

                            // 处理buffer中的所有完整行
                            while let Some(newline_pos) = thinking_buffer.find('\n') {
                                let line = &thinking_buffer[..newline_pos];
                                UI::thinking_block_content(line);
                                // 移除已处理的行（包括换行符）
                                thinking_buffer = thinking_buffer[newline_pos + 1..].to_string();
                            }
                        }

                        continue;
                    }

                    if let Some(fragment) = message.get("content")
                        && let Some(fragment) = fragment.as_str()
                    {
                        if self.verbose {
                            if status == 0 {
                                // 还没有看到 thinking，直接停止 spinner
                                spinner_running.store(false, Ordering::Relaxed);
                                sleep(Duration::from_millis(50)).await;
                                UI::clear_line();
                                UI::response_start();
                            } else if status == 1 {
                                // 完成思考块
                                if !thinking_buffer.is_empty() {
                                    UI::thinking_block_content(&thinking_buffer);
                                    thinking_buffer.clear();
                                }
                                UI::thinking_block_end();
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

        // 停止 spinner
        spinner_running.store(false, Ordering::Relaxed);

        if self.verbose {
            print!("\n");
            drop(stdout().flush());
        }

        Ok(OllamaResponse {
            content: response,
            tool_calls: if tool_calls_buffer.is_empty() { None } else { Some(tool_calls_buffer) },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ollama() {
        let result = Ollama::new().execute("qwen3-vl:32b", "1+2=").await;
        assert!(result.is_ok());
        // The response should contain content
        assert!(!result.unwrap().content.is_empty());
    }

    #[tokio::test]
    async fn test_ollama_with_image() {
        let images = ["http://172.16.200.202:9000/api/view?filename=ComfyUI_00811_.png&subfolder=&type=output"];
        let result = Ollama::new().execute_with_image("qwen3-vl:32b", "描述一下这张图片", &images).await;
        assert!(result.is_ok());
    }
}
