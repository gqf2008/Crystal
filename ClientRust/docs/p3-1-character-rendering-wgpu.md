# P3-1: 角色外观渲染 (wgpu) 实现报告

**实现日期**: 2025-10-04  
**状态**: 🔄 核心组件完成, 集成待完成  
**实现者**: GitHub Copilot  
**编译时间**: 7.68 秒  
**警告数**: 447 (非致命)

---

## 📋 任务概述

实现基于 wgpu 的自定义渲染管线，用于渲染 MIR2 角色精灵：
- 创建 wgpu 2D 精灵渲染器
- 加载 ChrSel.lib 角色资源
- 实现角色外观渲染
- 集成到 SelectScene

---

## 🏗️ 架构设计

### 组件层次

```
SelectScene (UI场景)
    ↓
CharacterRenderer (角色渲染器)
    ├─ MLibrary (ChrSel.lib 资源库)
    └─ SpriteRenderer (wgpu 2D 精灵渲染器)
        ├─ wgpu RenderPipeline
        ├─ Vertex/Index Buffers
        └─ WGSL Shader
```

---

## 🎨 已实现组件

### 1. SpriteRenderer - wgpu 2D 精灵渲染器

**文件**: `src/graphics/sprite_renderer.rs` (273 行)

**核心功能**:
- wgpu 渲染管线创建
- 顶点缓冲和索引缓冲管理
- 纹理绑定组创建
- 批量渲染支持

**数据结构**:

#### SpriteVertex (顶点数据)
```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteVertex {
    pub position: [f32; 2],      // 屏幕坐标 (NDC: -1 to 1)
    pub tex_coords: [f32; 2],    // 纹理坐标 (0 to 1)
}
```

**顶点布局**:
```
     (-0.5, 0.5) ──────── (0.5, 0.5)
          │                   │
          │   单位正方形       │
          │                   │
     (-0.5, -0.5) ──────── (0.5, -0.5)
```

#### SpriteInstance (实例数据)
```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteInstance {
    pub position: [f32; 2],   // 世界坐标 (像素)
    pub size: [f32; 2],        // 精灵尺寸 (像素)
    pub uv_offset: [f32; 2],   // UV偏移
    pub uv_scale: [f32; 2],    // UV缩放
    pub color: [f32; 4],       // 颜色调制 (RGBA)
}
```

**关键方法**:

```rust
impl SpriteRenderer {
    /// 创建渲染器
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self
    
    /// 创建纹理绑定组
    pub fn create_texture_bind_group(
        &self,
        device: &wgpu::Device,
        texture_view: &wgpu::TextureView,
    ) -> wgpu::BindGroup
    
    /// 渲染精灵批次
    pub fn render<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        bind_group: &'a wgpu::BindGroup,
        instance_buffer: &'a wgpu::Buffer,
        instance_count: u32,
    )
}
```

**渲染管线配置**:
- **拓扑**: TriangleList (三角形列表)
- **混合模式**: ALPHA_BLENDING (支持透明度)
- **剔除**: None (2D 不需要背面剔除)
- **深度测试**: 无 (2D 渲染)
- **采样器**: Linear 过滤 (线性插值)

---

### 2. WGSL Shader - GPU 着色器

**文件**: `src/graphics/shaders/sprite.wgsl` (67 行)

**顶点着色器**:
```wgsl
@vertex
fn vs_main(
    vertex: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    // 将单位正方形缩放到精灵尺寸
    let scaled_pos = vertex.position * instance.instance_size;
    
    // 平移到世界坐标
    let world_pos = scaled_pos + instance.instance_position;
    
    // 输出裁剪空间坐标
    out.clip_position = vec4<f32>(world_pos, 0.0, 1.0);
    
    // 应用UV变换
    out.tex_coords = vertex.tex_coords * instance.uv_scale + instance.uv_offset;
    
    return out;
}
```

**片段着色器**:
```wgsl
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // 采样纹理
    let tex_color = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    
    // 颜色调制
    let final_color = tex_color * in.color;
    
    return final_color;
}
```

**特性**:
- ✅ 支持纹理采样
- ✅ 支持颜色调制
- ✅ 支持Alpha透明度
- ✅ UV坐标变换
- ⚠️ **TODO**: 投影矩阵 (当前假设输入已是NDC坐标)

---

### 3. CharacterRenderer - 角色渲染器

**文件**: `src/graphics/character_renderer.rs` (123 行)

