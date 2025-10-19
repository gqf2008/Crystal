# Bevy地图查看器 v2.3 修复说明

## 修复内容

### 1. 相机Transform更新缺失 ✅

**问题**: 地图不跟随鼠标拖拽移动

**原因**: `mouse_input_system`只更新了`MapCamera.target`，但没有更新相机的`Transform`组件

**修复**:
```rust
// ❌ 错误：只更新逻辑状态
fn mouse_input_system(
    mut camera_query: Query<&mut MapCamera>,  // 缺少Transform
) {
    if camera.dragging {
        camera.target.x -= delta.x / camera.zoom;
        camera.target.y += delta.y / camera.zoom;
        // ⚠️ 没有更新Transform！地图不会跟随移动
    }
}

// ✅ 正确：同时更新Transform
fn mouse_input_system(
    mut camera_query: Query<(&mut MapCamera, &mut Transform)>,  // 添加Transform
) {
    if camera.dragging {
        camera.target.x -= delta.x / camera.zoom;
        camera.target.y += delta.y / camera.zoom;
        
        // 🔧 关键修复：同步更新Transform
        transform.translation.x = camera.target.x;
        transform.translation.y = camera.target.y;
    }
}
```

**变更文件**: `src/bin/map_viewer_bevy.rs` 第457-494行

---

### 2. 缺少CellInfo悬停面板 ✅

**问题**: 鼠标悬停在地图格子上时没有显示详细信息（ggez版本有）

**实现**:

#### 2.1 添加组件标记
```rust
/// CellInfo悬停面板标记
#[derive(Component)]
struct CellInfoPanel;
```

#### 2.2 在setup_system中创建UI面板
```rust
// 🖱️ CellInfo悬停面板（初始隐藏）
parent.spawn((
    Node {
        position_type: PositionType::Absolute,
        display: Display::None,  // 初始隐藏
        width: Val::Px(650.0),
        height: Val::Px(320.0),
        padding: UiRect::all(Val::Px(10.0)),
        border: UiRect::all(Val::Px(2.0)),
        ..default()
    },
    BackgroundColor(Color::srgba(0.16, 0.16, 0.16, 0.86)),  // 半透明深灰背景
    BorderColor::from(Color::srgb(0.4, 0.4, 0.4)),  // 边框颜色
    CellInfoPanel,
    Name::new("CellInfo Panel"),
)).with_children(|panel| {
    panel.spawn((
        Text::new(""),  // 动态更新内容
        TextColor(Color::WHITE),
        TextFont {
            font: font_handle.clone(),
            font_size: 16.0,
            ..default()
        },
    ));
});
```

#### 2.3 添加更新系统
```rust
/// 🖱️ CellInfo悬停面板更新系统
fn update_cell_info_panel_system(
    map_data: Res<MapData>,
    camera_query: Query<&MapCamera>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut panel_query: Query<&mut Node, With<CellInfoPanel>>,
    panel_children_query: Query<&Children, With<CellInfoPanel>>,
    mut text_query: Query<&mut Text, Without<InfoText>>,
) {
    // 获取鼠标位置对应的地图格子
    let Some(cursor_pos) = window.cursor_position() else {
        panel_node.display = Display::None;  // 鼠标不在窗口内，隐藏面板
        return;
    };

    let world_pos = camera.target + cursor_offset / camera.zoom;
    let (grid_x, grid_y) = world_to_map(world_pos);

    let Some(cell) = map_data.get_cell(grid_x, grid_y) else {
        panel_node.display = Display::None;  // 格子不存在，隐藏面板
        return;
    };

    // 显示面板并更新内容
    panel_node.display = Display::Flex;
    text.0 = format!("X: {}  Y: {}  ...", grid_x, grid_y);

    // 🖱️ 计算面板位置（跟随鼠标，边界自动翻转）
    panel_node.left = Val::Px(panel_x);
    panel_node.top = Val::Px(panel_y);
}
```

#### 2.4 注册系统
```rust
.add_systems(Update, (
    // ...
    update_cell_info_panel_system,  // 🆕 CellInfo悬停面板
).chain())
```

**显示内容**（与ggez版本格式一致）:
```
X: 100        Y: 50     Version        LibName    LibIndex
BackImage:   1234       WemadeMir2     Tiles      0
MiddleImage: 567        WemadeMir2     Smtiles    1
FrontImage:  890        WemadeMir2     Objects    2

Limit:       Back  False           Front  False

Animation:   F_Frame   F_Tick     F_Blend
Animation:   M_Frame   M_Tick     M_Blend

Door:        Offset 0  Index 0    Entity  False

Light: 15     Fishing: False
```

