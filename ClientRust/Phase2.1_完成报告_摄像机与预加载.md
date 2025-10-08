# Phase 2.1 完成报告 - 摄像机系统与瓦片预加载

## 📋 概述

**时间**: 2025-01-08  
**状态**: ✅ Phase 2.1 完成  
**目标**: 实现摄像机系统和瓦片纹理预加载机制

---

## ✅ 已完成工作

### 1. 摄像机系统 (`camera_x`, `camera_y`)

**添加字段**:
```rust
pub struct GameScene {
    // ...
    pub camera_x: f32,          // 摄像机X坐标 (像素)
    pub camera_y: f32,          // 摄像机Y坐标 (像素)
    pub viewport_width: f32,    // 视口宽度 (800.0)
    pub viewport_height: f32,   // 视口高度 (600.0)
    // ...
}
```

**摄像机更新逻辑** (`update_camera()`):
```rust
pub fn update_camera(&mut self) {
    if let Some(user) = &self.user {
        // 计算玩家位置 (地图坐标 → 屏幕像素)
        let player_x = user.player.map_object.current_location.x as f32 * 48.0;
        let player_y = user.player.map_object.current_location.y as f32 * 32.0;
        
        // 居中玩家
        self.camera_x = player_x - self.viewport_width / 2.0;
        self.camera_y = player_y - self.viewport_height / 2.0;
        
        // 限制在地图边界内
        if let Some(map) = &self.map_control {
            let max_camera_x = (map.width as f32 * 48.0) - self.viewport_width;
            let max_camera_y = (map.height as f32 * 32.0) - self.viewport_height;
            
            self.camera_x = self.camera_x.max(0.0).min(max_camera_x);
            self.camera_y = self.camera_y.max(0.0).min(max_camera_y);
        }
    }
}
```

**特性**:
- ✅ 自动跟随玩家
- ✅ 边界检测 (不超出地图范围)
- ✅ 居中显示
- ✅ 在 `update()` 中每帧更新

### 2. 瓦片预加载系统

**RefCell 内部可变性**:
```rust
pub tile_texture_manager: std::cell::RefCell<TileTextureManager>,
```

使用 `RefCell` 包装纹理管理器,允许在 `&self` 方法中进行可变借用。

**预加载方法** (`preload_visible_tiles()`):
```rust
pub fn preload_visible_tiles(&self, ggez_manager: &mut GgezManager) {
    // 1. 计算可见瓦片范围 (带缓冲区)
    const TILE_WIDTH: f32 = 48.0;
    const TILE_HEIGHT: f32 = 32.0;
    const BUFFER_TILES: i32 = 2;  // 周围2格缓冲
    
    let start_x = ((self.camera_x / TILE_WIDTH) as i32 - BUFFER_TILES).max(0);
    let start_y = ((self.camera_y / TILE_HEIGHT) as i32 - BUFFER_TILES).max(0);
    let end_x = (((self.camera_x + self.viewport_width) / TILE_WIDTH) as i32 + BUFFER_TILES)
        .min(map.width as i32);
    let end_y = (((self.camera_y + self.viewport_height) / TILE_HEIGHT) as i32 + BUFFER_TILES)
        .min(map.height as i32);
    
    // 2. 遍历并加载瓦片纹理
    for y in start_y..end_y {
        for x in start_x..end_x {
            if let Some(cell) = map.get_cell(x, y) {
                if cell.frame_index > 0 {
                    tile_manager.get_tile_texture(
                        cell.file_index,
                        cell.frame_index,
                        ggez_manager
                    );
                }
            }
        }
    }
}
```

**特性**:
- ✅ 只加载可见区域的瓦片
- ✅ +2格缓冲区(平滑滚动)
- ✅ 自动跳过无效瓦片
- ✅ 统计加载成功/失败数量

### 3. 地图绘制框架

**draw_map() 方法**:
```rust
fn draw_map(&self, _ctx: &mut Context, _canvas: &mut Canvas, map: &MapControl) {
    let tile_manager = self.tile_texture_manager.borrow();
    
    // 计算可见范围
    let start_x = (self.camera_x / TILE_WIDTH) as i32;
    let start_y = (self.camera_y / TILE_HEIGHT) as i32;
    let end_x = ((self.camera_x + self.viewport_width) / TILE_WIDTH) as i32 + 1;
    let end_y = ((self.camera_y + self.viewport_height) / TILE_HEIGHT) as i32 + 1;
    
    // 绘制瓦片
    for y in start_y..end_y {
        for x in start_x..end_x {
            if let Some(cell) = map.get_cell(x, y) {
                if let Some(_texture) = tile_manager.get_texture_from_cache(...) {
                    // TODO: 实际绘制纹理
                    // canvas.draw(&texture, DrawParam::default().dest([screen_x, screen_y]));
                }
            }
        }
    }
}
```

**当前状态**:
- ✅ 框架已搭建
- ✅ 可见性剔除
- ✅ 纹理缓存访问
- ⏳ 实际绘制 (待GPU纹理上传完成)

### 4. 调试信息显示

