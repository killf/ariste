# Plan 子代理测试指南

## 快速开始

### 1. 运行基础测试（不需要 Ollama）

```bash
cargo test agent::tests::test_subagent
```

这些测试只验证类型和系统提示，不调用 LLM。

### 2. 运行完整测试（需要 Ollama）

```bash
# 确保 Ollama 服务运行
ollama serve

# 运行 Plan 测试
cargo test agent::tests::test_spawn_task_plan
```

### 3. 运行示例程序

```bash
# 快速测试
cargo run --example quick_plan_test

# 完整演示
cargo run --example plan_demo

# 使用指南
cargo run --example plan_usage
```

## 测试场景

### 场景 1: 架构设计

测试 Plan 子代理设计新功能架构的能力：

```rust
agent.spawn_task(
    SubAgentType::Plan,
    "设计用户认证",
    "支持 JWT、刷新 token、权限管理"
).await?;
```

### 场景 2: 技术选型

测试 Plan 子代理进行技术决策的能力：

```rust
agent.spawn_task(
    SubAgentType::Plan,
    "选择数据库",
    "对比 PostgreSQL vs MySQL vs MongoDB"
).await?;
```

### 场景 3: 重构规划

测试 Plan 子代理规划代码重构的能力：

```rust
agent.spawn_task(
    SubAgentType::Plan,
    "重构工具系统",
    "当前工具系统扩展性差，需要重构"
).await?;
```

## 预期输出

Plan 子代理应该返回：

```
=== Subagent Task: {description} ===
Type: Software architect agent for designing implementation plans
Model: qwen3-vl:32b

{结构化的实现方案，包含：
1. 现状分析
2. 方案设计
3. 实施步骤
4. 风险评估
5. 替代方案
}

=== Task Complete ===
```

## 测试检查清单

- [ ] 基础类型测试通过
- [ ] 系统提示正确设置
- [ ] Plan 子代理能连接到 Ollama
- [ ] 返回格式化输出
- [ ] 输出包含架构分析
- [ ] 输出包含实施步骤
- [ ] 输出包含权衡考虑

## 与其他子代理对比

| 子代理 | 用途 | 系统提示 | 示例任务 |
|--------|------|----------|----------|
| **Plan** | 架构设计 | ✅ 专用提示 | "设计缓存系统架构" |
| **Explore** | 代码探索 | ✅ 专用提示 | "查找所有配置文件" |
| **GeneralPurpose** | 通用任务 | ❌ 无提示 | "解释什么是 REST API" |

## 调试技巧

### 1. 查看系统提示

```rust
let prompt = SubAgentType::Plan.system_prompt();
println!("Plan system prompt:\n{}", prompt.unwrap());
```

### 2. 测试不同模型

修改 `~/.ariste/settings.json`:

```json
{
  "model": "qwen2.5:14b"
}
```

### 3. 启用详细输出

在 `spawn_task` 方法中，`stream=false` 和 `think=false` 可以改为 `true`：

```rust
let ollama = Ollama::new()
    .stream(true)   // 显示流式输出
    .think(true);   // 显示思考过程
```

## 常见问题

### Q: 测试失败怎么办？

A: 检查 Ollama 是否运行：
```bash
curl http://localhost:11434/api/tags
```

### Q: 如何查看完整的 LLM 请求？

A: 在代码中添加调试输出：
```rust
println!("Messages: {:?}", messages);
```

### Q: Plan 子代理和普通 invoke 有什么区别？

A:
- `invoke()`: 使用完整工具集，适合交互式对话
- `spawn_task(Plan)`: 专注于架构设计，不带工具，避免递归

## 性能考虑

- Plan 子代理不携带工具，响应更快
- 建议用于规划阶段，实际实现使用主 Agent
- 复杂任务可以先用 Plan 规划，再分步执行
