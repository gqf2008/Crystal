# 坐标系统架构文档

## 📐 设计理念

**核心原则**: 所有坐标转换必须通过 `CoordinateSystem` 模块,避免重复实现和计算不一致。

## 🎯 三大坐标系

### 1. 地图坐标 (Grid Coordinates)
- **类型**: `(i32, i32)`
- **单位**: 格子
- **用途**: 逻辑层面的位置表示 (服务器通信、寻路、碰撞检测)
- **示例**: `(286, 617)` 表示第286列、第617行的格子

### 2. 世界坐标 (World Coordinates)
- **类型**: `(f32, f32)`
- **单位**: 像素
- **用途**: 物理层面的精确位置
- **示例**: 
  - 格子左上角: `(13728.0, 19744.0)` = `286 * 48, 617 * 32`
  - 格子中心点: `(13752.0, 19760.0)` = `286 * 48 + 24, 617 * 32 + 16`

### 3. 屏幕坐标 (Screen Coordinates)
- **类型**: `(f32, f32)`
- **单位**: 像素
- **用途**: 渲染层面的显示位置
- **计算**: 相对玩家位置 + 视野偏移

## 🔧 核心API

### `CoordinateSystem` (src/ecs/coordinate_system.rs)

#### 基础转换

```rust
// 格子 → 世界 (左上角)
pub fn grid_to_world(grid_x: i32, grid_y: i32) -> (f32, f32)

// 格子 → 世界 (中心点) 🎯 玩家/NPC/怪物位置
pub fn grid_to_world_center(grid_x: i32, grid_y: i32) -> (f32, f32)

// 世界 → 格子 (使用 floor!)
pub fn world_to_grid(world_x: f32, world_y: f32) -> (i32, i32)
```

#### 屏幕相关

```rust
// 对象世界坐标 → 屏幕坐标
pub fn to_screen_position(...) -> (f32, f32)

// 鼠标屏幕坐标 → 地图格子
pub fn screen_to_grid(...) -> (i32, i32)

// 检查是否在视野内
pub fn is_in_viewport(...) -> bool
```

## 🏗️ 委托关系

为保持兼容性,现有模块委托给 `CoordinateSystem`:

### `MapHelper` (src/ecs/map_helper.rs)
```rust
impl MapHelper {
    pub fn grid_to_world(grid_x: i32, grid_y: i32) -> (f32, f32) {
        CoordinateSystem::grid_to_world_center(grid_x, grid_y)  // 委托
    }
    
    pub fn world_to_grid(world_x: f32, world_y: f32) -> (i32, i32) {
        CoordinateSystem::world_to_grid(world_x, world_y)  // 委托
    }
}
```

### `Position` (src/ecs/components.rs)
```rust
impl Position {
    pub fn to_grid(&self) -> (i32, i32) {
        CoordinateSystem::world_to_grid(self.x, self.y)  // 委托
    }
}
```

## ⚠️ 关键注意事项

### 1. Floor vs Round vs Truncate

**❌ 错误方式**:
```rust
let grid_x = (world_x / 48.0).round() as i32;  // ❌ 会导致坐标跳变!
let grid_x = (world_x / 48.0) as i32;          // ❌ 负数时错误!
```

**✅ 正确方式**:
```rust
let grid_x = (world_x / 48.0).floor() as i32;  // ✅ 统一向下取整
```

**原因**:
```
格子 (5, 10) 中心 = (264.0, 336.0)

round(): (264/48).round() = 5.5.round() = 6  ❌ 错了!
floor(): (264/48).floor() = 5.5.floor() = 5  ✅ 正确!
```

### 2. 格子左上角 vs 中心点

| 用途 | 函数 | 返回值 |
|------|------|--------|
| 玩家/NPC/怪物位置 | `grid_to_world_center()` | 格子中心 |
| 地图块渲染 | `grid_to_world()` | 格子左上角 |

### 3. 格子尺寸常量

```rust
pub const CELL_WIDTH: i32 = 48;   // 格子宽度
pub const CELL_HEIGHT: i32 = 32;  // 格子高度 (等距视角)
```

⚠️ **不要在代码中硬编码 48/32!** 使用常量或调用转换函数。

## 📊 使用场景

### 场景1: A*寻路后移动
```rust
// 1. A*返回格子路径
let path: Vec<(i32, i32)> = pathfinder.find_path(...);

// 2. 转换为世界坐标 (格子中心)
let (target_x, target_y) = CoordinateSystem::grid_to_world_center(
    path[index].0, 
    path[index].1
);

// 3. 插值移动
player.position.x += dx * speed;
player.position.y += dy * speed;
```

### 场景2: 鼠标点击寻路
```rust
// 1. 屏幕坐标 → 格子坐标
let (target_grid_x, target_grid_y) = coord_system.screen_to_grid(
    mouse_x, 
    mouse_y, 
    player_grid
);

// 2. 检查是否可行走
if MapHelper::is_walkable(map_data, target_grid_x, target_grid_y) {
    // 3. 寻路...
}
```

### 场景3: 服务器位置同步
```rust
// 服务器发来格子坐标
let server_grid = (location.x, location.y);

// 转换为世界坐标 (中心点)
let (world_x, world_y) = CoordinateSystem::grid_to_world_center(
    server_grid.0, 
    server_grid.1
);

// 同步客户端位置
position.x = world_x;
position.y = world_y;
```

## 🔍 调试技巧

### 坐标对比日志
```rust
let grid = CoordinateSystem::world_to_grid(pos.x, pos.y);
let world_center = CoordinateSystem::grid_to_world_center(grid.0, grid.1);

tracing::info!(
    "位置: world=({:.1}, {:.1}) grid=({}, {}) center=({:.1}, {:.1})",
    pos.x, pos.y, grid.0, grid.1, world_center.0, world_center.1
);
```

### 验证一致性
```rust
// 往返转换应该一致 (误差 < 格子大小)
let original_grid = (10, 20);
let world = CoordinateSystem::grid_to_world_center(original_grid.0, original_grid.1);
let result_grid = CoordinateSystem::world_to_grid(world.0, world.1);
assert_eq!(original_grid, result_grid);  // 应该相等
```

## 🚀 未来扩展

### 潜在功能
1. **子像素精度**: 支持更平滑的移动动画
2. **坐标缓存**: 常用转换结果缓存
3. **批量转换**: SIMD优化大量坐标转换
4. **坐标验证**: Debug模式下自动检查坐标合法性

### 相机系统集成
```rust
pub struct Camera {
    coord_system: CoordinateSystem,
    zoom: f32,
    // ...
}
```

## 📚 参考

- **原版代码**: `Client/MirScenes/GameScene.cs`
- **相关模块**:
  - `src/ecs/coordinate_system.rs` - 坐标系统核心
  - `src/ecs/map_helper.rs` - 地图辅助函数
  - `src/ecs/components.rs` - Position组件
  - `src/ecs/systems/player.rs` - 玩家移动系统
  - `src/ecs/systems/network.rs` - 网络同步

---

**最后更新**: 2025-10-25  
**维护者**: Crystal Team
