# MirGraphics 模块移植 - 阶段 1 完成报告

**日期**: 2025年10月5日  
**目标**: 移植 C# 的 `Client/MirGraphics` 模块到 Rust，移除所有 eframe/egui 依赖

---

## ✅ 已完成的工作

### 1. 依赖清理
- ✅ **完全移除 eframe** - 避免 wgpu 版本冲突（eframe 0.32 使用 wgpu 25）
- ✅ **完全移除 egui** - C# 原版使用自己的 GUI 系统（MirControls），不依赖第三方框架
- ✅ **保留 wgpu 27.0** - 对应 C# 的 DirectX 9
- ✅ **改用 winit + wgpu 直接管理** - 照搬 C# 的 DirectX + Win32 窗口模式

### 2. Graphics 模块结构（已清理）

```
src/graphics/
├── dx_manager.rs (16.78 KB)  ← 对应 DXManager.cs
├── mlibrary.rs (11.06 KB)    ← 对应 MLibrary.cs (保持命名一致)
└── mod.rs (0.51 KB)           ← 模块导出
```

**总计**: 3 个文件，28.35 KB

### 3. DXManager 核心功能

#### 已实现的 API（对应 C# DXManager.cs）

| Rust 方法 | C# 方法 | 状态 | 说明 |
|----------|---------|------|------|
| `DXManager::new()` | `DXManager.Create()` | ✅ | wgpu 设备初始化 |
| `set_opacity()` | `SetOpacity()` | ✅ | 全局透明度 |
| `set_grayscale()` | `SetGrayscale()` | ✅ | 灰度模式 |
| `set_blend()` | `SetBlend()` | ✅ | 混合模式 |
| `load_texture()` | `Texture.FromMemory()` | ✅ | 上传纹理到 GPU |
| `draw()` | `Sprite.Draw()` | 🟡 | 核心绘制（占位实现）|
| `draw_opaque()` | `DrawOpaque()` | 🟡 | 带透明度绘制（占位实现）|
| `begin_frame()` | `BeginScene()` | 🟡 | 开始帧渲染 |
| `end_frame()` | `EndScene() + Present()` | 🟡 | 结束帧渲染 |
| `resize()` | `ResetDevice()` | ✅ | 窗口大小调整 |
| `clean_cache()` | `Clean()` | ✅ | 清理纹理缓存 |

**状态说明**:
- ✅ 完整实现
- 🟡 占位实现（等待渲染管道完成）

#### 数据结构

```rust
pub struct DXManager {
    device: Arc<wgpu::Device>,           // GPU 设备
    queue: Arc<wgpu::Queue>,             // 命令队列
    surface: Option<wgpu::Surface>,      // 渲染表面
    texture_cache: HashMap<String, TextureHandle>,  // 纹理缓存
    opacity: RefCell<f32>,               // 全局透明度
    blending: RefCell<bool>,             // 混合模式
    grayscale: RefCell<bool>,            // 灰度模式
    // ...
}

pub struct TextureHandle {
    texture: wgpu::Texture,              // GPU 纹理
    view: wgpu::TextureView,             // 纹理视图
    width: u32,
    height: u32,
}

pub enum BlendMode {
    Normal,
    InvLight,
}
```

### 4. MLibrary 核心功能

#### 已实现的 API（对应 C# MLibrary.cs）

| Rust 方法 | C# 方法 | 状态 | 说明 |
|----------|---------|------|------|
| `MLibrary::open()` | `MLibrary(path)` | ✅ | 打开 .lib 文件 |
| `count()` | `Count` 属性 | ✅ | 图像数量 |
| `get_image_info()` | `GetImageInfo()` | ✅ | 获取图像元数据 |
| `load_image_data()` | 内部解压逻辑 | ✅ | 解压 BGRA 数据 |
| `load_rgba_data()` | 内部 BGRA→RGBA 转换 | ✅ | 转换为 RGBA 格式 |
| `check_image()` | `CheckImage()` | ✅ | 检查索引有效性 |
| `get_image_bounds_mut()` | `Draw()` 边界计算 | ✅ | 绘制边界计算 |

