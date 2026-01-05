/// Plan 子代理使用指南
///
/// Plan 子代理是专门用于软件架构设计和实现规划的 AI 代理。
/// 它擅长分析代码库并生成结构化的实现方案。

use ariste::{Agent, SubAgentType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ============================================================================
    // 基础用法
    // ============================================================================

    let mut agent = Agent::load_from_config().await?;

    // 调用 Plan 子代理
    let result = agent
        .spawn_task(
            SubAgentType::Plan,           // 子代理类型
            "添加用户管理功能",            // 简短描述 (3-5词)
            "需要为系统添加用户管理功能，包括注册、登录、权限管理",  // 详细任务描述
        )
        .await?;

    println!("{}", result);


    // ============================================================================
    // 实际应用场景
    // ============================================================================

    // 场景 1: 新功能设计
    let design = agent
        .spawn_task(
            SubAgentType::Plan,
            "设计缓存层",
            "需要为 API 添加缓存层。请分析：
1. 当前 API 结构
2. 合适的缓存策略
3. Redis vs Memcached
4. 缓存失效策略
5. 实现步骤",
        )
        .await?;


    // 场景 2: 技术选型
    let tech_choice = agent
        .spawn_task(
            SubAgentType::Plan,
            "选择 ORM 框架",
            "项目需要选择 ORM 框架。对比：
- Diesel
- SeaORM
- SQLx

考虑因素：性能、易用性、异步支持",
        )
        .await?;


    // 场景 3: 重构规划
    let refactor = agent
        .spawn_task(
            SubAgentType::Plan,
            "重构工具系统",
            "当前工具系统存在以下问题：
1. 工具注册分散
2. 缺乏统一的错误处理
3. 难以扩展

请提供重构方案，保持向后兼容",
        )
        .await?;


    // 场景 4: 性能优化规划
    let optimization = agent
        .spawn_task(
            SubAgentType::Plan,
            "优化 LLM 调用性能",
            "当前系统 LLM 调用较慢。请分析：
1. 瓶颈在哪里
2. 可以优化的点
3. 是否需要缓存/批处理
4. 实施计划",
        )
        .await?;


    // 场景 5: 架构升级
    let migration = agent
        .spawn_task(
            SubAgentType::Plan,
            "迁移到微服务架构",
            "考虑将单体应用迁移到微服务。
请提供：
1. 迁移策略
2. 服务拆分方案
3. 通信方式选择
4. 数据一致性保证",
        )
        .await?;


    // ============================================================================
    // Plan 子代理的特点
    // ============================================================================

    // 1. 专注于架构和设计
    //    - 不会直接写代码，而是提供方案
    //    - 会考虑多种方案的权衡
    //    - 会提供分步骤的实施计划

    // 2. 带有专门的系统提示
    //    - 引导 AI 进行架构思考
    //    - 关注现有代码库模式
    //    - 考虑架构权衡

    // 3. 适合的场景
    //    - ✅ 设计新功能架构
    //    - ✅ 技术选型决策
    //    - ✅ 重构规划
    //    - ✅ 性能优化方案
    //    - ❌ 直接实现代码（使用 GeneralPurpose）
    //    - ❌ 快速搜索代码（使用 Explore）


    // ============================================================================
    // 与其他子代理的对比
    // ============================================================================

    // Explore: 快速探索代码库
    let explore_result = agent
        .spawn_task(
            SubAgentType::Explore,
            "查找工具定义",
            "在 src/tools/ 目录下查找所有工具的定义文件",
        )
        .await?;

    // GeneralPurpose: 通用任务
    let general_result = agent
        .spawn_task(
            SubAgentType::GeneralPurpose,
            "解释概念",
            "请解释什么是 SOLID 原则",
        )
        .await?;

    // Plan: 架构设计
    let plan_result = agent
        .spawn_task(
            SubAgentType::Plan,
            "设计模块结构",
            "为新的插件系统设计模块结构",
        )
        .await?;


    Ok(())
}
