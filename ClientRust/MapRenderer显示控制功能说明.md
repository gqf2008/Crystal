# MapRenderer 显示控制功能说明

## 📋 新增功能

在 `MapRenderer` 中新增了 7 个显示控制开关,用于调试和查看地图渲染细节。

## 🎮 显示控制参数

```rust
pub struct MapRenderer {
    // ... 其他字段 ...
    
    // 🎮 显示控制开关（调试用）
    pub show_grid: bool,         // G键：显示地图网格
    pub show_borders: bool,      // B键：显示纹理边框
    pub show_layer_back: bool,   // 1键：显示Back层
    pub show_layer_middle: bool, // 2键：显示Middle层
    pub show_layer_front: bool,  // 3键：显示Front层
    pub show_obstacles: bool,    // O键：显示障碍层
    pub show_animations: bool,   // A键：显示动画
}
```

## 📝 参数说明

### 1. `show_grid` - 地图网格
- **默认值**: `false`
- **功能**: 显示地图格子网格线
- **效果**: 绿色半透明网格线,每个格子 48x32 像素
- **用途**: 
  - 调试坐标系统
  - 查看瓦片对齐
  - 定位渲染问题

### 2. `show_borders` - 纹理边框
- **默认值**: `false`
- **功能**: 显示每个纹理的边框
- **效果**: 
  - Back/Middle 层: 红色边框
  - Front 层: 蓝色边框
- **用途**:
  - 查看纹理实际尺寸
  - 检查纹理重叠
  - 调试绘制顺序

### 3. `show_layer_back` - Back 层
- **默认值**: `true`
- **功能**: 控制 Back 层（大地砖）显示
- **内容**: 
  - 96x64 大瓦片（偶数行列）
  - 地面基础纹理
- **用途**: 单独查看地面层

### 4. `show_layer_middle` - Middle 层
- **默认值**: `true`
- **功能**: 控制 Middle 层（小地砖）显示
- **内容**:
  - 48x32 小瓦片
  - 地面细节纹理
  - 地面动画效果
- **用途**: 单独查看地面细节

### 5. `show_layer_front` - Front 层
- **默认值**: `true`
- **功能**: 控制 Front 层（前景）显示
- **内容**:
  - 建筑、树木等大型物体
  - 门动画
  - 前景特效
- **用途**: 单独查看前景物体

### 6. `show_obstacles` - 障碍层
- **默认值**: `false`
- **功能**: 显示障碍物标记
- **效果**: 半透明红色矩形覆盖不可行走格子
- **检测规则**:
  - `HighWall` (山、水等)
  - `DoorClosed` (关闭的门)
  - `Block` (阻挡)
  - `MiddleBlock/LowWall` (低墙)
- **用途**:
  - 调试寻路系统
  - 检查碰撞检测
  - 查看地图可行走区域

### 7. `show_animations` - 动画效果
- **默认值**: `true`
- **功能**: 控制所有动画播放
- **影响范围**:
  - Middle 层动画瓦片
  - Front 层动画瓦片
  - 门开关动画
  - TileAnimationImage (Shanda动画)
- **用途**:
  - 查看静态地图
  - 性能测试
  - 截图对比

## 🔧 使用示例

### 在 GameScene 中使用

```rust
// GameScene 初始化时
impl GameScene {
    pub fn new() -> Self {
        let mut map_renderer = MapRenderer::default();
        
        // 开启调试功能
        map_renderer.show_grid = true;       // 显示网格
        map_renderer.show_borders = true;    // 显示边框
        map_renderer.show_obstacles = true;  // 显示障碍
        
        Self {
            map_renderer,
            // ... 其他字段
        }
    }
}
```

### 键盘切换（建议在 GameScene::key_down_event 中实现）

```rust
fn key_down_event(&mut self, keycode: KeyCode) {
    match keycode {
        KeyCode::KeyG => {
            self.map_renderer.show_grid = !self.map_renderer.show_grid;
            println!("🔍 地图网格: {}", if self.map_renderer.show_grid { "开启" } else { "关闭" });
        },
        KeyCode::KeyB => {
            self.map_renderer.show_borders = !self.map_renderer.show_borders;
            println!("🔍 纹理边框: {}", if self.map_renderer.show_borders { "开启" } else { "关闭" });
        },
        KeyCode::Digit1 => {
            self.map_renderer.show_layer_back = !self.map_renderer.show_layer_back;
            println!("🎨 Back层: {}", if self.map_renderer.show_layer_back { "开启" } else { "关闭" });
        },
        KeyCode::Digit2 => {
            self.map_renderer.show_layer_middle = !self.map_renderer.show_layer_middle;
            println!("🎨 Middle层: {}", if self.map_renderer.show_layer_middle { "开启" } else { "关闭" });
        },
        KeyCode::Digit3 => {
            self.map_renderer.show_layer_front = !self.map_renderer.show_layer_front;
            println!("🎨 Front层: {}", if self.map_renderer.show_layer_front { "开启" } else { "关闭" });
        },
        KeyCode::KeyO => {
            self.map_renderer.show_obstacles = !self.map_renderer.show_obstacles;
            println!("🚧 障碍层: {}", if self.map_renderer.show_obstacles { "开启" } else { "关闭" });
        },
        KeyCode::KeyA => {
            self.map_renderer.show_animations = !self.map_renderer.show_animations;
            println!("🎬 动画效果: {}", if self.map_renderer.show_animations { "开启" } else { "关闭" });
        },
        _ => {}
    }
}
```

