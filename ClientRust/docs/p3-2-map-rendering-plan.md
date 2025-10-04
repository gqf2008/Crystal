# P3-2: 地图渲染系统 - 任务规划

**开始日期**: 2025-10-04  
**状态**: 🔄 规划中  
**优先级**: 🔥 高

---

## 📋 任务目标

实现基于 wgpu 的 2D 地图渲染系统，复用 P3-1 的 SpriteRenderer 基础设施。

### 核心功能

1. **地图数据加载** (.map 文件格式解析)
2. **地图库加载** (Map/*.lib 瓦片资源)
3. **Tile 渲染** (地面、墙壁、对象)
4. **相机系统** (跟随玩家移动)
5. **视野裁剪** (只渲染可见区域)
6. **层级渲染** (地面 → 对象 → 角色)

---

## 🔍 C# 原版分析

### MapControl 核心常量

```csharp
public const int CellWidth = 48;   // 每个单元格宽度 (像素)
public const int CellHeight = 32;  // 每个单元格高度 (像素)

public static int OffSetX;  // 视口偏移X (屏幕中心)
public static int OffSetY;  // 视口偏移Y (屏幕中心)

public static int ViewRangeX;  // 视野范围X
public static int ViewRangeY;  // 视野范围Y
```

### 地图资源结构

根据 `Client/MirGraphics/MLibrary.cs`:

```
WemadeMir2 (索引 0-99):
  - Tiles (0)
  - Smtiles (1)
  - Objects (2-90)

ShandaMir2 (索引 100-199):
  - Tiles (100-109)
  - SmTiles (110-119)
  - Objects (120-150)
  - AniTiles (190)

WemadeMir3 (索引 200-299):
  - Tilesc (200, 215, 230, 245, 260)
  - Tiles30c (201, 216, 231, 246, 261)
  - Tiles5c (202, 217, 232, 247, 262)
  - Smtilesc (203, 218, 233, 248, 263)
  - ...各种地形变体...

ShandaMir3 (索引 300-399):
  - 类似结构
```

### 地图文件格式

根据 `Server/MirEnvir/Map.cs`:

支持多个版本:
- v0, v1, v2, v3, v4, v5, v6, v7 (旧格式)
- v100 (新格式)

每个版本有不同的加载函数 `LoadMapCellsv*()`.

### Cell 数据结构

```csharp
public CellInfo[,] M2CellInfo;  // 二维数组存储所有单元格

// 每个 Cell 包含:
// - BackImage (地面图片索引)
// - MiddleImage (中层图片索引)
// - FrontImage (前景图片索引)
// - FrontAnimationFrame (动画帧)
// - Flags (单元格属性: 可行走, 飞行等)
```

---

## 🎯 Rust 实现计划

### Phase 1: 地图数据解析 (预计 200-250 行)

**目标**: 读取 .map 文件并解析为内存结构

**文件**: `src/map/map_loader.rs`

**数据结构**:
```rust
pub struct MapData {
    pub width: i32,
    pub height: i32,
    pub title: String,
    pub cells: Vec<Vec<Cell>>,  // 2D 网格
}

pub struct Cell {
    pub back_image: Option<(u16, u16)>,   // (library_id, image_id)
    pub middle_image: Option<(u16, u16)>,
    pub front_image: Option<(u16, u16)>,
    pub front_animation_frame: u8,
    pub flags: CellFlags,  // 位标志: 可行走, 可飞行等
}

bitflags! {
    pub struct CellFlags: u8 {
        const WALKABLE = 0b00000001;
        const FLYABLE  = 0b00000010;
        const SAFE     = 0b00000100;
        const DOOR     = 0b00001000;
    }
}
```

**API**:
```rust
impl MapData {
    pub fn load(path: impl AsRef<Path>) -> io::Result<Self>;
    pub fn get_cell(&self, x: i32, y: i32) -> Option<&Cell>;
    pub fn get_cell_mut(&mut self, x: i32, y: i32) -> Option<&mut Cell>;
}
```

---

### Phase 2: 地图渲染器 (预计 250-300 行)

**目标**: 使用 SpriteRenderer 渲染地图瓦片

**文件**: `src/map/map_renderer.rs`

**数据结构**:
```rust
pub struct MapRenderer {
    map_data: MapData,
    tile_libraries: HashMap<u16, MLibrary>,  // 地图库缓存
    sprite_renderer: SpriteRenderer,
    camera: Camera,
}

pub struct Camera {
    pub position: Point,      // 玩家位置 (单元格坐标)
    pub offset: Point,        // 屏幕中心偏移
    pub viewport_size: Size,  // 视口尺寸 (像素)
}
```

**渲染流程**:
```rust
impl MapRenderer {
    pub fn render(&mut self, render_pass: &mut wgpu::RenderPass) {
        // 1. 计算可见区域
        let visible_rect = self.calculate_visible_rect();
        
        // 2. 渲染地面层 (BackImage)
        self.render_layer(visible_rect, LayerType::Back, render_pass);
        
        // 3. 渲染中间层 (MiddleImage)
        self.render_layer(visible_rect, LayerType::Middle, render_pass);
        
        // 4. 渲染前景层 (FrontImage)
        self.render_layer(visible_rect, LayerType::Front, render_pass);
    }
    
    fn calculate_visible_rect(&self) -> Rect {
        // 基于相机位置和视口计算可见矩形
    }
    
    fn render_layer(&mut self, rect: Rect, layer: LayerType, render_pass: &mut RenderPass) {
        for y in rect.min_y..rect.max_y {
            for x in rect.min_x..rect.max_x {
                if let Some(cell) = self.map_data.get_cell(x, y) {
                    let image_info = self.get_layer_image(cell, layer);
                    if let Some((lib_id, img_id)) = image_info {
                        self.render_tile(x, y, lib_id, img_id, render_pass);
                    }
                }
            }
        }
    }
}
```

---

### Phase 3: 相机系统 (预计 100-150 行)

**目标**: 实现相机跟随和视野裁剪

**文件**: `src/map/camera.rs`

**功能**:
```rust
impl Camera {
    pub fn new(viewport_size: Size) -> Self;
    
    // 跟随玩家
    pub fn follow_player(&mut self, player_pos: Point);
    
    // 世界坐标 → 屏幕坐标
    pub fn world_to_screen(&self, world_pos: Point) -> Point;
    
    // 屏幕坐标 → 世界坐标
    pub fn screen_to_world(&self, screen_pos: Point) -> Point;
    
    // 计算可见单元格范围
    pub fn get_visible_cells(&self) -> Rect;
}
```

**坐标转换**:
```rust
// 世界坐标 (单元格) → 屏幕坐标 (像素)
screen_x = (world_x - camera_x + offset_x) * CELL_WIDTH
screen_y = (world_y - camera_y + offset_y) * CELL_HEIGHT
```

---

### Phase 4: 集成到 GameScene (预计 100 行)

**目标**: 在 GameScene 中显示地图

**修改**: `src/scenes/game_scene.rs`

```rust
pub struct GameScene {
    map_renderer: Option<MapRenderer>,
    // ... 其他字段
}

impl Scene for GameScene {
    fn initialize(&mut self) {
        // 加载地图
        let map_data = MapData::load("Data/Maps/0.map")?;
        let mut map_renderer = MapRenderer::new(map_data, sprite_renderer);
        map_renderer.load_tile_libraries()?;
        
        self.map_renderer = Some(map_renderer);
    }
    
    fn update(&mut self, delta_time: f32) {
        if let Some(ref mut renderer) = self.map_renderer {
            // 更新相机位置 (跟随玩家)
            renderer.camera.follow_player(player.position);
        }
    }
    
    fn draw(&self) {
        if let Some(ref renderer) = self.map_renderer {
            renderer.render();
        }
    }
}
```

---

## 📦 依赖和资源

### 新增依赖

```toml
# Cargo.toml
[dependencies]
bitflags = "2.4"  # 用于 CellFlags
```

### 资源文件需求

1. **地图文件**: `Data/Maps/*.map`
2. **地图库**: `Data/Map/WemadeMir2/*.lib`
3. **MiniMap**: `Data/MiniMap.lib` (小地图)

---

## 🎨 渲染优化

### 批量渲染

```rust
// 收集所有可见瓦片
let mut tiles_to_render: Vec<TileInstance> = Vec::new();

for y in visible_rect.y_range() {
    for x in visible_rect.x_range() {
        if let Some(cell) = map.get_cell(x, y) {
            if let Some((lib, img)) = cell.back_image {
                tiles_to_render.push(TileInstance {
                    position: [x as f32, y as f32],
                    texture_index: img,
                    // ...
                });
            }
        }
    }
}

// 一次性渲染所有瓦片 (实例化渲染)
sprite_renderer.render_batch(&tiles_to_render);
```

### 纹理图集

将常用瓦片打包到单个纹理中，减少纹理切换。

---

## 🧪 测试计划

### 单元测试

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn test_map_loading() {
        let map = MapData::load("test_maps/0.map").unwrap();
        assert!(map.width > 0);
        assert!(map.height > 0);
    }
    
    #[test]
    fn test_camera_world_to_screen() {
        let camera = Camera::new(Size::new(800, 600));
        let screen_pos = camera.world_to_screen(Point::new(10, 10));
        // 验证转换正确
    }
    
    #[test]
    fn test_visible_rect_calculation() {
        let camera = Camera::new(Size::new(800, 600));
        let rect = camera.get_visible_cells();
        assert!(rect.width() > 0);
        assert!(rect.height() > 0);
    }
}
```

### 集成测试

1. 加载测试地图 (0.map)
2. 渲染测试 (截图对比)
3. 性能测试 (FPS, GPU 占用)

---

## 📊 预计工作量

| 阶段 | 文件 | 行数 | 优先级 |
|------|------|------|--------|
| Phase 1: 地图数据解析 | map_loader.rs | 200-250 | 🔥 高 |
| Phase 2: 地图渲染器 | map_renderer.rs | 250-300 | 🔥 高 |
| Phase 3: 相机系统 | camera.rs | 100-150 | 🔥 高 |
| Phase 4: GameScene 集成 | game_scene.rs | 100 | 中 |
| 测试 | tests | 100-150 | 中 |
| 文档 | docs | 500+ | 低 |
| **总计** | | **1250-1450** | |

---

## 🎯 里程碑

### Milestone 1: 地图数据加载 ✅
- [x] 定义数据结构
- [ ] 实现 .map 解析
- [ ] 支持多个地图版本
- [ ] 单元测试

### Milestone 2: 基础渲染 ⏳
- [ ] 渲染单层瓦片 (BackImage)
- [ ] 集成 SpriteRenderer
- [ ] 静态地图显示

### Milestone 3: 完整渲染 ⏳
- [ ] 三层渲染 (Back/Middle/Front)
- [ ] 动画瓦片支持
- [ ] 相机跟随

### Milestone 4: 优化和集成 ⏳
- [ ] 视野裁剪
- [ ] 批量渲染
- [ ] GameScene 集成

---

## 🔄 与 P3-1 的关系

**复用**:
- ✅ SpriteRenderer (wgpu 渲染管线)
- ✅ WGSL 着色器
- ✅ SpriteVertex / SpriteInstance
- ✅ MLibrary (纹理加载)

**扩展**:
- 🆕 MapData (地图数据结构)
- 🆕 MapRenderer (地图特化渲染器)
- 🆕 Camera (相机系统)
- 🆕 Tile batching (批量优化)

---

## 📝 参考资料

### C# 代码参考

1. **Client/MirScenes/GameScene.cs** (MapControl 类)
   - 地图加载: `LoadMap()`
   - 单元格访问: `M2CellInfo[,]`
   - 坐标常量: `CellWidth`, `CellHeight`

2. **Client/MirGraphics/MapReader.cs**
   - 地图文件解析
   - Cell 数据结构

3. **Server/MirEnvir/Map.cs**
   - LoadMapCellsv* 函数
   - 地图版本检测

---

## 🎉 预期成果

完成 P3-2 后，将实现:
- ✅ 加载和显示 MIR2 地图
- ✅ 相机跟随玩家移动
- ✅ 高性能渲染 (60 FPS+)
- ✅ 视野裁剪优化
- ✅ 与 P3-1 角色渲染集成

---

**下一步**: 开始实现 Phase 1 - 地图数据解析器
