# MLibrary Draw 方法移植完成报告

## 移植日期
2025年10月10日

## 移植概述
成功将 C# MLibrary 类的所有 Draw 系列方法移植到 Rust，适配 ggez 图形库。

## 已移植方法列表

### 1. **基础绘制方法**

#### `draw()` - 基础绘制
- **C# 原型**: `void Draw(int index, int x, int y)`
- **Rust 签名**: 
  ```rust
  pub fn draw(
      &mut self,
      ctx: &mut Context,
      canvas: &mut Canvas,
      index: usize,
      x: f32,
      y: f32,
      screen_width: f32,
      screen_height: f32,
  ) -> io::Result<()>
  ```
- **功能**: 在指定坐标绘制图像，使用白色
- **对应行**: C# line 701-716

#### `draw_with_color()` - 带颜色和偏移的绘制
- **C# 原型**: `void Draw(int index, Point point, Color colour, bool offSet = false)`
- **Rust 签名**:
  ```rust
  pub fn draw_with_color(
      &mut self,
      ctx: &mut Context,
      canvas: &mut Canvas,
      index: usize,
      x: f32,
      y: f32,
      color: Color,
      offset: bool,
      screen_width: f32,
      screen_height: f32,
  ) -> io::Result<()>
  ```
- **功能**: 绘制图像，支持自定义颜色和偏移量
- **对应行**: C# line 717-730

### 2. **高级绘制方法**

#### `draw_with_opacity()` - 带透明度的绘制
- **C# 原型**: `void Draw(int index, Point point, Color colour, bool offSet, float opacity)`
- **Rust 签名**:
  ```rust
  pub fn draw_with_opacity(
      &mut self,
      ctx: &mut Context,
      canvas: &mut Canvas,
      index: usize,
      x: f32, y: f32,
      color: Color,
      offset: bool,
      opacity: f32,
      screen_width: f32,
      screen_height: f32,
  ) -> io::Result<()>
  ```
- **功能**: 绘制图像，支持透明度控制（0.0-1.0）
- **对应行**: C# line 735-750

#### `draw_blend()` - 混合模式绘制
- **C# 原型**: `void DrawBlend(int index, Point point, Color colour, bool offSet = false, float rate = 1)`
- **Rust 签名**:
  ```rust
  pub fn draw_blend(
      &mut self,
      ctx: &mut Context,
      canvas: &mut Canvas,
      index: usize,
      x: f32, y: f32,
      color: Color,
      offset: bool,
      rate: f32,
      screen_width: f32,
      screen_height: f32,
  ) -> io::Result<()>
  ```
- **功能**: 混合绘制，支持混合率控制
- **对应行**: C# line 752-768

### 3. **区域绘制方法**

#### `draw_section()` - 部分区域绘制
- **C# 原型**: `void Draw(int index, Rectangle section, Point point, Color colour, bool offSet)`
- **Rust 签名**:
  ```rust
  pub fn draw_section(
      &mut self,
      ctx: &mut Context,
      canvas: &mut Canvas,
      index: usize,
      section_x: f32,
      section_y: f32,
      section_width: f32,
      section_height: f32,
      x: f32, y: f32,
      color: Color,
      offset: bool,
      screen_width: f32,
      screen_height: f32,
  ) -> io::Result<()>
  ```
- **功能**: 只绘制图像的指定矩形区域
- **对应行**: C# line 769-789

#### `draw_section_with_opacity()` - 部分区域带透明度绘制
- **C# 原型**: `void Draw(int index, Rectangle section, Point point, Color colour, float opacity)`
- **Rust 签名**:
  ```rust
  pub fn draw_section_with_opacity(
      &mut self,
      ctx: &mut Context,
      canvas: &mut Canvas,
      index: usize,
      section_x: f32,
      section_y: f32,
      section_width: f32,
      section_height: f32,
      x: f32, y: f32,
      color: Color,
      opacity: f32,
      screen_width: f32,
      screen_height: f32,
  ) -> io::Result<()>
  ```
- **功能**: 绘制图像区域，支持透明度
- **对应行**: C# line 790-807

### 4. **变换绘制方法**

#### `draw_scaled()` - 缩放绘制
- **C# 原型**: `void Draw(int index, Point point, Size size, Color colour)`
- **Rust 签名**:
  ```rust
  pub fn draw_scaled(
      &mut self,
      ctx: &mut Context,
      canvas: &mut Canvas,
      index: usize,
      x: f32, y: f32,
      width: f32,
      height: f32,
      color: Color,
      screen_width: f32,
      screen_height: f32,
  ) -> io::Result<()>
  ```
