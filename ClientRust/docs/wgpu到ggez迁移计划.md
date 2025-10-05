# wgpu → ggez 迁移计划

**迁移原因**: wgpu对2D游戏来说过于底层，ggez提供更高层次的API，开发效率更高。

**当前状态**: ggez已添加到Cargo.toml，版本0.10.0-rc0

---

## 📋 迁移任务清单

### 阶段1: 核心渲染系统 (优先级: 🔴 HIGH)

#### 1.1 创建ggez图形管理器
- [ ] 创建 `src/graphics/ggez_manager.rs`
- [ ] 替代 `DXManager` 的功能
- [ ] 窗口创建和管理
- [ ] 渲染上下文初始化

#### 1.2 纹理系统
- [ ] 纹理加载（从.lib文件）
- [ ] 纹理缓存
- [ ] 精灵渲染

#### 1.3 基本绘制
- [ ] 绘制精灵（位置、旋转、缩放）
- [ ] 绘制文本
- [ ] 绘制矩形/线条（调试用）

### 阶段2: UI系统集成 (优先级: 🟡 MEDIUM)

#### 2.1 Forms适配
- [ ] MainWindow与ggez集成
- [ ] LauncherWindow渲染
- [ ] ConfigWindow渲染

#### 2.2 Controls渲染
- [ ] MirButton渲染
- [ ] MirLabel渲染
- [ ] MirImageControl渲染
- [ ] MirTextBox渲染

### 阶段3: 场景渲染 (优先级: 🟡 MEDIUM)

#### 3.1 Scene.draw()实现
- [ ] LoginScene渲染
- [ ] SelectScene渲染
- [ ] GameScene地图渲染

#### 3.2 游戏对象渲染
- [ ] 角色精灵
- [ ] 怪物精灵
- [ ] NPC精灵
- [ ] 物品精灵
- [ ] 特效精灵

### 阶段4: 音频系统 (优先级: 🟢 LOW)

- [ ] 评估是否使用ggez音频（或保留rodio）
- [ ] 背景音乐播放
- [ ] 音效播放

---

## 🔄 API对比和迁移示例

### 窗口创建

#### wgpu方式 (旧)
```rust
// DXManager::new()
let instance = wgpu::Instance::new(...);
let surface = instance.create_surface(&window)?;
let adapter = instance.request_adapter(...).await?;
let device = adapter.request_device(...).await?;
// ... 100+ 行配置代码
```

#### ggez方式 (新)
```rust
// GgezManager::new()
let cb = ggez::ContextBuilder::new("mir2_client", "Crystal")
    .window_setup(ggez::conf::WindowSetup::default().title("Legend of Mir 2"))
    .window_mode(ggez::conf::WindowMode::default().dimensions(1024.0, 768.0));
let (ctx, event_loop) = cb.build()?;
// 完成！只需几行
```

### 纹理加载

#### wgpu方式 (旧)
```rust
// 需要手动创建纹理、视图、采样器
let texture = device.create_texture(&wgpu::TextureDescriptor { ... });
let view = texture.create_view(&Default::default());
let sampler = device.create_sampler(&wgpu::SamplerDescriptor { ... });
// 需要手动管理bind group
```

#### ggez方式 (新)
```rust
// 自动处理所有细节
let image = graphics::Image::from_rgba8(ctx, width, height, &pixels)?;
// 或从文件加载
let image = graphics::Image::from_path(ctx, "/images/sprite.png")?;
```

### 精灵渲染

#### wgpu方式 (旧)
```rust
// 需要手写顶点数据、shader、渲染管线
let vertices = create_sprite_vertices(x, y, width, height);
let vertex_buffer = device.create_buffer_init(...);
render_pass.set_pipeline(&pipeline);
render_pass.set_bind_group(0, &bind_group, &[]);
render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
render_pass.draw(0..6, 0..1);
```

#### ggez方式 (新)
```rust
// 一行代码搞定！
graphics::draw(ctx, &image, DrawParam::default().dest([x, y]))?;
```

### 文本渲染

#### wgpu方式 (旧)
```rust
// 需要使用第三方库如wgpu_glyph，非常复杂
// ... 50+ 行代码
```

#### ggez方式 (新)
```rust
// 内置文本渲染
let text = graphics::Text::new("Hello World");
graphics::draw(ctx, &text, DrawParam::default().dest([x, y]))?;
```

---

## 🏗️ 新的Graphics模块结构

```
src/graphics/
├── mod.rs                      # 模块导出
├── ggez_manager.rs             # 新增：ggez渲染管理器
├── texture_cache.rs            # 纹理缓存
├── sprite_renderer.rs          # 精灵渲染器（简化版）
├── mlibrary.rs                 # .lib文件读取（保留）
└── particles/                  # 粒子系统（可选保留）

移除/废弃:
❌ dx_manager.rs                # 删除wgpu版本
❌ sprite_instanced_renderer.rs # 不再需要
❌ 复杂的shader文件             # ggez内置
```

