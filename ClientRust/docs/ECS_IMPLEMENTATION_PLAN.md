# ECS系统完善计划

> **重要原则**：严格按照ECS架构，所有系统从 `GlobalEvents` 独立读取事件

---

## 当前状态总结

### ✅ 架构基础完成
- [x] 6层系统分层架构
- [x] SystemScheduler 调度器
- [x] GlobalEvents 事件总线
- [x] Scene 纯协调者模式（不处理任何事件）
- [x] 所有系统通过派生宏支持 `IntoSystemKind`

### ✅ 当前可运行的系统
- [x] `MovementSystem` - 移动逻辑（Layer 4）
- [x] `AnimationSystem` - 动画状态机（Layer 5）
- [x] `CameraFollowSystem` - 相机跟随（Layer 5）
- [x] `MapRenderSystem` - 地图渲染（Layer 6）
- [x] `DebugSystem` - 调试显示 + 键盘快捷键（Layer 6, HybridSystem）

---

## 优先级1：地图查看器核心功能

### 🔧 Task 1.1: CameraSystem 完善鼠标交互
**文件**: `src/ecs/systems/logic/state_update/camera_system.rs`

**当前问题**:
- ✅ 有 `start_drag()`, `update_drag()`, `end_drag()`, `zoom()` 静态方法
- ❌ 但 `update()` 方法中**没有从 GlobalEvents 读取鼠标事件**
- ❌ 这些静态方法是给旧代码用的，需要改造为从事件驱动

**需要实现**:
```rust
fn update(&mut self, world: &mut World, _delay_time: f32) -> GameResult {
    // 1. 从 GlobalEvents 读取输入事件
    let input_events = world.global_events().input_events.clone();
    
    // 2. 查询 Camera + Draggable + Position 组件
    for (entity, (camera, draggable, pos)) in world.query_mut::<(&mut Camera, &mut Draggable, &mut Position)>() {
        // 3. 处理鼠标事件
        for event in &input_events {
            match event {
                InputEvent::MouseDown { button: MouseButton::Middle, x, y } => {
                    // 开始拖拽
                }
                InputEvent::MouseMove { x, y, .. } => {
                    // 更新拖拽
                }
                InputEvent::MouseUp { button: MouseButton::Middle, .. } => {
                    // 结束拖拽
                }
                InputEvent::MouseWheel { y } => {
                    // 缩放
                }
                _ => {}
            }
        }
    }
    
    // 4. 应用震动效果（已实现）
    Ok(())
}
```

**验收标准**:
- [ ] 中键拖拽可以移动相机
- [ ] 鼠标滚轮可以缩放
- [ ] 缩放以鼠标位置为中心
- [ ] 不依赖 Scene 的任何事件处理

---

### 🔧 Task 1.2: DebugSystem 完善渲染功能
**文件**: `src/ecs/systems/render/debug_system.rs`

**当前状态**:
- ✅ `update()` 已实现：读取键盘事件，修改 RenderConfig
- ❌ `draw()` 是空实现

**需要实现**:
```rust
fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas, world: &hecs::World) -> GameResult {
    // 1. 读取 TimeTracker 获取 FPS
    // 2. 读取 Camera 获取坐标和缩放
    // 3. 读取 RenderConfig 查看是否显示调试信息
    // 4. 绘制调试文本（左上角）:
    //    - FPS: 60
    //    - Camera: (350, 350)
    //    - Zoom: 1.0x
    //    - Entities: 162744
    // 5. 如果 show_grid, 绘制网格
    // 6. 如果 show_borders, 绘制实体边框
}
```

**验收标准**:
- [ ] 左上角显示 FPS、相机坐标、缩放
- [ ] 按 G 键可以切换网格显示
- [ ] 按 B 键可以切换边框显示
- [ ] 网格和边框正确绘制在世界坐标系中

---

### 🔧 Task 1.3: MapRenderSystem 优化
**文件**: `src/ecs/systems/render/map_render_system.rs`

