# 相机系统文档

## 📐 概述

相机系统基于 `CoordinateSystem` 构建,提供灵活的视野控制功能。

## 🎯 核心功能

### 1. 跟随模式 (Following)
- 平滑跟随玩家移动
- 可调节跟随速度
- 默认模式

### 2. 自由模式 (Free)
- 相机不自动移动
- 用于观察/调试
- 可手动控制位置

### 3. 过渡模式 (Transitioning)
- 平滑移动到目标位置
- 自定义过渡时间
- Ease-out 缓动效果

### 4. 镜头震动 (Shake)
- 可配置强度和持续时间
- 随机方向震动
- 适用于爆炸、攻击等特效

## 📦 快速开始

### 创建相机

```rust
use crate::ecs::coordinate_system::{CoordinateSystem, ViewportConfig, Camera};

// 1. 创建视野配置
let viewport = ViewportConfig::new(1024.0, 768.0);

// 2. 创建坐标系统
let coord_system = CoordinateSystem::new(viewport);

// 3. 创建相机
let mut camera = Camera::new(coord_system);

// 4. 可选: 设置地图边界
camera.set_map_bounds(0, 0, 500, 500);
```

### 每帧更新

```rust
// 在游戏主循环中
fn update(&mut self, delta_time: f32) {
    // 获取玩家位置
    let player_pos = (player.x, player.y);
    
    // 更新相机 (自动跟随)
    self.camera.update(delta_time, player_pos);
}
```

### 坐标转换

```rust
// 世界坐标 → 屏幕坐标
let screen_pos = camera.world_to_screen((world_x, world_y));

// 屏幕坐标 → 世界坐标 (鼠标点击)
let world_pos = camera.screen_to_world((mouse_x, mouse_y));

// 检查是否在视野内
if camera.is_visible(monster_pos) {
    // 渲染怪物
}
```

## 🎮 使用场景

### 场景1: 游戏开始,相机平滑移动到玩家

```rust
// GameScene 初始化
pub fn new(ctx: &mut Context) -> Self {
    let viewport = ViewportConfig::new(1024.0, 768.0);
    let coord_system = CoordinateSystem::new(viewport);
    let mut camera = Camera::new(coord_system);
    
    // 找到玩家出生点
    let spawn_pos = MapUtils::find_center_walkable_position(&map_data);
    let world_pos = CoordinateSystem::grid_to_world_center(spawn_pos.0, spawn_pos.1);
    
    // 相机平滑移动到玩家 (2秒过渡)
    camera.transition_to(world_pos, 2.0);
    
    Self { camera, /* ... */ }
}
```

### 场景2: 玩家攻击时触发镜头震动

```rust
impl PlayerSystem {
    pub fn on_attack(&mut self) {
        // 触发中等强度震动 (5像素, 0.3秒)
        self.camera.shake(5.0, 0.3);
    }
    
    pub fn on_critical_hit(&mut self) {
        // 暴击时强烈震动 (15像素, 0.5秒)
        self.camera.shake(15.0, 0.5);
    }
}
```

### 场景3: 观察模式 (调试用)

```rust
impl GameScene {
    pub fn toggle_free_camera(&mut self) {
        if self.camera_free {
            // 切换回跟随模式
            self.camera.set_follow_mode();
            self.camera_free = false;
        } else {
            // 切换到自由模式
            self.camera.set_free_mode();
            self.camera_free = true;
        }
    }
    
    pub fn move_camera(&mut self, dx: f32, dy: f32) {
        if self.camera_free {
            // 手动移动相机
            let (x, y) = self.camera.position;
            self.camera.position = (x + dx, y + dy);
        }
    }
}
```

### 场景4: 过场动画

```rust
impl CutsceneSystem {
    pub fn play_intro(&mut self) {
        // 相机从远处飞向玩家
        let start_pos = (0.0, 0.0);
        let end_pos = self.player_position;
        
        self.camera.jump_to(start_pos);
        self.camera.transition_to(end_pos, 3.0);
    }
}
```

### 场景5: 技能释放跟踪

```rust
impl SkillSystem {
    pub fn cast_fireball(&mut self, target_pos: (f32, f32)) {
        // 临时跟踪火球飞行
        self.camera.set_free_mode();
        self.camera.transition_to(target_pos, 1.0);
        
        // 1秒后回到跟随模式
        self.schedule_callback(1.0, || {
            self.camera.set_follow_mode();
        });
    }
}
```

## ⚙️ 配置选项

### 跟随平滑度

```rust
// 值越大跟随越快 (0.0-1.0)
camera.set_follow_smoothness(0.15);  // 默认: 较慢,更平滑
camera.set_follow_smoothness(0.5);   // 快速跟随
camera.set_follow_smoothness(1.0);   // 立即跟随 (无延迟)
```

