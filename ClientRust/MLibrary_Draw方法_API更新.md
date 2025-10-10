# MLibrary Draw 方法 API 更新说明

## 更新日期
2025年10月10日

## 更新内容

### 简化的 API 签名

所有 draw 方法现在自动从 `ctx.gfx.drawable_size()` 获取屏幕尺寸，**不再需要传递 `screen_width` 和 `screen_height` 参数**！

## 更新前后对比

### ❌ 旧 API（需要传递屏幕尺寸）

```rust
// 需要手动传递屏幕宽高
library.draw(
    &mut ctx,
    &mut canvas,
    0,
    100.0, 100.0,
    800.0, 600.0  // 👈 需要传递屏幕尺寸
)?;

library.draw_with_color(
    &mut ctx,
    &mut canvas,
    5,
    200.0, 150.0,
    Color::RED,
    true,
    800.0, 600.0  // 👈 需要传递屏幕尺寸
)?;
```

### ✅ 新 API（自动获取屏幕尺寸）

```rust
// 自动获取屏幕尺寸，调用更简洁
library.draw(
    &mut ctx,
    &mut canvas,
    0,
    100.0, 100.0
)?;

library.draw_with_color(
    &mut ctx,
    &mut canvas,
    5,
    200.0, 150.0,
    Color::RED,
    true
)?;
```

## 完整 API 列表

### 1. 基础绘制

```rust
pub fn draw(
    &mut self,
    ctx: &mut Context,
    canvas: &mut Canvas,
    index: usize,
    x: f32,
    y: f32,
) -> io::Result<()>
```

**示例：**
```rust
// 在 (100, 100) 位置绘制图像 0
library.draw(&mut ctx, &mut canvas, 0, 100.0, 100.0)?;
```

### 2. 带颜色和偏移的绘制

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
) -> io::Result<()>
```

**示例：**
```rust
use ggez::graphics::Color;

// 红色绘制，应用图像偏移
library.draw_with_color(
    &mut ctx,
    &mut canvas,
    5,
    200.0, 150.0,
    Color::RED,
    true  // 应用偏移
)?;
```

### 3. 带透明度的绘制

```rust
pub fn draw_with_opacity(
    &mut self,
    ctx: &mut Context,
    canvas: &mut Canvas,
    index: usize,
    x: f32,
    y: f32,
    color: Color,
    offset: bool,
    opacity: f32,
) -> io::Result<()>
```

**示例：**
```rust
// 50% 透明度绘制
library.draw_with_opacity(
    &mut ctx,
    &mut canvas,
    10,
    300.0, 200.0,
    Color::WHITE,
    false,  // 不应用偏移
    0.5     // 50% 透明度
)?;
```

### 4. 混合模式绘制

```rust
pub fn draw_blend(
    &mut self,
    ctx: &mut Context,
    canvas: &mut Canvas,
    index: usize,
    x: f32,
    y: f32,
    color: Color,
    offset: bool,
    rate: f32,
) -> io::Result<()>
```

**示例：**
```rust
// 混合绘制，混合率 0.75
library.draw_blend(
    &mut ctx,
    &mut canvas,
    15,
    400.0, 250.0,
    Color::WHITE,
    false,
    0.75  // 混合率
)?;
```

### 5. 部分区域绘制

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
    x: f32,
    y: f32,
    color: Color,
    offset: bool,
) -> io::Result<()>
```

**示例：**
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
    false
)?;
```

### 6. 部分区域带透明度绘制

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
    x: f32,
    y: f32,
    color: Color,
    opacity: f32,
) -> io::Result<()>
```

**示例：**
```rust
// 绘制区域，70% 不透明度
library.draw_section_with_opacity(
    &mut ctx,
    &mut canvas,
    8,
    16.0, 16.0,    // 源区域起点
    48.0, 48.0,    // 源区域尺寸
    500.0, 350.0,  // 目标位置
    Color::WHITE,
    0.7            // 70% 不透明度
)?;
```

### 7. 缩放绘制

