# 传奇2坐标系统设计文档

## 📐 三大坐标系统

### 1. 地图坐标系 (Map Coordinates)
- **单位**: 格子 (Grid)
- **用途**: 游戏逻辑、寻路、碰撞检测
- **坐标**: `(grid_x, grid_y)` - 整数格子坐标
- **示例**: 玩家站在地图 (100, 50) 格

```rust
pub struct GridPosition {
    pub x: i32,  // 格子X坐标
    pub y: i32,  // 格子Y坐标
}
```

### 2. 世界坐标系 (World Coordinates)
- **单位**: 像素 (Pixel)
- **用途**: 平滑移动、精确定位
- **坐标**: `(world_x, world_y)` - 浮点像素坐标
- **转换**: `world_x = grid_x * CELL_WIDTH` (48px)
- **转换**: `world_y = grid_y * CELL_HEIGHT` (32px)

```rust
pub struct WorldPosition {
    pub x: f32,  // 世界X坐标(像素)
    pub y: f32,  // 世界Y坐标(像素)
}

// 转换公式
pub fn grid_to_world(grid_x: i32, grid_y: i32) -> (f32, f32) {
    (
        grid_x as f32 * CELL_WIDTH,
        grid_y as f32 * CELL_HEIGHT
    )
}

pub fn world_to_grid(world_x: f32, world_y: f32) -> (i32, i32) {
    (
        (world_x / CELL_WIDTH).floor() as i32,
        (world_y / CELL_HEIGHT).floor() as i32
    )
}
```

### 3. 屏幕坐标系 (Screen Coordinates)
- **单位**: 像素 (Pixel)
- **用途**: 最终渲染到屏幕
- **坐标**: `(screen_x, screen_y)` - 屏幕像素坐标
- **原点**: 屏幕左上角 (0, 0)

```rust
pub struct ScreenPosition {
    pub x: f32,  // 屏幕X坐标(像素)
    pub y: f32,  // 屏幕Y坐标(像素)
}
```

---

## 🎮 原版C#客户端坐标系统

### 核心常量 (MapControl.cs)
```csharp
public const int CellWidth = 48;   // 每格宽度 48 像素
public const int CellHeight = 32;  // 每格高度 32 像素 (等距视角)

// 视野中心偏移 (格子数)
public static int OffSetX;  // 1024x768 → 1024/2/48 = 10
public static int OffSetY;  // 1024x768 → 768/2/32 - 1 = 11

// 视野范围 (格子数)
public static int ViewRangeX;  // OffSetX + 6 = 16
public static int ViewRangeY;  // OffSetY + 6 = 17
```

### 关键字段 (MapObject.cs)
```csharp
// 地图坐标
public Point CurrentLocation;  // 当前格子位置 (整数)
public Point Movement;         // 当前渲染位置 (移动中平滑变化)

// 屏幕坐标
public Point DrawLocation;      // 基础屏幕坐标 (格子*像素)
public Point FinalDrawLocation; // 最终屏幕坐标 (加上纹理偏移)
public Point OffSetMove;        // 像素级移动偏移 (0-47, 0-31)

// 显示矩形 (用于鼠标点击检测)
public Rectangle DisplayRectangle;

// Y排序 (用于正确的遮挡关系)
public int DrawY;  // Movement.Y 和 CurrentLocation.Y 的较大值
```

### 坐标转换公式 (PlayerObject.cs:971)

#### 1. 地图坐标 → 屏幕坐标 (DrawLocation)
```csharp
DrawLocation = new Point(
    (Movement.X - User.Movement.X + MapControl.OffSetX) * MapControl.CellWidth,
    (Movement.Y - User.Movement.Y + MapControl.OffSetY) * MapControl.CellHeight
);
```

**公式解析**:
- `Movement.X - User.Movement.X`: 对象相对玩家的偏移 (格子)
- `+ OffSetX`: 加上视野中心偏移 (10格)
- `* CellWidth`: 转换为像素坐标

**示例** (1024x768窗口):
- 玩家位置: `User.Movement = (100, 50)`
- 对象位置: `obj.Movement = (102, 50)` (玩家右侧2格)
- OffSetX = 10, CellWidth = 48
- `DrawLocation.X = (102 - 100 + 10) * 48 = 12 * 48 = 576px`

#### 2. 全局显示偏移
```csharp
DrawLocation.Offset(GlobalDisplayLocationOffset);
```
- 子类可重写此属性调整偏移 (默认 0,0)

