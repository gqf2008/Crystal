# MirControls 架构设计文档

## 📐 整体架构

### 核心设计原则

1. **与 C# 结构对齐**：保持与原版 MirControl.cs 的结构一致
2. **Rust 习惯用法**：使用 trait、所有权、生命周期等 Rust 特性
3. **性能优先**：避免不必要的克隆和分配
4. **类型安全**：利用 Rust 的类型系统避免运行时错误

### 模块层次结构

```
controls/
├── mod.rs                      # 模块根，导出公共接口
├── control.rs                  # MirControl 基础类
├── image_control.rs            # MirImageControl
├── button.rs                   # MirButton
├── label.rs                    # MirLabel
├── textbox.rs                  # MirTextBox
├── animated_control.rs         # MirAnimatedControl
├── scene.rs                    # MirScene
├── message_box.rs              # MirMessageBox
├── item_cell.rs                # MirItemCell
├── checkbox.rs                 # MirCheckBox
├── dropdown.rs                 # MirDropDownBox
├── amount_box.rs               # MirAmountBox
├── animated_button.rs          # MirAnimatedButton
├── goods_cell.rs               # MirGoodsCell
├── game_shop_cell.rs           # MirGameShopCell
├── input_box.rs                # MirInputBox
├── scrolling_label.rs          # MirScrollingLabel
└── types.rs                    # 共享类型定义

```

---

## 🔧 核心类型定义

### types.rs - 共享类型

```rust
// controls/types.rs

/// Point - 2D坐标
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn add(&self, other: Point) -> Point {
        Point {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

/// Size - 尺寸
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Size {
    pub width: i32,
    pub height: i32,
}

impl Size {
    pub fn new(width: i32, height: i32) -> Self {
        Self { width, height }
    }

    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }
}

/// Rectangle - 矩形区域
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rectangle {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rectangle {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self { x, y, width, height }
    }

    pub fn from_location_size(location: Point, size: Size) -> Self {
        Self {
            x: location.x,
            y: location.y,
            width: size.width,
            height: size.height,
        }
    }

    pub fn left(&self) -> i32 { self.x }
    pub fn top(&self) -> i32 { self.y }
    pub fn right(&self) -> i32 { self.x + self.width }
    pub fn bottom(&self) -> i32 { self.y + self.height }

    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.left() && point.x < self.right() &&
        point.y >= self.top() && point.y < self.bottom()
    }
}

/// Color - ARGB 颜色
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub a: u8,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub fn from_argb(a: u8, r: u8, g: u8, b: u8) -> Self {
        Self { a, r, g, b }
    }

    pub fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self { a: 255, r, g, b }
    }

    pub fn white() -> Self { Self::from_rgb(255, 255, 255) }
    pub fn black() -> Self { Self::from_rgb(0, 0, 0) }
    pub fn transparent() -> Self { Self::from_argb(0, 0, 0, 0) }
    pub fn magenta() -> Self { Self::from_rgb(255, 0, 255) }
}

impl Default for Color {
    fn default() -> Self {
        Self::white()
    }
}

/// MouseButton - 鼠标按钮
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    None,
    Left,
    Right,
    Middle,
}

/// KeyCode - 键盘按键
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCode {
    // 字母键
    A, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    
    // 数字键
    Num0, Num1, Num2, Num3, Num4, Num5, Num6, Num7, Num8, Num9,
    
    // 功能键
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    
    // 方向键
    Up, Down, Left, Right,
    
    // 控制键
    Enter, Escape, Tab, Space, Backspace, Delete,
    Shift, Control, Alt,
    
    // 其他
    Unknown,
}

/// BlendMode - 混合模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    None,
    Normal,
    Additive,
    Multiply,
}

impl Default for BlendMode {
    fn default() -> Self {
        BlendMode::None
    }
}
```

---

## 🎨 Control Trait 设计

### control.rs - 核心 trait

