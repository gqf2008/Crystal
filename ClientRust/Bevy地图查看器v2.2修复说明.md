# Bevy地图查看器 v2.2 修复说明

## 修复内容

### 1. 中文字体乱码修复 ✅

**问题**: UI文本显示为方块乱码

**原因**: 
- 字体路径错误：`../resources/font/AlibabaPuHuiTi-3-55-Regular.ttf`
- Bevy的AssetServer从`assets/`目录加载资源

**修复**:
```rust
// ❌ 错误路径
let font_handle = asset_server.load("../resources/font/AlibabaPuHuiTi-3-55-Regular.ttf");

// ✅ 正确路径（使用assets/fonts中的Noto Sans SC）
let font_handle = asset_server.load("fonts/NotoSansSC-Regular.ttf");
```

**变更文件**: `src/bin/map_viewer_bevy.rs` 第355行

---

### 2. 默认地图自动加载 ✅

**问题**: 启动后只显示灰色背景和UI文本，需要按M键手动加载地图

**原因**: `setup_system`没有自动加载地图数据

**修复**:
```rust
fn setup_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut clear_color: ResMut<ClearColor>,
    mut map_data: ResMut<MapData>,  // 🆕 添加参数
) {
    // ... 相机和UI初始化 ...

    // 🆕 自动加载默认地图
    if let Ok(reader) = MapReader::new("Map/0.map") {
        *map_data = MapData::from_reader(reader, "Map/0.map".to_string());
        info!("✅ 自动加载默认地图: Map/0.map ({}x{})", map_data.width, map_data.height);
    } else {
        warn!("⚠️ 无法加载默认地图 Map/0.map，请按M键手动加载");
    }
}
```

**变更文件**: `src/bin/map_viewer_bevy.rs` 第336-358行

---

## 验证清单

### ✅ 已修复
- [x] 字体路径改为`fonts/NotoSansSC-Regular.ttf`
- [x] 默认地图自动加载（`Map/0.map`）
- [x] 编译成功（11.30秒）
- [x] UI文字颜色调整为深灰色（在浅色背景上清晰）

### 🧪 需要测试
- [ ] 中文显示是否正常
- [ ] 地图瓦片是否正确渲染
- [ ] 相机初始位置是否在地图中心
- [ ] 三层（Back/Middle/Front）是否都可见

---

## 运行测试

```powershell
# 编译
cd d:\Users\gxh\Documents\GitHub\Crystal\ClientRust
cargo build --bin map_viewer_bevy --release

# 运行
cargo run --bin map_viewer_bevy --release
# 或直接运行
.\target\release\map_viewer_bevy.exe
```

---

## 技术细节

### Bevy资产系统

Bevy的`AssetServer`默认从**工作目录下的`assets/`目录**加载资源：

```
ClientRust/
├── assets/          ← AssetServer根目录
│   ├── fonts/
│   │   └── NotoSansSC-Regular.ttf  ✅
│   └── textures/
├── resources/       ← 不是AssetServer路径
│   └── font/
│       └── AlibabaPuHuiTi-3-55-Regular.ttf  ❌
└── src/
```

**加载路径规则**:
```rust
// 相对于assets/目录
asset_server.load("fonts/NotoSansSC-Regular.ttf")
// → 实际加载: ClientRust/assets/fonts/NotoSansSC-Regular.ttf
```

### 坐标系统

Bevy使用Y轴向上的坐标系，已在`map_to_world`中处理：

```rust
fn map_to_world(grid_x: i32, grid_y: i32) -> Vec2 {
    Vec2::new(
        (grid_x * CELL_WIDTH) as f32,
        -((grid_y * CELL_HEIGHT) as f32),  // Y轴翻转
    )
}
```

---

## 如果还有问题

### 问题：字体仍然乱码

**解决方案1**: 复制阿里巴巴字体到assets
```powershell
Copy-Item "resources/font/AlibabaPuHuiTi-3-55-Regular.ttf" "assets/fonts/"
```

然后修改代码：
```rust
let font_handle = asset_server.load("fonts/AlibabaPuHuiTi-3-55-Regular.ttf");
```

**解决方案2**: 使用内嵌字体
```rust
// 在main()中添加
App::new()
    .add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            // ...
        }),
        ..default()
    }))
    // 🆕 注册字体（如果需要）
    .init_asset::<Font>()
    // ...
```

### 问题：地图渲染不正确

**检查项**:
1. `Map/0.map`文件是否存在
2. `Data/`目录下的MLibrary文件是否完整
3. 查看控制台日志输出
4. 尝试按M键手动加载其他地图

---

## 版本历史

| 版本 | 修复内容 | 状态 |
|------|---------|------|
| v2.0 | 动画系统 + 门系统 | ✅ |
| v2.1 | 性能优化（静态瓦片缓存） | ✅ |
| v2.2 | 字体修复 + 默认地图加载 | ✅ |

---

## 下一步优化

1. **资产预加载** - 在启动时预加载常用纹理
2. **错误提示** - 地图加载失败时显示友好提示
3. **字体选择** - 支持用户自定义字体
4. **调试模式** - 添加FPS显示、内存监控等