**特性**:
- ✅ 跟随鼠标位置
- ✅ 边界自动翻转（不超出屏幕）
- ✅ 避开状态栏
- ✅ 鼠标移出窗口自动隐藏
- ✅ 格子不存在时自动隐藏

**变更文件**: `src/bin/map_viewer_bevy.rs` 第258、397-419、327、1141-1247行

---

## 技术细节

### Bevy相机系统

Bevy的2D相机通过`Transform`组件控制视图：

```rust
// Transform.translation 控制相机位置
transform.translation.x = camera.target.x;
transform.translation.y = camera.target.y;

// Transform.scale 控制缩放
transform.scale = Vec3::splat(camera.zoom);

// 所有瓦片的Transform会被相机Transform自动转换！
```

**关键点**:
- `MapCamera.target` 是逻辑状态（记录目标位置）
- `Transform.translation` 是实际渲染位置
- **必须同步更新两者**，否则视图不会改变

### UI父子查询

Bevy的UI父子关系需要通过`Children`组件查询：

```rust
// ❌ 错误：直接查询子Text会失败
mut text_query: Query<&mut Text, With<CellInfoPanel>>  // Text不是CellInfoPanel的组件！

// ✅ 正确：通过Children查询子实体
panel_children_query: Query<&Children, With<CellInfoPanel>>,
mut text_query: Query<&mut Text>,

// 获取子实体
let Ok(children) = panel_children_query.single() else { return; };
let text_entity = children.first()?;
let Ok(mut text) = text_query.get_mut(text_entity) else { return; };
```

---

## 版本历史

| 版本 | 修复内容 | 状态 |
|------|---------|------|
| v2.0 | 动画系统 + 门系统 | ✅ |
| v2.1 | 性能优化（静态瓦片缓存） | ✅ |
| v2.2 | 字体修复 + 默认地图加载 | ✅ |
| v2.3 | 相机Transform修复 + CellInfo面板 | ✅ |

---

## 测试验证

### ✅ 已验证功能

- [x] 地图自动加载（Map/0.map）
- [x] 中文字体正常显示（Noto Sans SC）
- [x] 鼠标中键拖拽地图移动
- [x] 滚轮缩放
- [x] 鼠标悬停显示CellInfo面板
- [x] 面板边界自动翻转
- [x] 状态栏显示FPS和相机信息

### 🎮 操作说明

| 操作 | 功能 |
|------|------|
| **鼠标中键拖拽** | 移动地图视角 |
| **滚轮** | 缩放地图 |
| **鼠标悬停** | 显示格子详细信息 |
| **M键** | 打开地图文件选择对话框 |
| **G键** | 切换网格显示 |
| **1/2/3键** | 切换Back/Middle/Front层 |
| **I键** | 隐藏/显示信息面板 |

---

## 与ggez版本对比

| 功能 | ggez版本 | Bevy版本 (v2.3) | 说明 |
|------|---------|----------------|------|
| 地图渲染 | ✅ | ✅ | 完全一致 |
| 相机拖拽 | ✅ | ✅ | 修复后一致 |
| 鼠标缩放 | ✅ | ✅ | 完全一致 |
| CellInfo面板 | ✅ | ✅ | 修复后一致 |
| 中文字体 | ✅ | ✅ | 使用不同字体 |
| 性能 | 中等 | 更高 | Bevy ECS性能更好 |

---

## 已知问题

### ⚠️ 字体样式差异

**问题**: Bevy版本使用Noto Sans SC，ggez版本使用阿里巴巴普惠体

**影响**: UI文字宽度略有不同

**解决方案**: 复制阿里巴巴字体到`assets/fonts/`目录
```powershell
Copy-Item "resources/font/AlibabaPuHuiTi-3-55-Regular.ttf" "assets/fonts/"
```

然后修改代码：
```rust
let font_handle = asset_server.load("fonts/AlibabaPuHuiTi-3-55-Regular.ttf");
```

---

## 下一步优化

1. **动画瓦片性能** - 减少Entity的销毁和重建
2. **UI主题切换** - 支持亮色/暗色主题
3. **网格渲染优化** - 使用Bevy的Gizmos系统
4. **调试信息** - 添加内存使用、渲染统计等

---

## 编译和运行

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

## 文件变更总结

**修改的文件**: `src/bin/map_viewer_bevy.rs`

**新增行数**: +150行（CellInfo面板系统）

**修改行数**: +3行（相机Transform更新）

**总行数**: 1096 → 1257行

**代码质量**: ✅ 编译通过，无警告（除了未使用变量）