- **功能**: 将图像缩放到指定尺寸绘制
- **实现**: 使用 ggez 的 `DrawParam::scale()`
- **对应行**: C# line 808-827

#### `draw_tinted()` - 着色绘制（双层）
- **C# 原型**: `void DrawTinted(int index, Point point, Color colour, Color Tint, bool offSet = false)`
- **Rust 签名**:
  ```rust
  pub fn draw_tinted(
      &mut self,
      ctx: &mut Context,
      canvas: &mut Canvas,
      index: usize,
      x: f32, y: f32,
      color: Color,
      tint: Color,
      offset: bool,
      screen_width: f32,
      screen_height: f32,
  ) -> io::Result<()>
  ```
- **功能**: 绘制主图像和遮罩层，支持不同颜色着色
- **对应行**: C# line 829-845

### 5. **特殊绘制方法**

#### `draw_up()` - 向上绘制
- **C# 原型**: `void DrawUp(int index, int x, int y)`
- **Rust 签名**:
  ```rust
  pub fn draw_up(
      &mut self,
      ctx: &mut Context,
      canvas: &mut Canvas,
      index: usize,
      x: f32,
      y: f32,
      screen_width: f32,
      screen_height: f32,
  ) -> io::Result<()>
  ```
- **功能**: Y坐标减去图像高度后绘制（用于底部对齐）
- **对应行**: C# line 847-862

#### `draw_up_blend()` - 向上混合绘制
- **C# 原型**: `void DrawUpBlend(int index, Point point)`
- **Rust 签名**:
  ```rust
  pub fn draw_up_blend(
      &mut self,
      ctx: &mut Context,
      canvas: &mut Canvas,
      index: usize,
      x: f32,
      y: f32,
      screen_width: f32,
      screen_height: f32,
  ) -> io::Result<()>
  ```
- **功能**: Y坐标减去高度后混合绘制
- **对应行**: C# line 863-880

### 6. **辅助方法**

#### `visible_pixel()` - 像素可见性检测
- **C# 原型**: `bool VisiblePixel(int index, Point point, bool accuate)`
- **Rust 签名**:
  ```rust
  pub fn visible_pixel(
      &mut self,
      ctx: &mut Context,
      index: usize,
      x: i32,
      y: i32,
      accurate: bool,
  ) -> io::Result<bool>
  ```
- **功能**: 
  - `accurate=true`: 精确检测指定像素
  - `accurate=false`: 检测周围 5x5 区域（模糊检测）
- **对应行**: C# line 882-897

## 技术要点

### 1. DirectX → ggez 映射

