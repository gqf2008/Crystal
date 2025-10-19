# Bevy地图查看器 v2.4 关键修复

## 问题分析

### 问题1: 地图原点在屏幕中间 ❌
**现象**: 地图(0,0)点显示在屏幕中央，而不是地图中心

**根本原因**:
```rust
// ❌ 错误的初始化顺序
commands.spawn((
    Camera2d,
    MapCamera::default(),  // target = Vec2::ZERO (0, 0)
    Name::new("MapCamera"),
));

// 然后才加载地图
*map_data = MapData::from_reader(reader, "Map/0.map".to_string());
```

**问题**: 
- 相机在地图加载**之前**创建
- `MapCamera::default()` 的 `target` 是 `Vec2::ZERO`
- 相机Transform也是默认的(0, 0, 0)
- 结果：相机停在地图的左上角(0, 0)，而不是地图中心

---

### 问题2: 只绘制了一个方向的纹理 ❌
**现象**: 只能看到一小块地图瓦片

**根本原因**:
```rust
#[derive(Resource, Default)]
struct VisibleArea {
    start_x: i32,      // Default = 0
    end_x: i32,        // Default = 0
    start_y: i32,      // Default = 0
    end_y: i32,        // Default = 0
    zoom: f32,         // Default = 0.0
}

fn render_static_tiles_system(...) {
    // 检测可见区域是否变化
    let area_changed = visible_area.start_x != start_x
        || visible_area.end_x != end_x
        || visible_area.start_y != start_y
        || visible_area.end_y != end_y
        || (visible_area.zoom - camera.zoom).abs() > 0.001;

    if !area_changed {
        return;  // ⚠️ 第一帧时，如果计算出的区域恰好是(0,0,0,0,1.0)，则跳过渲染！
    }
}
```

**问题**:
- 如果相机在(0,0)，可见区域计算结果可能恰好是(0, 0, 0, 0)
- `VisibleArea`的默认值也是(0, 0, 0, 0, 0.0)
- `area_changed = false` → 跳过渲染
- 只有在相机移动后才会触发第一次渲染

---

## 修复方案

### 修复1: 正确的初始化顺序 ✅

```rust
fn setup_system(
    mut commands: Commands,
    mut map_data: ResMut<MapData>,
) {
    // 1️⃣ 先初始化资源
    let mlibrary = MLibraryAssets::new(PathBuf::from("Data"));
    commands.insert_resource(mlibrary);

    // 2️⃣ 加载地图数据
    if let Ok(reader) = MapReader::new("Map/0.map") {
        *map_data = MapData::from_reader(reader, "Map/0.map".to_string());
        info!("✅ 自动加载默认地图: Map/0.map ({}x{})", map_data.width, map_data.height);
    }

    // 3️⃣ 计算地图中心
    let map_center_x = (map_data.width / 2) as f32 * CELL_WIDTH as f32;
    let map_center_y = -((map_data.height / 2) as f32 * CELL_HEIGHT as f32);  // Y轴翻转

    // 4️⃣ 创建相机，设置到地图中心
    let mut camera = MapCamera::default();
    camera.target = Vec2::new(map_center_x, map_center_y);
    
    commands.spawn((
        Camera2d,
        Transform::from_xyz(map_center_x, map_center_y, 0.0),  // 🔧 初始Transform也设置到中心
        camera,
        Name::new("MapCamera"),
    ));
    
    info!("📍 相机初始位置: 地图中心 ({:.1}, {:.1})", map_center_x, map_center_y);
}
```

**关键点**:
- ✅ 先加载地图 → 后创建相机
- ✅ 相机Transform和target都设置到地图中心
- ✅ 确保第一帧就能看到地图中心区域

---

### 修复2: 强制第一帧渲染 ✅

```rust
#[derive(Resource)]
struct VisibleArea {
    start_x: i32,
    end_x: i32,
    start_y: i32,
    end_y: i32,
    zoom: f32,
}

impl Default for VisibleArea {
    fn default() -> Self {
        Self {
            start_x: -999999,  // 🔧 使用极端值
            end_x: -999999,
            start_y: -999999,
            end_y: -999999,
            zoom: -1.0,
        }
    }
}
```

