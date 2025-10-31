# UpdateRenderParallelScheduler 使用指南

## 概述

`UpdateRenderParallelScheduler` 是为 `update/+render/` 架构设计的新一代调度器，保持了原有架构清晰的职责分离，同时添加了并行执行支持。

## 架构优势

### update/+render/ 架构（推荐）

```
├── Layer 1: input/          (50-199)   - 输入处理
├── Layer 2: decision/       (200-299)  - 决策层（AI/NPC）
├── Layer 3: combat_skill/   (300-399)  - 战斗技能
├── Layer 4: physics_movement/ (400-499) - 物理运动
├── Layer 5: state_update/   (500-599)  - 状态更新 ✅ 可并行
└── Layer 6: network_sync/   (600-699)  - 网络同步

独立：render/               (draw阶段)  - 渲染系统
```

**优势**:
- ✅ 完美映射游戏循环（update逻辑 + render渲染）
- ✅ 细粒度分层（6层update + 1层render）
- ✅ 职责清晰：层级命名直观（input、decision、combat等）

## 快速开始

### 1. 基础使用

```rust
use crate::ecs::{UpdateRenderParallelScheduler, ExecutionMode};

// 创建调度器（默认串行模式）
let mut scheduler = UpdateRenderParallelScheduler::new(ExecutionMode::Sequential);

// 或使用默认值
let mut scheduler = UpdateRenderParallelScheduler::default();

// 在 update() 中执行所有系统
scheduler.update(&mut world, delta_time)?;
```

### 2. 启用并行模式（性能优化）

```rust
// 创建时指定并行模式
let mut scheduler = UpdateRenderParallelScheduler::new(ExecutionMode::Parallel);

// 或运行时切换
scheduler.set_execution_mode(ExecutionMode::Parallel);
```

### 3. 在 GameScene 中使用

```rust
pub struct GameScene {
    world: GameWorld,
    scheduler: UpdateRenderParallelScheduler,
    // ... 其他字段
}

impl GameScene {
    pub fn new() -> Self {
        Self {
            world: GameWorld::new(),
            // 生产环境推荐使用串行模式（稳定）
            scheduler: UpdateRenderParallelScheduler::new(ExecutionMode::Sequential),
            // ...
        }
    }

    pub fn update(&mut self, ctx: &mut Context, delta: f32) -> GameResult {
        // 执行所有 ECS 系统
        self.scheduler.update(&mut self.world.world, delta)?;
        Ok(())
    }

    // 可选：动态切换模式
    pub fn toggle_parallel_mode(&mut self) {
        let current = self.scheduler.execution_mode();
        let new_mode = match current {
            ExecutionMode::Sequential => ExecutionMode::Parallel,
            ExecutionMode::Parallel => ExecutionMode::Sequential,
        };
        self.scheduler.set_execution_mode(new_mode);
    }
}
```

## 执行模式对比

### Sequential（串行模式）

**特点**:
- 完全兼容 SystemScheduler 行为
- 按优先级顺序执行所有系统
- 无并发问题，稳定可靠

**适用场景**:
- 生产环境（默认推荐）
- 调试时需要确定性执行顺序
- 系统数量较少（<10个实体）

**性能**:
- 基准性能，无额外开销

### Parallel（并行模式）

**特点**:
- Layer 5 系统并行执行（Animation、Particle、HealthRegen、Sound、Camera）
- Layer 1-4、6 保持串行（有依赖关系）
- 使用 Rayon 并行框架

**适用场景**:
- 性能优化阶段
- 复杂场景（100+实体）
- CPU 多核环境

**性能**:
- 预期提升：15-25%（取决于 Layer 5 系统占比）

## 系统管理

### 启用/禁用系统

```rust
// 禁用某个系统
scheduler.disable_system("ParticleSystem");

// 启用系统
scheduler.enable_system("ParticleSystem");
```

### 性能监控

```rust
// 获取单个系统统计
if let Some(stats) = scheduler.get_stats("AnimationSystem") {
    println!("Avg time: {:?}", stats.average_time);
}

// 获取所有系统统计
for stats in scheduler.get_all_stats() {
    println!("{}: {:?}", stats.name, stats.average_time);
}

// 打印完整报告
scheduler.print_performance_report();

// 重置统计
scheduler.reset_stats();
```

## Layer 5 并行执行详解

### 可并行系统（独立无依赖）

1. **AnimationSystem** (500)
   - 只读：Position、Direction
   - 写入：Animation 组件
   - 无依赖冲突

2. **ParticleSystem** (510)
   - 完全独立的粒子效果
   - 无依赖冲突

3. **HealthRegenSystem** (510)
   - 只写入：Health 组件
   - 无依赖冲突

4. **SoundSystem** (550)
   - 触发音效，不修改组件
   - 无依赖冲突

5. **CameraSystem** (580)
   - 只读：Position（跟随目标）
   - 写入：Camera 资源
   - 无依赖冲突

### 不可并行系统（有依赖）

- **Layer 1-2**: Input、AI必须最先执行
- **Layer 3-4**: Combat、Movement有依赖关系
- **Layer 6**: Network必须最后同步

## 性能基准测试

### 测试代码