**核心功能**:
- 加载 ChrSel.lib 资源库
- 计算角色精灵索引
- 加载角色纹理数据
- 与 egui 集成

**数据结构**:

#### CharacterAppearance (角色外观)
```rust
#[derive(Debug, Clone)]
pub struct CharacterAppearance {
    pub class: mir2_shared::enums::MirClass,
    pub gender: mir2_shared::enums::MirGender,
    pub frame_index: usize,  // 当前动画帧
}
```

**精灵索引计算**:

ChrSel.lib 布局 (基于 C# SelectScene.cs):
```
索引 = base_index + gender_offset + frame

职业基础索引:
- Warrior:   0
- Wizard:   20
- Taoist:   40
- Assassin: 60
- Archer:   80

性别偏移:
- Male:    0-9  (10帧)
- Female: 10-19 (10帧)
```

**示例**:
```
女性战士第3帧: 0 + 10 + 3 = 索引 13
男性法师第5帧: 20 + 0 + 5 = 索引 25
```

**关键方法**:

```rust
impl CharacterRenderer {
    /// 创建渲染器
    pub fn new() -> Self
    
    /// 加载 ChrSel.lib
    pub fn load_chrsel_library<P: AsRef<Path>>(&mut self, path: P) -> io::Result<()>
    
    /// 获取精灵索引
    pub fn get_character_sprite_index(
        &self,
        class: MirClass,
        gender: MirGender,
        frame: usize,
    ) -> usize
    
    /// 加载精灵数据 (RGBA 字节)
    pub fn load_character_sprite_data(
        &mut self,
        class: MirClass,
        gender: MirGender,
        frame: usize,
    ) -> io::Result<(ImageInfo, Vec<u8>)>
    
    /// 加载为 egui ColorImage
    pub fn load_character_color_image(
        &mut self,
        class: MirClass,
        gender: MirGender,
        frame: usize,
    ) -> io::Result<(ImageInfo, egui::ColorImage)>
}
```

---

## 📦 依赖更新

### Cargo.toml

新增依赖:
```toml
bytemuck = { version = "1.14", features = ["derive"] }  # wgpu 顶点数据
```

现有依赖 (已满足):
```toml
wgpu = "27.0.1"          # GPU 渲染
egui-wgpu = "0.29"       # egui wgpu 集成
```

---

## 🔧 技术细节

### 1. bytemuck - 零拷贝类型转换

```rust
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteVertex {
    pub position: [f32; 2],
    pub tex_coords: [f32; 2],
}

// 直接转换为字节数组，无需序列化
let bytes = bytemuck::cast_slice(&vertices);
```

**优势**:
- 零拷贝转换
- 编译时类型安全
- wgpu 缓冲创建的标准方式

### 2. wgpu 缓冲创建

```rust
use wgpu::util::DeviceExt;

let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
    label: Some("Sprite Vertex Buffer"),
    contents: bytemuck::cast_slice(vertices),
    usage: wgpu::BufferUsages::VERTEX,
});
```

### 3. 纹理上传

```rust
queue.write_texture(
    wgpu::TexelCopyTextureInfo {
        texture: &texture,
        mip_level: 0,
        origin: wgpu::Origin3d::ZERO,
        aspect: wgpu::TextureAspect::All,
    },
    &image_data,  // RGBA 字节数组
    wgpu::TexelCopyBufferLayout {
        offset: 0,
        bytes_per_row: Some(4 * width as u32),
        rows_per_image: Some(height as u32),
    },
    texture_size,
);
```

### 4. Alpha 混合配置

```rust
wgpu::ColorTargetState {
    format: surface_format,
    blend: Some(wgpu::BlendState::ALPHA_BLENDING),  // 支持透明度
    write_mask: wgpu::ColorWrites::ALL,
}
```

**混合公式**:
```
final_color = src_color * src_alpha + dst_color * (1 - src_alpha)
```

---

## 🎯 使用示例

### 基础用法

```rust
// 1. 创建角色渲染器
let mut character_renderer = CharacterRenderer::new();

// 2. 加载资源库
character_renderer.load_chrsel_library("Data/ChrSel.lib")?;

// 3. 加载角色精灵为 egui 图像
let (image_info, color_image) = character_renderer.load_character_color_image(
    MirClass::Warrior,
    MirGender::Male,
    0  // 第一帧
)?;

// 4. 在 egui 中显示
let texture_handle = ctx.load_texture(
    "character_preview",
    color_image,
    egui::TextureOptions::default(),
);

ui.image(&texture_handle);
```

### 动画循环

```rust
struct CharacterPreview {
    frame: usize,
    last_update: Instant,
}

impl CharacterPreview {
    fn update(&mut self, delta_time: f32) {
        const FRAME_DURATION: f32 = 0.1;  // 每帧100ms
        
        if self.last_update.elapsed().as_secs_f32() >= FRAME_DURATION {
            self.frame = (self.frame + 1) % 10;  // 循环10帧
            self.last_update = Instant::now();
        }
    }
}
```

---

## ⚠️ 当前限制

### 1. 生命周期问题

**问题**: 无法在 `render_pass` 中使用临时创建的缓冲区和绑定组

```rust
// ❌ 错误: bind_group 生命周期不够长
pub fn render(&self, render_pass: &mut wgpu::RenderPass) {
    let bind_group = self.create_bind_group();
    render_pass.set_bind_group(0, &bind_group, &[]);
    // bind_group 在这里被释放，但 render_pass 还需要它
}
```

**解决方案**: 
- 方案 A: 预先创建并缓存绑定组
- 方案 B: 使用 egui 集成 (当前方案)

### 2. 投影矩阵未实现

着色器中的 TODO:
```wgsl
// TODO: 使用投影矩阵将世界坐标转换为NDC
// ndc_x = (world_x / screen_width) * 2.0 - 1.0
// ndc_y = 1.0 - (world_y / screen_height) * 2.0
```

**影响**: 需要手动转换坐标到 NDC 范围 (-1 to 1)

### 3. 批量渲染未优化

当前实现: 每个精灵一个draw call
优化方案: 实例化渲染 (已支持，但未充分利用)

---

## 🔄 集成方案

### 方案 A: egui Image (推荐，简单)

**优势**:
- ✅ 简单易用
- ✅ 与现有 UI 无缝集成
- ✅ 无生命周期问题

**实现**:
```rust
// 在 SelectScene 中
pub struct SelectScene {
    character_renderer: CharacterRenderer,
    character_preview_textures: HashMap<usize, egui::TextureHandle>,
}

// 渲染角色预览
fn render_character_preview(&mut self, ui: &mut egui::Ui, character: &SelectCharacter) {
    if !self.character_preview_textures.contains_key(&(character.index as usize)) {
        // 加载角色精灵
        let (_, color_image) = self.character_renderer
            .load_character_color_image(character.class, character.gender, 0)
            .unwrap();
        
        // 创建 egui 纹理
        let texture = ui.ctx().load_texture(
            format!("char_{}", character.index),
            color_image,
            egui::TextureOptions::default(),
        );
        
        self.character_preview_textures.insert(character.index as usize, texture);
    }
    
    // 显示图像
    if let Some(texture) = self.character_preview_textures.get(&(character.index as usize)) {
        ui.image(texture);
    }
}
```

**缺点**:
- ⚠️ 每帧上传纹理到 GPU (动画时)
- ⚠️ 无法使用自定义着色器效果

---

### 方案 B: egui-wgpu 自定义渲染 (高级)

**优势**:
- ✅ 完全控制渲染
- ✅ 高性能批量渲染
- ✅ 支持自定义着色器

**实现**:
```rust
impl eframe::App for MirClientApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // egui UI
        egui::CentralPanel::default().show(ctx, |ui| {
            // ...
        });
        
        // 自定义 wgpu 渲染
        frame.wgpu_render_state(|rs| {
            let device = &rs.device;
            let queue = &rs.queue;
            
            // 创建渲染pass
            let mut encoder = device.create_command_encoder(&Default::default());
            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    // ...
                });
                
                // 渲染角色精灵
                self.sprite_renderer.render(&mut render_pass, ...);
            }
            
            queue.submit([encoder.finish()]);
        });
    }
}
```

**缺点**:
- ⚠️ 复杂度高
- ⚠️ 需要手动管理 渲染pass
- ⚠️ egui 和 自定义渲染的层级管理

---

## 📊 性能分析

### 理论性能

**单个精灵渲染**:
- 顶点数: 4
- 索引数: 6 (2个三角形)
- GPU Draw Call: 1
- 纹理上传: 仅首次

**批量渲染 (实例化)**:
- 精灵数: N
- 顶点数: 4 (共享)
- 索引数: 6 (共享)
- GPU Draw Call: 1
- 性能提升: N倍

### 实际测试 (预期)

| 场景 | 精灵数 | FPS | GPU 占用 |
|------|--------|-----|----------|
| SelectScene 角色预览 | 4 | 60 | < 5% |
| 角色动画 (10帧) | 4 | 60 | < 10% |
| 游戏场景 (50个单位) | 50 | 60 | < 20% |

---

## 🐛 已知问题

### 1. 编译警告

```
warning: unused import: `SpriteInstance`
warning: method `render` is never used
```

**原因**: SpriteRenderer 的完整功能尚未集成到 SelectScene

**影响**: 无，仅警告

### 2. 坐标系统

**问题**: WGSL 着色器假设输入坐标已是 NDC (-1 to 1)

**临时方案**: 使用 egui 集成，让 egui 处理坐标转换

**完整方案**: 实现投影矩阵 uniform buffer

---

## 🔄 后续优化

### P3-1-B: SelectScene 集成 (剩余工作)

- [ ] 在 SelectScene 中添加 CharacterRenderer
- [ ] 加载 ChrSel.lib
- [ ] 为每个角色显示预览图像
- [ ] 实现待机动画循环
- [ ] 添加鼠标悬停高亮效果

**预计工作量**: 100-150 行代码

---

### P3-1-C: 投影矩阵系统

- [ ] 添加 uniform buffer
- [ ] 传递屏幕尺寸
- [ ] 更新着色器使用投影矩阵
- [ ] 支持相机变换

**预计工作量**: 150-200 行代码

---

### P3-1-D: 性能优化

- [ ] 实例化批量渲染
- [ ] 纹理图集 (Texture Atlas)
- [ ] LOD (细节层次) 系统
- [ ] 视锥剔除

**预计工作量**: 200-300 行代码

---

## 💡 设计亮点

### 1. bytemuck 零拷贝

```rust
#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteVertex { ... }

// 直接转换，无运行时开销
let bytes = bytemuck::cast_slice(&vertices);
```

### 2. 实例化渲染架构

```rust
pub struct SpriteInstance {
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub uv_offset: [f32; 2],
    pub uv_scale: [f32; 2],
    pub color: [f32; 4],
}

// 一次 draw call 渲染多个精灵
render_pass.draw_indexed(0..6, 0, 0..instance_count);
```

### 3. 职业/性别索引算法

```rust
// 简洁的索引计算
let sprite_index = match class {
    Warrior => 0,
    Wizard => 20,
    Taoist => 40,
    Assassin => 60,
    Archer => 80,
} + match gender {
    Male => 0,
    Female => 10,
} + (frame % 10);
```

### 4. 模块化设计

```
SpriteRenderer   (通用2D精灵渲染)
    ↑
CharacterRenderer (角色特化)
    ↑
SelectScene      (UI集成)
```

每层独立，可单独测试和复用

---

## 🎓 学习要点

### wgpu 核心概念

1. **Device**: GPU设备抽象
2. **Queue**: 命令队列
3. **Buffer**: GPU缓冲区 (顶点/索引/Uniform)
4. **Texture**: GPU纹理
5. **BindGroup**: 着色器资源绑定
6. **RenderPipeline**: 渲染管线状态

### WGSL 着色器语言

- 类似 GLSL 但更现代
- 强类型系统
- `@vertex` / `@fragment` 入口点
- `@location` 绑定输入/输出

### bytemuck 类型转换

- `Pod`: Plain Old Data (可安全转换为字节)
- `Zeroable`: 可安全初始化为全0

---

## 📝 代码统计

| 文件 | 行数 | 功能 |
|------|------|------|
| `sprite_renderer.rs` | 273 | wgpu 精灵渲染器 |
| `sprite.wgsl` | 67 | WGSL 着色器 |
| `character_renderer.rs` | 123 | 角色渲染器 |
| **总计** | **463** | **核心渲染系统** |

---

## 🎯 下一步

### 立即可做:
1. **SelectScene 集成** - 显示角色预览 (方案A: egui Image)
2. **动画系统** - 实现待机动画循环
3. **测试验证** - 加载 ChrSel.lib 测试

### 未来优化:
4. **投影矩阵** - 实现正确的坐标变换
5. **批量渲染** - 充分利用实例化渲染
6. **纹理图集** - 优化纹理切换

---

**报告结束**

**当前状态**: ✅ 核心组件完成并编译通过  
**下一步**: SelectScene 集成 (100-150 行代码)  
**完成度**: P3-1 约60% (核心渲染完成,UI集成待完成)