#### TextureManager（新增的 Rust 抽象）

```rust
pub struct TextureManager {
    libraries: HashMap<String, MLibrary>,           // 库集合
    textures: HashMap<TextureKey, Arc<TextureHandle>>,  // 纹理缓存
}

impl TextureManager {
    pub fn load_library(&mut self, name: &str, path: &Path) -> io::Result<()>
    pub fn get_texture(&mut self, dx: &DXManager, library: &str, index: usize) 
        -> io::Result<(ImageInfo, Arc<TextureHandle>)>
    pub fn get_image_info(&mut self, library: &str, index: usize) 
        -> io::Result<ImageInfo>
    pub fn clear_cache(&mut self)
}
```

**C# 对比**: C# 中纹理管理分散在 `DXManager.TextureList` 和 `MLibrary` 之间，Rust 统一到 `TextureManager`。

### 5. 文件格式支持

✅ **完整支持 MIR2 .lib 格式**:
- 文件头解析（version, count, frame_seek）
- 索引表读取（offsets）
- 图像元数据（width, height, x, y, shadow 等）
- GZip 压缩数据解压
- BGRA → RGBA 颜色格式转换

### 6. 主程序重构

#### main.rs 变化

**移除**:
```rust
// ❌ 不再使用 eframe
eframe::run_native("mir2_client", native_options, ...);
```

**改为**:
```rust
// ✅ 使用 winit + wgpu 27 直接管理
let event_loop = EventLoop::new()?;
let window = event_loop.create_window(...)?;
let dx_manager = pollster::block_on(DXManager::new(window_arc.clone()));

event_loop.run(move |event, elwt| {
    match event {
        Event::WindowEvent { event: WindowEvent::RedrawRequested, .. } => {
            // TODO: 渲染逻辑
        }
        // ...
    }
})?;
```

#### 暂时注释的模块

为了专注于 MirGraphics 移植，以下模块暂时注释：

```rust
// mod app;         // 依赖 eframe
// mod forms;       // 依赖 egui
// mod controls;    // 依赖 egui
// mod objects;     // 依赖其他模块
// mod scenes;      // 依赖 egui
// mod sounds;      // rodio API 变化，待修复
// mod resolution;  // 待验证
```

保留模块：
```rust
mod graphics;    // ✅ 当前移植目标
mod network;     // ✅ 不依赖 GUI
mod utils;       // ✅ 工具函数
```

---

## 🔴 待完成的工作

### 阶段 2: 实现渲染管道

#### 2.1 精灵渲染器（对应 C# Sprite.Draw()）

**C# 实现**:
```csharp
Sprite.Draw(texture, sourceRect, Vector3.Zero, position, color);
```

**Rust 需要实现**:
```rust
// 需要创建：
// 1. 顶点缓冲区（sprite vertices）
// 2. 渲染管道（render pipeline）
// 3. 纹理绑定组（bind group）
// 4. 批处理优化（batch rendering）

impl DXManager {
    pub fn draw(&self, texture: &TextureHandle, ...) {
        // 1. 创建命令编码器
        // 2. 开始渲染通道
        // 3. 绑定管道和纹理
        // 4. 绘制四边形（2 triangles）
        // 5. 提交命令
    }
}
```

#### 2.2 Shader 实现

**需要的 WGSL Shaders**:

1. **sprite.wgsl** - 基础精灵渲染
```wgsl
// 顶点着色器
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    // ...
}

// 片段着色器
@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // 采样纹理，应用颜色和透明度
}
```

2. **grayscale.wgsl** - 灰度效果
3. **blend.wgsl** - 混合效果

#### 2.3 性能优化

- [ ] **批处理渲染** - 合并相同纹理的多次 draw call
- [ ] **纹理图集** - 合并小纹理到大纹理
- [ ] **遮挡剔除** - 跳过屏幕外的对象
- [ ] **帧缓冲优化** - 双缓冲/三缓冲