#### 3. 非玩家对象偏移修正
```csharp
if (this != User) {
    DrawLocation.Offset(User.OffSetMove);      // 加上玩家平滑移动偏移
    DrawLocation.Offset(-OffSetMove.X, -OffSetMove.Y);  // 减去对象自身偏移
}
```

**OffSetMove**: 像素级移动偏移
- 移动中: (0-47, 0-31) 像素
- 站立时: (0, 0)

#### 4. 纹理偏移 (FinalDrawLocation)
```csharp
FinalDrawLocation = DrawLocation.Add(BodyLibrary.GetOffSet(DrawFrame));
```
- 加上纹理库中定义的偏移量 (OffSetX, OffSetY)
- 不同动画帧有不同偏移

#### 5. 显示矩形 (DisplayRectangle)
```csharp
DisplayRectangle = new Rectangle(DrawLocation, BodyLibrary.GetTrueSize(DrawFrame));
```
- 位置: `DrawLocation` (不是 FinalDrawLocation!)
- 尺寸: 纹理的实际大小 (TrueSize)
- 用于: 鼠标点击检测、名字/血条定位

---

## 🦀 Rust ECS 坐标系统设计

### 组件定义

```rust
/// 地图格子位置组件 (逻辑坐标)
#[derive(Component, Debug, Clone, Copy)]
pub struct GridPosition {
    pub x: i32,
    pub y: i32,
}

/// 世界像素位置组件 (渲染坐标)
#[derive(Component, Debug, Clone, Copy)]
pub struct WorldPosition {
    pub x: f32,
    pub y: f32,
}

/// 移动偏移组件 (像素级平滑移动)
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct PixelOffset {
    pub x: f32,  // 0.0 ~ 48.0
    pub y: f32,  // 0.0 ~ 32.0
}

/// 显示矩形组件 (屏幕空间)
#[derive(Component, Debug, Clone, Copy)]
pub struct DisplayRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// 绘制Y坐标 (用于深度排序)
#[derive(Component, Debug, Clone, Copy)]
pub struct DrawY(pub i32);
```

### 统一坐标转换模块

```rust
// src/ecs/coordinate_system.rs

use hecs::World;

pub const CELL_WIDTH: f32 = 48.0;
pub const CELL_HEIGHT: f32 = 32.0;

/// 视野配置
pub struct ViewportConfig {
    pub screen_width: f32,
    pub screen_height: f32,
    pub offset_x: i32,  // ScreenWidth / 2 / CellWidth
    pub offset_y: i32,  // ScreenHeight / 2 / CellHeight - 1
}

impl ViewportConfig {
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        Self {
            screen_width,
            screen_height,
            offset_x: (screen_width / 2.0 / CELL_WIDTH) as i32,
            offset_y: (screen_height / 2.0 / CELL_HEIGHT) as i32 - 1,
        }
    }
}

/// 坐标转换工具
pub struct CoordinateSystem {
    pub viewport: ViewportConfig,
}

impl CoordinateSystem {
    /// 地图坐标 → 世界坐标
    pub fn grid_to_world(grid_x: i32, grid_y: i32) -> (f32, f32) {
        (
            grid_x as f32 * CELL_WIDTH,
            grid_y as f32 * CELL_HEIGHT
        )
    }
    
    /// 世界坐标 → 地图坐标
    pub fn world_to_grid(world_x: f32, world_y: f32) -> (i32, i32) {
        (
            (world_x / CELL_WIDTH).floor() as i32,
            (world_y / CELL_HEIGHT).floor() as i32
        )
    }
    
    /// 计算屏幕坐标 (对象相对玩家)
    /// 
    /// 对应原版: DrawLocation = (Movement.X - User.Movement.X + OffSetX) * CellWidth
    pub fn to_screen_position(
        &self,
        obj_world: (f32, f32),          // 对象世界坐标
        player_world: (f32, f32),       // 玩家世界坐标
        player_pixel_offset: (f32, f32), // 玩家像素偏移 (OffSetMove)
        obj_pixel_offset: (f32, f32),    // 对象像素偏移
        is_player: bool,
    ) -> (f32, f32) {
        // 转换为格子坐标
        let obj_grid = Self::world_to_grid(obj_world.0, obj_world.1);
        let player_grid = Self::world_to_grid(player_world.0, player_world.1);
        
        // 计算基础屏幕坐标
        let mut screen_x = (obj_grid.0 - player_grid.0 + self.viewport.offset_x) as f32 * CELL_WIDTH;
        let mut screen_y = (obj_grid.1 - player_grid.1 + self.viewport.offset_y) as f32 * CELL_HEIGHT;
        
        // 非玩家对象需要修正偏移
        if !is_player {
            screen_x += player_pixel_offset.0 - obj_pixel_offset.0;
            screen_y += player_pixel_offset.1 - obj_pixel_offset.1;
        }
        
        (screen_x, screen_y)
    }
    
    /// 屏幕坐标 → 地图坐标 (鼠标点击)
    pub fn screen_to_grid(
        &self,
        screen_x: f32,
        screen_y: f32,
        player_grid: (i32, i32),
    ) -> (i32, i32) {
        let grid_x = (screen_x / CELL_WIDTH) as i32 - self.viewport.offset_x + player_grid.0;
        let grid_y = (screen_y / CELL_HEIGHT) as i32 - self.viewport.offset_y + player_grid.1;
        (grid_x, grid_y)
    }
}
```