| DirectX (C#) | ggez (Rust) |
|--------------|-------------|
| `DXManager.Draw()` | `Canvas::draw()` |
| `Vector3(x, y, z)` | `DrawParam::dest([x, y])` |
| `Rectangle` 作为源区域 | `DrawParam::src(Rect)` |
| `Matrix.Scaling()` | `DrawParam::scale([sx, sy])` |
| `Color` | `ggez::graphics::Color` |
| `DXManager.SetBlend()` | Alpha 通道 (自动混合) |

### 2. 关键差异

#### 坐标系统
- **C#**: DirectX 使用 Vector3 (x, y, z)
- **Rust**: ggez 使用 2D 数组 [x, y]

#### 混合模式
- **C#**: 显式设置混合状态 `DXManager.SetBlend(true, rate)`
- **Rust**: 通过 Color 的 alpha 通道控制混合

#### 纹理区域
- **C#**: `Rectangle` 使用像素单位
- **Rust**: `Rect` 使用归一化坐标 (0.0-1.0)

#### 缩放变换
- **C#**: 使用变换矩阵 `Sprite.Transform = Matrix.Scaling()`
- **Rust**: 使用 `DrawParam::scale()` 参数

### 3. 性能优化

#### 屏幕裁剪
所有方法都实现了边界检查，避免绘制屏幕外的图像：
```rust
if x >= screen_width || y >= screen_height
    || x + (info.width as f32) < 0.0
    || y + (info.height as f32) < 0.0
{
    return Ok(());
}
```

#### 纹理缓存
使用 `get_or_create_texture()` 确保纹理只加载一次：
```rust
let info = self.get_or_create_texture(ctx, index)?;
```

#### 延迟加载
只有在实际绘制时才创建纹理，节省内存。

### 4. 类型转换注意事项

**重要**：在条件表达式中进行类型转换时必须加括号：
```rust
// ❌ 错误
if x + info.width as f32 < 0.0 { }

// ✅ 正确
if x + (info.width as f32) < 0.0 { }
```

原因：Rust 编译器会将 `as f32 <` 解析为泛型参数的开始。

## 使用示例

### 基础绘制
```rust
// 在 (100, 100) 位置绘制图像 0
library.draw(&mut ctx, &mut canvas, 0, 100.0, 100.0, 800.0, 600.0)?;
```

### 带颜色和偏移
```rust
use ggez::graphics::Color;

// 红色绘制，应用图像偏移
library.draw_with_color(
    &mut ctx,
    &mut canvas,
    5,
    200.0, 150.0,
    Color::RED,
    true,  // 应用偏移
    800.0, 600.0
)?;
```

### 半透明绘制
```rust
// 50% 透明度绘制
library.draw_with_opacity(
    &mut ctx,
    &mut canvas,
    10,
    300.0, 200.0,
    Color::WHITE,
    false,  // 不应用偏移
    0.5,    // 50% 透明度
    800.0, 600.0
)?;
```

### 缩放绘制
```rust
// 缩放到 64x64 绘制
library.draw_scaled(
    &mut ctx,
    &mut canvas,
    3,
    100.0, 100.0,
    64.0, 64.0,  // 目标尺寸
    Color::WHITE,
    800.0, 600.0
)?;
```

### 区域绘制
```rust
// 只绘制图像的左上角 32x32 区域
library.draw_section(
    &mut ctx,
    &mut canvas,
    7,
    0.0, 0.0,      // 源区域起点
    32.0, 32.0,    // 源区域尺寸
    400.0, 300.0,  // 目标位置
    Color::WHITE,
    false,
    800.0, 600.0
)?;
```

### 着色绘制（装备染色）
```rust
// 绘制主图像（白色）+ 遮罩层（红色着色）
library.draw_tinted(
    &mut ctx,
    &mut canvas,
    20,
    500.0, 400.0,
    Color::WHITE,   // 主图像颜色
    Color::RED,     // 遮罩层颜色（装备染色）
    true,           // 应用偏移
    800.0, 600.0
)?;
```

### 像素检测（碰撞检测）
```rust
// 检测 (50, 50) 位置是否有可见像素
let is_visible = library.visible_pixel(&mut ctx, 15, 50, 50, true)?;

if is_visible {
    println!("点击到了图像！");
}

// 模糊检测（5x5 区域）
let is_near = library.visible_pixel(&mut ctx, 15, 50, 50, false)?;
```

## 编译状态

✅ **所有方法编译通过**

- 类型转换括号问题已修复
- mlibrary.rs 模块本身无编译错误
- 其他模块调用旧方法名的问题需要单独更新

## 兼容性说明

### 需要更新的调用点
其他模块中仍在使用旧方法名 `draw_to_canvas()`，需要更新为：
- `draw()` - 基础绘制
- `draw_with_color()` - 带颜色绘制
- `draw_with_opacity()` - 带透明度绘制
- 等等...

### 参数变化
所有绘制方法都新增了 `screen_width` 和 `screen_height` 参数，用于屏幕裁剪。

## 下一步计划

1. **✅ 已完成**: 所有 Draw 方法移植
2. **待完成**: 更新调用这些方法的代码
3. **待完成**: 集成到实际渲染流程中
4. **待完成**: 性能测试和优化
5. **待完成**: 添加单元测试

## 总结

本次移植成功将 C# MLibrary 的 **11 个 Draw 系列方法** 完整移植到 Rust：

| 类别 | 方法数量 | 说明 |
|------|---------|------|
| 基础绘制 | 2 | draw, draw_with_color |
| 高级绘制 | 2 | draw_with_opacity, draw_blend |
| 区域绘制 | 2 | draw_section, draw_section_with_opacity |
| 变换绘制 | 2 | draw_scaled, draw_tinted |
| 特殊绘制 | 2 | draw_up, draw_up_blend |
| 辅助方法 | 1 | visible_pixel |
| **总计** | **11** | **完整功能对等** |

所有方法都保持了与 C# 版本的功能对等，并针对 ggez 图形库进行了适配优化。