### 阶段 3: 测试与验证

#### 3.1 单元测试
- [ ] MLibrary 文件读取测试
- [ ] 纹理上传测试
- [ ] 坐标转换测试

#### 3.2 集成测试
- [ ] 加载真实 .lib 文件
- [ ] 渲染单个精灵
- [ ] 渲染多个精灵
- [ ] 测试透明度/混合效果

#### 3.3 性能测试
- [ ] 帧率测试（目标 60 FPS）
- [ ] 内存使用测试
- [ ] Draw call 数量统计

---

## 📋 C# 原版对照

### DXManager.cs (591 lines)

**核心职责**:
1. DirectX 9 设备初始化
2. Sprite 渲染器管理
3. 纹理缓存管理
4. 灰度/混合效果控制
5. 光照纹理生成

**Rust 等效**:
- DirectX 9 → wgpu 27.0
- SlimDX.Sprite → 自实现精灵渲染器（待完成）
- PixelShader → WGSL shaders（待完成）

### MLibrary.cs (1087 lines)

**核心职责**:
1. .lib 文件格式解析 ✅
2. GZip 解压 ✅
3. BGRA 颜色格式处理 ✅
4. 纹理缓存管理 ✅
5. 绘制辅助方法（部分完成）

**Rust 实现**:
- 完全复刻文件格式解析
- 使用 flate2 替代 C# GZipStream
- 添加 TextureManager 抽象层

---

## 🎯 设计原则总结

### ✅ 遵循的原则

1. **照搬原版，不过早抽象** - 删除了 ~34 KB 不存在的模块（sprite_pipeline, character_renderer 等）
2. **直接使用 wgpu，不依赖框架** - 移除 eframe/egui，自己管理渲染
3. **保持 C# 结构对应** - 文件组织和命名与 C# 一致
4. **API 设计相似** - 方法名和参数尽量接近 C# 原版

### 🔄 合理的改进

1. **TextureManager 抽象** - 统一管理多个 MLibrary 实例
2. **Arc<TextureHandle>** - 使用 Rust 的引用计数，避免纹理复制
3. **错误处理** - 使用 Result<T, io::Error> 替代 C# 异常

---

## 🚀 下一步行动

### 立即任务（高优先级）

1. **实现 sprite 渲染管道** 🔴
   ```rust
   // 创建 src/graphics/sprite_renderer.rs
   // 实现简单的四边形渲染
   ```

2. **编写基础 WGSL shaders** 🔴
   ```
   创建 shaders/ 目录
   - sprite.wgsl (基础渲染)
   - grayscale.wgsl (灰度效果)
   ```

3. **实现 DXManager::draw() 完整逻辑** 🔴

### 中期任务

4. **添加批处理支持** 🟡
5. **实现光照系统** 🟡 (对应 C# LoadTextures() 中的 Lights)
6. **性能优化和测试** 🟡

### 长期任务

7. **重构其他模块**（sounds, scenes, controls 等）🟢
8. **完整功能测试** 🟢

---

## 📊 编译状态

```bash
$ cargo check
   Compiling mir2_client v0.1.0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.84s
```

**状态**: ✅ 编译成功，无错误

**警告**: 9 个 warnings（主要是未使用的变量和导入）

---

## 💡 经验教训

1. **依赖冲突处理** - eframe 0.32 使用 wgpu 25，与 wgpu 27 冲突。解决方案：移除 eframe
2. **过早抽象的危害** - 之前创建的 sprite_pipeline 等模块在 C# 原版中不存在，浪费了时间
3. **照搬原版的重要性** - "不要改进，只要移植" - 这个原则避免了很多不必要的设计决策
4. **模块化注释** - 暂时注释不相关的模块，专注于当前目标，提高效率

---

**总结**: MirGraphics 模块的基础架构已经完成，数据加载和管理功能齐全。下一步重点是实现渲染管道，让游戏能够真正显示图像。

**进度**: 📊 MirGraphics 移植进度 ~60% (基础架构完成，渲染管道待实现)
