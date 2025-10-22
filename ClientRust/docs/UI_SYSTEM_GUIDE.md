# Crystal UI 系统使用指南

## 概述

Crystal 项目已经内置了一个**强大的声明式 UI 系统**（位于 `src/ui/`），可以大大简化界面开发！

### 当前状态
- ✅ **完整的 UI 框架已实现** - `src/ui/` 目录
- ❌ **ECS 版本未使用** - SelectScene/LoginScene 都是手动绘制
- 🎯 **建议迁移** - 可以大幅简化代码

## 核心组件

### 1. 基础元素 (`src/ui/basic/`)
- **ImageElement** - 图片元素（自动处理缩放、宽高比）
- **TextElement** - 文本元素（自动测量尺寸）
- **EmptyElement** - 空元素（占位符/间距）

### 2. 布局容器 (`src/ui/containers/`)
- **VerticalBox** - 垂直布局（从上到下）
- **HorizontalBox** - 水平布局（从左到右）
- **GridBox** - 网格布局
- **StackBox** - 堆叠布局（层叠）
- **DurationBox** - 定时显示

### 3. 布局系统 (`src/ui/layout.rs`)
- **Size**:
  - `Fixed(f32)` - 固定尺寸
  - `Fill(min, max)` - 填充可用空间
  - `Shrink(min, max)` - 收缩到内容大小
- **Alignment**: Start, Center, End
- **Padding**: (left, right, top, bottom)

### 4. 交互系统 (`src/ui/message.rs`)
- **UiMessage** - 消息系统（点击、悬停、键盘等）
- **Transition** - 动画过渡
- **MessageHandler** - 事件处理

## 使用示例

### 示例 1: 简单按钮行（当前 SelectScene 的底部按钮）

**当前实现（手动绘制，约100行代码）：**
```rust
// 定义常量
const BUTTON_Y: f32 = 736.0;
const BUTTON_WIDTH: f32 = 96.0;
// ... 更多常量

// 绘制按钮
let get_button_index = |base: i32, button_type: BottomButton| -> i32 { ... };
if let Some(lib_arc) = get_library(LibraryName::Title) {
    if let Ok(mut lib) = lib_arc.try_lock() {
        let start_btn_index = get_button_index(340, BottomButton::StartGame);
        lib.draw_with_color(ctx, canvas, start_btn_index, x, y, Color::WHITE, false);
        // ... 重复4次
    }
}

// 处理点击
for button_type in &all_buttons {
    if button_type.contains(x, y) {
        self.handle_button_click(*button_type, network_tx);
    }
}

// 处理悬停
for button_type in &all_buttons {
    if button_type.contains(x, y) {
        self.hovered_button = Some(*button_type);
    }
}
```

**使用 UI 系统（约20行代码）：**
```rust
use crate::ui::*;
use crate::ui::containers::HorizontalBox;

// 定义消息类型
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
enum SelectSceneMessage {
    StartGame,
    NewCharacter,
    DeleteCharacter,
    Credits,
    ExitGame,
}

// 创建底部按钮栏
fn create_bottom_buttons(ctx: &Context) -> UiElement<SelectSceneMessage> {
    // 水平布局容器
    HorizontalBox::new_spaced(50.0)  // 50像素间距
        .to_element_builder(0, ctx)
        .with_alignment(Alignment::Center, Alignment::End)  // 水平居中，底部对齐
        .with_padding((0., 0., 0., 32.))  // 底部32像素边距
        
        // 添加按钮（自动处理悬停、点击）
        .with_child(create_button(ctx, "开始游戏", 340, SelectSceneMessage::StartGame))
        .with_child(create_button(ctx, "新建角色", 343, SelectSceneMessage::NewCharacter))
        .with_child(create_button(ctx, "删除角色", 346, SelectSceneMessage::DeleteCharacter))
        .with_child(create_button(ctx, "制作人员", 349, SelectSceneMessage::Credits))
        .with_child(create_button(ctx, "退出游戏", 352, SelectSceneMessage::ExitGame))
        .build()
}

// 创建单个按钮
fn create_button(
    ctx: &Context, 
    _label: &str,  // 可选：如果需要文字标签
    image_index: usize, 
    message: SelectSceneMessage
) -> UiElement<SelectSceneMessage> {
    // 从库加载图片
    let image = load_image_from_library(LibraryName::Title, image_index);
    
    image.to_element_builder(message as u32, ctx)
        .with_hover_visuals(Visuals::new()
            .with_tint(Color::from_rgb(255, 255, 200)))  // 悬停时微黄
        .with_trigger_sound(load_sound("button_click.wav"))  // 点击音效
        .build()
}

// 在 SelectScene 中使用
struct SelectScene {
    ui_root: UiElement<SelectSceneMessage>,
    // ... 其他字段
}

impl SelectScene {
    fn new(ctx: &Context) -> Self {
        Self {
            ui_root: create_bottom_buttons(ctx),
            // ...
        }
    }
    
    fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas) {
        // 绘制所有 UI（一行代码！）
        self.ui_root.draw_to_screen(ctx, canvas);
    }
    
    fn handle_click(&mut self, x: f32, y: f32) {
        // 处理点击（自动分发到子元素）
        let messages = self.ui_root.trigger_at(x, y);
        
        for msg in messages {
            match msg {
                SelectSceneMessage::StartGame => self.start_game(),
                SelectSceneMessage::NewCharacter => self.open_new_character_dialog(),
                // ...
            }
        }
    }
    
    fn handle_hover(&mut self, x: f32, y: f32) {
        // 处理悬停（自动更新视觉效果）
        self.ui_root.update_hover(x, y);
    }
}
```

