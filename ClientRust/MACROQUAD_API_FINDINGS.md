# Macroquad API 最新发现报告

基于对 [macroquad GitHub仓库](https://github.com/not-fl3/macroquad) 的深入分析，以下是关键 API 和最佳实践：

## ✅ API 验证结果

### 1. 纹理管理

```rust
// ✅ 正确：从 RGBA 数据创建纹理
let texture = Texture2D::from_rgba8(width, height, &data);

// ✅ 正确：设置过滤模式
texture.set_filter(FilterMode::Nearest);  // 像素艺术
texture.set_filter(FilterMode::Linear);   // 平滑

// ❌ 错误：没有 delete() 方法！
// texture.delete();  // 这个方法不存在

// ✅ 正确：自动清理
// Texture2D 实现了 Drop trait，会自动清理
drop(texture);  // 或者让它超出作用域
```

### 2. 文本渲染

```rust
// ✅ 正确的 API签名（从源码确认）
draw_text_ex(
    text: &str,
    x: f32,
    y: f32,
    TextParams {
        font: Option<&Font>,      // 注意：是引用，不是 Option<Font>
        font_size: u16,           // 不是 f32！
        font_scale: f32,          // 额外的缩放
        color: Color,
        rotation: f32,
        ..Default::default()
    }
);

// measure_text 签名
fn measure_text(
    text: &str,
    font: Option<&Font>,
    font_size: u16,
    font_scale: f32,
) -> TextDimensions;
```

### 3. 主循环结构

```rust
fn window_conf() -> Conf {
    Conf {
        window_title: "My Game".to_owned(),
        window_width: 1280,
        window_height: 960,
        window_resizable: true,
        fullscreen: false,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    // 初始化
    let texture = load_texture("path.png").await.unwrap();
    
    loop {
        // 输入处理
        if is_key_pressed(KeyCode::Escape) {
            break;
        }
        
        // 更新逻辑
        
        // 清屏
        clear_background(BLACK);
        
        // 绘制
        draw_texture(&texture, 100.0, 100.0, WHITE);
        
        // 关键：必须 await
        next_frame().await
    }
}
```

### 4. 输入处理

```rust
// 鼠标
let (x, y) = mouse_position();
let wheel = mouse_wheel();  // 返回 (x, y) Vec2
let is_clicked = is_mouse_button_pressed(MouseButton::Left);
let is_down = is_mouse_button_down(MouseButton::Left);

// 键盘
if is_key_pressed(KeyCode::Space) { }
if is_key_down(KeyCode::W) { }
if is_key_released(KeyCode::Escape) { }

// 触摸（移动端）
let touches = touches();
for touch in touches {
    println!("Touch at ({}, {})", touch.position.x, touch.position.y);
}
```

### 5. 绘制函数

```rust
// 纹理
draw_texture(&texture, x, y, WHITE);
draw_texture_ex(&texture, x, y, WHITE, DrawTextureParams {
    dest_size: Some(vec2(width, height)),
    source: Some(Rect::new(sx, sy, sw, sh)),  // 精灵图集
    rotation: 0.0,
    flip_x: false,
    flip_y: false,
    pivot: None,
});

// 形状
draw_rectangle(x, y, w, h, color);
draw_circle(x, y, radius, color);
draw_line(x1, y1, x2, y2, thickness, color);

// 文本
draw_text("Hello", x, y, font_size, color);
```

### 6. 相机系统

```rust
// 2D 相机
let camera = Camera2D {
    target: vec2(player_x, player_y),
    zoom: vec2(2.0, 2.0),  // 放大2倍
    rotation: 0.0,
    ..Default::default()
};
set_camera(&camera);

// 重置为屏幕空间
set_default_camera();
```

## 🔧 我们实现中需要修复的地方

### 1. MacroquadRenderer::delete_texture

**当前（错误）：**

```rust
fn delete_texture(&mut self, id: TextureId) {
    if let Some(texture) = self.textures.remove(&id) {
        texture.delete();  // ❌ 这个方法不存在
    }
}
```

**应该（正确）：**

```rust
fn delete_texture(&mut self, id: TextureId) {
    // ✅ Texture2D 自动清理，只需从 HashMap 移除
    self.textures.remove(&id);
}
```

### 2. 文本渲染参数

**当前（可能有问题）：**

```rust
let params = TextParams {
    font: font.copied(),  // ❌ Font 不实现 Copy
    font_size: params.font_size as u16,
    // ...
};
```

**应该（正确）：**

```rust
let params = macroquad::text::TextParams {
    font: font,  // ✅ 直接使用 Option<&Font>
    font_size: params.font_size as u16,  // ✅ 正确
    color: to_macroquad_color(params.color),
    ..Default::default()
};
```

## 📝 最佳实践

### 1. 资源加载

```rust
// 异步加载（推荐）
let texture = load_texture("image.png").await.unwrap();
let font = load_ttf_font("font.ttf").await.unwrap();

// 从内存加载
let image = Image::from_file_with_format(
    include_bytes!("../assets/image.png"),
    None
).unwrap();
let texture = Texture2D::from_image(&image);
```

### 2. 错误处理

```rust
#[macroquad::main("Game")]
async fn main() -> anyhow::Result<()> {  // ✅ 可以返回 Result
    let texture = load_texture("image.png")
        .await
        .context("Failed to load texture")?;
    
    // 游戏循环
    Ok(())
}
```

### 3. 性能优化

```rust
// 批量绘制相同纹理
for (x, y) in positions {
    draw_texture(&texture, x, y, WHITE);  // ✅ 自动批处理
}

// 纹理图集
build_textures_atlas();  // 将所有纹理打包到一个大纹理中
```

### 4. 移动端支持

```rust
fn window_conf() -> Conf {
    Conf {
        // 自适应窗口大小
        window_resizable: true,
        
        // 高 DPI 支持
        high_dpi: true,
        
        // 移动端特定设置
        platform: Platform {
            // ...
        },
        ..Default::default()
    }
}
```

## 🎯 对我们项目的影响

### 已修复

1. ✅ 删除了 `texture.delete()` 调用
2. ✅ 修正了 `TextParams` 的 `font` 字段类型
3. ✅ 确认 `font_size` 使用 `u16`

### 待修复

1. ⚠️ `mlibrary.rs` 中的 ggez 依赖需要条件编译
2. ⚠️ 所有使用 `GraphicsContext` 的函数需要 `#[cfg(feature = "backend-ggez")]`
3. ⚠️ 考虑创建 macroquad 版本的 MLibrary 加载器

## 🚀 下一步建议

1. **选项 A：最小实现**
   - 让 `demo_macroquad` 编译通过（不依赖 MLibrary）
   - 硬编码一些测试纹理
   - 验证 macroquad 基础功能

2. **选项 B：部分重构**
   - 将 `ImageInfo` 的 ggez 字段条件编译
   - 创建纯数据版本的 MLibrary
   - macroquad 版本单独加载纹理

3. **选项 C：完全重构**（推荐但耗时）
   - `ImageInfo` 只存储 RGBA 数据
   - 渲染器负责创建GPU纹理
   - 完全解耦数据层和渲染层

---

**生成时间**: 2025-11-08  
**基于版本**: macroquad 0.4.14  
**参考仓库**: <https://github.com/not-fl3/macroquad>
