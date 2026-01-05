use crate::agent::message::Message;
use crate::config::AgentConfig;
use crate::error::Error;
use crate::llm::Ollama;
use crate::tools::{BashTool, EditTool, GlobTool, GrepTool, ReadTool, TaskTool, TodoWriteTool, Tool, ToolDefinition, WebFetchTool, WriteTool};
use crate::ui::UI;
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Execution status of a subagent task
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum SubAgentStatus {
    Pending,
    Running,
    Completed,
    Failed(String),
}

/// Execution tracking for subagent tasks
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SubAgentExecution {
    pub id: usize,
    pub task: SubAgentTask,
    pub status: SubAgentStatus,
    pub start_time: Option<Instant>,
    pub end_time: Option<Instant>,
    pub result: Option<String>,
}

#[allow(dead_code)]
impl SubAgentExecution {
    pub fn new(id: usize, task: SubAgentTask) -> Self {
        Self {
            id,
            task,
            status: SubAgentStatus::Pending,
            start_time: None,
            end_time: None,
            result: None,
        }
    }

    pub fn start(&mut self) {
        self.status = SubAgentStatus::Running;
        self.start_time = Some(Instant::now());
    }

    pub fn complete(&mut self, result: String) {
        self.status = SubAgentStatus::Completed;
        self.end_time = Some(Instant::now());
        self.result = Some(result);
    }

    pub fn fail(&mut self, error: String) {
        self.status = SubAgentStatus::Failed(error);
        self.end_time = Some(Instant::now());
    }

    pub fn duration(&self) -> Option<Duration> {
        match (self.start_time, self.end_time) {
            (Some(start), Some(end)) => Some(end.duration_since(start)),
            (Some(start), None) => Some(start.elapsed()),
            _ => None,
        }
    }
}

/// Global counter for subagent IDs
#[derive(Debug)]
#[allow(dead_code)]
pub struct SubAgentIdCounter(Arc<AtomicUsize>);

#[allow(dead_code)]
impl SubAgentIdCounter {
    pub fn new() -> Self {
        Self(Arc::new(AtomicUsize::new(0)))
    }

    pub fn next(&self) -> usize {
        self.0.fetch_add(1, Ordering::SeqCst)
    }
}

impl Default for SubAgentIdCounter {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for a subagent task
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SubAgentTask {
    pub subagent_type: SubAgentType,
    pub description: String,
    pub prompt: String,
    pub include_context: bool,
    pub include_tools: bool,
}

#[allow(dead_code)]
impl SubAgentTask {
    pub fn new(
        subagent_type: SubAgentType,
        description: impl Into<String>,
        prompt: impl Into<String>,
    ) -> Self {
        Self {
            subagent_type,
            description: description.into(),
            prompt: prompt.into(),
            include_context: false,
            include_tools: false,
        }
    }

    pub fn with_context(mut self, include: bool) -> Self {
        self.include_context = include;
        self
    }

    pub fn with_tools(mut self, include: bool) -> Self {
        self.include_tools = include;
        self
    }
}

/// Types of subagents that can be spawned
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubAgentType {
    /// General-purpose agent for complex tasks
    GeneralPurpose,
    /// Fast agent for exploring codebases
    Explore,
    /// Software architect agent for designing implementation plans
    Plan,
    /// Code reviewer agent for analyzing code quality
    CodeReview,
    /// Test runner agent for testing and validation
    TestRunner,
}

impl SubAgentType {
    fn description(&self) -> &str {
        match self {
            SubAgentType::GeneralPurpose => "General-purpose agent for complex tasks",
            SubAgentType::Explore => "Fast agent for exploring codebases",
            SubAgentType::Plan => "Software architect agent for designing implementation plans",
            SubAgentType::CodeReview => "Code reviewer agent for analyzing code quality",
            SubAgentType::TestRunner => "Test runner agent for testing and validation",
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
            SubAgentType::CodeReview => Some(
                "You are a code reviewer agent. Your goal is to analyze code quality, \
                 identify potential bugs, suggest improvements, and ensure best practices. \
                 Focus on: correctness, performance, security, and maintainability.",
            ),
            SubAgentType::TestRunner => Some(
                "You are a test runner agent. Your goal is to design and execute tests, \
                 validate functionality, and report issues. Be thorough in testing edge cases \
                 and providing actionable feedback.",
            ),
            SubAgentType::GeneralPurpose => None,
        }
    }

