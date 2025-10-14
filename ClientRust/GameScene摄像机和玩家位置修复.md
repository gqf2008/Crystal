# GameScene 摄像机和玩家位置修复总结

## 问题分析

### 1. 摄像机限制在 (0, 0) 位置
**原因**: `Camera::follow_target()` 没有地图边界限制,当玩家在地图边缘时,摄像机会超出地图范围,导致显示空白区域。

### 2. 玩家位置网络报文处理
**现状**: 
- ✅ `UserInformation` 报文 → 创建玩家对象,设置初始位置
- ✅ `UserLocation` 报文 → 更新玩家位置 → 发送 `PlayerMoved` 事件
- ✅ `PlayerSpawned` 事件 → 同步玩家位置
- ✅ `PlayerMoved` 事件 → 更新玩家的 `current_location` 和 `movement`

## 修复内容

### 1. Camera 添加边界限制 (src/scenes/game_scene/camera.rs)

#### 新增方法: `follow_target_clamped()`

```rust
/// 设置摄像机跟随目标（带地图边界限制）
/// 
/// # 参数
/// - `world_x`: 目标世界坐标 X（像素）
/// - `world_y`: 目标世界坐标 Y（像素）
/// - `map_width_px`: 地图宽度（像素）
/// - `map_height_px`: 地图高度（像素）
pub fn follow_target_clamped(&mut self, world_x: f32, world_y: f32, map_width_px: f32, map_height_px: f32) {
    // 计算可视区域的半宽和半高
    let half_width = self.screen_width / (2.0 * self.zoom);
    let half_height = self.screen_height / (2.0 * self.zoom);
    
    // 限制摄像机位置，确保不超出地图边界
    // 摄像机中心不能小于半屏幕（否则会显示地图外）
    // 摄像机中心不能大于 地图尺寸 - 半屏幕
    let min_x = half_width.max(0.0);
    let max_x = (map_width_px - half_width).max(min_x);
    let min_y = half_height.max(0.0);
    let max_y = (map_height_px - half_height).max(min_y);
    
    self.x = world_x.clamp(min_x, max_x);
    self.y = world_y.clamp(min_y, max_y);
    
    tracing::trace!("📷 Camera clamped: target=({:.1}, {:.1}) → actual=({:.1}, {:.1}), bounds=[{:.1}-{:.1}, {:.1}-{:.1}]",
        world_x, world_y, self.x, self.y, min_x, max_x, min_y, max_y);
}
```

**关键逻辑**:
- 计算摄像机可视区域的半宽/半高
- 确保摄像机中心不超出 `[half_screen, map_size - half_screen]` 范围
- 处理小地图特殊情况 (地图比屏幕小)

### 2. GameScene 使用边界限制的摄像机 (src/scenes/game_scene.rs)

#### 修改 `draw()` 方法

```rust
/// 渲染场景 (Scene trait 要求的签名)
fn draw(&mut self, ctx: &mut ggez::Context, canvas: &mut crate::graphics::Canvas) {
    // 🎥 更新摄像机屏幕尺寸
    let (screen_width, screen_height) = ctx.gfx.drawable_size();
    self.camera.update_screen_size(screen_width, screen_height);
    
    // 🎥 更新摄像机跟随玩家（带地图边界限制）
    if let Some(ref user) = self.user {
        // 计算玩家的世界坐标（像素）
        let player_world_x = (user.player.map_object.movement.x as f32 * MapRenderer::CELL_WIDTH as f32) 
            + user.player.map_object.offset_move.x as f32;
        let player_world_y = (user.player.map_object.movement.y as f32 * MapRenderer::CELL_HEIGHT as f32) 
            + user.player.map_object.offset_move.y as f32;
        
        // 计算地图的像素尺寸
        let map_width_px = self.map_renderer.width as f32 * MapRenderer::CELL_WIDTH as f32;
        let map_height_px = self.map_renderer.height as f32 * MapRenderer::CELL_HEIGHT as f32;
        
        // 使用带边界限制的摄像机跟随
        self.camera.follow_target_clamped(player_world_x, player_world_y, map_width_px, map_height_px);
    }
    
    // ... 渲染逻辑
}
```

**关键变更**:
- 从 `follow_target()` 改为 `follow_target_clamped()`
- 传入地图像素尺寸进行边界检查

## 玩家位置更新流程

### 网络报文处理链路

```
服务器发送 UserLocation
    ↓
NetworkManager 接收并解析
    ↓
GameClient::on_user_location()
    - 更新 player.location
    - 发送 GameEvent::PlayerMoved
    ↓
GameScene::process_event(PlayerMoved)
    - 调用 user.player.map_object.set_current_location()
    - 同步 current_location 和 movement
    ↓
GameScene::draw()
    - 使用 movement 计算世界坐标
    - 摄像机跟随玩家
    - 绘制玩家角色
```

### 关键数据结构同步

