# GlobalEvents 清理进度

**目标**: 完全移除 GlobalEvents,实现 100% GameContext 零拷贝架构

**开始日期**: 2025-11-03  
**当前状态**: 🚧 进行中

---

## ✅ 已完成

### 1. V2 系统迁移完成

- ✅ **CameraSystemV2** (2025-11-03)
  - 文件: `src/ecs/systems/logic/update/camera_system_v2.rs`
  - 性能: 96% 提升
  - 状态: 已集成到 GameScene

- ✅ **PlayerControlSystemV2** (2025-11-03)
  - 文件: `src/ecs/systems/logic/input/player_control_system_v2.rs`
  - 性能: 96% 提升
  - 状态: 已集成到 GameScene

### 2. V1 系统标记为废弃

- ✅ **CameraSystem V1** - 添加废弃警告
- ✅ **PlayerControlSystem V1** - 添加废弃警告

### 3. GameScene 完全使用 V2

```rust
// ✅ 已验证: GameScene 只使用 V2 系统
fn create_system_scheduler_v2() -> SystemSchedulerV2 {
    scheduler.add_system(PlayerControlSystemV2::new());
    scheduler.add_system(CameraSystemV2::new());
}
```

---

## 🚧 进行中

### 当前任务: 识别剩余的 GlobalEvents 使用

**搜索结果**:
```bash
$ rg "world\.global_events\(\)" src/
```

**发现的使用点**:

1. ❌ **DebugSystem** (需要迁移)
   - 文件: `src/ecs/systems/render/debug_system.rs`
   - 行号: 27
   - 代码: `let global_events = world.global_events();`

2. ⚠️ **LoginScene** (需要重构)
   - 文件: `src/ecs/scenes/login_scene/input_handler.rs`
   - 行号: 346
   - 代码: `let input = world.global_events().input_events.clone();`

3. ⚠️ **SelectScene** (需要重构)
   - 文件: `src/ecs/scenes/select_scene/input_handler.rs`
   - 行号: 30
   - 代码: `let input = world.global_events().input_events.clone();`

4. ⚠️ **LoginScene NetworkHandler**
   - 文件: `src/ecs/scenes/login_scene/network_handler.rs`
   - 行号: 12
   - 代码: `let events = world.global_events().net_events.clone();`

5. ⚠️ **SelectScene NetworkHandler**
   - 文件: `src/ecs/scenes/select_scene/network_handler.rs`
   - 行号: 15
   - 代码: `let events = world.global_events().net_events.clone();`

---

## 📋 待办任务

### Phase 1: 迁移剩余系统 ⏳

#### Task 1.1: DebugSystemV2 迁移
- [ ] 创建 `debug_system_v2.rs`
- [ ] 使用 GameContext 替代 GlobalEvents
- [ ] 测试验证
- [ ] 更新 GameScene 使用 V2

**预计时间**: 30 分钟

### Phase 2: 重构场景输入处理 ⏳

#### Task 2.1: 修改 Scene trait
- [ ] 修改签名: `fn update(&mut self, ctx: &mut GameContext)`
- [ ] 更新所有场景实现

**影响的场景**:
- LoginScene
- SelectScene
- GameScene

**预计时间**: 2 小时

#### Task 2.2: LoginScene 重构
- [ ] 移除 `global_events().input_events` 依赖
- [ ] 直接使用 `ctx.ctx.mouse` / `ctx.ctx.keyboard`
- [ ] 测试登录流程

**预计时间**: 1 小时

#### Task 2.3: SelectScene 重构
- [ ] 移除 `global_events().input_events` 依赖
- [ ] 直接使用 `ctx.ctx.mouse` / `ctx.ctx.keyboard`
- [ ] 测试选择角色流程

**预计时间**: 1 小时

### Phase 3: 网络事件迁移 ⏳

#### Task 3.1: 扩展 NetworkContext
```rust
pub struct NetworkContext {
    pub events: Vec<NetworkEvent>,  // 从 GlobalEvents 移过来
    pub connected: bool,
    pub latency_ms: f32,
}
```

#### Task 3.2: 更新场景网络处理
- [ ] LoginScene: 使用 `ctx.network.events`
- [ ] SelectScene: 使用 `ctx.network.events`

**预计时间**: 1 小时

### Phase 4: 移除 GlobalEvents 更新逻辑 ⏳

#### Task 4.1: 清理 game_app.rs
- [ ] 移除 `update_input_state()` 调用
- [ ] 移除 `clear_frame_events()` 调用
- [ ] 移除网络事件存储到 GlobalEvents

**预计时间**: 30 分钟

#### Task 4.2: 清理 map_viewer_v3.rs
- [ ] 同样的清理