    /// Returns whether this subagent type should have access to tools
    fn uses_tools(&self) -> bool {
        match self {
            SubAgentType::Explore => true,
            SubAgentType::CodeReview => true,
            SubAgentType::TestRunner => true,
            SubAgentType::GeneralPurpose => true,
            SubAgentType::Plan => false, // Plan agent focuses on analysis
        }
    }
}

pub struct Agent {
    pub config: AgentConfig,
    pub ollama: Ollama,
    pub messages: Vec<Message>,
    pub tools: Vec<Tool>,
    #[allow(dead_code)]
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
        let task = Tool::Task(TaskTool);
        let task_def = task.definition();
        let tools: Vec<Tool> = vec![bash, read, write, glob, grep, edit, web_fetch, todo_write, task];
        let tool_definitions = vec![bash_def, read_def, write_def, glob_def, grep_def, edit_def, web_fetch_def, todo_write_def, task_def];

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
            let model = self.config.model.as_deref().unwrap_or("qwen3");
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

    /// Run a complete message loop for a subagent (used by Task tool)
    /// This allows the subagent to have multi-turn conversations and use tools
    pub async fn run_subagent_loop(
        &mut self,
        initial_messages: Vec<Message>,
        max_turns: usize,
    ) -> Result<String, Error> {
        // Set initial messages
        self.messages = initial_messages;

        let max_iterations = 5;
        let mut iteration = 0;
        let mut turn = 0;

        loop {
            turn += 1;
            if turn > max_turns {
                break; // Reached max turns, return current best result
            }

            iteration += 1;
            if iteration > max_iterations {
                return Err(Error::Message("Subagent: Too many iterations in one turn".to_string()));
            }

            // Call LLM
            let model = self.config.model.as_deref().unwrap_or("qwen3");
            let ollama_response = self
                .ollama
                .execute_with_messages(model, &self.messages)
                .await?;

            // Check for tool calls
            if let Some(tool_calls) = ollama_response.tool_calls {
                // Add assistant message
                self.messages.push(Message {
                    role: "assistant".to_string(),
                    content: ollama_response.content.clone(),
                    tool_calls: Some(tool_calls.clone()),
                    tool_call_id: None,
                });

                // Execute tools
                for tool_call in &tool_calls {
                    if let Some(function) = tool_call.get("function") {
                        let name = function
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let default_args = json!({});
                        let arguments = function.get("arguments").unwrap_or(&default_args);

                        let tool_call_id = tool_call
                            .get("id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");

                        // Subagents cannot spawn additional subagents (prevent infinite recursion)
                        if name == "task" {
                            let result = json!({
                                "error": "Subagents cannot spawn additional subagents",
                                "suggestion": "Complete the task yourself using available tools"
                            }).to_string();

                            self.messages.push(Message {
                                role: "tool".to_string(),
                                content: result,
                                tool_calls: None,
                                tool_call_id: Some(tool_call_id.to_string()),
                            });
                            continue;
                        }

                        // Execute tool
                        let result = match Box::pin(self.execute_tool(name, arguments)).await {
                            Ok(result) => result,
                            Err(e) => {
                                format!("Tool execution error: {}", e)
                            }
                        };

                        self.messages.push(Message {
                            role: "tool".to_string(),
                            content: result,
                            tool_calls: None,
                            tool_call_id: Some(tool_call_id.to_string()),
                        });
                    }
                }

                continue;
            } else {
                // No tool calls - add final response
                self.messages.push(Message {
                    role: "assistant".to_string(),
                    content: ollama_response.content.clone(),
                    tool_calls: None,
                    tool_call_id: None,
                });

                // Return the final response content
                return Ok(ollama_response.content);
            }
        }

        // If we exited due to max_turns, return the last assistant message
        if let Some(last_msg) = self.messages.iter().rev().find(|m| m.role == "assistant") {
            Ok(last_msg.content.clone())
        } else {
            Err(Error::Message("Subagent: No response generated".to_string()))
        }
    }