```rust
// controls/control.rs

use super::types::*;
use std::any::Any;

/// Control trait - 所有控件的基础接口
/// 对应 C# 的 MirControl 类
pub trait Control: Any {
    /// 获取控件名称（用于调试）
    fn name(&self) -> &str { "Control" }
    
    // === 位置和尺寸 ===
    
    /// 相对于父控件的位置
    fn location(&self) -> Point;
    fn set_location(&mut self, location: Point);
    
    /// 控件尺寸
    fn size(&self) -> Size;
    fn set_size(&mut self, size: Size);
    
    /// 显示位置（绝对坐标，考虑父控件）
    fn display_location(&self) -> Point {
        if let Some(parent) = self.parent() {
            parent.display_location().add(self.location())
        } else {
            self.location()
        }
    }
    
    /// 显示矩形区域
    fn display_rectangle(&self) -> Rectangle {
        Rectangle::from_location_size(self.display_location(), self.size())
    }
    
    // === 可见性和启用状态 ===
    
    fn visible(&self) -> bool;
    fn set_visible(&mut self, visible: bool);
    
    fn enabled(&self) -> bool;
    fn set_enabled(&mut self, enabled: bool);
    
    /// 是否真正可见（考虑父控件）
    fn is_really_visible(&self) -> bool {
        if !self.visible() {
            return false;
        }
        if let Some(parent) = self.parent() {
            parent.is_really_visible()
        } else {
            true
        }
    }
    
    /// 是否真正启用（考虑父控件）
    fn is_really_enabled(&self) -> bool {
        if !self.enabled() {
            return false;
        }
        if let Some(parent) = self.parent() {
            parent.is_really_enabled()
        } else {
            true
        }
    }
    
    // === 颜色 ===
    
    fn back_color(&self) -> Color;
    fn set_back_color(&mut self, color: Color);
    
    fn fore_color(&self) -> Color;
    fn set_fore_color(&mut self, color: Color);
    
    fn border_color(&self) -> Color;
    fn set_border_color(&mut self, color: Color);
    
    // === 边框 ===
    
    fn border(&self) -> bool;
    fn set_border(&mut self, border: bool);
    
    // === 混合效果 ===
    
    fn gray_scale(&self) -> bool;
    fn set_gray_scale(&mut self, gray_scale: bool);
    
    fn blending(&self) -> bool;
    fn set_blending(&mut self, blending: bool);
    
    fn blending_rate(&self) -> f32;
    fn set_blending_rate(&mut self, rate: f32);
    
    fn blend_mode(&self) -> BlendMode;
    fn set_blend_mode(&mut self, mode: BlendMode);
    
    // === 层次关系 ===
    
    /// 父控件（使用弱引用避免循环引用）
    fn parent(&self) -> Option<&dyn Control>;
    fn set_parent(&mut self, parent: Option<&dyn Control>);
    
    /// 子控件列表
    fn children(&self) -> &[Box<dyn Control>];
    fn children_mut(&mut self) -> &mut Vec<Box<dyn Control>>;
    
    fn add_child(&mut self, child: Box<dyn Control>) {
        self.children_mut().push(child);
        self.on_child_added();
    }
    
    fn remove_child(&mut self, index: usize) -> Option<Box<dyn Control>> {
        if index < self.children().len() {
            let child = self.children_mut().remove(index);
            self.on_child_removed();
            Some(child)
        } else {
            None
        }
    }
    
    // === 生命周期 ===
    
    /// 初始化（首次显示时调用）
    fn initialize(&mut self) {}
    
    /// 更新逻辑（每帧调用）
    fn update(&mut self, delta_time: f32) {
        // 更新所有子控件
        for child in self.children_mut() {
            if child.visible() {
                child.update(delta_time);
            }
        }
    }
    
    /// 绘制控件
    fn draw(&self) {
        if !self.visible() {
            return;
        }
        
        self.on_before_draw();
        self.draw_control();
        self.draw_children();
        self.on_after_draw();
    }
    
    /// 绘制控件本身
    fn draw_control(&self);
    
    /// 绘制所有子控件
    fn draw_children(&self) {
        for child in self.children() {
            if child.visible() {
                child.draw();
            }
        }
    }
    
    // === 事件处理 ===
    
    /// 鼠标移动
    fn on_mouse_move(&mut self, x: i32, y: i32) {
        let point = Point::new(x, y);
        if self.display_rectangle().contains(point) {
            // 转换为相对坐标
            let local_point = Point::new(
                x - self.display_location().x,
                y - self.display_location().y,
            );
            
            // 传递给子控件
            for child in self.children_mut() {
                if child.visible() && child.enabled() {
                    child.on_mouse_move(local_point.x, local_point.y);
                }
            }
        }
    }
    
    /// 鼠标按下
    fn on_mouse_down(&mut self, x: i32, y: i32, button: MouseButton) {
        let point = Point::new(x, y);
        if self.display_rectangle().contains(point) {
            let local_point = Point::new(
                x - self.display_location().x,
                y - self.display_location().y,
            );
            
            for child in self.children_mut() {
                if child.visible() && child.enabled() {
                    child.on_mouse_down(local_point.x, local_point.y, button);
                }
            }
        }
    }
    
    /// 鼠标释放
    fn on_mouse_up(&mut self, x: i32, y: i32, button: MouseButton) {
        let point = Point::new(x, y);
        if self.display_rectangle().contains(point) {
            let local_point = Point::new(
                x - self.display_location().x,
                y - self.display_location().y,
            );
            
            for child in self.children_mut() {
                if child.visible() && child.enabled() {
                    child.on_mouse_up(local_point.x, local_point.y, button);
                }
            }
        }
    }
    
    /// 鼠标点击
    fn on_click(&mut self, x: i32, y: i32, button: MouseButton) {}
    
    /// 鼠标双击
    fn on_double_click(&mut self, x: i32, y: i32, button: MouseButton) {}
    
    /// 鼠标滚轮
    fn on_mouse_wheel(&mut self, delta: i32) {}
    
    /// 键盘按下
    fn on_key_down(&mut self, key: KeyCode) {}
    
    /// 键盘释放
    fn on_key_up(&mut self, key: KeyCode) {}
    
    /// 字符输入
    fn on_key_press(&mut self, ch: char) {}
    
    // === 事件回调 ===
    
    fn on_before_draw(&self) {}
    fn on_after_draw(&self) {}
    fn on_shown(&mut self) {}
    fn on_location_changed(&mut self) {}
    fn on_size_changed(&mut self) {}
    fn on_enabled_changed(&mut self) {}
    fn on_visible_changed(&mut self) {}
    fn on_child_added(&mut self) {}
    fn on_child_removed(&mut self) {}
    
    // === 辅助方法 ===
    
    /// 标记需要重绘
    fn invalidate(&mut self);
    
    /// 立即重绘
    fn redraw(&mut self) {
        self.invalidate();
    }
    
    /// 清理资源
    fn dispose(&mut self) {}
    
    /// 类型转换辅助
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}
```