### 地图边界

```rust
// 限制相机不超出地图范围
camera.set_map_bounds(
    0,              // min_x
    0,              // min_y
    map_width - 1,  // max_x
    map_height - 1  // max_y
);
```

### 缩放 (预留功能)

```rust
// 未来可实现缩放功能
camera.zoom = 1.5;  // 放大 1.5 倍
camera.zoom = 0.5;  // 缩小 0.5 倍
```

## 🧪 测试

```bash
# 运行相机系统测试
cargo test camera_tests
```

测试包括:
- ✅ 跟随模式收敛测试
- ✅ 镜头震动测试
- ✅ 坐标转换准确性

## 🎨 渲染集成

### 在 GameScene 中使用

```rust
impl GameScene {
    fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas) {
        // 获取相机最终位置 (包含震动)
        let camera_pos = self.camera.get_final_position();
        let camera_grid = self.camera.get_grid_position();
        
        // 渲染地图
        self.render_system.draw_map(
            ctx,
            canvas,
            &self.map_data,
            camera_pos,
        );
        
        // 渲染玩家
        for (entity, (player, position)) in world.query::<(&Player, &Position)>().iter() {
            if self.camera.is_visible((position.x, position.y)) {
                let screen_pos = self.camera.world_to_screen((position.x, position.y));
                
                // 绘制玩家...
            }
        }
        
        // 渲染怪物/NPC (只渲染可见对象)
        for monster in &self.monsters {
            if self.camera.is_visible(monster.position) {
                let screen_pos = self.camera.world_to_screen(monster.position);
                // 绘制怪物...
            }
        }
    }
}
```

## 🔧 API 参考

### Camera 方法

| 方法 | 说明 | 示例 |
|------|------|------|
| `new(coord_system)` | 创建相机 | `Camera::new(coord)` |
| `update(dt, player_pos)` | 更新相机 (每帧) | `camera.update(0.016, pos)` |
| `jump_to(pos)` | 立即跳转 | `camera.jump_to((1000.0, 2000.0))` |
| `transition_to(pos, duration)` | 平滑过渡 | `camera.transition_to(pos, 2.0)` |
| `shake(intensity, duration)` | 触发震动 | `camera.shake(10.0, 0.5)` |
| `set_follow_mode()` | 切换到跟随模式 | `camera.set_follow_mode()` |
| `set_free_mode()` | 切换到自由模式 | `camera.set_free_mode()` |
| `world_to_screen(pos)` | 世界→屏幕 | `camera.world_to_screen(world_pos)` |
| `screen_to_world(pos)` | 屏幕→世界 | `camera.screen_to_world(mouse_pos)` |
| `is_visible(pos)` | 检查是否可见 | `camera.is_visible(monster_pos)` |
| `get_final_position()` | 获取相机位置 | `camera.get_final_position()` |
| `get_grid_position()` | 获取格子坐标 | `camera.get_grid_position()` |

### 属性

```rust
pub struct Camera {
    pub position: (f32, f32),  // 相机世界坐标
    pub zoom: f32,             // 缩放级别 (预留)
    // ...
}
```

## 📊 性能考虑

### 视野剔除优化

```rust
// ✅ 好: 只渲染可见对象
for monster in &monsters {
    if camera.is_visible(monster.position) {
        render_monster(monster);
    }
}

// ❌ 差: 渲染所有对象
for monster in &monsters {
    render_monster(monster);  // 浪费性能
}
```

### 批量检查

```rust
// 对于大量对象,考虑空间分区
let visible_objects = spatial_grid.query_range(camera_view_rect);
for obj in visible_objects {
    render(obj);
}
```

## 🐛 常见问题

### Q: 相机跟随太慢/太快?
A: 调整 `follow_smoothness`:
```rust
camera.set_follow_smoothness(0.3);  // 增加数值加快跟随
```

### Q: 相机超出地图边界?
A: 设置地图边界限制:
```rust
camera.set_map_bounds(0, 0, max_x, max_y);
```

### Q: 震动效果不明显?
A: 增加震动强度:
```rust
camera.shake(20.0, 0.8);  // 更强烈的震动
```

### Q: 如何实现镜头缩放?
A: 目前 `zoom` 属性预留,未来版本实现。

## 🚀 未来扩展

计划功能:
- [ ] 镜头缩放实现
- [ ] 多相机支持
- [ ] 相机路径动画
- [ ] 景深效果
- [ ] 运动模糊
- [ ] 视差滚动

## 📚 相关模块

- `CoordinateSystem` - 坐标转换基础
- `MapUtils` - 地图工具函数
- `ViewportConfig` - 视野配置
- `RenderSystem` - 渲染系统

---

**创建日期**: 2025-10-25  
**维护者**: Crystal Team