**在 draw() 中显示**:
```rust
let map_info_text = format!(
    "🗺️  {} ({}x{}) | Camera: ({:.0}, {:.0}) | Cache: {:.1}% ({}/{})", 
    map.title, map.width, map.height, 
    self.camera_x, self.camera_y,
    hit_rate, hits, misses
);
```

显示内容:
- 地图名称和尺寸
- 摄像机坐标
- 纹理缓存命中率

### 5. TileTextureManager 增强

**新增方法**:
```rust
pub fn get_texture_from_cache(&self, file_index: i32, tile_index: u16) 
    -> Option<&TileTexture>
```

用于只读访问缓存(不触发加载),适合在 `draw()` 中使用。

---

## 📂 修改文件

| 文件 | 修改 | 说明 |
|------|------|------|
| `game_scene.rs` | +140行 | 摄像机字段、预加载方法、地图绘制框架 |
| `tile_texture_manager.rs` | +8行 | `get_texture_from_cache()` 方法 |

---

## 🎯 技术亮点

### 1. RefCell 内部可变性模式

**问题**: Scene trait 的 `draw()` 方法是 `&self` 不可变借用,无法修改纹理缓存。

**解决方案**:
```rust
pub tile_texture_manager: RefCell<TileTextureManager>
```

- 在 `&self` 方法中通过 `borrow_mut()` 获取可变引用
- 安全的运行时借用检查
- 避免修改 Scene trait 签名

### 2. 可见性剔除 (Frustum Culling)

**优化**: 只渲染屏幕可见的瓦片

**算法**:
```
visible_x_range = [camera_x / tile_width, (camera_x + viewport_width) / tile_width]
visible_y_range = [camera_y / tile_height, (camera_y + viewport_height) / tile_height]
```

**效果**:
- 地图: 100x100 = 10,000 瓦片
- 可见: ~20x15 = 300 瓦片 (97% 剔除率)

### 3. 缓冲区预加载

**策略**: 在可见区域周围加载 +2 格瓦片

**好处**:
- 平滑滚动无卡顿
- 减少加载等待
- 提前准备邻近瓦片

---

## ⚠️ 架构限制与解决方案

### 问题: draw() 中无法预加载

**根本原因**:
- `Scene::draw(&self)` 是不可变借用
- `preload_visible_tiles()` 需要 `&mut GgezManager`
- `GgezManager` 传入时是 `&GgezManager` 不可变引用

**当前方案**: 
- ❌ 去掉 draw() 中的预加载调用
- ✅ 提供 `pub fn preload_visible_tiles()` 供外部调用

**未来解决方案 (3选1)**:

#### 方案 A: 在事件处理时预加载 ⭐ 推荐
```rust
impl Scene for GameScene {
    fn process_event(&mut self, event: &GameEvent) {
        match event {
            GameEvent::MapInformation { ... } => {
                // 加载地图后立即预加载瓦片
                self.preload_visible_tiles(&mut ggez_manager);
            }
            GameEvent::UserLocation { ... } => {
                // 玩家移动后预加载新区域
                self.preload_visible_tiles(&mut ggez_manager);
            }
            _ => {}
        }
    }
}
```

优点:
- 不修改 trait 签名
- 在合适的时机预加载
- 最小化架构变动

缺点:
- 需要传递 `ggez_manager` 到 `process_event()`

#### 方案 B: 修改 Scene trait
```rust
pub trait Scene {
    fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas, 
            ggez_manager: &mut GgezManager);
}
```

优点:
- 彻底解决问题
- draw() 中可以自由加载资源

缺点:
- 需要修改所有 Scene 实现
- 架构变动较大

#### 方案 C: 后台加载线程
```rust
// 启动后台加载线程
thread::spawn(move || {
    loop {
        // 根据摄像机位置预加载瓦片
        preload_tiles_around_camera();
        thread::sleep(Duration::from_millis(16));
    }
});
```

优点:
- 不阻塞主线程
- 自动化

缺点:
- 复杂度高
- 需要线程同步
- 可能过度工程化

---

## 🎮 当前功能

### 可工作的部分

1. **摄像机跟随** ✅
   - 自动居中玩家
   - 边界限制

2. **纹理缓存** ✅
   - `get_texture_from_cache()` 只读访问
   - 缓存统计显示

3. **调试信息** ✅
   - 地图信息
   - 摄像机坐标
   - 缓存命中率

### 待完成的部分

1. **瓦片预加载触发** ⏳
   - 需要在 `process_event()` 中调用
   - 需要传递 `ggez_manager`

2. **GPU 纹理上传** ⏳
   - `TileTextureManager` 已加载像素数据
   - 需要调用 `ggez_manager.create_texture_from_rgba()`

3. **实际绘制** ⏳
   - `draw_map()` 框架已搭建
   - 需要 `canvas.draw()` 调用

---

## 📊 性能指标

### 预期性能

**地图大小**: 100x100 = 10,000 瓦片  
**可见瓦片**: ~300 瓦片 (800x600视口)  
**缓冲区**: +4格 = ~400 瓦片预加载  
**剔除率**: 96%