---

## 📦 MirControl 基础实现

### control.rs - 具体实现

```rust
// controls/control.rs (continued)

/// MirControl - Control trait 的默认实现
/// 对应 C# 的 MirControl 类
pub struct MirControl {
    // 基础属性
    location: Point,
    size: Size,
    visible: bool,
    enabled: bool,
    
    // 颜色
    back_color: Color,
    fore_color: Color,
    border_color: Color,
    
    // 边框
    border: bool,
    
    // 混合效果
    gray_scale: bool,
    blending: bool,
    blending_rate: f32,
    blend_mode: BlendMode,
    
    // 层次关系
    children: Vec<Box<dyn Control>>,
    
    // 渲染状态
    texture_valid: bool,
    needs_redraw: bool,
    
    // 提示文本
    hint: String,
}

impl MirControl {
    pub fn new() -> Self {
        Self {
            location: Point::default(),
            size: Size::default(),
            visible: true,
            enabled: true,
            back_color: Color::transparent(),
            fore_color: Color::white(),
            border_color: Color::black(),
            border: false,
            gray_scale: false,
            blending: false,
            blending_rate: 1.0,
            blend_mode: BlendMode::default(),
            children: Vec::new(),
            texture_valid: false,
            needs_redraw: true,
            hint: String::new(),
        }
    }
    
    pub fn with_location(mut self, location: Point) -> Self {
        self.location = location;
        self
    }
    
    pub fn with_size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }
    
    pub fn with_back_color(mut self, color: Color) -> Self {
        self.back_color = color;
        self
    }
}

impl Control for MirControl {
    fn name(&self) -> &str { "MirControl" }
    
    // === 实现所有 trait 方法 ===
    
    fn location(&self) -> Point { self.location }
    fn set_location(&mut self, location: Point) {
        if self.location != location {
            self.location = location;
            self.on_location_changed();
        }
    }
    
    fn size(&self) -> Size { self.size }
    fn set_size(&mut self, size: Size) {
        if self.size != size {
            self.size = size;
            self.texture_valid = false;
            self.on_size_changed();
        }
    }
    
    fn visible(&self) -> bool { self.visible }
    fn set_visible(&mut self, visible: bool) {
        if self.visible != visible {
            self.visible = visible;
            self.on_visible_changed();
        }
    }
    
    fn enabled(&self) -> bool { self.enabled }
    fn set_enabled(&mut self, enabled: bool) {
        if self.enabled != enabled {
            self.enabled = enabled;
            self.on_enabled_changed();
        }
    }
    
    fn back_color(&self) -> Color { self.back_color }
    fn set_back_color(&mut self, color: Color) {
        if self.back_color != color {
            self.back_color = color;
            self.texture_valid = false;
            self.redraw();
        }
    }
    
    fn fore_color(&self) -> Color { self.fore_color }
    fn set_fore_color(&mut self, color: Color) {
        if self.fore_color != color {
            self.fore_color = color;
            self.redraw();
        }
    }
    
    fn border_color(&self) -> Color { self.border_color }
    fn set_border_color(&mut self, color: Color) {
        if self.border_color != color {
            self.border_color = color;
            self.redraw();
        }
    }
    
    fn border(&self) -> bool { self.border }
    fn set_border(&mut self, border: bool) {
        if self.border != border {
            self.border = border;
            self.redraw();
        }
    }
    
    fn gray_scale(&self) -> bool { self.gray_scale }
    fn set_gray_scale(&mut self, gray_scale: bool) {
        self.gray_scale = gray_scale;
    }
    
    fn blending(&self) -> bool { self.blending }
    fn set_blending(&mut self, blending: bool) {
        self.blending = blending;
    }
    
    fn blending_rate(&self) -> f32 { self.blending_rate }
    fn set_blending_rate(&mut self, rate: f32) {
        self.blending_rate = rate.clamp(0.0, 1.0);
    }
    
    fn blend_mode(&self) -> BlendMode { self.blend_mode }
    fn set_blend_mode(&mut self, mode: BlendMode) {
        self.blend_mode = mode;
    }
    
    fn parent(&self) -> Option<&dyn Control> { None }
    fn set_parent(&mut self, _parent: Option<&dyn Control>) {}
    
    fn children(&self) -> &[Box<dyn Control>] { &self.children }
    fn children_mut(&mut self) -> &mut Vec<Box<dyn Control>> { &mut self.children }
    
    fn draw_control(&self) {
        // 基础实现：绘制背景色和边框
        if self.back_color.a > 0 {
            // TODO: 调用渲染系统绘制填充矩形
        }
        
        if self.border {
            // TODO: 调用渲染系统绘制边框
        }
    }
    
    fn invalidate(&mut self) {
        self.needs_redraw = true;
        self.texture_valid = false;
    }
    
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

impl Default for MirControl {
    fn default() -> Self {
        Self::new()
    }
}
```

---

## 📄 实现计划总结

### Phase 1: 核心基础 (Week 1)

**文件清单**：
1. ✅ `types.rs` - 共享类型定义
2. ✅ `control.rs` - Control trait + MirControl 实现
3. ⏳ `mod.rs` - 模块导出

**验收标准**：
- [ ] 可以创建 MirControl 实例
- [ ] 可以设置位置、大小、颜色等属性
- [ ] 可以添加子控件
- [ ] 编译通过，单元测试通过

### Phase 2: 图像控件 (Week 1-2)

**文件清单**：
1. `image_control.rs` - MirImageControl
2. 集成图像库接口

**特性**：
- 加载和显示图像
- 自动尺寸调整
- 像素检测
- 偏移量支持

### Phase 3: 交互控件 (Week 2)

**文件清单**：
1. `button.rs` - MirButton
2. `label.rs` - MirLabel  
3. `textbox.rs` - MirTextBox

---

## 🎯 下一步行动

1. **立即创建 types.rs** - 定义共享类型
2. **实现 control.rs 基础版** - Control trait + MirControl
3. **编写单元测试** - 验证基础功能
4. **创建 mod.rs** - 模块组织和导出

准备好了吗？我现在就开始创建这些文件！