**当前状态**:
- ✅ 基础地图瓦片渲染
- ❌ 可能缺少性能优化

**需要验证**:
- [ ] 是否实现了视野裁剪（只渲染可见区域）
- [ ] 是否实现了 LOD（根据缩放级别选择细节）
- [ ] 是否正确读取 RenderConfig（show_back/middle/front）

**验收标准**:
- [ ] 地图正常显示
- [ ] 按 1/2/3 可以切换图层显示
- [ ] 缩放时性能稳定（60 FPS）

---

## 优先级2：场景切换和地图加载

### 🔧 Task 2.1: 创建 MapLoadSystem
**文件**: `src/ecs/systems/logic/state_update/map_load_system.rs` (新建)

**职责**:
- 从 `GlobalEvents.network_incoming` 读取 `MapChanged` 事件
- 清理当前地图实体
- 加载新地图数据
- 生成新的瓦片实体

**ECS 模式**:
```rust
pub struct MapLoadSystem;

impl System for MapLoadSystem {
    fn priority(&self) -> u32 {
        priority::MAP_LOAD // 550
    }
    
    fn update(&mut self, world: &mut World, _delay_time: f32) -> GameResult {
        // 1. 读取网络事件
        let network_events = world.global_events().network_incoming.clone();
        
        // 2. 查找 MapChanged 事件
        for event in network_events {
            if let GameEvent::MapChanged { file_name, .. } = event {
                // 3. 清理旧地图实体（查询所有带 TileLayer 组件的实体）
                // 4. 加载新地图
                // 5. 生成瓦片实体
                // 6. 更新相机位置
            }
        }
        
        Ok(())
    }
}
```

**验收标准**:
- [ ] 收到 MapChanged 事件后自动加载新地图
- [ ] 旧地图实体被正确清理
- [ ] 新地图正确显示

---

## 优先级3：实体渲染系统

### 🔧 Task 3.1: 创建 EntityRenderSystem
**文件**: `src/ecs/systems/render/entity_render_system.rs` (新建)

**职责**:
- 渲染玩家、怪物、NPC、特效
- 根据 Position、Direction、AnimationState 渲染精灵
- 实现 Z-Order 排序

**实现要点**:
```rust
pub struct EntityRenderSystem;

impl DrawSystem for EntityRenderSystem {
    fn priority(&self) -> u32 {
        priority::ENTITY_RENDER // 1100
    }
    
    fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas, world: &hecs::World) -> GameResult {
        // 1. 查询所有可渲染实体
        let mut entities: Vec<_> = world
            .query::<(&Position, &RenderSprite, Option<&AnimationState>)>()
            .iter()
            .collect();
        
        // 2. 按 Y 坐标排序（实现 Z-Order）
        entities.sort_by(|(_, (pos1, ..)), (_, (pos2, ..))| {
            pos1.y.partial_cmp(&pos2.y).unwrap()
        });
        
        // 3. 逐个绘制
        for (entity, (pos, sprite, anim)) in entities {
            // 根据 AnimationState 选择帧
            // 从 LibraryManager 获取图像
            // 绘制到 Canvas
        }
        
        Ok(())
    }
}
```

**验收标准**:
- [ ] 玩家精灵正确显示
- [ ] 怪物精灵正确显示
- [ ] Z-Order 排序正确（近处遮挡远处）
- [ ] 动画播放流畅

---

## 优先级4：网络事件处理

### 🔧 Task 4.1: 完善 NetworkSyncSystem
**文件**: `src/ecs/systems/logic/input/network_sync_system.rs`

**当前状态**: 可能已存在但需验证

**职责**:
- 从 `NetworkContext` 接收服务器消息
- 转换为 `GameEvent` 写入 `GlobalEvents.network_incoming`
- 从 `GlobalEvents.network_outgoing` 读取客户端命令
- 转换为 `ClientPacket` 发送到服务器