---

## 📝 第一步：创建GgezManager

这是最核心的模块，替代DXManager。

### 基本结构

```rust
// src/graphics/ggez_manager.rs

use ggez::{Context, GameResult};
use ggez::graphics::{self, DrawParam, Image, Text, Color};
use std::collections::HashMap;
use std::path::PathBuf;

/// Ggez图形管理器 - 替代DXManager
pub struct GgezManager {
    /// 纹理缓存 (路径 -> Image)
    textures: HashMap<String, Image>,
    
    /// 默认字体
    default_font: graphics::Font,
    
    /// 屏幕尺寸
    screen_width: f32,
    screen_height: f32,
}

impl GgezManager {
    /// 创建新的图形管理器
    pub fn new(ctx: &mut Context) -> GameResult<Self> {
        let (width, height) = graphics::drawable_size(ctx);
        
        Ok(Self {
            textures: HashMap::new(),
            default_font: graphics::Font::default(),
            screen_width: width,
            screen_height: height,
        })
    }
    
    /// 加载纹理
    pub fn load_texture(&mut self, ctx: &mut Context, path: &str) -> GameResult<&Image> {
        // 如果已经加载，直接返回
        if !self.textures.contains_key(path) {
            let image = Image::from_path(ctx, path)?;
            self.textures.insert(path.to_string(), image);
        }
        
        Ok(self.textures.get(path).unwrap())
    }
    
    /// 从原始像素数据创建纹理
    pub fn create_texture_from_rgba(
        &mut self,
        ctx: &mut Context,
        width: u16,
        height: u16,
        pixels: &[u8],
        key: String,
    ) -> GameResult<&Image> {
        if !self.textures.contains_key(&key) {
            let image = Image::from_rgba8(ctx, width, height, pixels)?;
            self.textures.insert(key.clone(), image);
        }
        
        Ok(self.textures.get(&key).unwrap())
    }
    
    /// 绘制精灵
    pub fn draw_sprite(
        &self,
        ctx: &mut Context,
        image: &Image,
        x: f32,
        y: f32,
        scale: f32,
        rotation: f32,
        color: Color,
    ) -> GameResult<()> {
        let params = DrawParam::default()
            .dest([x, y])
            .scale([scale, scale])
            .rotation(rotation)
            .color(color);
        
        graphics::draw(ctx, image, params)
    }
    
    /// 绘制文本
    pub fn draw_text(
        &self,
        ctx: &mut Context,
        text: &str,
        x: f32,
        y: f32,
        color: Color,
    ) -> GameResult<()> {
        let text_obj = Text::new(text);
        let params = DrawParam::default()
            .dest([x, y])
            .color(color);
        
        graphics::draw(ctx, &text_obj, params)
    }
    
    /// 清空屏幕
    pub fn clear(&self, ctx: &mut Context, color: Color) {
        graphics::clear(ctx, color);
    }
    
    /// 呈现到屏幕
    pub fn present(&self, ctx: &mut Context) -> GameResult<()> {
        graphics::present(ctx)
    }
}
```

---

## 🔄 MainWindow适配ggez

### 方案A: 完全使用ggez (推荐)

