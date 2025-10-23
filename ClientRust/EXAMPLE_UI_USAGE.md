# UI组件使用示例

## 结构调整

UI组件已从 `login_scene/ui/` 移到 `scenes/ui/`，现在可以被所有场景复用。

```
src/ecs/scenes/
├── mod.rs
├── ui/                    # ✅ 共享UI组件
│   ├── mod.rs
│   ├── button.rs
│   └── text_input.rs
├── login_scene/
│   ├── mod.rs
│   └── dialogs/
└── select_scene/
    ├── select_scene.rs
    └── dialogs/
```

## 在其他场景中使用UI组件

### 示例1：在SelectScene中使用Button

```rust
// src/ecs/scenes/select_scene.rs 或 select_scene/mod.rs

use crate::ecs::scenes::ui::{Button, TextInput};
use crate::graphics::LibraryName;

pub struct SelectScene {
    // 使用共享UI组件
    create_button: Button,
    delete_button: Button,
    start_button: Button,
    exit_button: Button,
}

impl SelectScene {
    pub fn new() -> Self {
        Self {
            create_button: Button::new(100.0, 300.0, LibraryName::Prguse, 505),
            delete_button: Button::new(200.0, 300.0, LibraryName::Prguse, 515),
            start_button: Button::new(300.0, 300.0, LibraryName::Prguse, 525),
            exit_button: Button::new(400.0, 300.0, LibraryName::Prguse, 535),
        }
    }
    
    pub fn on_mouse_move(&mut self, x: f32, y: f32) {
        self.create_button.update_hover(x, y);
        self.delete_button.update_hover(x, y);
        self.start_button.update_hover(x, y);
        self.exit_button.update_hover(x, y);
    }
    
    pub fn on_mouse_down(&mut self, x: f32, y: f32) -> Option<Action> {
        if self.create_button.contains(x, y) {
            return Some(Action::CreateCharacter);
        }
        if self.delete_button.contains(x, y) {
            return Some(Action::DeleteCharacter);
        }
        if self.start_button.contains(x, y) {
            return Some(Action::StartGame);
        }
        if self.exit_button.contains(x, y) {
            return Some(Action::Exit);
        }
        None
    }
    
    pub fn draw(&self, ctx: &mut ggez::Context, canvas: &mut ggez::graphics::Canvas) -> anyhow::Result<()> {
        self.create_button.draw(ctx, canvas)?;
        self.delete_button.draw(ctx, canvas)?;
        self.start_button.draw(ctx, canvas)?;
        self.exit_button.draw(ctx, canvas)?;
        Ok(())
    }
}

enum Action {
    CreateCharacter,
    DeleteCharacter,
    StartGame,
    Exit,
}
```

### 示例2：在自定义对话框中使用TextInput

```rust
// src/ecs/scenes/select_scene/dialogs/new_character_dialog.rs

use ggez::{Context, graphics::Canvas};
use crate::ecs::scenes::ui::{Button, TextInput};
use crate::graphics::{LibraryName, draw_sprite_at};

pub struct NewCharacterDialog {
    x: f32,
    y: f32,
    name_input: TextInput,
    ok_button: Button,
    cancel_button: Button,
}

impl NewCharacterDialog {
    pub fn new() -> Self {
        let (x, y) = (300.0, 200.0);
        Self {
            x,
            y,
            name_input: TextInput::new(x + 50.0, y + 50.0, 200.0, false),
            ok_button: Button::new(x + 50.0, y + 100.0, LibraryName::Prguse, 984),
            cancel_button: Button::new(x + 150.0, y + 100.0, LibraryName::Prguse, 985),
        }
    }
    
    pub fn on_char(&mut self, ch: char) {
        self.name_input.add_char(ch);
    }
    
    pub fn on_backspace(&mut self) {
        self.name_input.backspace();
    }
    
    pub fn get_character_name(&self) -> Option<String> {
        let name = self.name_input.text.trim();
        if name.is_empty() {
            None
        } else {
            Some(name.to_string())
        }
    }
    
    pub fn draw(&self, ctx: &mut Context, canvas: &mut Canvas) -> anyhow::Result<()> {
        // 绘制对话框背景
        draw_sprite_at(ctx, canvas, &LibraryName::Prguse, 950, self.x, self.y)?;
        
        // 绘制输入框和按钮
        self.name_input.draw(ctx, canvas)?;
        self.ok_button.draw(ctx, canvas)?;
        self.cancel_button.draw(ctx, canvas)?;
        
        Ok(())
    }
}
```

## 可用的UI组件

### Button - 可点击按钮
- `new(x, y, library, base_index)` - 创建按钮
- `update_hover(x, y)` - 更新悬停状态
- `contains(x, y)` - 检测点击
- `draw(ctx, canvas)` - 绘制

### TextInput - 文本输入框
- `new(x, y, width, is_password)` - 创建输入框
- `add_char(ch)` - 添加字符
- `backspace()` - 删除字符
- `set_focus(focused)` - 设置焦点
- `draw(ctx, canvas)` - 绘制（带光标闪烁）

## 导入路径

```rust
// 从任何场景模块中导入
use crate::ecs::scenes::ui::{Button, TextInput};

// 或者使用父模块相对路径（如果在scenes目录下）
use super::ui::{Button, TextInput};
```

## 优势

✅ **代码复用** - 避免重复实现相同的UI组件  
✅ **统一风格** - 所有场景使用相同的UI组件保持一致性  
✅ **易于维护** - UI组件改进自动应用到所有场景  
✅ **模块化** - 清晰的关注点分离
