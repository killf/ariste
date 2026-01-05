use crate::agent::message::Message;
use crate::config::AgentConfig;
use crate::error::Error;
use crate::llm::Ollama;
use crate::tools::{BashTool, EditTool, GlobTool, GrepTool, ReadTool, Tool, ToolDefinition, WebFetchTool, WriteTool};
use crate::ui::UI;
use serde_json::Value;

pub struct Agent {
    pub config: AgentConfig,
    pub ollama: Ollama,
    pub messages: Vec<Message>,
    pub tools: Vec<Tool>,
    pub tool_definitions: Vec<ToolDefinition>,
}

impl Agent {
    pub async fn load_from_config() -> Result<Self, Error> {
        let config_file = ".ariste/settings.json";
        let config = if !tokio::fs::try_exists(&config_file).await? {
            AgentConfig::default()
        } else {
            let buf = tokio::fs::read(&config_file).await?;
            serde_json::from_slice(&buf)?
        };

        let url = if let Some(base) = &config.base {
            format!("{}/api/chat", base)
        } else {
            "http://localhost:11434/api/chat".to_string()
        };

        // Register tools
        let bash = Tool::Bash(BashTool);
        let bash_def = bash.definition();
        let read = Tool::Read(ReadTool);
        let read_def = read.definition();
        let write = Tool::Write(WriteTool);
        let write_def = write.definition();
        let glob = Tool::Glob(GlobTool);
        let glob_def = glob.definition();
        let grep = Tool::Grep(GrepTool);
        let grep_def = grep.definition();
        let edit = Tool::Edit(EditTool);
        let edit_def = edit.definition();
        let web_fetch = Tool::WebFetch(WebFetchTool);
        let web_fetch_def = web_fetch.definition();
        let tools: Vec<Tool> = vec![bash, read, write, glob, grep, edit, web_fetch];
        let tool_definitions = vec![bash_def, read_def, write_def, glob_def, grep_def, edit_def, web_fetch_def];

        let tool_defs_for_ollama = tool_definitions.clone();
        let ollama = Ollama::new()
            .url(url)
            .think(false)
            .tools(tool_defs_for_ollama);

        Ok(Self {
            config,
            ollama,
            messages: Vec::new(),
            tools,
            tool_definitions,
        })
    }

    pub async fn invoke(&mut self, prompt: &str) -> Result<(), Error> {
        // 添加用户消息到历史
        self.messages.push(Message {
            role: "user".to_string(),
            content: prompt.to_string(),
            tool_calls: None,
            tool_call_id: None,
        });

        // Tool calling 循环
        let max_iterations = 5;
        let mut iteration = 0;

        loop {
            iteration += 1;
            if iteration > max_iterations {
                return Err(Error::Message("Too many tool call iterations".to_string()));
            }

            // 使用完整的消息历史调用 Ollama
            let model = self.config.model.as_deref().unwrap_or("qwen3-vl:32b");
            let ollama_response = self
                .ollama
                .execute_with_messages(model, &self.messages)
                .await?;

            // 检查是否有 tool calls
            if let Some(tool_calls) = ollama_response.tool_calls {
                // 添加助手消息（包含 tool_calls）到历史
                self.messages.push(Message {
                    role: "assistant".to_string(),
                    content: ollama_response.content.clone(),
                    tool_calls: Some(tool_calls.clone()),
                    tool_call_id: None,
                });

                // 执行每个工具调用
                for tool_call in &tool_calls {
                    if let Some(function) = tool_call.get("function") {
                        let name = function
                            .get("name")
                            .and_then(|v: &Value| v.as_str())
                            .unwrap_or("");
                        let default_args = serde_json::json!({});
                        let arguments = function.get("arguments").unwrap_or(&default_args);

                        // 获取 tool_call_id
                        let tool_call_id = tool_call
                            .get("id")
                            .and_then(|v: &Value| v.as_str())
                            .unwrap_or("");

                        // 查找并执行工具
                        let result = self.execute_tool(name, arguments).await?;

                        // 将工具结果作为 tool 角色的消息添加到历史
                        self.messages.push(Message {
                            role: "tool".to_string(),
                            content: result,
                            tool_calls: None,
                            tool_call_id: Some(tool_call_id.to_string()),
                        });
                    }
                }

                // 继续循环，让模型基于工具结果生成最终回复
                continue;
            } else {
                // 没有 tool calls，这是最终回复
                self.messages.push(Message {
                    role: "assistant".to_string(),
                    content: ollama_response.content.clone(),
                    tool_calls: None,
                    tool_call_id: None,
                });

                return Ok(());
            }
        }
    }

    async fn execute_tool(&self, name: &str, arguments: &Value) -> Result<String, Error> {
        for tool in &self.tools {
            if tool.definition().function.name == name {
                // 显示工具开始执行的消息
                let args_str = if arguments.is_null() {
                    None
                } else {
                    Some(serde_json::to_string_pretty(arguments).unwrap_or_default())
                };
                UI::tool_start(name, args_str.as_deref());

                // 执行工具
                let result = match tool.execute(arguments).await {
                    Ok(result) => result,
                    Err(e) => {
                        // 显示工具执行错误
                        UI::tool_error(&e);
                        return Err(Error::Message(format!("Tool execution error: {}", e)));
                    }
                };

                // 显示工具执行结果
                UI::tool_content(&result);
                UI::tool_end();

                return Ok(result);
            }
        }
        Err(Error::Message(format!("Tool not found: {}", name)))
    }

    pub fn clear_history(&mut self) {
        self.messages.clear();
    }

    pub async fn quit(&mut self) -> Result<(), Error> {
        Ok(())
    }
}
