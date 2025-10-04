# 精灵渲染器使用示例

## ✅ 已完成的工作

### 1. WGSL Shader (shaders/sprite.wgsl)
- 顶点着色器：屏幕坐标 → 裁剪空间转换
- 片段着色器：纹理采样 + 颜色调制 + 灰度效果

### 2. SpriteRenderer (src/graphics/sprite_renderer.rs)
- 完整的 2D 精灵渲染管道
- 支持纹理采样、透明混合、灰度效果
- 对应 C# 的 SlimDX.Sprite 功能

### 3. DXManager 集成
- `draw()` 方法：完整实现精灵绘制
- `begin_frame()` 方法：清空屏幕
- `end_frame()` 方法：呈现帧

---

## 📖 使用示例

### 基础用法

```rust
use graphics::{DXManager, MLibrary};

async fn example() {
    // 1. 创建 DXManager
    let window = /* winit::window::Window */;
    let dx_manager = DXManager::new(window).await;
    
    // 2. 加载图像库
    let mut mlibrary = MLibrary::open("Data/Prguse.lib")?;
    
    // 3. 加载纹理到 GPU
    let (image_info, rgba_data) = mlibrary.load_rgba_data(0)?;
    let texture = dx_manager.load_texture(
        "test_texture".to_string(),
        image_info.width as u32,
        image_info.height as u32,
        &rgba_data,
    );
    
    // 4. 渲染循环
    loop {
        // 清空屏幕（黑色）
        if let Some(frame) = dx_manager.begin_frame([0.0, 0.0, 0.0, 1.0]) {
            // 绘制精灵
            dx_manager.draw(
                &texture,
                None,                           // 使用整个纹理
                Some((100.0, 100.0, 0.0)),     // 屏幕位置
                [1.0, 1.0, 1.0, 1.0],          // 白色，完全不透明
            );
            
            // 呈现帧
            dx_manager.end_frame(frame);
        }
        
        // 处理事件...
    }
}
```

### 高级用法 - 部分纹理绘制

```rust
// 只绘制纹理的一部分（源矩形）
dx_manager.draw(
    &texture,
    Some((10, 10, 64, 64)),        // 源矩形：(x, y, width, height)
    Some((200.0, 200.0, 0.0)),     // 目标位置
    [1.0, 1.0, 1.0, 1.0],          // 颜色
);
```

### 高级用法 - 颜色调制和透明度

```rust
// 半透明的红色精灵
dx_manager.draw(
    &texture,
    None,
    Some((300.0, 300.0, 0.0)),
    [1.0, 0.0, 0.0, 0.5],          // 红色，50% 透明度
);
```

### 高级用法 - 灰度效果

```rust
// 开启灰度模式
dx_manager.set_grayscale(true);

dx_manager.draw(
    &texture,
    None,
    Some((400.0, 400.0, 0.0)),
    [1.0, 1.0, 1.0, 1.0],
);

// 关闭灰度模式
dx_manager.set_grayscale(false);
```

### 高级用法 - 全局透明度

```rust
// 设置全局透明度（影响所有后续绘制）
dx_manager.set_opacity(0.5);

dx_manager.draw(
    &texture,
    None,
    Some((500.0, 500.0, 0.0)),
    [1.0, 1.0, 1.0, 1.0],          // 颜色 alpha 会与全局 opacity 相乘
);

// 恢复全局透明度
dx_manager.set_opacity(1.0);
```

---

## 🔄 与 C# 原版对比

### C# (DXManager.cs + SlimDX)

```csharp
// 开始渲染
Device.BeginScene();
Sprite.Begin(SpriteFlags.AlphaBlend);

// 绘制精灵
Sprite.Draw(texture, sourceRect, Vector3.Zero, position, color);
CMain.DPSCounter++;

// 结束渲染
Sprite.End();
Device.EndScene();
Device.Present();
```

### Rust (dx_manager.rs + wgpu)