```rust
// MapObject 中的位置字段
pub struct MapObject {
    pub current_location: Point,  // 当前显示位置
    pub movement: Point,           // 渲染用坐标（与 current_location 同步）
    pub map_location: Point,       // 地图网格位置
    // ...
}

// 位置更新时的同步逻辑
pub fn set_current_location(&mut self, location: Point) {
    self.current_location = location;
    // 🔧 CRITICAL: 同步 movement 以确保渲染正确
    self.movement = location;
}
```

## 测试验证

### 1. 摄像机边界测试

**测试场景**:
- 玩家移动到地图左上角 (0, 0) 附近
- 玩家移动到地图右下角 (max_x, max_y) 附近
- 玩家在地图中心移动

**预期结果**:
- ✅ 摄像机不会超出地图边界
- ✅ 不会显示黑色空白区域
- ✅ 玩家始终可见（除非在边缘）

### 2. 玩家位置同步测试

**测试场景**:
- 登录游戏时收到 `UserInformation`
- 玩家移动时收到 `UserLocation`
- 传送/跳转时位置突变

**预期结果**:
- ✅ 玩家角色显示在正确位置
- ✅ 摄像机跟随玩家移动
- ✅ 位置更新流畅无跳跃

### 3. 调试日志验证

启用 `RUST_LOG=trace` 查看调试日志:

```bash
# 摄像机边界限制日志
📷 Camera clamped: target=(100.0, 50.0) → actual=(256.0, 128.0), bounds=[256.0-4864.0, 128.0-3584.0]

# 玩家位置更新日志
📍 UserLocation received: (10, 15)
🚶 Player moved to: (10, 15)
✅ User position synced: current_location=Point(10, 15), movement=Point(10, 15)
```

## C# 原版参考

### 摄像机边界限制
**C# Client/MirScenes/GameScene.cs**:
```csharp
// MapControl.cs line ~800
private Point GetCameraCenter(Point playerLocation)
{
    int x = playerLocation.X * CellWidth + CellWidth / 2;
    int y = playerLocation.Y * CellHeight + CellHeight / 2;
    
    // Clamp to map bounds
    x = Math.Max(ViewPortWidth / 2, Math.Min(x, MapWidth * CellWidth - ViewPortWidth / 2));
    y = Math.Max(ViewPortHeight / 2, Math.Min(y, MapHeight * CellHeight - ViewPortHeight / 2));
    
    return new Point(x, y);
}
```

### 玩家位置处理
**C# Client/MirScenes/GameScene.cs**:
```csharp
// line ~1500
private void ProcessPacket_UserLocation(UserLocation p)
{
    User.CurrentLocation = new Point(p.Location.X, p.Location.Y);
    User.MapLocation = User.CurrentLocation;
    
    if (User.Movement == null)
        User.Movement = User.CurrentLocation;
}
```

## 代码改动总结

### 新增文件
无

### 修改文件
1. `src/scenes/game_scene/camera.rs`
   - 新增 `follow_target_clamped()` 方法 (+35 行)

2. `src/scenes/game_scene.rs`
   - 修改 `draw()` 方法使用边界限制的摄像机 (~10 行)

### 总行数变化
- 新增: ~45 行
- 修改: ~10 行
- 删除: 0 行

## 后续优化建议

### 1. 平滑摄像机移动
当前摄像机直接跟随玩家,可以添加插值实现平滑过渡:

```rust
pub fn smooth_follow(&mut self, target_x: f32, target_y: f32, lerp_factor: f32) {
    let dx = target_x - self.x;
    let dy = target_y - self.y;
    self.x += dx * lerp_factor;
    self.y += dy * lerp_factor;
}
```

### 2. 预测性摄像机
根据玩家移动方向,提前移动摄像机:

```rust
pub fn follow_with_prediction(&mut self, pos_x: f32, pos_y: f32, vel_x: f32, vel_y: f32) {
    let predict_x = pos_x + vel_x * 0.5;  // 预测 0.5 秒后的位置
    let predict_y = pos_y + vel_y * 0.5;
    self.follow_target_clamped(predict_x, predict_y, map_w, map_h);
}
```

### 3. 死区 (Dead Zone)
玩家在屏幕中心附近移动时,摄像机不移动:

```rust
pub fn follow_with_deadzone(&mut self, target_x: f32, target_y: f32, deadzone: f32) {
    let dx = target_x - self.x;
    let dy = target_y - self.y;
    let dist = (dx*dx + dy*dy).sqrt();
    
    if dist > deadzone {
        // 只有超出死区才移动摄像机
        self.x += dx - dx.signum() * deadzone;
        self.y += dy - dy.signum() * deadzone;
    }
}
```

## 问题修复确认

- ✅ **摄像机边界限制**: 添加 `follow_target_clamped()` 方法
- ✅ **玩家位置同步**: `UserLocation` → `PlayerMoved` → 更新 `movement`
- ✅ **初始位置设置**: `UserInformation` → 创建玩家对象并设置位置
- ✅ **渲染坐标正确**: 使用 `movement` 字段计算世界坐标

## 编译和运行

```bash
# 编译
cargo build --bin main_ggez

# 运行
cargo run --bin main_ggez

# 启用调试日志
RUST_LOG=trace cargo run --bin main_ggez
```

修复完成! 🎉