### 渲染系统集成

```rust
// 在 RenderSystem 中使用统一坐标系统

pub fn render_map_objects(
    world: &World,
    coord_system: &CoordinateSystem,
    canvas: &mut Canvas,
) -> anyhow::Result<()> {
    // 获取玩家世界坐标和偏移
    let (player_world, player_pixel_offset) = get_player_position(world)?;
    
    // 遍历所有可渲染对象
    for (entity, (world_pos, pixel_offset, sprite)) in world.query::<(
        &WorldPosition,
        Option<&PixelOffset>,
        &Sprite,
    )>().iter() {
        let is_player = has_component::<LocalPlayer>(world, entity);
        let obj_offset = pixel_offset.map(|o| (o.x, o.y)).unwrap_or((0.0, 0.0));
        
        // 统一坐标转换
        let (screen_x, screen_y) = coord_system.to_screen_position(
            (world_pos.x, world_pos.y),
            player_world,
            player_pixel_offset,
            obj_offset,
            is_player,
        );
        
        // 绘制纹理
        canvas.draw_texture(sprite.texture, screen_x, screen_y)?;
    }
    
    Ok(())
}
```

---

## 📊 坐标转换流程图

```
服务器网络包 (UserLocation)
    ↓
GridPosition (286, 617)  [地图格子坐标]
    ↓ grid_to_world()
WorldPosition (13728.0, 19744.0)  [世界像素坐标]
    ↓ + PixelOffset (移动中的亚像素偏移)
RenderPosition (13728.0 + 24.0, 19744.0 + 16.0)
    ↓ to_screen_position() [相对玩家 + 视野偏移]
ScreenPosition (480.0, 352.0)  [屏幕像素坐标]
    ↓ + TextureOffset (纹理库中的偏移)
FinalScreenPosition (480.0 + offsetX, 352.0 + offsetY)
    ↓
canvas.draw() [最终渲染]
```

---

## ✅ 实现要点

1. **坐标系统分离**: 地图/世界/屏幕三个坐标系独立存储，转换明确
2. **统一转换接口**: 所有坐标转换通过 `CoordinateSystem` 模块
3. **组件化设计**: Position/Offset/DisplayRect 独立组件，便于查询
4. **原版对齐**: 完全复现原版C#的坐标计算逻辑
5. **性能优化**: 避免重复转换，缓存中间结果

---

## 🐛 常见问题

### Q1: 为什么需要 OffSetMove (像素偏移)?
A: 玩家移动时需要平滑插值，`GridPosition` 是离散的格子坐标，`PixelOffset` 提供 0-48 像素的亚格子精度。

### Q2: DrawLocation 和 FinalDrawLocation 的区别?
A: 
- `DrawLocation`: 对象脚底中心点的屏幕坐标
- `FinalDrawLocation`: 加上纹理偏移后的最终绘制坐标 (纹理左上角)

### Q3: 为什么非玩家对象要减去 OffSetMove?
A: 玩家移动时，屏幕是固定的，所以其他对象相对移动。需要补偿玩家的像素偏移。

### Q4: DisplayRectangle 为什么用 DrawLocation 而非 FinalDrawLocation?
A: 鼠标点击检测基于对象的逻辑位置 (脚底)，而非纹理渲染位置。

---

## 📝 示例代码

详见:
- `src/ecs/coordinate_system.rs` - 坐标转换模块
- `src/ecs/systems/render.rs` - 渲染系统
- `src/ecs/components.rs` - 坐标相关组件

---

**作者**: GitHub Copilot  
**日期**: 2025-10-25  
**版本**: 1.0
