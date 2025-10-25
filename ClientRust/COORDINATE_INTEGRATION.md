# 坐标系统整合方案

## 🎯 当前状况

### 现有系统
1. **CameraSystem** (`src/ecs/systems/camera.rs`)
   - 简单的摄像机视图转换
   - `world_to_screen()`: 相对摄像机中心的坐标转换
   - 用于缩放、拖拽

2. **CoordinateSystem** (`src/ecs/coordinate_system.rs`) - 新创建
   - 完整复现原版传奇2的坐标计算
   - 支持格子坐标、世界坐标、屏幕坐标三大系统
   - 包含 OffSetX/Y、ViewRange、PixelOffset 等原版概念

### 问题
两个系统功能重叠，需要明确分工和整合。

---

## 🏗️ 整合方案

### 方案A: CameraSystem 专注视图变换，CoordinateSystem 处理逻辑坐标

```rust
// CameraSystem: 只处理渲染层面的变换 (缩放、拖拽、屏幕适配)
// - 输入: 逻辑屏幕坐标 (CoordinateSystem 计算出的)
// - 输出: 最终渲染坐标 (考虑缩放、偏移)

// CoordinateSystem: 处理游戏逻辑坐标 (格子→世界→逻辑屏幕)
// - 输入: 地图格子坐标
// - 输出: 逻辑屏幕坐标 (未缩放)
```

#### 渲染流程
```
地图坐标 (286, 617)
    ↓ CoordinateSystem::grid_to_world()
世界坐标 (13728.0, 19744.0)
    ↓ CoordinateSystem::to_screen_position()
逻辑屏幕坐标 (480.0, 352.0)  ← 这是"标准"屏幕坐标
    ↓ CameraSystem::world_to_screen()  [可选]
最终渲染坐标 (480.0 * zoom + offset_x, 352.0 * zoom + offset_y)
    ↓
canvas.draw()
```

### 方案B: 完全使用 CoordinateSystem，废弃 CameraSystem

```rust
// 移除 CameraSystem，所有坐标计算由 CoordinateSystem 统一处理
// 优点: 完全复现原版逻辑
// 缺点: 需要在 CoordinateSystem 中添加缩放、拖拽支持
```

---

## ✅ 推荐方案: 方案A (职责分离)

### 理由
1. **职责清晰**: 
   - `CoordinateSystem`: 游戏逻辑坐标系 (格子/世界/屏幕)
   - `CameraSystem`: 渲染视图变换 (缩放/拖拽/适配)

2. **兼容性好**: 
   - 保留现有 CameraSystem 的缩放、拖拽功能
   - 新增 CoordinateSystem 复现原版坐标逻辑

3. **扩展性强**:
   - 未来可以添加更多视图效果 (震屏、慢动作等)
   - CoordinateSystem 保持纯净的坐标转换逻辑

### 实现步骤

#### 1. 重构 render.rs 中的角色渲染

```rust
// 旧代码 (render.rs:708-730)
let world_x = player_pos.x + 24.0;
let world_y = player_pos.y + 32.0 - char_h as f32;

let (screen_x, screen_y) = CameraSystem::world_to_screen(
    camera_pos, 
    camera, 
    world_x,
    world_y
);

// 新代码 (使用 CoordinateSystem)
use crate::ecs::coordinate_system::{CoordinateSystem, CELL_WIDTH, CELL_HEIGHT};

// 1. 获取玩家的世界坐标和像素偏移
let (player_world, player_pixel_offset) = get_player_position(world)?;

// 2. 计算对象的逻辑屏幕坐标 (DrawLocation)
let viewport = ViewportConfig::new(800.0, 600.0);  // 从配置读取
let coord_sys = CoordinateSystem::new(viewport);

let (draw_x, draw_y) = coord_sys.to_screen_position(
    (world_x, world_y),              // 对象世界坐标
    player_world,                     // 玩家世界坐标
    player_pixel_offset,              // 玩家像素偏移
    (0.0, 0.0),                      // 对象像素偏移
    true,                             // 是否是玩家
);

// 3. 应用纹理偏移 (FinalDrawLocation)
let final_x = draw_x + offset_x as f32;
let final_y = draw_y + offset_y as f32;

// 4. [可选] 应用摄像机变换 (缩放、拖拽)
let (screen_x, screen_y) = if camera.zoom != 1.0 {
    CameraSystem::apply_camera_transform(camera, final_x, final_y)
} else {
    (final_x, final_y)
};

// 5. 绘制
canvas.draw(texture, DrawParam::default().dest([screen_x, screen_y]));
```

#### 2. 添加辅助函数