```rust
// 开始渲染（清空屏幕）
if let Some(frame) = dx_manager.begin_frame([0.0, 0.0, 0.0, 1.0]) {
    // 绘制精灵
    dx_manager.draw(
        &texture,
        source_rect,
        Some((position.x, position.y, 0.0)),
        [color.r, color.g, color.b, color.a],
    );
    // DPS 计数器（TODO）
    
    // 结束渲染
    dx_manager.end_frame(frame);
}
```

---

## ⚠️ 当前限制

### 1. 即时绘制模式
- 每次 `draw()` 调用都创建新的渲染通道
- **性能影响**：适合少量精灵，大量精灵时性能较差
- **改进方向**：实现批处理渲染（下一阶段）

### 2. 无批处理
- C# 的 SlimDX.Sprite 内部有批处理优化
- Rust 版本目前是即时绘制，没有批处理
- **改进方向**：收集多个 draw 调用，一次性渲染

### 3. 简化的 API
- begin_frame() 会清空屏幕
- 如果需要保留上一帧内容，需要修改实现
- **改进方向**：分离 begin_frame 和 clear_screen

---

## 🚀 下一步优化

### 优先级 1: 批处理渲染 🔴

```rust
// 伪代码
pub struct SpriteBatch {
    sprites: Vec<SpriteInstance>,
}

impl SpriteBatch {
    pub fn add(&mut self, texture, position, color) {
        self.sprites.push(SpriteInstance { ... });
    }
    
    pub fn flush(&mut self, dx_manager: &DXManager) {
        // 一次性渲染所有精灵
    }
}

// 使用
let mut batch = SpriteBatch::new();
batch.add(&texture1, ...);
batch.add(&texture2, ...);
batch.add(&texture3, ...);
batch.flush(&dx_manager);  // 只创建一次渲染通道
```

### 优先级 2: 纹理图集支持 🟡
- 将多个小纹理合并到一个大纹理
- 减少纹理切换次数

### 优先级 3: 遮挡剔除 🟢
- 跳过屏幕外的精灵
- 提高大场景渲染性能

---

## ✅ 功能对照表

| 功能 | C# (SlimDX.Sprite) | Rust (SpriteRenderer) | 状态 |
|-----|-------------------|----------------------|------|
| 基础绘制 | ✅ Sprite.Draw() | ✅ draw() | ✅ |
| 纹理采样 | ✅ 自动 | ✅ Linear 过滤 | ✅ |
| Alpha 混合 | ✅ AlphaBlend | ✅ ALPHA_BLENDING | ✅ |
| 源矩形 | ✅ sourceRect | ✅ source_rect | ✅ |
| 颜色调制 | ✅ color 参数 | ✅ color 参数 | ✅ |
| 灰度效果 | ✅ GrayScalePixelShader | ✅ grayscale uniform | ✅ |
| 全局透明度 | ✅ Opacity | ✅ opacity uniform | ✅ |
| 批处理 | ✅ 内部自动 | ❌ 未实现 | 🔴 待实现 |
| 混合模式 | ✅ SetBlend() | 🟡 部分实现 | 🟡 |
| 旋转/缩放 | ⚠️ 需要矩阵 | ❌ 未实现 | 🟢 低优先级 |

---

## 📝 技术细节

### Shader 流程

```
顶点着色器:
  屏幕坐标 (x, y) → 归一化坐标 → 裁剪空间坐标 (-1, 1)
  纹理坐标 (u, v) → 传递给片段着色器

片段着色器:
  采样纹理 → 颜色调制 → 应用透明度 → 灰度转换(可选) → 输出颜色
```

### 坐标系统

- **C# 屏幕坐标**: (0, 0) 左上角，Y 轴向下
- **Rust 屏幕坐标**: (0, 0) 左上角，Y 轴向下（保持一致）
- **wgpu 裁剪空间**: (-1, -1) 左下角，(1, 1) 右上角
- **Shader 转换**: 自动处理坐标系转换

### 纹理格式

- **C# 输入**: BGRA (从 .lib 文件)
- **Rust 输入**: BGRA → 转换为 RGBA
- **GPU 格式**: Rgba8UnormSrgb (标准 sRGB)

---

**状态**: ✅ 渲染管道完整实现，可以正常绘制 2D 精灵！