### 示例 2: 角色选择卡片（垂直布局）

```rust
use crate::ui::containers::{VerticalBox, HorizontalBox};

fn create_character_card(ctx: &Context, character: &CharacterInfo) -> UiElement<Message> {
    VerticalBox::new_spaced(10.0)
        .to_element_builder(character.id, ctx)
        .with_padding((20., 20., 20., 20.))
        .with_visuals(Visuals::new()
            .with_background_color(Color::from_rgba(0, 0, 0, 180))
            .with_border(2.0, Color::CYAN))
        
        // 角色头像
        .with_child(character.portrait_image()
            .to_element_builder(0, ctx)
            .with_size(Size::Fixed(64.), Size::Fixed(64.))
            .with_alignment(Alignment::Center, None)
            .build())
        
        // 角色名字
        .with_child(Text::new(&character.name)
            .set_font("AlibabaPuHuiTi")
            .set_scale(18.0)
            .to_element_builder(0, ctx)
            .with_alignment(Alignment::Center, None)
            .build())
        
        // 等级和职业
        .with_child(HorizontalBox::new()
            .to_element_builder(0, ctx)
            .with_child(create_label(ctx, &format!("Lv.{}", character.level)))
            .with_child(create_label(ctx, character.class.name()))
            .build())
        
        .build()
}
```

### 示例 3: 对话框（堆叠布局）

```rust
use crate::ui::containers::StackBox;

fn create_dialog(ctx: &Context, title: &str, content: &str) -> UiElement<Message> {
    StackBox::new()
        .to_element_builder(0, ctx)
        .with_alignment(Alignment::Center, Alignment::Center)
        
        // 背景遮罩层
        .with_child(EmptyElement
            .to_element_builder(0, ctx)
            .with_size(Size::Fill(0., f32::INFINITY), Size::Fill(0., f32::INFINITY))
            .with_visuals(Visuals::new()
                .with_background_color(Color::from_rgba(0, 0, 0, 128)))
            .build())
        
        // 对话框主体
        .with_child(VerticalBox::new()
            .to_element_builder(0, ctx)
            .with_size(Size::Fixed(400.), Size::Shrink(200., 600.))
            .with_padding((20., 20., 20., 20.))
            .with_visuals(Visuals::new()
                .with_background_color(Color::from_rgb(40, 40, 60))
                .with_border(2.0, Color::GOLD))
            
            // 标题
            .with_child(create_label(ctx, title))
            
            // 内容
            .with_child(Text::new(content)
                .to_element_builder(0, ctx)
                .build())
            
            // 按钮行
            .with_child(HorizontalBox::new()
                .to_element_builder(0, ctx)
                .with_child(create_button(ctx, "确定", Message::DialogConfirm))
                .with_child(create_button(ctx, "取消", Message::DialogCancel))
                .build())
            
            .build())
        
        .build()
}
```

## 对比总结

### 当前手动绘制方式
❌ **缺点：**
- 代码量大（绘制、点击检测、悬停检测都要手写）
- 位置计算容易出错（绘制和检测不一致）
- 难以维护（修改布局需要改多处）
- 没有自动动画/过渡效果
- 重复代码多

✅ **优点：**
- 完全控制

### UI 系统方式
✅ **优点：**
- **代码量减少80%** 
- **声明式布局** - 自动计算位置
- **自动事件处理** - 点击、悬停、键盘
- **内置动画系统** - Transition
- **类型安全的消息** - 编译时检查
- **可组合** - UI 元素可以嵌套复用
- **响应式** - 自动适应窗口大小

❌ **缺点：**
- 需要学习 API
- 可能性能略低（但可以优化）

## 迁移建议

### 短期（快速见效）
1. ✅ **迁移底部按钮栏** - 最简单，效果明显
2. ✅ **迁移角色卡片列表** - VerticalBox 完美适配
3. ✅ **迁移对话框** - StackBox + VerticalBox

### 中期
4. 迁移登录界面
5. 迁移输入框组件
6. 添加动画效果

### 长期
7. 创建可复用的 UI 组件库
8. 实现主题系统
9. 添加 UI 编辑器（可视化设计）

## 实现步骤

### 步骤 1: 创建辅助函数
在 `src/ecs/ui/` 中创建 `ui_helpers.rs`：
```rust
use crate::ui::*;
use crate::graphics::libraries::{get_library, LibraryName};

pub fn load_button_image(library: LibraryName, index: usize) -> Image {
    // 从图像库加载按钮图片
    // TODO: 实现
}

pub fn create_button<T: Copy + Eq + Hash>(
    ctx: &Context,
    image: Image,
    message: T,
    hover_tint: Option<Color>,
) -> UiElement<T> {
    image.to_element_builder(message as u32, ctx)
        .with_hover_visuals(hover_tint.map(|color| 
            Visuals::new().with_tint(color)
        ))
        .build()
}
```

### 步骤 2: 修改 SelectScene
```rust
// 添加字段
ui_root: UiElement<SelectSceneMessage>,

// 在 new() 中创建
ui_root: create_select_scene_ui(ctx, characters),

// 在 draw() 中绘制
self.ui_root.draw_to_screen(ctx, canvas);

// 在事件处理中使用
fn on_mouse_down(&mut self, x: f32, y: f32) {
    for msg in self.ui_root.trigger_at(x, y) {
        self.handle_message(msg);
    }
}
```

## 总结

**强烈建议使用 UI 系统！** 它可以：
- ✅ 减少 80% 的 UI 代码
- ✅ 消除位置计算错误
- ✅ 自动处理交互
- ✅ 支持动画和过渡
- ✅ 提高代码可维护性

如果需要帮助迁移，请告诉我从哪个场景开始！
