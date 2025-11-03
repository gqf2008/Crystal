# WorldExt GlobalEvents 清理计划

**目标**: 逐步移除 WorldExt 中的 `global_events()` 和 `global_events_mut()` 方法

**原因**: GameContext 零拷贝架构已经提供了更好的输入访问方式

---

## 📊 当前使用情况

### 仍在使用 GlobalEvents 的地方

#### 1. V1 系统 (即将淘汰)
```rust
// src/ecs/systems/logic/update/camera_system.rs
let global_events = world.global_events();  // V1 旧方式

// src/ecs/systems/render/debug_system.rs
let global_events = world.global_events();  // V1 旧方式
```

#### 2. 游戏主循环 (输入收集)
```rust
// src/ecs/game_app.rs
self.world.global_events_mut().update_input_state(ctx);  // 每帧更新
self.world.global_events_mut().clear_frame_events();     // 清理事件

// src/bin/map_viewer_v3.rs
self.world.global_events_mut().update_input_state(ctx);  // 每帧更新
```

#### 3. 登录/选择场景
```rust
// src/ecs/scenes/login_scene/input_handler.rs
let input = world.global_events().input_events.clone();  // 读取输入事件

// src/ecs/scenes/select_scene/input_handler.rs
let input = world.global_events().input_events.clone();  // 读取输入事件
```

---

## 🎯 清理策略

### Phase 1: 移除 V1 系统依赖 ✅ (部分完成)

**已完成**:
- ✅ CameraSystem → CameraSystemV2 (已迁移)
- ✅ PlayerControlSystem → PlayerControlSystemV2 (已迁移)

**待完成**:
- ⏳ DebugSystem → DebugSystemV2 (需要迁移)
- ⏳ 其他使用 GlobalEvents 的 V1 系统

### Phase 2: 重构场景输入处理 ⏳

**问题**: LoginScene 和 SelectScene 还在用旧的输入事件模式

**解决方案 1**: 使用 GameContext (推荐)
```rust
// LoginScene 和 SelectScene 也应该使用 GameContext
impl Scene for LoginScene {
    fn update(&mut self, ctx: &mut Context, world: &mut World) -> GameResult {
        // 创建 GameContext
        let network_ctx = NetworkContext::new();
        let mut game_ctx = GameContext::new(ctx, world, &network_ctx);
        
        // 直接访问输入
        if game_ctx.ctx.mouse.button_pressed(MouseButton::Left) {
            let pos = game_ctx.ctx.mouse.position();
            self.handle_click(pos.x, pos.y);
        }
        
        Ok(())
    }
}
```

**解决方案 2**: 保留 GlobalEvents 但简化
```rust
// 只保留必要的输入状态
pub struct GlobalEvents {
    pub mouse: MouseContext,      // 保留
    pub keyboard: KeyboardContext, // 保留
    // 移除其他不必要的字段
}
```

### Phase 3: 移除 update_input_state ⏳

**当前流程**:
```
game_app::update()
    └─> global_events_mut().update_input_state(ctx)  // 克隆 Context
            └─> 存储到 GlobalEvents
                    └─> V1 系统读取
```

**新流程**:
```
game_app::update()
    └─> 创建 GameContext { ctx, world, network }
            └─> V2 系统直接访问 ctx (零拷贝)
```

**实现**:
```rust
// game_app.rs - 移除 update_input_state 调用
impl GameState for CrystalApp {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        // ❌ 旧方式: 每帧克隆输入状态
        // self.world.global_events_mut().update_input_state(ctx);
        
        // ✅ 新方式: 直接传递 Context
        let network_ctx = NetworkContext::new();
        let mut game_ctx = GameContext::new(ctx, &mut self.world, &network_ctx);
        
        self.current_scene.update(&mut game_ctx)?;
        
        Ok(())
    }
}
```

### Phase 4: 清理 WorldExt ✅ (最终目标)

**移除方法**:
```rust
pub trait WorldExt {
    fn spawn_settings(&mut self, settings: ClientSettings) -> &mut Self;
    fn spawn_network(&mut self, net_ctx: NetContext) -> &mut Self;
    // ❌ 移除以下方法:
    // fn spawn_global_events(&mut self, events: GlobalEvents) -> &mut Self;
    // fn global_events(&self) -> hecs::Ref<'_, GlobalEvents>;
    // fn global_events_mut(&mut self) -> &mut GlobalEvents;
    
    fn settings(&self) -> hecs::Ref<'_, ClientSettings>;
    fn network(&self) -> hecs::Ref<'_, crate::network::NetContext>;
}
```

**移除常量**:
```rust
pub const SETTING_ENTITY: Option<hecs::Entity> = hecs::Entity::from_bits(0x100000001);
pub const NETWORK_ENTITY: Option<hecs::Entity> = hecs::Entity::from_bits(0x100000002);
// ❌ 移除: pub const GAME_EVENTS_ENTITY
```

---

## 📝 详细行动计划

### 第1步: 迁移剩余的 V1 系统 (1-2周)

**优先级**: 高

**任务**:
1. DebugSystemV2 迁移
2. 检查所有使用 `world.global_events()` 的 V1 系统
3. 逐个迁移到 SystemV2

**验证**:
```bash
# 检查是否还有 V1 系统使用 GlobalEvents
rg "world\.global_events\(\)" --type rust -g "!docs" -g "!*.md"
```

### 第2步: 重构 Scene trait (2-3天)