**原理**:
- 使用不可能出现的极端值作为初始值
- 第一帧计算出的可见区域**必定**与(-999999, -999999, -999999, -999999, -1.0)不同
- `area_changed = true` → 触发渲染
- 确保地图在第一帧就被正确绘制

---

## 技术细节

### Bevy坐标系统

```
Bevy 2D坐标系（Y轴向上）:
    Y+
    ↑
    |
    |_____ X+
    →

地图坐标系（Y轴向下）:
    _____ X+
    →    |
         |
         ↓ Y+
```

**转换公式**:
```rust
// 地图格子 → Bevy世界坐标
fn map_to_world(grid_x: i32, grid_y: i32) -> Vec2 {
    Vec2::new(
        (grid_x * CELL_WIDTH) as f32,
        -((grid_y * CELL_HEIGHT) as f32),  // Y轴翻转
    )
}

// 地图中心坐标
let map_center_x = (width / 2) * CELL_WIDTH;       // 正常
let map_center_y = -((height / 2) * CELL_HEIGHT);  // 翻转
```

### 相机Transform的作用

```rust
// Bevy的相机系统
Transform {
    translation: Vec3(x, y, z),  // 相机在世界中的位置
    scale: Vec3(s, s, s),        // 缩放（s越大，看到的越小）
    rotation: Quat,              // 旋转（2D通常不用）
}

// 所有实体的Transform会被相机Transform转换
Entity_Screen_Pos = (Entity_World_Pos - Camera_Pos) * Camera_Scale
```

**关键**: 必须同时设置:
1. `MapCamera.target` - 逻辑状态
2. `Transform.translation` - 实际渲染位置

---

## 修改总结

### 变更文件
`src/bin/map_viewer_bevy.rs`

### 修改内容

**1. setup_system函数** (第340-369行)
- ✅ 调整初始化顺序：资源 → 地图 → 相机
- ✅ 计算地图中心坐标
- ✅ 相机Transform初始位置设置到地图中心
- ✅ 添加初始位置日志

**2. VisibleArea结构体** (第246-262行)
- ✅ 移除`#[derive(Default)]`
- ✅ 手动实现`Default` trait
- ✅ 使用极端值(-999999)作为初始值
- ✅ 确保第一帧必定触发渲染

---

## 测试验证

### ✅ 预期结果
1. 程序启动后自动加载Map/0.map
2. 相机位于地图中心
3. 可以看到地图中心区域的瓦片
4. 所有方向的瓦片都正确渲染
5. 鼠标中键拖拽平滑移动

### 🔍 调试信息
查看控制台输出：
```
📚 正在初始化地图库...
✅ 地图库初始化完成
✅ 自动加载默认地图: Map/0.map (XXXxYYY)
📍 相机初始位置: 地图中心 (xxxx.x, yyyy.y)
```

---

## 对比

### ggez版本 vs Bevy版本

| 特性 | ggez | Bevy v2.3 | Bevy v2.4 |
|------|------|----------|----------|
| 初始位置 | 地图中心 ✅ | 地图原点 ❌ | 地图中心 ✅ |
| 第一帧渲染 | 立即 ✅ | 需要移动 ❌ | 立即 ✅ |
| 坐标系统 | Y向下 | Y向上（已转换）✅ | Y向上（已转换）✅ |
| 相机拖拽 | ✅ | ✅ | ✅ |
| CellInfo面板 | ✅ | ✅ | ✅ |

---

## 版本历史

| 版本 | 修复内容 | 状态 |
|------|---------|------|
| v2.0 | 动画系统 + 门系统 | ✅ |
| v2.1 | 性能优化（静态瓦片缓存） | ✅ |
| v2.2 | 字体修复 + 默认地图加载 | ✅ |
| v2.3 | 相机Transform修复 + CellInfo面板 | ✅ |
| v2.4 | 相机初始位置修复 + 第一帧渲染修复 | ✅ |

---

## 编译和运行

```powershell
# 编译
cd d:\Users\gxh\Documents\GitHub\Crystal\ClientRust
cargo build --bin map_viewer_bevy --release

# 运行
.\target\release\map_viewer_bevy.exe
```

现在程序应该：
- ✅ 启动时立即显示地图中心
- ✅ 所有方向的瓦片都可见
- ✅ 平滑拖拽和缩放
- ✅ 鼠标悬停显示CellInfo
