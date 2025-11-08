# Macroquad 移植指南

## 📋 架构设计

本项目支持两种渲染后端：

- **ggez** - 基于 wgpu 的桌面游戏框架 (默认)
- **macroquad** - 跨平台游戏框架 (支持 Web/移动端)

### 渲染抽象层

所有渲染相关的代码都通过 `src/backends/` 模块抽象：

```
src/backends/
├── mod.rs                    # Renderer trait 定义
├── types.rs                  # 通用类型 (Vec2, Color, DrawParams 等)
├── ggez_backend.rs           # ggez 实现 (待完成)
└── macroquad_backend.rs      # macroquad 实现 (已完成)
```

### 核心 Trait

```rust
pub trait Renderer {
    fn clear(&mut self, color: Color);
    fn draw_texture(&mut self, texture_id: TextureId, params: DrawParams);
    fn draw_rect(&mut self, rect: Rect, color: Color);
    fn draw_line(&mut self, start: Vec2, end: Vec2, thickness: f32, color: Color);
    fn draw_text(&mut self, text: &str, pos: Vec2, params: TextParams);
    fn present(&mut self) -> Result<(), RenderError>;
    fn screen_size(&self) -> (f32, f32);
}

pub trait TextureManager {
    fn create_texture_from_rgba(&mut self, width: u16, height: u16, data: &[u8]) 
        -> Result<TextureId, RenderError>;
    fn delete_texture(&mut self, id: TextureId);
    fn texture_size(&self, id: TextureId) -> Option<(u16, u16)>;
}
```

## 🔧 构建和运行

### 使用 ggez 后端 (默认)

```bash
# 运行原版地图查看器 (ggez)
cargo run --bin map_viewer_v3

# 或显式指定 ggez 后端
cargo run --bin map_viewer_v3 --features backend-ggez
```

### 使用 macroquad 后端

```bash
# 运行 macroquad 版地图查看器
cargo run --bin map_viewer_macroquad --features backend-macroquad --no-default-features

# 或使用简化命令
cargo run --bin map_viewer_macroquad --no-default-features --features backend-macroquad
```

### Web 平台 (WASM)

macroquad 的主要优势之一是原生支持 Web：

```bash
# 添加 wasm32 目标
rustup target add wasm32-unknown-unknown

# 构建 WASM
cargo build --bin map_viewer_macroquad --target wasm32-unknown-unknown \
    --no-default-features --features backend-macroquad --release

# 或使用 macroquad 的工具
cargo install cargo-quad-apk
cargo quad-apk build --bin map_viewer_macroquad --release
```

## 📊 功能对比

| 功能 | ggez | macroquad |
|------|------|-----------|
| 桌面平台 (Windows/macOS/Linux) | ✅ 优秀 | ✅ 良好 |
| Web (WASM) | ⚠️ 实验性 | ✅ 原生支持 |
| 移动端 (iOS/Android) | ❌ 不支持 | ✅ 原生支持 |
| API 复杂度 | 中等 (需要 Context) | 简单 (全局函数) |
| 渲染性能 (桌面) | ✅ 更快 (wgpu) | ✅ 良好 (miniquad) |
| 2D 图形 API | 完善 | 完善 |
| 中文字体支持 | ✅ 良好 | ✅ 良好 (需手动加载) |
| IME 输入法 | ✅ 完整支持 | ⚠️ 基础支持 |
| 社区生态 | 成熟 | 活跃 |

## 🎯 迁移策略

### 已完成 ✅

1. ✅ 添加 macroquad 依赖和 feature flags
2. ✅ 创建渲染后端抽象层 (Renderer trait)
3. ✅ 实现 MacroquadRenderer
4. ✅ 创建 map_viewer_macroquad 演示程序
5. ✅ MLibrary 图像转 macroquad 纹理

### 待完成 📝

1. **创建 ggez 后端适配器** - 将现有 ggez 代码封装到 Renderer trait
2. **重构 ECS 渲染系统** - 使用 Renderer trait 而非直接调用 ggez
3. **统一输入处理** - 抽象键盘/鼠标事件
4. **UI 系统迁移** - 考虑使用 egui (支持多后端)
5. **音频系统** - 抽象音频 API
6. **移动端测试** - 在 iOS/Android 上验证

## 💡 使用建议

### 何时使用 ggez

- ✅ 桌面端为主要目标平台
- ✅ 需要完整的 IME 输入法支持
- ✅ 已有大量 ggez 代码
- ✅ 追求桌面端性能极致优化

### 何时使用 macroquad

- ✅ 需要 Web 版本 (浏览器内运行)
- ✅ 计划发布移动端 (iOS/Android)
- ✅ 快速原型开发
- ✅ 跨平台兼容性优先
- ✅ 团队成员对游戏开发经验较少

## 🔍 代码示例

### macroquad 版本的渲染代码

```rust
#[macroquad::main("Mir2 Client")]
async fn main() {
    let mut renderer = MacroquadRenderer::new();
    
    // 加载字体
    renderer.load_font("AlibabaPuHuiTi", "resources/font/xxx.ttf").await?;
    
    loop {
        // 清屏
        renderer.clear(Color::BLACK);
        
        // 绘制纹理
        renderer.draw_texture(texture_id, DrawParams {
            position: Vec2::new(100.0, 100.0),
            scale: Vec2::new(2.0, 2.0),
            color: Color::WHITE,
            ..Default::default()
        });
        
        // 绘制文本
        renderer.draw_text("传奇世界", Vec2::new(10.0, 30.0), TextParams {
            font_size: 24.0,
            color: Color::WHITE,
            font_name: Some("AlibabaPuHuiTi".to_string()),
            ..Default::default()
        });
        
        // 提交渲染
        renderer.present()?;
        next_frame().await
    }
}
```

### 与 ggez 版本的对比

**ggez (当前):**

```rust
impl EventHandler<GameContext> for App {
    fn draw(&mut self, ctx: &mut GameContext) -> GameResult {
        let (gfx_ctx, world) = ctx.split_gfx_world();
        let mut canvas = Canvas::from_frame(gfx_ctx, Color::BLACK);
        // ... 复杂的上下文传递
        canvas.finish(gfx_ctx)?;
        Ok(())
    }
}
```

**macroquad (新):**

```rust
loop {
    clear_background(BLACK);
    draw_texture_ex(texture, x, y, WHITE, params);
    draw_text("Hello", 10.0, 30.0, 24.0, WHITE);
    next_frame().await
}
```

## 🚀 下一步计划

1. **测试 macroquad 版本**

   ```bash
   cargo run --bin map_viewer_macroquad --no-default-features --features backend-macroquad
   ```

2. **验证资源加载** - 确认 MLibrary 图像正确转换为纹理

3. **性能测试** - 对比 ggez 和 macroquad 的帧率

4. **Web 部署** - 构建 WASM 版本并在浏览器中测试

5. **ECS 集成** - 将现有的 hecs 系统适配到 macroquad

## 📚 参考资料

- [macroquad 官方文档](https://macroquad.rs/)
- [macroquad GitHub](https://github.com/not-fl3/macroquad)
- [ggez 官方文档](https://ggez.rs/)
- [渲染抽象层设计模式](https://www.gamedevelopment.blog/render-backend-abstraction/)