```rust
// src/ecs/systems/render.rs

/// 获取玩家位置和偏移 (用于坐标计算)
fn get_player_position(world: &World) -> anyhow::Result<((f32, f32), (f32, f32))> {
    for (_, (pos, pixel_offset, _local)) in world.query::<(
        &Position,
        Option<&PixelOffset>,
        &LocalPlayer
    )>().iter() {
        let world_pos = (pos.x, pos.y);
        let offset = pixel_offset.map(|o| (o.x, o.y)).unwrap_or((0.0, 0.0));
        return Ok((world_pos, offset));
    }
    
    Err(anyhow::anyhow!("玩家实体不存在"))
}

/// 应用摄像机变换 (缩放、偏移)
impl CameraSystem {
    pub fn apply_camera_transform(camera: &Camera, x: f32, y: f32) -> (f32, f32) {
        // 如果需要缩放或偏移，在这里处理
        (x * camera.zoom, y * camera.zoom)
    }
}
```

#### 3. 添加 PixelOffset 组件

```rust
// src/ecs/components.rs

/// 像素级移动偏移 (对应原版 OffSetMove)
/// 
/// 用于平滑移动动画:
/// - 站立时: (0, 0)
/// - 移动中: (0~48, 0~32) 像素
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct PixelOffset {
    pub x: f32,
    pub y: f32,
}
```

#### 4. 更新网络系统添加 PixelOffset

```rust
// src/ecs/systems/network.rs

fn handle_player_moved(&mut self, world: &mut World, location: &Point) {
    // ... 现有代码 ...
    
    // 添加 PixelOffset 组件 (如果不存在)
    if world.get::<&PixelOffset>(entity).is_err() {
        world.insert_one(entity, PixelOffset::default())?;
    }
}
```

---

## 📊 完整坐标流程图

```
┌─────────────────────────────────────────────────────────────┐
│  服务器网络包: UserLocation { x: 286, y: 617 }             │
└────────────────────────┬────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────────┐
│  NetworkSystem: handle_player_moved()                       │
│  - 更新 GridPosition(286, 617)                             │
│  - 计算 WorldPosition(13728.0, 19744.0)                   │
│  - 初始化 PixelOffset(0.0, 0.0)                           │
└────────────────────────┬────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────────┐
│  MovementSystem: 处理平滑移动                               │
│  - 更新 PixelOffset (0→24→48 像素插值)                    │
│  - 移动完成后重置为 (0, 0)                                │
└────────────────────────┬────────────────────────────────────┘
                         ↓
┌─────────────────────────────────────────────────────────────┐
│  RenderSystem: 绘制角色                                     │
│                                                             │
│  1. 查询玩家位置:                                          │
│     player_world = (13728.0, 19744.0)                      │
│     player_pixel_offset = (0.0, 0.0)                       │
│                                                             │
│  2. 计算纹理位置:                                          │
│     world_x = player_world.x + 24.0  (AABB中心)            │
│     world_y = player_world.y + 32.0 - char_h               │
│                                                             │
│  3. CoordinateSystem 转换:                                 │
│     draw_location = to_screen_position(...)                │
│     → (480.0, 352.0)  [逻辑屏幕坐标]                      │
│                                                             │
│  4. 应用纹理偏移:                                          │
│     final_location = draw_location + texture_offset        │
│     → (480.0 + offsetX, 352.0 + offsetY)                  │
│                                                             │
│  5. [可选] CameraSystem 变换:                              │
│     render_pos = apply_camera_transform(final_location)    │
│     → (final_x * zoom, final_y * zoom)                    │
│                                                             │
│  6. 绘制:                                                  │
│     canvas.draw(texture, render_pos)                       │
└─────────────────────────────────────────────────────────────┘
```

---

## 🚀 迁移计划

### Phase 1: 添加新组件和系统 ✅
- [x] 创建 `CoordinateSystem` 模块
- [x] 创建 `COORDINATE_SYSTEM.md` 文档
- [x] 添加到 `mod.rs` 导出

### Phase 2: 添加 PixelOffset 组件
- [ ] 在 `components.rs` 添加 `PixelOffset` 组件
- [ ] 在 `NetworkSystem` 中初始化
- [ ] 在 `MovementSystem` 中更新 (平滑移动)

### Phase 3: 重构 RenderSystem
- [ ] 重构角色渲染使用 `CoordinateSystem`
- [ ] 重构地图瓦片渲染
- [ ] 重构UI元素定位

### Phase 4: 测试和优化
- [ ] 对比原版客户端验证坐标正确性
- [ ] 性能测试和优化
- [ ] 添加单元测试

---

## 📝 注意事项

1. **保持向后兼容**: 不要立即删除 `CameraSystem`，先并存运行
2. **逐步迁移**: 先迁移角色渲染，再迁移地图、UI
3. **充分测试**: 每个阶段都要对比原版验证坐标正确性
4. **文档同步**: 更新所有相关文档说明新坐标系统

---

**作者**: GitHub Copilot  
**日期**: 2025-10-25  
**版本**: 1.0