## 🎯 实现细节

### 1. 图层绘制控制

在 `draw()` 方法中添加了条件判断:

```rust
// Back层
if self.show_layer_back {
    self.draw_back(ctx, canvas, camera, start_x, end_x, start_y, end_y)?;
}

// Middle层
if self.show_layer_middle && draw_middle {
    self.draw_middle(ctx, canvas, camera, start_x, end_x, start_y, end_y)?;
}

// Front层
if self.show_layer_front && draw_front {
    self.draw_front(ctx, canvas, camera, start_x, end_x, start_y, end_y)?;
}
```

### 2. 动画控制

在 `draw_middle()` 和 `draw_front()` 中:

```rust
// Middle 层动画
} else if self.show_animations {
    // 只有 show_animations 为 true 时才绘制动画瓦片
    // ...
}

// Front 层动画
if has_animation && self.show_animations {
    // 动画帧计算
}

// 门动画
if has_door && self.show_animations {
    // 门动画处理
}
```

### 3. 边框绘制

在 `draw_tile_normal()` 和 `draw_tile_blend()` 中:

```rust
// 绘制纹理边框（如果启用）
if self.show_borders {
    let border_rect = ggez::graphics::Mesh::new_rectangle(
        ctx,
        ggez::graphics::DrawMode::stroke(1.0),
        ggez::graphics::Rect::new(
            screen_x,
            screen_y,
            info.width as f32 * camera.zoom,
            info.height as f32 * camera.zoom,
        ),
        border_color,
    )?;
    canvas.draw(&border_rect, DrawParam::default());
}
```

### 4. 网格和障碍绘制

新增了两个辅助方法:

```rust
fn draw_grid(&self, ctx: &mut Context, canvas: &mut Canvas, camera: &Camera) -> GameResult<()>
fn draw_obstacles(&self, ctx: &mut Context, canvas: &mut Canvas, camera: &Camera) -> GameResult<()>
```

这两个方法从 `map_viewer.rs` 完整移植过来,保持了相同的实现逻辑。

## 📊 性能影响

| 功能 | 性能影响 | 说明 |
|------|---------|------|
| `show_grid` | 低 | 只绘制可见区域的网格线 |
| `show_borders` | 低 | 每个瓦片增加一个边框矩形 |
| `show_layer_*` | 高 | 关闭图层可显著提升性能 |
| `show_obstacles` | 低 | 遍历可见格子,绘制半透明矩形 |
| `show_animations` | 中 | 关闭动画可节省帧计算开销 |

## 🔍 调试建议

### 问题 1: 地图不显示
```rust
// 检查图层开关
map_renderer.show_layer_back = true;
map_renderer.show_layer_middle = true;
map_renderer.show_layer_front = true;
```

### 问题 2: 性能低
```rust
// 关闭不必要的图层
map_renderer.show_layer_middle = false;  // 先关闭 Middle 层
map_renderer.show_animations = false;    // 关闭动画
```

### 问题 3: 查看碰撞问题
```rust
// 开启障碍层可视化
map_renderer.show_obstacles = true;
map_renderer.show_grid = true;  // 配合网格更清晰
```

### 问题 4: 纹理对齐问题
```rust
// 开启边框和网格
map_renderer.show_borders = true;
map_renderer.show_grid = true;
```

## 📚 参考

- **源文件**: `src/scenes/game_scene/map_renderer.rs`
- **参考实现**: `src/bin/map_viewer.rs` (独立地图查看器)
- **相关文档**: 
  - `GameScene绘制流程详细分析.md`
  - `map_viewer渲染架构重构说明.md`

## ✅ 总结

通过这 7 个显示控制开关,你可以:
1. 🔍 **调试渲染** - 网格、边框帮助定位问题
2. 🎨 **分层查看** - 单独查看每一层的内容
3. 🚧 **检查碰撞** - 可视化不可行走区域
4. 🎬 **性能优化** - 关闭图层/动画测试性能
5. 📸 **截图对比** - 静态地图便于对比和记录

所有开关都是 `pub` 的,可以在 GameScene 中直接访问和修改!

---

**最后更新**: 2025-10-15  
**作者**: GitHub Copilot  
**版本**: 1.0
