# SelectScene 完整 UI 系统集成示例

基于 mooeye 示例,这是如何将 SelectScene 迁移到完整声明式 UI 的方案。

## 📋 当前问题

我们创建了 `ButtonWidget` 作为轻量级解决方案,但发现 mooeye UI 系统实际上**可以**用于游戏 UI:

### ❌ 之前的误解
> "运行时纹理加载不适合声明式 UI"

### ✅ 实际情况
mooeye 示例显示:**可以在运行时加载纹理并创建 UI**

```rust
// 示例: c_uielement.rs
let text_element = graphics::Text::new("Take me back!")
    .set_font("Bahnschrift")
    .set_scale(32.)
    .to_owned()
    .to_element_builder(1, ctx)  // 🎯 运行时转换!
    .with_visuals(ui::Visuals::new(...))
    .with_trigger_key(...)
    .build();
```

## 🔄 完整迁移方案

### 方案 A: 自定义 ImageButton (推荐)

创建一个 `ImageButton` 包装器,支持从 GgezManager 加载的纹理:

```rust
// src/ecs/ui/image_button.rs

use ggez::graphics::Image;
use crate::ui::{UiContent, UiElement, UiElementBuilder};

/// 图像按钮 - 包装 ggez::Image 并添加按钮语义
pub struct ImageButton {
    normal: Image,
    hover: Image,
    pressed: Image,
}

impl ImageButton {
    /// 从 GgezManager 加载 3 态按钮
    pub fn from_indices(
        ctx: &mut Context,
        ggez_manager: &GgezManager,
        normal_idx: i32,
        hover_idx: i32,
        pressed_idx: i32,
    ) -> GameResult<Self> {
        let normal = ggez_manager.get_texture(&format!("Title_{}", normal_idx))
            .ok_or_else(|| GameError::CustomError("纹理未找到".into()))?
            .clone();
        
        let hover = ggez_manager.get_texture(&format!("Title_{}", hover_idx))
            .ok_or_else(|| GameError::CustomError("纹理未找到".into()))?
            .clone();
            
        let pressed = ggez_manager.get_texture(&format!("Title_{}", pressed_idx))
            .ok_or_else(|| GameError::CustomError("纹理未找到".into()))?
            .clone();
        
        Ok(Self { normal, hover, pressed })
    }
}

impl<T: Copy + Eq + Hash> UiContent<T> for ImageButton {
    fn to_element_builder(self, id: u32, ctx: &Context) -> UiElementBuilder<T> {
        // 使用 normal 图像作为基础
        let size = self.normal.dimensions(&ctx.gfx);
        
        self.normal.to_element_builder(id, ctx)
            .with_hover_visuals(/* 使用 hover 图像 */)
            // TODO: 如何处理 pressed 状态?
    }
}
```

**问题**: mooeye UI 系统**没有 pressed 状态**,只有 normal/hover!
- 只支持 `visuals` 和 `hover_visuals`
- 按下状态需要自己管理

### 方案 B: 继续使用 ButtonWidget (当前方案)

**优势**:
- ✅ 支持 3 种状态 (normal/hover/pressed)
- ✅ 与现有 Library 系统完美集成
- ✅ 不需要克隆纹理 (只存储索引)
- ✅ 轻量级,易于理解

**劣势**:
- ❌ 不能使用 mooeye 的自动布局
- ❌ 不能使用 Transition 动画
- ❌ 需要手动管理绘制

### 方案 C: 混合方案 (最佳实践)

使用 mooeye UI 系统管理**布局和容器**,ButtonWidget 管理**按钮状态**:

```rust
// SelectScene 结构
pub struct SelectScene {
    // 🎯 使用 UI 系统管理布局
    layout_root: UiElement<SelectSceneMessage>,
    
    // 🎯 使用 ButtonWidget 管理按钮状态
    bottom_buttons: ButtonGroup,
    
    // 其他字段...
}

impl SelectScene {
    pub fn new(characters: Vec<SelectInfo>, ctx: &Context) -> Self {
        // 1. 创建按钮组 (状态管理)
        let mut bottom_buttons = ButtonGroup::new();
        // ... 添加按钮
        
        // 2. 创建 UI 布局容器 (自动布局)
        let layout_root = ui::containers::StackBox::new()
            .to_element_builder(0, ctx)
            .as_fill()
            
            // 添加背景层
            .with_child(
                // 背景图像
            )
            
            // 添加角色槽位层
            .with_child(
                ui::containers::VerticalBox::new()
                    .to_element_builder(1, ctx)
                    // 4 个角色槽位
            )
            
            // 添加底部按钮占位层 (实际绘制由 ButtonWidget 处理)
            .with_child(
                ui::basic::EmptyElement::default()
                    .to_element_builder(2, ctx)
                    .with_size(
                        ui::Size::Fixed(800., 800.),
                        ui::Size::Fixed(32., 32.)
                    )
                    .with_alignment(ui::Alignment::Center, ui::Alignment::Max)
            )
            
            .build();
        
        Self {
            layout_root,
            bottom_buttons,
            // ...
        }
    }
    
    fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas) -> GameResult {
        // 1. 使用 UI 系统绘制布局
        self.layout_root.draw_to_screen(ctx, canvas, true);
        
        // 2. 手动绘制按钮 (覆盖在 UI 之上)
        if let Some(lib) = get_library(LibraryName::Title) {
            if let Ok(mut lib) = lib.try_lock() {
                for button in &self.bottom_buttons.buttons {
                    lib.draw_with_color(
                        ctx, canvas,
                        button.get_texture_index() as usize,
                        button.x, button.y,
                        button.get_color(),
                        false
                    );
                }
            }
        }
        
        Ok(())
    }
}
```