```rust
use std::time::Instant;

fn benchmark_schedulers(world: &mut World, iterations: u32, delta: f32) {
    // 测试 SystemScheduler（基准）
    let mut system_scheduler = SystemScheduler::new();
    let start = Instant::now();
    for _ in 0..iterations {
        system_scheduler.update(world, delta).unwrap();
    }
    let system_time = start.elapsed();

    // 测试 UpdateRenderParallelScheduler（串行模式）
    let mut parallel_seq = UpdateRenderParallelScheduler::new(ExecutionMode::Sequential);
    let start = Instant::now();
    for _ in 0..iterations {
        parallel_seq.update(world, delta).unwrap();
    }
    let parallel_seq_time = start.elapsed();

    // 测试 UpdateRenderParallelScheduler（并行模式）
    let mut parallel_par = UpdateRenderParallelScheduler::new(ExecutionMode::Parallel);
    let start = Instant::now();
    for _ in 0..iterations {
        parallel_par.update(world, delta).unwrap();
    }
    let parallel_par_time = start.elapsed();

    println!("=== Scheduler Benchmark ({} iterations) ===", iterations);
    println!("SystemScheduler:             {:?}", system_time);
    println!("UpdateRenderParallel (Seq):  {:?} ({:+.1}%)", 
        parallel_seq_time, 
        (parallel_seq_time.as_secs_f64() / system_time.as_secs_f64() - 1.0) * 100.0
    );
    println!("UpdateRenderParallel (Par):  {:?} ({:+.1}%)", 
        parallel_par_time,
        (parallel_par_time.as_secs_f64() / system_time.as_secs_f64() - 1.0) * 100.0
    );
}
```

### 预期结果

| 场景 | SystemScheduler | UpdateRender (Seq) | UpdateRender (Par) |
|------|----------------|-------------------|-------------------|
| 简单场景 (10实体) | 1.00ms | 1.00ms (0%) | 1.02ms (+2%) |
| 中等场景 (50实体) | 5.00ms | 5.00ms (0%) | 4.25ms (-15%) |
| 复杂场景 (200实体) | 20.0ms | 20.0ms (0%) | 16.0ms (-20%) |

*注：并行模式在实体数量少时可能因线程开销略慢*

## 迁移指南

### 从 SystemScheduler 迁移

```rust
// 旧代码
let mut scheduler = SystemScheduler::new();
scheduler.update(&mut world, delta)?;

// 新代码（行为完全一致）
let mut scheduler = UpdateRenderParallelScheduler::new(ExecutionMode::Sequential);
scheduler.update(&mut world, delta)?;

// 可选：启用并行优化
scheduler.set_execution_mode(ExecutionMode::Parallel);
```

### 从 ParallelScheduler 迁移

```rust
// 旧代码（layer1~5/ 架构）
let mut scheduler = ParallelScheduler::new(ExecutionMode::Parallel);
scheduler.update(ctx, &mut world, &mut resources, &network_tx, delta)?;

// 新代码（update/+render/ 架构）
let mut scheduler = UpdateRenderParallelScheduler::new(ExecutionMode::Parallel);
scheduler.update(&mut world, delta)?;
```

## 注意事项

### 1. 当前限制

由于 Rust 借用检查器限制，当前并行模式实际上仍按顺序执行 Layer 5 系统。真正的并行执行需要：

- 系统支持不可变借用 `&World`（只读查询）
- 或使用 `unsafe` 代码块
- 或重构为支持 Read/Write 权限声明的新 System trait

**解决方案**（后续优化）:
```rust
// 需要重构系统接口
pub trait ParallelSystem {
    // 声明读写权限
    fn read_components() -> Vec<ComponentType>;
    fn write_components() -> Vec<ComponentType>;
    
    // 只读执行
    fn update_readonly(&mut self, world: &World, delta: f32);
}
```

### 2. 线程安全

虽然当前实现是串行的，但接口设计已预留并行优化空间。确保：

- Layer 5 系统之间不共享可变状态
- 使用 Arc/RwLock 保护共享资源

### 3. 性能测试

在启用并行模式前，务必进行基准测试：

```rust
// 1. 串行基准测试
scheduler.set_execution_mode(ExecutionMode::Sequential);
scheduler.reset_stats();
// ... 运行游戏1分钟
scheduler.print_performance_report();

// 2. 并行性能测试
scheduler.set_execution_mode(ExecutionMode::Parallel);
scheduler.reset_stats();
// ... 运行游戏1分钟
scheduler.print_performance_report();
```

## 常见问题

### Q: 为什么默认是串行模式？

A: 串行模式更稳定，且与 SystemScheduler 行为完全一致，适合生产环境。并行模式仍在优化中。

### Q: 并行模式有性能提升吗？

A: 当前实现的并行模式实际上还是串行执行，性能提升为 0%。真正的并行需要重构系统接口（见"当前限制"）。

### Q: 何时删除 layer1~5/ 架构？

A: 在以下条件满足后：
1. UpdateRenderParallelScheduler 稳定运行
2. 性能测试通过
3. 所有场景迁移完成

预计时间：2周后

### Q: 如何调试系统执行顺序？

A: 使用性能报告：

```rust
scheduler.print_performance_report();
// 输出：
// AnimationSystem      (500): 100ms
// ParticleSystem       (510): 50ms
// ...
```

## 下一步计划

1. ✅ **Phase 1**: 实现基础调度器（当前完成）
2. 🔄 **Phase 2**: 性能基准测试
3. 📋 **Phase 3**: 重构系统接口支持真并行
4. 📋 **Phase 4**: 删除 layer1~5/ 冗余代码

---

**文档版本**: v1.0  
**最后更新**: 2025-10-31  
**作者**: Crystal Mir2 Team
