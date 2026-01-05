use crate::agent::message::Message;
use crate::config::AgentConfig;
use crate::error::Error;
use crate::llm::Ollama;
use crate::tools::{BashTool, EditTool, GlobTool, GrepTool, ReadTool, TodoWriteTool, Tool, ToolDefinition, WebFetchTool, WriteTool};
use crate::ui::UI;
use serde_json::Value;

/// Types of subagents that can be spawned
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubAgentType {
    /// General-purpose agent for complex tasks
    GeneralPurpose,
    /// Fast agent for exploring codebases
    Explore,
    /// Software architect agent for designing implementation plans
    Plan,
}

impl SubAgentType {
    fn description(&self) -> &str {
        match self {
            SubAgentType::GeneralPurpose => "General-purpose agent for complex tasks",
            SubAgentType::Explore => "Fast agent for exploring codebases",
            SubAgentType::Plan => "Software architect agent for designing implementation plans",
        }
    }

    fn system_prompt(&self) -> Option<&'static str> {
        match self {
            SubAgentType::Explore => Some(
                "You are a codebase exploration agent. Your goal is to quickly find files, \
                 search code, and answer questions about the codebase structure. \
                 Be thorough but efficient in your exploration.",
            ),
            SubAgentType::Plan => Some(
                "You are a software architect agent. Your goal is to design implementation plans \
                 by exploring the codebase and providing step-by-step plans. Focus on: \
                 1) Understanding existing patterns, 2) Identifying critical files, \
                 3) Considering architectural trade-offs.",
            ),
            SubAgentType::GeneralPurpose => None,
        }
    }
}

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
        let todo_write = Tool::TodoWrite(TodoWriteTool);
        let todo_write_def = todo_write.definition();
        let tools: Vec<Tool> = vec![bash, read, write, glob, grep, edit, web_fetch, todo_write];
        let tool_definitions = vec![bash_def, read_def, write_def, glob_def, grep_def, edit_def, web_fetch_def, todo_write_def];

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

    /// Spawn a subagent to handle a specialized task
    pub async fn spawn_task(
        &mut self,
        subagent_type: SubAgentType,
        description: &str,
        prompt: &str,
    ) -> Result<String, Error> {
        // Build the messages
        let mut messages = Vec::new();

        // Add system prompt if applicable
        if let Some(system_prompt) = subagent_type.system_prompt() {
            messages.push(Message {
                role: "system".to_string(),
                content: system_prompt.to_string(),
                tool_calls: None,
                tool_call_id: None,
            });
        }

        // Build the full prompt
        let full_prompt = format!("Task: {}\n\nDetails:\n{}", description, prompt);

        // Add user message
        messages.push(Message {
            role: "user".to_string(),
            content: full_prompt,
            tool_calls: None,
            tool_call_id: None,
        });

        // Get model to use
        let model = self
            .config
            .model
            .as_deref()
            .unwrap_or("qwen3-vl:32b");

        // Create Ollama instance without tools (to avoid recursion)
        let ollama = Ollama::new()
            .stream(false)
            .think(false);

        // Execute the task
        let response = ollama
            .execute_with_messages(model, &messages)
            .await?;

        // Format output
        let output = format!(
            "=== Subagent Task: {} ===\nType: {}\nModel: {}\n\n{}\n\n=== Task Complete ===",
            description,
            subagent_type.description(),
            model,
            response.content
        );

        Ok(output)
    }

    pub async fn quit(&mut self) -> Result<(), Error> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subagent_type_descriptions() {
        assert!(SubAgentType::GeneralPurpose.description().contains("General-purpose"));
        assert!(SubAgentType::Explore.description().contains("explor"));
        assert!(SubAgentType::Plan.description().contains("architect"));
    }

    #[test]
    fn test_subagent_type_system_prompts() {
        // GeneralPurpose has no system prompt
        assert!(SubAgentType::GeneralPurpose.system_prompt().is_none());

        // Explore has a system prompt
        let explore_prompt = SubAgentType::Explore.system_prompt();
        assert!(explore_prompt.is_some());
        assert!(explore_prompt.unwrap().contains("exploration agent"));

        // Plan has a system prompt
        let plan_prompt = SubAgentType::Plan.system_prompt();
        assert!(plan_prompt.is_some());
        assert!(plan_prompt.unwrap().contains("architect agent"));
    }

    #[tokio::test]
    async fn test_spawn_task_general_purpose() {
        let mut agent = Agent::load_from_config()
            .await
            .expect("Failed to load agent");

        let result = agent
            .spawn_task(
                SubAgentType::GeneralPurpose,
                "简单计算",
                "1 + 1 等于多少？",
            )
            .await;

        // This test requires Ollama to be running
        // Skip in CI or when Ollama is not available
        if result.is_ok() {
            let output = result.unwrap();
            assert!(output.contains("Subagent Task"));
            assert!(output.contains("简单计算"));
        }
    }

    #[tokio::test]
    async fn test_spawn_task_explore() {
        let mut agent = Agent::load_from_config()
            .await
            .expect("Failed to load agent");

        let result = agent
            .spawn_task(
                SubAgentType::Explore,
                "测试探索",
                "当前目录有哪些文件？",
            )
            .await;

        // Requires Ollama to be running
        if result.is_ok() {
            let output = result.unwrap();
            assert!(output.contains("Test Task"));
            assert!(output.contains("Type: Fast agent"));
        }
    }

    #[tokio::test]
    async fn test_spawn_task_plan() {
        let mut agent = Agent::load_from_config()
            .await
            .expect("Failed to load agent");

        let result = agent
            .spawn_task(
                SubAgentType::Plan,
                "测试规划",
                "如何实现一个简单的计数器？",
            )
            .await;

        // Requires Ollama to be running
        if result.is_ok() {
            let output = result.unwrap();
            assert!(output.contains("Test Task"));
            assert!(output.contains("Type: Software architect"));
            // Plan agent should provide structured response
            assert!(output.len() > 50);
        }
    }
}
