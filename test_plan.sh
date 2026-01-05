#!/bin/bash
# Plan 子代理快速测试脚本

echo "=========================================="
echo "  Plan 子代理测试"
echo "=========================================="
echo ""

# 1. 运行基础测试（不需要 Ollama）
echo "1️⃣  运行基础类型测试..."
cargo test agent::tests::test_subagent --quiet

if [ $? -eq 0 ]; then
    echo "✅ 基础测试通过"
else
    echo "❌ 基础测试失败"
    exit 1
fi

echo ""
echo "2️⃣  检查 Ollama 服务..."
if curl -s http://localhost:11434/api/tags > /dev/null 2>&1; then
    echo "✅ Ollama 服务运行中"
else
    echo "❌ Ollama 服务未运行"
    echo "   请先启动: ollama serve"
    exit 1
fi

echo ""
echo "3️⃣  运行 Plan 集成测试..."
echo "   这将测试 Plan 子代理的实际功能..."
echo ""

# 创建临时的测试程序
cat > /tmp/test_plan.rs <<'EOF'
use ariste::{Agent, SubAgentType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut agent = Agent::load_from_config().await?;

    println!("📋 测试 Plan 子代理");
    println━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println("");

    let result = agent
        .spawn_task(
            SubAgentType::Plan,
            "测试规划",
            "请为 CLI 工具设计一个配置文件加载功能。
要求支持 JSON 和 YAML 格式。
请简要说明实现步骤。",
        )
        .await?;

    println!("{}", result);
    println━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println("✅ 测试完成");

    Ok(())
}
EOF

# 复制到 examples 目录并运行
cp /tmp/test_plan.rs examples/test_plan.rs
cargo run --example test_plan 2>&1 | grep -v "warning"

echo ""
echo "=========================================="
echo "  测试完成"
echo "=========================================="