```rust
pub fn draw_scaled(
    &mut self,
    ctx: &mut Context,
    canvas: &mut Canvas,
    index: usize,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: Color,
) -> io::Result<()>
```

**示例：**
```rust
// 缩放到 64x64 绘制
library.draw_scaled(
    &mut ctx,
    &mut canvas,
    3,
    100.0, 100.0,
    64.0, 64.0,  // 目标尺寸
    Color::WHITE
)?;
```

### 8. 着色绘制（双层）

```rust
pub fn draw_tinted(
    &mut self,
    ctx: &mut Context,
    canvas: &mut Canvas,
    index: usize,
    x: f32,
    y: f32,
    color: Color,
    tint: Color,
    offset: bool,
) -> io::Result<()>
```

**示例：**
```rust
// 绘制主图像（白色）+ 遮罩层（红色着色）
// 用于装备染色系统
library.draw_tinted(
    &mut ctx,
    &mut canvas,
    20,
    500.0, 400.0,
    Color::WHITE,   // 主图像颜色
    Color::RED,     // 遮罩层颜色（装备染色）
    true            // 应用偏移
)?;
```

### 9. 向上绘制

```rust
pub fn draw_up(
    &mut self,
    ctx: &mut Context,
    canvas: &mut Canvas,
    index: usize,
    x: f32,
    y: f32,
) -> io::Result<()>
```

**示例：**
```rust
// Y坐标自动减去图像高度（底部对齐）
library.draw_up(
    &mut ctx,
    &mut canvas,
    25,
    300.0, 600.0  // 将在 (300, 600-图像高度) 位置绘制
)?;
```

### 10. 向上混合绘制

```rust
pub fn draw_up_blend(
    &mut self,
    ctx: &mut Context,
    canvas: &mut Canvas,
    index: usize,
    x: f32,
    y: f32,
) -> io::Result<()>
```

**示例：**
```rust
// Y坐标自动减去高度，混合绘制
library.draw_up_blend(
    &mut ctx,
    &mut canvas,
    30,
    400.0, 600.0
)?;
```

### 11. 像素可见性检测

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

**示例：**
```rust
// 精确检测
let is_visible = library.visible_pixel(&mut ctx, 15, 50, 50, true)?;
if is_visible {
    println!("点击到了图像！");
}

// 模糊检测（5x5 区域）
let is_near = library.visible_pixel(&mut ctx, 15, 50, 50, false)?;
if is_near {
    println!("点击在图像附近！");
}
```

## 实现细节

### 自动屏幕尺寸获取

每个 draw 方法内部都会自动获取屏幕尺寸：

```rust
pub fn draw(
    &mut self,
    ctx: &mut ggez::Context,
    canvas: &mut ggez::graphics::Canvas,
    index: usize,
    x: f32,
    y: f32,
) -> io::Result<()> {
    // ✅ 自动获取屏幕尺寸
    let (screen_width, screen_height) = ctx.gfx.drawable_size();
    
    // 屏幕裁剪检查
    if x >= screen_width || y >= screen_height {
        return Ok(());
    }
    
    // ... 绘制逻辑
}
```

### 优势

1. **更简洁的 API** - 减少参数数量
2. **自适应分辨率** - 自动适应窗口大小变化
3. **减少错误** - 不需要手动维护屏幕尺寸变量
4. **与 C# 版本更一致** - C# 版本也是自动从 `Settings.ScreenWidth/Height` 获取

## 迁移指南

如果你的代码使用了旧 API，只需要**删除最后两个参数**即可：

```rust
// ❌ 旧代码
library.draw(&mut ctx, &mut canvas, 0, 100.0, 100.0, 800.0, 600.0)?;

// ✅ 新代码 - 删除最后两个参数
library.draw(&mut ctx, &mut canvas, 0, 100.0, 100.0)?;
```

## 性能说明

`ctx.gfx.drawable_size()` 是一个非常轻量级的调用，它只是读取已缓存的值，**不会**每次都重新计算窗口尺寸，因此对性能没有影响。

## 总结

新 API 保持了所有原有功能的同时，使接口更加简洁和易用。所有 11 个 draw 方法都已更新，可以直接使用！