    async fn execute_tool(&mut self, name: &str, arguments: &Value) -> Result<String, Error> {
        // Special handling for Task tool
        if name == "task" {
            // Format a concise description for Task tool
            let display_args = if let Some(desc) = arguments.get("description").and_then(|v| v.as_str()) {
                Some(format!("\"{}\"", desc))
            } else {
                None
            };
            UI::tool_start("Task", display_args.as_deref());

            // Parse arguments
            let subagent_type_str = arguments
                .get("subagent_type")
                .and_then(|v| v.as_str())
                .unwrap_or("general-purpose");

            let subagent_type = match subagent_type_str {
                "general-purpose" => SubAgentType::GeneralPurpose,
                "explore" => SubAgentType::Explore,
                "plan" => SubAgentType::Plan,
                "code-review" => SubAgentType::CodeReview,
                "test-runner" => SubAgentType::TestRunner,
                _ => return Err(Error::Message(format!("Invalid subagent type: {}", subagent_type_str))),
            };

            let description = arguments
                .get("description")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::Message("Missing 'description' argument".to_string()))?;

            let prompt = arguments
                .get("prompt")
                .and_then(|v| v.as_str())
                .ok_or_else(|| Error::Message("Missing 'prompt' argument".to_string()))?;

            let include_tools = arguments
                .get("include_tools")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let start_time = Instant::now();

            UI::info(&format!(
                "🤖 Spawning {} subagent: {}",
                subagent_type.description(),
                description
            ));

            // Build initial messages for subagent
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

            messages.push(Message {
                role: "user".to_string(),
                content: full_prompt,
                tool_calls: None,
                tool_call_id: None,
            });

            // Create a new Agent instance for the subagent
            let mut subagent = Agent::load_from_config().await?;

            // Configure if subagent should use tools
            if !include_tools || !subagent_type.uses_tools() {
                // Remove tools from subagent
                subagent.ollama = Ollama::new()
                    .stream(false)
                    .think(false);
            }

            // Run the subagent's complete message loop
            // Allow multiple turns (default 10) for complex tasks
            let max_turns = 10;
            let result_content = subagent.run_subagent_loop(messages, max_turns).await?;

            let elapsed = start_time.elapsed();

            let output = json!({
                "task": description,
                "agent_type": subagent_type.description(),
                "model": subagent.config.model.as_deref().unwrap_or("qwen3"),
                "duration_ms": elapsed.as_millis(),
                "used_tools": include_tools && subagent_type.uses_tools(),
                "result": result_content,
            });

            let result = format!(
                "=== Subagent Task Complete ===\n{}",
                serde_json::to_string_pretty(&output).unwrap_or_default()
            );

            UI::tool_content(&result);
            UI::tool_end();

            return Ok(result);
        }