**缓存效率**:
- 首次进入: 0% 命中率 (加载 400 瓦片)
- 静止不动: 100% 命中率
- 平滑移动: 95%+ 命中率 (只加载新出现的瓦片)

### 内存使用

**单个瓦片**: ~48x32x4 = 6 KB (RGBA)  
**400 瓦片缓存**: 2.4 MB  
**完整地图**: 10,000 × 6 KB = 60 MB (不建议全部缓存)

---

## 🧪 测试步骤

### 1. 编译运行
```powershell
cd ClientRust
cargo run --bin mir2_client
```

### 2. 进入游戏
- 登录账号
- 选择角色
- 进入 GameScene

### 3. 观察调试信息
应该看到:
```
🗺️  Map_Name (100x100) | Camera: (0, 0) | Cache: 0.0% (0/0)
```

### 4. 验证摄像机
- 玩家移动时 Camera 坐标应该跟随变化
- 摄像机不应超出地图边界

### 5. 缓存统计 (需预加载触发后)
```
Cache: 95.5% (382/400)
```

---

## 🔧 下一步: Phase 2.2

### 目标: 完成瓦片渲染管线

**任务清单**:

1. **触发预加载** (高优先级)
   ```rust
   // 选择方案A: 在 process_event 中预加载
   impl Scene for GameScene {
       fn process_event(&mut self, event: &GameEvent, ggez_manager: &mut GgezManager) {
           match event {
               GameEvent::MapInformation { ... } => {
                   self.preload_visible_tiles(ggez_manager);
               }
               _ => {}
           }
       }
   }
   ```
   
   **需要修改**:
   - `Scene` trait 添加 `ggez_manager` 参数到 `process_event()`
   - 或在 `SceneManager` 层面处理

2. **GPU 纹理上传**
   修改 `TileTextureManager::get_tile_texture()`:
   ```rust
   // 加载像素数据后上传到GPU
   let (info, pixels) = library_lock.load_rgba_data(tile_index as usize)?;
   let texture_name = format!("Tile_{}_{}", file_index, tile_index);
   
   // 上传到GPU
   ggez_manager.create_texture_from_rgba(
       ctx,
       info.width as u16,
       info.height as u16,
       &pixels,
       texture_name.clone()
   )?;
   ```

3. **实际绘制瓦片**
   完成 `draw_map()` 中的 TODO:
   ```rust
   if let Some(texture) = tile_manager.get_texture_from_cache(...) {
       let texture_handle = ggez_manager.get_texture(&texture.texture_name)?;
       canvas.draw(
           texture_handle,
           DrawParam::default()
               .dest([screen_x, screen_y])
       );
   }
   ```

4. **多层渲染**
   当 `CellInfo` 扩展为支持 back/middle/front 层时:
   ```rust
   // Layer 1: Back (地面)
   draw_cell_layer(cell.back_file, cell.back_image);
   // Layer 2: Middle (物体)
   draw_cell_layer(cell.middle_file, cell.middle_image);
   // Layer 3: Front (遮挡)
   draw_cell_layer(cell.front_file, cell.front_image);
   ```

---

## 📝 代码示例

### 使用摄像机系统

```rust
// 在 update() 中自动调用
fn update(&mut self, delta_time: f32) {
    self.update_objects(delta_time);
    self.update_camera();  // ← 自动跟随玩家
}

// 手动调整摄像机 (例如小地图点击)
game_scene.camera_x = target_x;
game_scene.camera_y = target_y;
```

### 预加载瓦片 (待集成)

```rust
// 方案A: 在事件处理中
impl Scene for GameScene {
    fn process_event(&mut self, event: &GameEvent, ggez_manager: &mut GgezManager) {
        match event {
            GameEvent::MapInformation { file_name, .. } => {
                // 加载地图
                self.map_control = Some(map_loader::load_map_by_name(file_name)?);
                
                // 立即预加载可见瓦片
                self.preload_visible_tiles(ggez_manager);
            }
            GameEvent::UserLocation { location } => {
                // 玩家移动后更新摄像机
                self.update_camera();
                
                // 预加载新区域瓦片
                self.preload_visible_tiles(ggez_manager);
            }
            _ => {}
        }
    }
}
```

---

## 🎉 总结

Phase 2.1 完成了地图渲染的基础架构:

- ✅ **摄像机系统**: 自动跟随玩家,边界限制
- ✅ **预加载框架**: RefCell 模式,可见性剔除,缓冲区
- ✅ **绘制框架**: draw_map() 方法,纹理缓存访问
- ✅ **调试信息**: 实时显示地图/摄像机/缓存状态

下一步 (Phase 2.2) 将完成渲染管线:
1. 集成预加载触发
2. GPU 纹理上传
3. 实际瓦片绘制

预计工作量: 3-4 小时

---

**状态**: 🟢 Ready for Phase 2.2 - Complete Rendering Pipeline  
**下一步**: 集成预加载触发机制  
**阻塞问题**: 需要在 `process_event()` 中传递 `ggez_manager`