```rust
// src/main.rs (主程序入口)

use ggez::{Context, GameResult};
use ggez::event::{self, EventHandler};
use ggez::graphics;

struct GameState {
    scene_manager: SceneManager,
    graphics_manager: GgezManager,
    fps: u32,
    // ... 其他状态
}

impl GameState {
    fn new(ctx: &mut Context) -> GameResult<Self> {
        let mut graphics_manager = GgezManager::new(ctx)?;
        let mut scene_manager = SceneManager::new();
        scene_manager.switch_scene(SceneType::Login)?;
        
        Ok(Self {
            scene_manager,
            graphics_manager,
            fps: 0,
        })
    }
}

impl EventHandler for GameState {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        let delta = ctx.time.delta().as_secs_f32();
        
        // 处理场景切换
        self.scene_manager.process_transitions()?;
        
        // 更新当前场景
        self.scene_manager.update(delta);
        
        Ok(())
    }
    
    fn draw(&mut self, ctx: &mut Context) -> GameResult<()> {
        // 清空屏幕
        self.graphics_manager.clear(ctx, graphics::Color::BLACK);
        
        // 渲染当前场景
        self.scene_manager.draw(ctx, &self.graphics_manager)?;
        
        // 渲染FPS
        if self.show_fps {
            let fps_text = format!("FPS: {}", self.fps);
            self.graphics_manager.draw_text(ctx, &fps_text, 10.0, 10.0, graphics::Color::WHITE)?;
        }
        
        // 呈现到屏幕
        self.graphics_manager.present(ctx)?;
        
        Ok(())
    }
    
    fn mouse_button_down_event(
        &mut self,
        _ctx: &mut Context,
        button: event::MouseButton,
        x: f32,
        y: f32,
    ) -> GameResult<()> {
        let winit_button = match button {
            event::MouseButton::Left => winit::event::MouseButton::Left,
            event::MouseButton::Right => winit::event::MouseButton::Right,
            event::MouseButton::Middle => winit::event::MouseButton::Middle,
            _ => return Ok(()),
        };
        
        self.scene_manager.handle_mouse_button(winit_button, true, x as i32, y as i32);
        Ok(())
    }
    
    fn key_down_event(
        &mut self,
        _ctx: &mut Context,
        input: event::KeyInput,
        _repeated: bool,
    ) -> GameResult<()> {
        if let Some(keycode) = input.keycode {
            // 转换ggez keycode到winit keycode
            // ... 或直接修改Scene trait使用ggez类型
        }
        Ok(())
    }
}

fn main() -> GameResult {
    let cb = ggez::ContextBuilder::new("mir2_client", "Crystal")
        .window_setup(ggez::conf::WindowSetup::default()
            .title("Legend of Mir 2")
            .vsync(true))
        .window_mode(ggez::conf::WindowMode::default()
            .dimensions(1024.0, 768.0)
            .resizable(false));
    
    let (mut ctx, event_loop) = cb.build()?;
    let state = GameState::new(&mut ctx)?;
    
    event::run(ctx, event_loop, state)
}
```

### 方案B: 混合模式 (过渡期)

保留winit窗口管理，只用ggez做渲染：

```rust
// 保留现有的MainWindow + winit
// 在render()中使用GgezManager

impl MainWindow {
    pub fn render(&mut self, ctx: &mut ggez::Context) {
        self.graphics_manager.clear(ctx, Color::BLACK);
        self.scene_manager.draw(ctx, &self.graphics_manager);
        self.graphics_manager.present(ctx);
    }
}
```

---

## ⚠️ 注意事项和挑战

### 1. 事件系统差异

**问题**: ggez有自己的事件循环，与winit不完全兼容

**解决方案**:
- 方案A: 完全使用ggez事件系统（推荐）
- 方案B: 创建转换层（winit ↔ ggez）

### 2. 异步网络

**问题**: ggez的事件循环是同步的，而我们的网络是tokio异步的

**解决方案**:
```rust
// 在update中检查异步消息
impl EventHandler for GameState {
    fn update(&mut self, ctx: &mut Context) -> GameResult<()> {
        // 使用try_recv非阻塞接收网络消息
        while let Ok(event) = self.network_rx.try_recv() {
            self.scene_manager.process_event(&event);
        }
        
        // ... 其他更新
        Ok(())
    }
}
```

### 3. .lib文件支持

**好消息**: 已经实现的MLibrary可以继续使用！

```rust
// MLibrary读取像素数据
let pixels = mlibrary.get_image_pixels(index)?;

// 转换为ggez纹理
let image = graphics_manager.create_texture_from_rgba(
    ctx,
    width,
    height,
    &pixels,
    format!("lib_{}_{}", lib_name, index),
)?;
```

### 4. 性能考虑

**ggez性能**: 对于2D游戏完全足够
- 可以轻松渲染数千个精灵
- 内置批量渲染优化
- 使用wgpu作为后端（所以底层还是GPU加速）

---

## 📈 迁移优先级

### 立即执行 (本次)
1. ✅ 添加ggez依赖 (已完成)
2. 🔄 创建GgezManager基础结构
3. 🔄 创建简单示例程序验证

### 第二阶段
1. 适配LoginScene渲染
2. 适配UI Controls
3. 迁移主程序入口

### 第三阶段
1. 完整的游戏场景渲染
2. 性能优化
3. 移除wgpu依赖

---

## 🎯 预期收益

### 开发效率
- **代码量**: 减少60-70%（渲染相关）
- **开发速度**: 提升3-5倍
- **调试难度**: 大幅降低

### 代码质量
- **可读性**: 大幅提升
- **维护性**: 更容易理解和修改
- **稳定性**: ggez已经在众多项目中验证

### 性能
- **2D性能**: 优于手写wgpu (ggez有优化)
- **内存占用**: 相近或更低
- **启动时间**: 更快（无需复杂初始化）

---

## 🚀 下一步行动

我建议按以下顺序进行：

1. **创建GgezManager模块** - 替代DXManager
2. **创建简单示例** - 验证纹理加载和渲染
3. **迁移LoginScene** - 先从最简单的场景开始
4. **逐步迁移其他模块** - Forms, Controls, 其他Scenes

是否要我现在开始创建GgezManager和示例代码？