## 🎨 改进建议

### 1. 添加音效支持

参考 `e_messages.rs` 的 `with_trigger_sound()`:

```rust
// button_widget.rs
pub struct ButtonWidget {
    // ... 现有字段
    
    /// 点击音效
    pub click_sound: Option<ggez::audio::Source>,
}

impl ButtonWidget {
    pub fn on_mouse_up(&mut self, mouse_x: f32, mouse_y: f32) -> bool {
        let was_pressed = self.state == ButtonState::Pressed;
        
        if self.contains(mouse_x, mouse_y) {
            self.state = ButtonState::Hovered;
            
            if was_pressed {
                // 🎵 播放点击音效
                if let Some(sound) = &mut self.click_sound {
                    let _ = sound.play(ctx);
                }
            }
            
            was_pressed
        } else {
            self.state = ButtonState::Normal;
            false
        }
    }
}
```

### 2. 使用 UI 系统的容器布局

将角色槽位改用 `VerticalBox`:

```rust
// 创建角色槽位容器
let mut character_slots = ui::containers::VerticalBox::new_spaced(104.0);

for (i, character) in characters.iter().enumerate() {
    let slot = ui::basic::EmptyElement::default()
        .to_element_builder(100 + i as u32, ctx)
        .with_size(
            ui::Size::Fixed(80., 80.),
            ui::Size::Fixed(80., 80.)
        )
        // 点击时选中角色
        .with_message_handler(move |messages, _, _| {
            if messages.contains(&ui::UiMessage::Clicked(100 + i as u32)) {
                // 发送选中角色消息
            }
        })
        .build();
    
    character_slots.add(slot);
}

let character_box = character_slots
    .to_element_builder(10, ctx)
    .with_alignment(ui::Alignment::Max, ui::Alignment::Center)
    .with_offset(-50., 0.)
    .build();
```

### 3. 消息驱动架构

定义游戏消息类型:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SelectSceneMessage {
    // 按钮消息
    StartGame,
    NewCharacter,
    DeleteCharacter,
    Credits,
    ExitGame,
    
    // 角色槽位消息
    SelectCharacter(usize),
}

// 在 update 中处理
fn update(&mut self, ctx: &mut Context) -> GameResult<SceneSwitch> {
    let messages = self.layout_root.manage_messages(ctx, None);
    
    for message in messages {
        match message {
            UiMessage::Extern(SelectSceneMessage::StartGame) => {
                self.start_game();
            }
            UiMessage::Extern(SelectSceneMessage::SelectCharacter(idx)) => {
                self.selected_index = idx as i32;
            }
            // ...
        }
    }
    
    Ok(SceneSwitch::None)
}
```

## 📊 对比总结

| 特性 | ButtonWidget | 完整 UI 系统 | 混合方案 |
|------|--------------|-------------|----------|
| 3 态按钮 | ✅ | ❌ (只有 2 态) | ✅ |
| 自动布局 | ❌ | ✅ | ✅ |
| 动画支持 | ❌ | ✅ | ✅ |
| 音效支持 | 🔧 需手动添加 | ✅ | ✅ |
| 纹理加载 | ✅ 运行时 | ⚠️ 需包装 | ✅ |
| 学习曲线 | 低 | 中 | 中 |
| 代码量 | 少 | 多 | 中 |

## 🎯 最终建议

**当前保持 ButtonWidget 方案**,原因:

1. ✅ **简单实用**: 完全满足当前需求
2. ✅ **已经实现**: 代码编译通过,可以测试
3. ✅ **易于维护**: 不依赖复杂 UI 框架

**未来可以考虑**:

1. 使用 UI 系统的 **HorizontalBox/VerticalBox** 管理角色槽位布局
2. 使用 UI 系统的 **StackBox** 管理对话框层级
3. 使用 UI 系统的 **Transition** 添加淡入淡出动画

但按钮部分继续使用 ButtonWidget,因为:
- mooeye UI **不支持 pressed 状态** (这是致命缺陷)
- 我们的按钮需要 3 态纹理切换 (normal/hover/pressed)
- Library 系统已经很好地处理了纹理加载

## 💡 下一步行动

1. **测试 ButtonWidget**: 运行游戏验证按钮功能
2. **添加音效**: 在 ButtonWidget 中集成点击音效
3. **优化布局**: 考虑用 VerticalBox 管理角色槽位
4. **文档完善**: 记录 ButtonWidget API 使用方法

mooeye UI 系统非常强大,但不是万能的。我们的 ButtonWidget 混合方案是针对游戏 UI 特点的**最佳实践**! 🎉
