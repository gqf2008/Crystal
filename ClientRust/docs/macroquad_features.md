# Macroquad 关键特性参考

本文档记录从官方示例学习到的关键特性，供传奇2客户端移植时参考。

## 目录
- [渲染系统](#渲染系统)
- [相机系统](#相机系统)
- [UI系统](#ui系统)
- [后处理效果](#后处理效果)
- [瓦片地图](#瓦片地图)
- [实战技巧](#实战技巧)

---

## 渲染系统

### 1. RenderTarget（离屏渲染）

**用途**：虚拟分辨率、后处理效果、分屏渲染

```rust
// 创建 RenderTarget
let render_target = render_target(1600, 1200);
render_target.texture.set_filter(FilterMode::Nearest); // 像素风格
// 或
render_target.texture.set_filter(FilterMode::Linear);  // 平滑缩放

// 带抗锯齿
let render_target_msaa = render_target_msaa(1600, 1200); // sample_count=4

// 完全自定义
let render_target = render_target_ex(1600, 1200, RenderTargetParams {
    depth: true,           // 深度缓冲
    sample_count: 4,       // MSAA 抗锯齿
});
```

**渲染流程**：
```rust
// 阶段1：渲染到 RenderTarget
set_camera(&Camera2D {
    render_target: Some(render_target.clone()),
    ..camera_params
});
clear_background(LIGHTGRAY);
draw_game_world(); // 游戏内容

// 阶段2：渲染到屏幕
set_default_camera();
clear_background(BLACK);
draw_texture_ex(&render_target.texture, 0., 0., WHITE, DrawTextureParams {
    dest_size: Some(vec2(screen_width(), screen_height())),
    flip_y: true,  // ⭐ 关键！RenderTarget 需要翻转 Y 轴
    ..Default::default()
});

// 阶段3：UI 叠加（屏幕空间，不会翻转）
draw_ui();
```

**关键点**：
- ✅ `flip_y: true` 是必需的（除非使用着色器处理坐标）
- ✅ UI 应该在屏幕空间绘制，避免被翻转
- ✅ `set_filter()` 控制纹理过滤（像素 vs 平滑）

---

## 相机系统

### 1. Camera2D 基础

```rust
#[derive(Debug)]
pub struct Camera2D {
    pub rotation: f32,              // 旋转角度（度）
    pub zoom: Vec2,                 // 缩放 (1.0, 1.0) = 正常
    pub target: Vec2,               // 相机看向的世界坐标
    pub offset: Vec2,               // 视口偏移
    pub render_target: Option<RenderTarget>,  // 渲染目标
    pub viewport: Option<(i32, i32, i32, i32)>, // 视口区域（分屏）
}
```

### 2. 快速创建固定分辨率相机

```rust
// 方式1：使用 from_display_rect（推荐）
let camera = Camera2D::from_display_rect(Rect::new(0.0, 0.0, 1600.0, 1200.0));
// 自动计算 target 和 zoom，创建 1600x1200 虚拟屏幕

// 方式2：手动计算 zoom
let camera = Camera2D {
    zoom: vec2(1.0 / 800.0, 1.0 / 600.0),  // 1600x1200 视野
    target: vec2(800.0, 600.0),             // 中心点
    render_target: Some(render_target.clone()),
    ..Default::default()
};

// 方式3：保持纵横比
let camera = Camera2D {
    zoom: vec2(1., screen_width() / screen_height()),
    ..Default::default()
};
```

### 3. 相机变换

```rust
// 平移
camera.target = vec2(player_x, player_y);

// 缩放
camera.zoom = vec2(2.0, 2.0); // 放大2倍

// 旋转
camera.rotation = 45.0; // 旋转45度

// 偏移（视口移动，不改变世界坐标）
camera.offset = vec2(100.0, 50.0);
```

### 4. 坐标转换

```rust
// 世界坐标 → 屏幕坐标
let screen_pos = camera.world_to_screen(world_pos);

// 屏幕坐标 → 世界坐标
let world_pos = camera.screen_to_world(mouse_position().into());
```

### 5. 相机切换模式

```rust
// 世界空间（游戏内容）
set_camera(&game_camera);
draw_map();
draw_sprites();

// 屏幕空间（UI）
set_default_camera();
draw_text("HP: 100", 10.0, 30.0, 20.0, WHITE);
```

**关键理解**：
- `render_target.is_some()` 时，Y 轴**不会自动翻转**
- 源码中：`invert_y = if render_target.is_some() { 1.0 } else { -1.0 }`
- 所以绘制到 RenderTarget 时用正常 Y 坐标，最后用 `flip_y: true` 翻转纹理

---

## UI系统

### 1. 内置 UI (megaui)

```rust
use macroquad::ui::{hash, root_ui, widgets};

// 窗口
root_ui().window(hash!(), vec2(100., 100.), vec2(400., 300.), |ui| {
    // 标签
    ui.label(None, "文本标签");
    
    // 按钮
    if ui.button(None, "点击我") {
        println!("按钮被点击");
    }
    
    // 输入框
    ui.input_text(hash!(), "名称:", &mut player_name);
    
    // 复选框
    ui.checkbox(hash!(), "显示网格", &mut show_grid);
    
    // 滑块
    ui.slider(hash!(), "音量", 0.0..1.0, &mut volume);
    
    // 下拉框
    ui.combo_box(hash!(), "难度", &["简单", "普通", "困难"], &mut difficulty);
    
    // 分组布局
    widgets::Group::new(hash!(), vec2(200., 100.)).ui(ui, |ui| {
        ui.label(None, "分组内容");
    });
});
```

### 2. 高级窗口

```rust
widgets::Window::new(hash!(), vec2(100., 100.), vec2(400., 300.))
    .label("装备栏")
    .titlebar(true)          // 显示标题栏
    .movable(true)           // 可拖动
    .close_button(true)      // 关闭按钮
    .ui(&mut *root_ui(), |ui| {
        // 窗口内容
    });
```

### 3. 自定义皮肤

```rust
let font = load_ttf_font("assets/font.ttf").await.unwrap();

let button_style = root_ui()
    .style_builder()
    .with_font(&font).unwrap()
    .text_color(Color::from_rgba(180, 180, 100, 255))
    .font_size(24)
    .background(background_image)
    .background_margin(RectOffset::new(10.0, 10.0, 10.0, 10.0))
    .build();

let skin = Skin {
    button_style,
    ..root_ui().default_skin()
};

root_ui().push_skin(&skin);
// ... 绘制 UI ...
root_ui().pop_skin();
```

### 4. 直接绘制文本（无UI系统）

```rust
// 方式1：使用默认字体
draw_text("文本", 100.0, 100.0, 40.0, WHITE);

// 方式2：加载自定义字体
let font = load_ttf_font("assets/font.ttf").await.unwrap();
set_default_font(font); // 设置为默认
draw_text("文本", 100.0, 100.0, 40.0, WHITE);

// 方式3：高级参数
draw_text_ex("文本", 100.0, 100.0, TextParams {
    font: Some(&custom_font),
    font_size: 45,
    color: RED,
    rotation: 0.27,
    ..Default::default()
});

// 测量文本尺寸
let dimensions = measure_text("文本", Some(&font), 40, 1.0);
println!("宽度: {}, 高度: {}", dimensions.width, dimensions.height);
```

---

## 后处理效果

### 1. 基础后处理流程

```rust
// 加载着色器
let material = load_material(
    ShaderSource::Glsl {
        vertex: VERTEX_SHADER,
        fragment: FRAGMENT_SHADER,
    },
    MaterialParams {
        uniforms: vec![
            UniformDesc::new("Time", UniformType::Float1),
            UniformDesc::new("Center", UniformType::Float2),
        ],
        ..Default::default()
    },
).unwrap();

// 使用着色器
material.set_uniform("Time", get_time() as f32);
material.set_uniform("Center", mouse_position());

gl_use_material(&material);
draw_texture_ex(&render_target.texture, 0., 0., WHITE, params);
gl_use_default_material();
```

### 2. 常见后处理效果

**CRT 显示器效果**（扫描线 + 畸变）：
```glsl
// Fragment Shader
vec2 CRTCurveUV(vec2 uv) {
    uv = uv * 2.0 - 1.0;
    vec2 offset = abs(uv.yx) / vec2(6.0, 4.0);
    uv = uv + uv * offset * offset;
    uv = uv * 0.5 + 0.5;
    return uv;
}

void DrawScanline(inout vec3 color, vec2 uv) {
    float scanline = clamp(0.95 + 0.05 * cos(3.14 * uv.y * 240.0 * 1.0), 0.0, 1.0);
    float grille = 0.85 + 0.15 * clamp(1.5 * cos(3.14 * uv.x * 640.0 * 1.0), 0.0, 1.0);
    color *= scanline * grille * 1.2;
}
```

**像素化效果**：
```rust
// 渲染到低分辨率 RenderTarget
let render_target = render_target(160, 120);
render_target.texture.set_filter(FilterMode::Nearest);

// 放大到全屏时保持像素边缘锐利
```

**模糊/景深效果**：
```glsl
vec4 blur(sampler2D tex, vec2 uv) {
    vec4 color = vec4(0.0);
    for(int x = -2; x <= 2; x++) {
        for(int y = -2; y <= 2; y++) {
            vec2 offset = vec2(float(x), float(y)) * pixel_size;
            color += texture2D(tex, uv + offset);
        }
    }
    return color / 25.0;
}
```

---

## 瓦片地图

### 1. macroquad-tiled 集成

```rust
use macroquad_tiled as tiled;

// 加载纹理集
let tileset = load_texture("tileset.png").await.unwrap();
tileset.set_filter(FilterMode::Nearest);

// 加载地图 JSON
let tiled_map_json = load_string("map.json").await.unwrap();
let tiled_map = tiled::load_map(
    &tiled_map_json, 
    &[("tileset.png", tileset)],
    &[]
).unwrap();

// 绘制图层
tiled_map.draw_tiles(
    "main layer",                           // 图层名
    Rect::new(0.0, 0.0, 320.0, 152.0),     // 可见区域
    None                                    // 源区域（None=全部）
);

// 绘制单个精灵
tiled_map.spr(
    "tileset",
    sprite_id,
    Rect::new(x, y, width, height)
);

// 绘制精灵（带参数）
tiled_map.spr_ex(
    "tileset",
    Rect::new(tile_x, tile_y, tile_w, tile_h),  // 源矩形
    Rect::new(dest_x, dest_y, dest_w, dest_h)   // 目标矩形
);
```

### 2. 传奇2 地图适配思路

```rust
// 传奇2 地图格式
struct Mir2Map {
    width: u32,
    height: u32,
    tiles: Vec<TileInfo>,
}

struct TileInfo {
    bg_tile: u16,      // 背景图块
    mid_tile: u16,     // 中层图块
    fg_tile: u16,      // 前景图块
    door_tile: u16,    // 门图块
    door_offset: u8,   // 门偏移
    ani_frame: u8,     // 动画帧
    ani_tick: u8,      // 动画时间
    flags: u8,         // 标志位
}

// 渲染函数
fn draw_mir2_map(map: &Mir2Map, camera_x: f32, camera_y: f32) {
    // 计算可见区域
    let visible_rect = calculate_visible_tiles(camera_x, camera_y);
    
    // 分层渲染
    for layer in ["background", "middle", "foreground"] {
        for tile in visible_tiles(map, visible_rect, layer) {
            let texture_id = get_tile_texture(tile.tile_id);
            draw_texture_ex(&texture_id, tile.x, tile.y, WHITE, params);
        }
    }
}
```

---

## 实战技巧

### 1. 虚拟分辨率 + Letterbox（黑边）

```rust
const VIRTUAL_WIDTH: f32 = 1600.0;
const VIRTUAL_HEIGHT: f32 = 1200.0;

let render_target = render_target(VIRTUAL_WIDTH as u32, VIRTUAL_HEIGHT as u32);
render_target.texture.set_filter(FilterMode::Linear);

let render_target_cam = Camera2D::from_display_rect(
    Rect::new(0., 0., VIRTUAL_WIDTH, VIRTUAL_HEIGHT)
);
render_target_cam.render_target = Some(render_target.clone());

loop {
    // 计算缩放比例（保持纵横比）
    let scale = f32::min(
        screen_width() / VIRTUAL_WIDTH,
        screen_height() / VIRTUAL_HEIGHT,
    );
    
    // 计算 letterbox 偏移
    let x = (screen_width() - VIRTUAL_WIDTH * scale) * 0.5;
    let y = (screen_height() - VIRTUAL_HEIGHT * scale) * 0.5;
    
    // 渲染到虚拟屏幕
    set_camera(&render_target_cam);
    clear_background(LIGHTGRAY);
    draw_game();
    
    // 渲染到物理屏幕（带黑边）
    set_default_camera();
    clear_background(BLACK); // 黑边颜色
    draw_texture_ex(&render_target.texture, x, y, WHITE, DrawTextureParams {
        dest_size: Some(vec2(VIRTUAL_WIDTH * scale, VIRTUAL_HEIGHT * scale)),
        flip_y: true,
        ..Default::default()
    });
}
```

### 2. 虚拟鼠标坐标转换

```rust
// 物理鼠标 → 虚拟坐标
let scale = f32::min(
    screen_width() / VIRTUAL_WIDTH,
    screen_height() / VIRTUAL_HEIGHT,
);

let virtual_mouse_pos = Vec2 {
    x: (mouse_position().0 - (screen_width() - VIRTUAL_WIDTH * scale) * 0.5) / scale,
    y: (mouse_position().1 - (screen_height() - VIRTUAL_HEIGHT * scale) * 0.5) / scale,
};
```

### 3. 性能优化

```rust
// 1. 纹理图集（减少绘制调用）
build_textures_atlas();

// 2. 批量绘制
gl_use_material(&material);
for sprite in &sprites {
    draw_texture_ex(&sprite.texture, sprite.x, sprite.y, WHITE, params);
}
gl_use_default_material();

// 3. 视锥剔除
fn is_visible(camera: &Camera2D, entity: &Entity) -> bool {
    let screen_pos = camera.world_to_screen(entity.pos);
    screen_pos.x >= 0.0 && screen_pos.x <= screen_width() &&
    screen_pos.y >= 0.0 && screen_pos.y <= screen_height()
}
```

### 4. 高 DPI 支持

```rust
// window_conf 配置
fn window_conf() -> Conf {
    Conf {
        window_title: "传奇2".to_owned(),
        window_width: 1024,
        window_height: 768,
        high_dpi: true,  // ⭐ macOS Retina 支持
        ..Default::default()
    }
}

// 获取 DPI 缩放
let dpi_scale = screen_dpi_scale();
```

### 5. 分屏渲染

```rust
// 玩家1视口（左半屏）
let camera1 = Camera2D {
    viewport: Some((0, 0, screen_width() as i32 / 2, screen_height() as i32)),
    ..Default::default()
};

// 玩家2视口（右半屏）
let camera2 = Camera2D {
    viewport: Some((screen_width() as i32 / 2, 0, screen_width() as i32 / 2, screen_height() as i32)),
    ..Default::default()
};
```

---

## 传奇2客户端移植要点

### 阶段1：基础框架
- [x] 虚拟分辨率系统（1600x1200 → 窗口大小）
- [x] RenderTarget + Camera2D
- [x] 双阶段渲染（游戏 + UI）
- [x] 中文字体支持
- [ ] root_ui() 集成

### 阶段2：资源加载
- [ ] MLibrary 读取器（Wil/Wix/Wzl）
- [ ] 图像解码（8位索引色 + 调色板）
- [ ] 动画系统（帧序列）
- [ ] 音效/音乐加载

### 阶段3：地图渲染
- [ ] 地图文件解析（Map/Tiles/Smtiles）
- [ ] 分层渲染（背景/中层/前景）
- [ ] 瓦片动画
- [ ] 相机跟随角色

### 阶段4：精灵/角色
- [ ] 精灵渲染器
- [ ] 动作系统（站立/行走/攻击）
- [ ] 方向控制（8方向）
- [ ] 装备渲染叠加

### 阶段5：UI系统
- [ ] 主界面（血条/蓝条/经验）
- [ ] 背包系统
- [ ] 聊天框
- [ ] 小地图

### 阶段6：网络/逻辑
- [ ] 服务器连接
- [ ] 包协议解析
- [ ] 状态同步
- [ ] 技能系统

---

## 参考资源

- [macroquad 官方文档](https://docs.rs/macroquad/)
- [macroquad 示例代码](https://github.com/not-fl3/macroquad/tree/master/examples)
- [letterbox.rs](https://github.com/not-fl3/macroquad/blob/master/examples/letterbox.rs) - 虚拟分辨率
- [post_processing.rs](https://github.com/not-fl3/macroquad/blob/master/examples/post_processing.rs) - 后处理
- [platformer.rs](https://github.com/not-fl3/macroquad/blob/master/examples/platformer.rs) - 瓦片地图
- [ui.rs](https://github.com/not-fl3/macroquad/blob/master/examples/ui.rs) - UI 系统

---

**最后更新**: 2025-01-08
**当前版本**: macroquad 0.4.14
