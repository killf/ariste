# Plan 子代理测试完整指南

## 🚀 快速开始

### 方式 1: 使用测试脚本（推荐）

```bash
./test_plan.sh
```

### 方式 2: 手动测试

#### 步骤 1: 运行单元测试
```bash
cargo test agent::tests::test_subagent
```

#### 步骤 2: 运行示例程序
```bash
# 确保已安装 Ollama 并运行服务
ollama serve

# 运行快速测试
cargo run --example quick_plan_test

# 运行完整演示
cargo run --example plan_demo
```

## 📋 测试场景

### 1. 架构设计测试

测试 Plan 子代理设计新功能的能力：

```bash
cargo run --example plan_demo
```

这会测试：
- 用户认证系统设计
- 数据库方案选择
- 代码重构规划

### 2. 单元测试

```bash
# 类型描述测试
cargo test test_subagent_type_descriptions

# 系统提示测试
cargo test test_subagent_type_system_prompts

# Plan 功能测试（需要 Ollama）
cargo test test_spawn_task_plan
```

## 🧪 测试检查清单

- [x] 基础类型测试通过
- [x] 系统提示正确设置
- [ ] Plan 子代理能连接到 Ollama
- [ ] 返回格式化输出
- [ ] 输出包含架构分析
- [ ] 输出包含实施步骤

## 📊 预期输出示例

```
=== Subagent Task: 设计计数器 ===
Type: Software architect agent for designing implementation plans
Model: qwen3-vl:32b

# 计数器功能设计方案

## 1. 需求分析
- 需要记录任务完成数量
- 支持增量/减量操作
- 支持重置
- 数据需要持久化

## 2. 实现方案

### 2.1 数据结构
```rust
struct Counter {
    value: i64,
    storage_path: PathBuf,
}
```

### 2.2 核心功能
- `increment()` - 增加计数
- `decrement()` - 减少计数
- `reset()` - 重置为 0
- `save()` - 保存到磁盘
- `load()` - 从磁盘加载

### 2.3 持久化方案
- 使用 JSON 文件存储
- 位置: `~/.ariste/counter.json`
- 每次修改后自动保存

## 3. 实施步骤
1. 创建 Counter 结构体
2. 实现基本操作方法
3. 添加 JSON 序列化
4. 集成到 CLI 命令

## 4. 架构考虑
- 线程安全: 使用 Arc<Mutex<>> 如果需要多线程
- 错误处理: 适当的 Result 类型
- 扩展性: 预留接口用于未来的统计功能

=== Task Complete ===
```

## 🔍 调试技巧

### 查看 Plan 的系统提示

```rust
use ariste::SubAgentType;

fn main() {
    let prompt = SubAgentType::Plan.system_prompt();
    println!("Plan system prompt:\n{}", prompt.unwrap());
}
```

### 测试不同的子代理类型

```rust
use ariste::{Agent, SubAgentType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut agent = Agent::load_from_config().await?;

    // Explore: 快速探索
    agent.spawn_task(
        SubAgentType::Explore,
        "查找工具文件",
        "在 src/tools/ 下查找所有 .rs 文件"
    ).await?;

    // Plan: 架构设计
    agent.spawn_task(
        SubAgentType::Plan,
        "设计新功能",
        "为工具系统添加插件机制"
    ).await?;

    // GeneralPurpose: 通用任务
    agent.spawn_task(
        SubAgentType::GeneralPurpose,
        "解释概念",
        "什么是 SOLID 原则？"
    ).await?;

    Ok(())
}
```

## 🎯 实际应用示例

### 示例 1: 设计新功能

```rust
agent.spawn_task(
    SubAgentType::Plan,
    "设计缓存层",
    "需要为 API 添加 Redis 缓存层。请提供架构设计和实施步骤"
).await?;
```

### 示例 2: 技术选型

```rust
agent.spawn_task(
    SubAgentType::Plan,
    "选择 ORM",
    "对比 Diesel、SeaORM、SQLx，为项目选择合适的 ORM"
).await?;
```

### 示例 3: 重构规划

```rust
agent.spawn_task(
    SubAgentType::Plan,
    "重构工具系统",
    "当前工具系统扩展性差，需要重构。请提供详细的重构计划"
).await?;
```

## 📚 相关文档

- [examples/README.md](examples/README.md) - 详细使用指南
- [src/agent/agent.rs](src/agent/agent.rs) - Agent 实现
- [src/agent/mod.rs](src/agent/mod.rs) - 模块导出

## ⚡ 性能提示

- Plan 子代理不携带工具，响应比普通 Agent 快
- 适合用于规划阶段，实际实现建议使用主 Agent
- 复杂任务可以先 Plan 规划，再分步执行

## 🐛 常见问题

### Q: 测试失败，提示连接 Ollama 失败？

A: 确保 Ollama 服务正在运行：
```bash
ollama serve
```

### Q: 如何使用其他模型？

A: 编辑 `~/.ariste/settings.json`:
```json
{
  "model": "qwen2.5:14b"
}
```

### Q: Plan 子代理和普通 Agent 有什么区别？

A:
| 特性 | Plan 子代理 | 普通 Agent |
|------|------------|-----------|
| 工具 | 无 | 完整工具集 |
| 系统提示 | 专用架构提示 | 无 |
| 用途 | 架构设计 | 通用对话 |
| 速度 | 更快 | 较慢 |
| 递归风险 | 无 | 有 |

---

最后更新: 2026-01-05