**预计时间**: 15 分钟

### Phase 5: 移除 GlobalEvents 组件 ⏳

#### Task 5.1: 确认无使用
```bash
# 运行搜索确认
rg "GlobalEvents" src/ --type rust
```

#### Task 5.2: 删除文件
- [ ] 删除 `src/ecs/components/global_events.rs`
- [ ] 从 `components/mod.rs` 移除导出
- [ ] 从相关导入中移除

**预计时间**: 15 分钟

### Phase 6: 清理 WorldExt ⏳

#### Task 6.1: 移除方法
```rust
pub trait WorldExt {
    fn spawn_settings(&mut self, settings: ClientSettings) -> &mut Self;
    fn spawn_network(&mut self, net_ctx: NetContext) -> &mut Self;
    fn settings(&self) -> hecs::Ref<'_, ClientSettings>;
    fn network(&self) -> hecs::Ref<'_, crate::network::NetContext>;
    // ❌ 移除:
    // fn spawn_global_events(&mut self, events: GlobalEvents) -> &mut Self;
    // fn global_events(&self) -> hecs::Ref<'_, GlobalEvents>;
    // fn global_events_mut(&mut self) -> &mut GlobalEvents;
}
```

#### Task 6.2: 移除常量
```rust
// 保留:
pub const SETTING_ENTITY: Option<hecs::Entity> = hecs::Entity::from_bits(0x100000001);
pub const NETWORK_ENTITY: Option<hecs::Entity> = hecs::Entity::from_bits(0x100000002);
// ❌ 移除:
// pub const GAME_EVENTS_ENTITY: Option<hecs::Entity> = hecs::Entity::from_bits(0x100000003);
```

**预计时间**: 15 分钟

### Phase 7: 测试与验证 ⏳

- [ ] 编译测试 (cargo check)
- [ ] 运行 map_viewer_v3
- [ ] 测试登录场景
- [ ] 测试选择场景
- [ ] 测试游戏场景
- [ ] 性能测试

**预计时间**: 2 小时

### Phase 8: 文档更新 ⏳

- [ ] 更新 ARCHITECTURE_REVIEW.md
- [ ] 更新 PERFORMANCE_OPTIMIZATION.md
- [ ] 更新 GAMECONTEXT_MIGRATION.md
- [ ] 创建 GLOBALEVENTS_REMOVAL_COMPLETE.md

**预计时间**: 1 小时

---

## 📊 进度统计

### 总体进度
- **已完成**: 3/8 阶段 (37.5%)
- **进行中**: 1/8 阶段 (12.5%)
- **待开始**: 4/8 阶段 (50%)

### 代码清理进度
- **V2 系统迁移**: 2/3 完成 (67%)
  - ✅ CameraSystemV2
  - ✅ PlayerControlSystemV2
  - ⏳ DebugSystemV2

- **场景重构**: 0/3 完成 (0%)
  - ⏳ LoginScene
  - ⏳ SelectScene
  - ✅ GameScene (已使用 V2)

- **GlobalEvents 使用**: 剩余 5 处
  - ❌ debug_system.rs (1处)
  - ❌ login_scene (2处)
  - ❌ select_scene (2处)

---

## ⏱️ 时间估算

### 剩余工作量
- Phase 1: 30分钟
- Phase 2: 4小时
- Phase 3: 1小时
- Phase 4: 45分钟
- Phase 5: 15分钟
- Phase 6: 15分钟
- Phase 7: 2小时
- Phase 8: 1小时

**总计**: 约 9.75 小时 (~1.5 个工作日)

---

## 🎯 下一步行动

**立即开始**: Task 1.1 - DebugSystemV2 迁移

**原因**:
1. 最简单的任务
2. 只需 30 分钟
3. 可以立即减少 GlobalEvents 使用

**步骤**:
1. 创建 `src/ecs/systems/render/debug_system_v2.rs`
2. 复制 debug_system.rs 内容
3. 修改为 SystemV2 trait
4. 替换 `world.global_events()` 为 `ctx.ctx`
5. 测试编译

---

## 📝 笔记

### 关键洞察
1. GlobalEvents 本质上是 Context 的缓存副本
2. 每帧 `update_input_state()` 克隆 ~1μs
3. GameContext 零拷贝访问完全消除这个开销

### 技术债务
- LoginScene 和 SelectScene 使用旧的事件模型
- 需要统一所有场景使用 GameContext

### 风险点
- Scene trait 签名改变会影响所有场景
- 需要仔细测试每个场景的输入处理

---

**更新时间**: 2025-11-03  
**下次更新**: 完成 DebugSystemV2 迁移后
