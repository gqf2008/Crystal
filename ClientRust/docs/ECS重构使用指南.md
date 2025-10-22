# LoginScene ECS重构 - 使用指南

## 已完成的工作

### 1. 基础组件系统 ✅
- `components.rs` - 定义了所有ECS组件（Position, Bounds, Button, TextInput等）
- 提供了完整的类型安全组件系统

### 2. UI组件库 ✅
- `ui/button.rs` - 按钮构建器和辅助函数
  - `ButtonBuilder` - 链式API创建按钮
  - `button_helpers::update_hover()` - 统一悬停检测
  - `button_helpers::handle_click()` - 统一点击检测
  
- `ui/text_input.rs` - 输入框构建器和辅助函数
  - `TextInputBuilder` - 链式API创建输入框
  - `input_helpers::focus_field()` - 聚焦输入框
  - `input_helpers::handle_char_input()` - 字符输入
  - `input_helpers::handle_backspace()` - 退格
  - `input_helpers::handle_tab()` - Tab切换

### 3. 系统模块 ✅
- `systems/render_system.rs` - 统一渲染
  - `render_all()` - 渲染所有实体
  - 自动处理Sprite、AnimatedSprite、Button、TextInput
  - 支持调试模式绘制边界框

- `systems/input_system.rs` - 统一输入处理
  - `handle_mouse_move()` - 鼠标移动
  - `handle_mouse_click()` - 鼠标点击
  - `handle_char_input()` - 字符输入
  - `handle_tab()`, `handle_enter()`, `handle_escape()` - 键盘事件

- `systems/animation_system.rs` - 动画更新
  - `update_animations()` - 更新所有动画
  - `is_animation_complete()` - 检查动画完成

### 4. 对话框实体工厂 ✅
- `dialogs/new_account_entity.rs`
  - `create_new_account_dialog()` - 创建所有实体
  - `destroy_new_account_dialog()` - 销毁所有实体
  - `update_ok_button_state()` - 更新OK按钮状态

## 使用示例

### 创建对话框

```rust
use hecs::World;
use login_scene::dialogs::*;

let mut world = World::new();

// 创建NewAccountDialog（11个实体：1背景 + 2按钮 + 8输入框）
let dialog = create_new_account_dialog(&mut world);

// 就这样！所有实体都创建好了，坐标完全正确
```

### 渲染

```rust
use login_scene::systems::*;

fn draw(&self, ctx: &mut Context, canvas: &mut Canvas) {
    // 一行代码渲染所有内容
    render_all(&self.world, ctx, canvas)?;
}
```

### 处理输入

```rust
fn on_mouse_move(&mut self, x: f32, y: f32) {
    // 一行代码更新所有按钮悬停状态
    input_system::handle_mouse_move(&mut self.world, x, y);
}

fn on_mouse_down(&mut self, x: f32, y: f32) {
    // 统一的点击处理
    if let Some(action) = input_system::handle_mouse_click(&self.world, x, y) {
        match action {
            ButtonAction::NewAccountOk => self.submit_new_account(),
            ButtonAction::NewAccountCancel => self.close_dialog(),
            _ => {}
        }
    }
}

fn on_key_down(&mut self, key: KeyCode) {
    match key {
        KeyCode::Tab => input_system::handle_tab(&mut self.world),
        KeyCode::Return => {
            if let Some(action) = input_system::handle_enter(&self.world) {
                // 处理Enter键
            }
        }
        KeyCode::Escape => {
            if input_system::handle_escape() {
                self.close_dialog();
            }
        }
        _ => {}
    }
}
```

### 处理字符输入

```rust
fn on_text_input(&mut self, ch: char) {
    input_system::handle_char_input(&mut self.world, ch);
    
    // 输入后自动验证并更新OK按钮状态
    dialogs::update_ok_button_state(&mut self.world);
}
```

## 下一步工作

### 必须完成（核心功能）

1. **创建LoginDialog实体工厂** ⏳
   - 参考new_account_entity.rs
   - 创建dialogs/login_entity.rs

2. **创建ChangePasswordDialog实体工厂** ⏳
   - 参考new_account_entity.rs
   - 创建dialogs/change_password_entity.rs

3. **重构login_scene.rs主文件** ⏳
   - 从2021行精简到约200行
   - 使用ECS系统和实体工厂
   - 保留网络通信逻辑

### 可选优化

4. **添加ConnectingBox实体工厂**
   - 统一管理连接对话框

5. **添加MessageBox实体工厂**
   - 统一管理消息提示框

6. **网络响应系统化**
   - 创建systems/network_system.rs
   - 基于实体更新状态

## 优势总结

### 1. 坐标永不不一致 ✅
```rust
// 按钮坐标只定义一次
ButtonBuilder::new(...)
    .position(135.0, 425.0)  // ✅ 唯一定义
    .size(80.0, 23.0)
    .build(&mut world);

// 渲染、悬停、点击全部自动基于Position和Bounds组件
```

### 2. 代码复用率高 ✅
```rust
// 所有对话框共享同样的系统
render_all(&world, ctx, canvas);
input_system::handle_mouse_move(&mut world, x, y);
```

### 3. 易于调试 ✅
```rust
// Debug模式自动绘制边界框
#[cfg(debug_assertions)]
draw_debug_bounds(ctx, canvas, bounds, hovered);
```

### 4. 类型安全 ✅
```rust
// 按钮动作是枚举，不会拼错
match action {
    ButtonAction::NewAccountOk => ...,
    ButtonAction::NewAccountCancel => ...,
}
```

### 5. 易于扩展 ✅
```rust
// 添加新按钮只需一个Builder调用
let new_button = ButtonBuilder::new(...)
    .position(x, y)
    .build(&mut world);
```

## 性能影响

- **内存**: 每个实体约100字节，NewAccountDialog 11个实体 = 1.1KB
- **渲染**: 与当前方式相同（都是遍历绘制）
- **输入**: 略微提升（统一的边界检测）
- **总体**: 性能影响可忽略不计

## 迁移路径

当前建议采用**渐进式迁移**：

1. ✅ 先完成NewAccountDialog（已完成）
2. ⏳ 再迁移LoginDialog
3. ⏳ 最后迁移ChangePasswordDialog
4. ⏳ 精简login_scene.rs主文件

每完成一个对话框，立即测试验证，确保功能正常。