**修改 Scene trait 签名**:
```rust
pub trait Scene {
    // ❌ 旧签名
    // fn update(&mut self, ctx: &mut Context, world: &mut World) -> GameResult;
    
    // ✅ 新签名
    fn update(&mut self, ctx: &mut GameContext) -> GameResult;
    
    fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas, world: &World) -> GameResult;
}
```

**修改所有场景实现**:
- LoginScene
- SelectScene  
- GameScene (已经在用 GameContext,只需调整接口)

### 第3步: 清理 game_app.rs (1天)

**移除 GlobalEvents 更新**:
```rust
impl GameState for CrystalApp {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        // 收集网络事件
        let events = self.net_context.poll_events();
        
        // ❌ 移除
        // self.world.global_events_mut().net_events = events;
        // self.world.global_events_mut().update_input_state(ctx);
        // self.world.global_events_mut().clear_frame_events();
        
        // ✅ 新方式
        let mut network_ctx = NetworkContext::new();
        network_ctx.events = events;
        
        let mut game_ctx = GameContext::new(ctx, &mut self.world, &network_ctx);
        self.current_scene.update(&mut game_ctx)?;
        
        Ok(())
    }
}
```

### 第4步: 移除 GlobalEvents 组件 (1天)

**检查 GlobalEvents 是否还被使用**:
```bash
rg "GlobalEvents" --type rust -g "!docs" -g "!*.md"
```

**如果不再使用,移除**:
- 删除 `components/global_events.rs`
- 从 `components/mod.rs` 移除导出
- 从 WorldExt 移除相关方法

### 第5步: 清理 WorldExt (30分钟)

```rust
// src/ecs/mod.rs
pub trait WorldExt {
    fn spawn_settings(&mut self, settings: ClientSettings) -> &mut Self;
    fn spawn_network(&mut self, net_ctx: NetContext) -> &mut Self;
    fn settings(&self) -> hecs::Ref<'_, ClientSettings>;
    fn network(&self) -> hecs::Ref<'_, crate::network::NetContext>;
}

pub const SETTING_ENTITY: Option<hecs::Entity> = hecs::Entity::from_bits(0x100000001);
pub const NETWORK_ENTITY: Option<hecs::Entity> = hecs::Entity::from_bits(0x100000002);
// GAME_EVENTS_ENTITY 已移除
```

---

## ⚠️ 注意事项

### 1. 网络事件处理

**问题**: 网络事件目前存储在 GlobalEvents 中

**解决方案**: 移到 NetworkContext
```rust
pub struct NetworkContext {
    pub events: Vec<NetworkEvent>,  // 从 GlobalEvents 移过来
    pub connected: bool,
    pub latency_ms: f32,
}
```

### 2. 向后兼容

在清理过程中:
- ✅ 保持 V1 和 V2 系统并存
- ✅ 渐进式迁移,不破坏现有功能
- ✅ 每步都要测试验证

### 3. 文档更新

需要更新的文档:
- `ARCHITECTURE_REVIEW.md`
- `PERFORMANCE_OPTIMIZATION.md`
- `src/ecs/components/README.md`
- `src/ecs/systems/README.md`

---

## 📈 收益

### 性能提升
- ❌ 移除每帧 ~1μs 的 update_input_state 开销
- ❌ 移除每帧 2-3 次内存分配
- ✅ 零拷贝访问,零开销

### 架构简化
- ❌ 移除 GlobalEvents 复杂的事件管理
- ❌ 移除 WorldExt 中的事件相关方法
- ✅ 统一使用 GameContext,架构更清晰

### 代码质量
- ❌ 减少间接层
- ❌ 减少状态同步点
- ✅ 提高可维护性

---

## 🎯 最终目标

```rust
// 清理后的 WorldExt (简洁版)
pub trait WorldExt {
    fn spawn_settings(&mut self, settings: ClientSettings) -> &mut Self;
    fn spawn_network(&mut self, net_ctx: NetContext) -> &mut Self;
    fn settings(&self) -> hecs::Ref<'_, ClientSettings>;
    fn network(&self) -> hecs::Ref<'_, crate::network::NetContext>;
}

// GlobalEvents 完全移除
// ❌ pub struct GlobalEvents { ... }

// 所有系统使用 GameContext
impl SystemV2 for AnySystem {
    fn update(&mut self, ctx: &mut GameContext, dt: f32) -> GameResult {
        // ✅ 直接访问,零拷贝
        let mouse = ctx.ctx.mouse;
        let keyboard = ctx.ctx.keyboard;
        // ...
    }
}
```

---

## 📅 时间表

| 阶段 | 任务 | 预计时间 | 优先级 |
|------|------|----------|--------|
| Phase 1 | 迁移剩余 V1 系统 | 1-2周 | 高 |
| Phase 2 | 重构 Scene trait | 2-3天 | 高 |
| Phase 3 | 清理 game_app.rs | 1天 | 中 |
| Phase 4 | 移除 GlobalEvents | 1天 | 中 |
| Phase 5 | 清理 WorldExt | 30分钟 | 低 |

**总计**: 约 2-3 周

---

## ✅ 验证清单

完成后检查:
- [ ] `rg "GlobalEvents"` 只在文档中出现
- [ ] `rg "global_events\(\)"` 无结果
- [ ] `rg "update_input_state"` 无结果
- [ ] 所有测试通过
- [ ] 性能无回归
- [ ] 文档已更新

---

**状态**: 📋 **计划中**  
**下一步**: 迁移 DebugSystem → DebugSystemV2