        // Regular tool execution
        for tool in &self.tools {
            if tool.definition().function.name == name {
                // Format display args - special handling for todo_write
                let display_args = if name == "todo_write" {
                    // For todo_write, show a clean header instead of JSON
                    Some("updated".to_string())
                } else if !arguments.is_null() {
                    Some(serde_json::to_string_pretty(arguments).unwrap_or_default())
                } else {
                    None
                };
                UI::tool_start(name, display_args.as_deref());

                // 执行工具
                let result = match tool.execute(arguments).await {
                    Ok(result) => result,
                    Err(e) => {
                        // 显示工具执行错误
                        UI::tool_error(&e);
                        return Err(Error::Message(format!("Tool execution error: {}", e)));
                    }
                };

                // 显示工具执行结果 - special handling for todo_write
                if name == "todo_write" {
                    // For todo_write, display with proper line breaks
                    println!();
                    for line in result.lines() {
                        println!("{}", line);
                    }
                } else {
                    UI::tool_content(&result);
                }
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
    #[allow(dead_code)]
    pub async fn spawn_task(
        &mut self,
        subagent_type: SubAgentType,
        description: &str,
        prompt: &str,
    ) -> Result<String, Error> {
        self.spawn_task_with_options(subagent_type, description, prompt, None, false).await
    }

    /// Spawn a subagent with additional options
    #[allow(dead_code)]
    pub async fn spawn_task_with_options(
        &mut self,
        subagent_type: SubAgentType,
        description: &str,
        prompt: &str,
        context_messages: Option<&[Message]>,
        include_tools: bool,
    ) -> Result<String, Error> {
        let start_time = Instant::now();

        UI::info(&format!(
            "🤖 Spawning {} subagent: {}",
            subagent_type.description(),
            description
        ));

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

        // Add context if provided (limit to last 10 messages to avoid overwhelming the agent)
        if let Some(context) = context_messages {
            let context_len = context.len();
            let start = if context_len > 10 { context_len - 10 } else { 0 };
            for msg in &context[start..] {
                // Skip system messages in context
                if msg.role != "system" {
                    messages.push(msg.clone());
                }
            }
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

        // Create a new Agent instance for the subagent
        let mut subagent = Agent::load_from_config().await?;

        // Configure if subagent should use tools
        if !include_tools || !subagent_type.uses_tools() {
            // Remove tools from subagent
            subagent.ollama = Ollama::new()
                .stream(false)
                .think(false);
        }

        // Run the subagent's complete message loop
        let max_turns = 10;
        let result_content = subagent.run_subagent_loop(messages, max_turns).await?;

        let elapsed = start_time.elapsed();

        // Format structured output
        let output = json!({
            "task": description,
            "agent_type": subagent_type.description(),
            "model": subagent.config.model.as_deref().unwrap_or("qwen3"),
            "duration_ms": elapsed.as_millis(),
            "used_tools": include_tools && subagent_type.uses_tools(),
            "result": result_content,
        });

        let formatted = format!(
            "=== Subagent Task Complete ===\n{}",
            serde_json::to_string_pretty(&output).unwrap_or_default()
        );

        UI::success(&format!("✓ Subagent completed in {:.2}s", elapsed.as_secs_f64()));

        Ok(formatted)
    }

    /// Spawn multiple subagent tasks concurrently
    #[allow(dead_code)]
    pub async fn spawn_multiple_tasks(&mut self, tasks: Vec<SubAgentTask>) -> Result<Vec<String>, Error> {
        use futures_util::future::join_all;

        let total = tasks.len();
        UI::info(&format!("🚀 Spawning {} subagent tasks concurrently...", total));

        let start_time = Instant::now();

        // Build futures for all tasks
        let mut futures = Vec::new();

        for task in tasks {
            let future = async move {
                let mut agent = Agent::load_from_config().await?;
                agent
                    .spawn_task(
                        task.subagent_type,
                        &task.description,
                        &task.prompt,
                    )
                    .await
            };
            futures.push(future);
        }

        // Execute all tasks concurrently
        let results = join_all(futures).await;

        let elapsed = start_time.elapsed();
        UI::success(&format!(
            "✓ All {} subagent tasks completed in {:.2}s",
            total,
            elapsed.as_secs_f64()
        ));

        // Collect results
        let mut outputs = Vec::new();
        for result in results {
            outputs.push(result?);
        }

        Ok(outputs)
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
        assert!(SubAgentType::CodeReview.description().contains("review"));
        assert!(SubAgentType::TestRunner.description().contains("test"));
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

        // CodeReview has a system prompt
        let review_prompt = SubAgentType::CodeReview.system_prompt();
        assert!(review_prompt.is_some());
        assert!(review_prompt.unwrap().contains("code reviewer"));

        // TestRunner has a system prompt
        let test_prompt = SubAgentType::TestRunner.system_prompt();
        assert!(test_prompt.is_some());
        assert!(test_prompt.unwrap().contains("test runner"));
    }

    #[test]
    fn test_subagent_type_uses_tools() {
        // These agents should use tools
        assert!(SubAgentType::Explore.uses_tools());
        assert!(SubAgentType::CodeReview.uses_tools());
        assert!(SubAgentType::TestRunner.uses_tools());
        assert!(SubAgentType::GeneralPurpose.uses_tools());

        // Plan agent should not use tools by default
        assert!(!SubAgentType::Plan.uses_tools());
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
            assert!(output.contains("Subagent Task Complete"));
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
            assert!(output.contains("Subagent Task Complete"));
            assert!(output.contains("agent_type"));
            assert!(output.contains("duration_ms"));
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
            assert!(output.contains("Subagent Task Complete"));
            assert!(output.contains("agent_type"));
            // Plan agent should provide structured response
            assert!(output.len() > 50);
        }
    }

    #[tokio::test]
    async fn test_spawn_task_with_options() {
        let mut agent = Agent::load_from_config()
            .await
            .expect("Failed to load agent");

        // Create some context messages
        let context = vec![
            Message {
                role: "user".to_string(),
                content: "Previous context".to_string(),
                tool_calls: None,
                tool_call_id: None,
            }
        ];

        let result = agent
            .spawn_task_with_options(
                SubAgentType::Explore,
                "测试带选项的任务",
                "列出当前目录的文件",
                Some(&context),
                true, // Enable tools
            )
            .await;

        // Requires Ollama to be running
        if result.is_ok() {
            let output = result.unwrap();
            assert!(output.contains("Subagent Task Complete"));
            assert!(output.contains("used_tools"));
            assert!(output.contains("duration_ms"));
        }
    }

    #[tokio::test]
    async fn test_spawn_task_code_review() {
        let mut agent = Agent::load_from_config()
            .await
            .expect("Failed to load agent");

        let result = agent
            .spawn_task(
                SubAgentType::CodeReview,
                "代码审查测试",
                "Review this function: fn add(a: i32, b: i32) -> i32 { a + b }",
            )
            .await;

        // Requires Ollama to be running
        if result.is_ok() {
            let output = result.unwrap();
            assert!(output.contains("Subagent Task Complete"));
            assert!(output.contains("Code reviewer"));
        }
    }

    #[tokio::test]
    async fn test_spawn_task_test_runner() {
        let mut agent = Agent::load_from_config()
            .await
            .expect("Failed to load agent");

        let result = agent
            .spawn_task(
                SubAgentType::TestRunner,
                "测试执行测试",
                "Design tests for a simple calculator",
            )
            .await;

        // Requires Ollama to be running
        if result.is_ok() {
            let output = result.unwrap();
            assert!(output.contains("Subagent Task Complete"));
            assert!(output.contains("Test runner"));
        }
    }

    #[test]
    fn test_subagent_task_builder() {
        let task = SubAgentTask::new(
            SubAgentType::Explore,
            "Test task",
            "Test prompt",
        )
        .with_context(true)
        .with_tools(true);

        assert!(task.include_context);
        assert!(task.include_tools);
        assert_eq!(task.description, "Test task");
    }

    #[tokio::test]
    async fn test_spawn_multiple_tasks() {
        let mut agent = Agent::load_from_config()
            .await
            .expect("Failed to load agent");

        let tasks = vec![
            SubAgentTask::new(
                SubAgentType::GeneralPurpose,
                "Task 1",
                "What is 1 + 1?",
            ),
            SubAgentTask::new(
                SubAgentType::GeneralPurpose,
                "Task 2",
                "What is 2 + 2?",
            ),
        ];

        let result = agent.spawn_multiple_tasks(tasks).await;

        // Requires Ollama to be running
        if result.is_ok() {
            let outputs = result.unwrap();
            assert_eq!(outputs.len(), 2);
            for output in outputs {
                assert!(output.contains("Subagent Task Complete"));
            }
        }
    }

    #[test]
    fn test_subagent_execution_lifecycle() {
        let task = SubAgentTask::new(
            SubAgentType::Explore,
            "Test task",
            "Test prompt",
        );

        let mut execution = SubAgentExecution::new(1, task);

        // Initial state
        assert_eq!(execution.id, 1);
        assert_eq!(execution.status, SubAgentStatus::Pending);
        assert!(execution.start_time.is_none());
        assert!(execution.end_time.is_none());
        assert!(execution.duration().is_none());

        // Start
        execution.start();
        assert_eq!(execution.status, SubAgentStatus::Running);
        assert!(execution.start_time.is_some());

        // Complete
        execution.complete("Task result".to_string());
        assert_eq!(execution.status, SubAgentStatus::Completed);
        assert!(execution.end_time.is_some());
        assert_eq!(execution.result, Some("Task result".to_string()));
        assert!(execution.duration().is_some());
    }

    #[test]
    fn test_subagent_execution_failure() {
        let task = SubAgentTask::new(
            SubAgentType::Explore,
            "Test task",
            "Test prompt",
        );

        let mut execution = SubAgentExecution::new(1, task);
        execution.start();
        execution.fail("Error message".to_string());

        assert_eq!(execution.status, SubAgentStatus::Failed("Error message".to_string()));
        assert!(execution.end_time.is_some());
    }

    #[test]
    fn test_subagent_id_counter() {
        let counter = SubAgentIdCounter::new();

        let id1 = counter.next();
        let id2 = counter.next();
        let id3 = counter.next();

        assert_eq!(id1, 0);
        assert_eq!(id2, 1);
        assert_eq!(id3, 2);
    }

    #[test]
    fn test_subagent_status_equality() {
        assert_eq!(SubAgentStatus::Pending, SubAgentStatus::Pending);
        assert_eq!(SubAgentStatus::Running, SubAgentStatus::Running);
        assert_eq!(SubAgentStatus::Completed, SubAgentStatus::Completed);

        // Failed status with same message should be equal
        assert_eq!(
            SubAgentStatus::Failed("error".to_string()),
            SubAgentStatus::Failed("error".to_string())
        );

        // Failed status with different message should not be equal
        assert_ne!(
            SubAgentStatus::Failed("error1".to_string()),
            SubAgentStatus::Failed("error2".to_string())
        );
    }
}
