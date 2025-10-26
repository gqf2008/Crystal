# NPC系统自动化测试

## 📋 概述

这是Crystal项目NPC系统的自动化测试套件,通过以下方式验证功能:

1. **组件测试** - 直接检查ECS组件状态
2. **日志分析** - 监控tracing输出验证行为
3. **时间模拟** - 快进游戏时间测试动作切换
4. **性能基准** - 测量不同场景下的帧时间

## 🚀 快速开始

### 运行所有测试
```powershell
cd ClientRust
cargo test --tests
```

### 使用测试脚本(推荐)
```powershell
cd ClientRust/tests
./run_npc_tests.ps1
```

### 运行特定测试
```powershell
# 只运行动作切换相关测试
cargo test action

# 只运行性能测试
cargo test performance

# 显示详细输出
cargo test -- --nocapture
```

## 📦 测试结构

```
tests/
├── npc_system_tests.rs      # 核心功能测试(7个测试)
├── npc_logging_tests.rs     # 日志和性能测试(3个测试)
├── test_helpers.rs           # 测试工具模块
├── run_npc_tests.ps1         # PowerShell测试运行器
└── README.md                 # 本文档
```

## 🧪 测试用例清单

### npc_system_tests.rs

1. **test_npc_creation** ✅
   - 验证NPC组件正确创建
   - 检查NPCData字段(name, colour, timer等)
   - 验证Animation初始状态

2. **test_npc_action_switching** ✅
   - 模拟10秒游戏时间
   - 验证动作切换发生(1-5次)
   - 检查Standing/Harvest交替

3. **test_npc_animation_frames** ✅
   - 验证动画帧正确更新
   - 检查帧循环机制

4. **test_npc_colour_system** ✅
   - 测试多种颜色(红/绿/蓝/默认)
   - 验证ARGB格式正确

5. **test_quest_marker** ✅
   - 测试任务标记添加/更新/删除
   - 验证Available/Complete/Incomplete三种状态

6. **test_npc_performance** ✅
   - 创建100个NPC
   - 模拟200帧更新
   - 断言创建<100ms, 更新<500ms

7. **test_action_weight_distribution** ✅
   - 触发50次动作切换
   - 统计Standing/Harvest分布
   - 验证70/30权重(允许±20%误差)

### npc_logging_tests.rs

1. **test_npc_creation_logs** ✅
   - 捕获NPC创建日志
   - 验证关键信息输出

2. **test_action_switch_logs** ✅
   - 监控动作切换日志
   - 验证"切换动作"消息

3. **test_npc_system_performance_benchmark** ✅
   - 4个场景: 5/20/50/100 NPC
   - 测量1000帧平均时间
   - 性能阈值断言

## 📊 性能基准

| 场景 | NPC数 | 阈值 | 预期FPS |
|------|-------|------|---------|
| 小场景 | 5 | <100μs | >10000 |
| 中场景 | 20 | <500μs | >2000 |
| 大场景 | 50 | <2ms | >500 |
| 极限 | 100 | <5ms | >200 |

## 🔧 测试工具API

### TestContext

```rust
let mut ctx = TestContext::new();

// 创建NPC
let npc_id = ctx.create_test_npc(
    "铁匠",           // 名字
    100,              // npc_index
    &Point {x: 10, y: 10}, // 位置
    0xFF00FF00,       // 颜色(ARGB)
);

// 前进时间
ctx.advance_time(1000); // 1秒

// 获取统计
let stats = ctx.get_action_stats();
println!("Standing: {}, Harvest: {}", 
    stats.standing_count, 
    stats.harvest_count
);
```

### LogCapture

```rust
let (collector, logs) = LogCollector::new();
let _guard = tracing_subscriber::registry()
    .with(collector)
    .set_default();

// ... 执行测试代码 ...

let captured = logs.lock().unwrap();
assert!(captured.iter().any(|log| log.contains("NPC创建")));
```

## ✅ 测试覆盖率

| 功能模块 | 测试状态 | 备注 |
|---------|---------|------|
| NPC创建 | ✅ 已覆盖 | 组件初始化 |
| 动作切换 | ✅ 已覆盖 | 权重+时间 |
| 动画更新 | ✅ 已覆盖 | 帧循环 |
| 颜色染色 | ✅ 已覆盖 | ARGB格式 |
| 任务标记 | ✅ 已覆盖 | 3种状态 |
| 性能基准 | ✅ 已覆盖 | 4个场景 |
| 日志输出 | ✅ 已覆盖 | tracing监控 |
| 特效渲染 | ⏸️ 暂缓 | 需要GPU上下文 |
| 名字渲染 | ⏸️ 暂缓 | 需要ggez Context |

## 🐛 已知限制

1. **渲染测试**: 需要GPU上下文,当前只能测试逻辑层
2. **随机性**: 某些测试依赖随机数,极少数情况可能失败
3. **时间依赖**: 动作切换测试依赖时间模拟,可能需要多次迭代

## 💡 扩展测试

### 添加新测试

1. 在`tests/`目录创建新文件或扩展现有文件
2. 使用`test_helpers::TestContext`简化setup
3. 添加`#[test]`属性
4. 运行`cargo test`验证

示例:
```rust
#[test]
fn test_my_new_feature() {
    let mut ctx = TestContext::new();
    let npc_id = ctx.create_test_npc("测试", 1, &Point {x: 0, y: 0}, 0);
    
    // 你的测试逻辑
    
    assert!(/* 断言条件 */);
}
```

## 📈 持续集成

建议在CI/CD管道中运行:

```yaml
# .github/workflows/test.yml
- name: Run NPC Tests
  run: |
    cd ClientRust
    cargo test --tests
```

## 🔍 调试测试

```powershell
# 显示所有println输出
cargo test -- --nocapture

# 只运行一个测试
cargo test test_npc_creation -- --nocapture

# 显示详细backtrace
$env:RUST_BACKTRACE = "1"
cargo test
```

## 📞 联系方式

如果测试失败或有疑问:
1. 检查日志输出(`-- --nocapture`)
2. 查看`test_helpers.rs`源码
3. 提交Issue到项目仓库

---

**最后更新**: 2025-10-26  
**测试版本**: v1.0  
**通过率**: 10/10 (100%) ✅