**验收标准**:
- [ ] 能接收服务器发送的所有消息类型
- [ ] 能发送客户端命令到服务器
- [ ] 事件队列正常工作

---

## 优先级5：玩家控制系统

### 🔧 Task 5.1: 完善 PlayerControlSystem
**文件**: `src/ecs/systems/logic/input/player_control_system.rs`

**需要验证**:
- [ ] 是否从 GlobalEvents 读取键盘输入
- [ ] 是否写入移动命令到玩家实体
- [ ] 是否处理攻击、技能等指令

---

## 实现流程建议

### 阶段1：让地图查看器完全可用（本周）
1. ✅ CameraSystem - 鼠标拖拽和缩放
2. ✅ DebugSystem - FPS 和调试信息显示
3. ✅ MapRenderSystem - 验证图层切换

### 阶段2：添加实体显示（下周）
1. EntityRenderSystem - 渲染玩家和怪物
2. MapLoadSystem - 支持切换地图

### 阶段3：完善网络和交互（下下周）
1. NetworkSyncSystem - 完整网络通信
2. PlayerControlSystem - 玩家移动和攻击

---

## ECS 设计原则 Checklist

每实现一个系统，必须确保：

- [ ] **只读 GlobalEvents**：从 `world.global_events()` 读取事件
- [ ] **不依赖 Scene**：Scene 不传递任何数据给系统
- [ ] **单一职责**：每个系统只做一件事
- [ ] **组件驱动**：所有数据存储在组件中
- [ ] **优先级正确**：确保系统按正确顺序执行
- [ ] **性能优化**：避免不必要的查询和克隆
- [ ] **可测试**：可以单独测试系统逻辑

---

## 参考代码模板

### 逻辑系统模板
```rust
use crate::ecs::components::GlobalEvents;
use crate::ecs::systems::{System, priority};
use crate::ecs::WorldExt;
use ggez::GameResult;
use hecs::World;

pub struct MyLogicSystem;

impl System for MyLogicSystem {
    fn priority(&self) -> u32 {
        priority::MY_PRIORITY
    }
    
    fn update(&mut self, world: &mut World, delay_time: f32) -> GameResult {
        // 1. 读取事件
        let events = world.global_events().input_events.clone();
        
        // 2. 查询组件
        for (entity, (comp1, comp2)) in world.query_mut::<(&mut Comp1, &Comp2)>() {
            // 3. 处理逻辑
        }
        
        Ok(())
    }
}
```

### 渲染系统模板
```rust
use crate::ecs::render::DrawSystem;
use crate::ecs::systems::priority;
use ggez::GameResult;
use ggez::graphics::Canvas;
use ggez::Context;
use hecs::World;

pub struct MyRenderSystem;

impl DrawSystem for MyRenderSystem {
    fn priority(&self) -> u32 {
        priority::MY_RENDER_PRIORITY
    }
    
    fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas, world: &hecs::World) -> GameResult {
        // 1. 查询组件（只读）
        for (entity, (pos, sprite)) in world.query::<(&Position, &Sprite)>().iter() {
            // 2. 绘制
        }
        
        Ok(())
    }
}
```

---

## 常见陷阱

### ❌ 错误：在 Scene 中处理事件
```rust
// scene.rs - 错误示例
fn handle_mouse_down(&mut self, x: f32, y: f32) {
    // 不应该在这里处理！
}
```

### ✅ 正确：在系统中读取事件
```rust
// my_system.rs - 正确示例
fn update(&mut self, world: &mut World, _: f32) -> GameResult {
    let events = world.global_events().input_events.clone();
    for event in events {
        if let InputEvent::MouseDown { x, y, .. } = event {
            // 在这里处理
        }
    }
    Ok(())
}
```

---

## 下一步行动

**立即开始**: Task 1.1 - CameraSystem 完善鼠标交互

这是让地图查看器可用的第一步，也是验证 ECS 架构的关键。
