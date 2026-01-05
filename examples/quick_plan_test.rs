#!/usr/bin/env cargo
//!
//! 这是一个快速测试 Plan 子代理的示例
//!
//! 运行方式：
//!   cargo run --example quick_plan_test
//

use ariste::{Agent, SubAgentType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 快速测试 Plan 子代理\n");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let mut agent = Agent::load_from_config().await?;

    println!("📋 测试任务: 设计计数器功能\n");

    let result = agent
        .spawn_task(
            SubAgentType::Plan,
            "设计计数器",
            "请为这个 CLI 工具设计一个计数器功能。
要求：
1. 可以记录任务完成数量
2. 支持增量/减量
3. 可以重置
4. 数据持久化

请提供简要的实现方案。",
        )
        .await?;

    println!("{}\n", result);

    println!("✅ 测试完成！");

    Ok(())
}
