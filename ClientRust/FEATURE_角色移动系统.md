# 🎮 角色移动功能实现

## 实现状态

✅ **已完成**:
1. 添加了移动辅助方法（can_walk, can_walk_adjust, empty_cell等）
2. 实现了方向计算和坐标转换
3. 实现了鼠标右键点击移动
4. 集成到update循环中

⏳ **待实现**:
1. 发送移动网络包给服务器
2. 平滑的移动动画（插值）
3. 路径寻找（A*）

## 核心代码

### 移动检测方法 (`src/scenes/game_scene.rs`)

```rust
// 检查是否可以向指定方向移动
pub fn can_walk(&self, dir: MirDirection) -> bool

// 检查移动并尝试调整方向（如果碰墙尝试相邻方向）
pub fn can_walk_adjust(&self, dir: MirDirection) -> Option<MirDirection>

// 检查格子是否可行走
fn empty_cell(&self, p: Point) -> bool

// 计算从某点向指定方向移动n格后的位置
fn point_move(&self, p: Point, d: MirDirection, count: i32) -> Point

// 根据两点计算方向
fn direction_from_point(&self, source: Point, dest: Point) -> MirDirection
```

### 鼠标点击处理 (`src/scenes/game_scene.rs`)

```rust
pub fn on_mouse_down(&mut self, button: MouseButton, location: Point) {
    match button {
        MouseButton::Right => {
            // 1. 屏幕坐标 -> 地图坐标
            let map_location = self.screen_to_map_location(location);
            
            // 2. 检查是否可行走
            if !self.empty_cell(map_location) {
                return;
            }
            
            // 3. 计算方向并移动
            let direction = self.direction_from_point(current_loc, map_location);
            if let Some(adjusted_dir) = self.can_walk_adjust(direction) {
                let target_loc = self.point_move(current_loc, adjusted_dir, 1);
                
                // 更新玩家位置
                user.player.map_object.movement = target_loc;
                user.player.map_object.direction = adjusted_dir;
                user.player.map_object.current_action = MirAction::Walking;
            }
        }
        _ => {}
    }
}
```

## 测试方法

1. **启动游戏**:
   ```powershell
   cargo run
   ```

2. **登录并进入游戏**

3. **测试移动**:
   - 鼠标右键点击地面
   - 观察控制台日志：
     ```
     INFO 🖱️ 鼠标右键点击: 屏幕(x, y) -> 地图(x, y)
     INFO ✅ 设置移动: 从(x, y) 向Down 到(x, y)
     ```

## 当前限制

1. **单步移动**: 每次点击只移动一格
   - 原因：未实现路径寻找
   - 解决方案：后续添加A*寻路算法

2. **瞬间移动**: 没有平滑动画
   - 原因：直接更新位置，未实现插值
   - 解决方案：添加移动插值（offset_move）

3. **不发送网络包**: 本地移动，服务器不知道
   - 原因：网络层集成未完成
   - 解决方案：发送`C.Walk`包

4. **坐标转换简化**: screen_to_map_location使用简单除法
   - 原因：传奇2的等距视角转换较复杂
   - 解决方案：实现精确的等距坐标转换

## 下一步计划

### 1. 网络包发送

```rust
// 在移动时发送C.Walk包
use mir2_shared::packets::client::Walk;
network.send(Walk {
    direction: adjusted_dir,
});
```

### 2. 平滑移动动画

```rust
// 使用offset_move实现插值
pub fn update_movement_interpolation(&mut self, delta_time: f32) {
    if self.is_moving {
        // 插值计算
        self.offset_move.x += speed * delta_time;
        // ...
    }
}
```

### 3. 路径寻找

```rust
// 使用A*算法计算路径
let path = pathfinder.find_path(start, end);
for step in path {
    // 分步移动
}
```

## 参考文件

- **C#原版**: `Client/MirScenes/GameScene.cs`
  - Line 12365: `OnMouseClick` - 鼠标点击处理
  - Line 13174: `CanWalk` - 移动检测
  - Line 13179: `CanWalk(out)` - 调整方向的移动检测

- **Rust实现**: `ClientRust/src/scenes/game_scene.rs`
  - Line 1113: `on_mouse_down` - 鼠标点击处理
  - Line 1150: 移动辅助方法

## 调试技巧

### 启用移动日志

代码中已包含详细日志：
```rust
tracing::info!("🖱️ 鼠标右键点击: 屏幕({}, {}) -> 地图({}, {})", ...);
tracing::info!("✅ 设置移动: 从({}, {}) 向{:?} 到({}, {})", ...);
tracing::warn!("❌ 目标位置({}, {})不可行走", ...);
```

### 检查障碍物

查看地图渲染器的可行走检测：
```rust
// MapRenderer中
pub fn is_walkable(&self, x: i32, y: i32) -> bool {
    self.get_cell(x, y)
        .map(|cell| cell.is_walkable())
        .unwrap_or(false)
}
```

## 已知问题

暂无

## 编译状态

✅ 编译成功 (2025-10-15)
- 无错误
- 仅有一些未使用变量警告（正常）
