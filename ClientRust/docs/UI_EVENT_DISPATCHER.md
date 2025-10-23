# UI事件分发器设计文档

## 概述

UI事件分发器是一个用于管理游戏UI事件传播和分发的系统，解决了以下问题：

1. **优先级管理混乱**：之前每个事件处理方法都要手动判断UI元素的优先级
2. **事件穿透不清晰**：不清楚哪些事件应该被拦截，哪些应该继续传播
3. **焦点管理缺失**：没有统一的焦点管理机制
4. **模态对话框处理复杂**：需要手动阻止底层UI接收事件

## 核心概念

### 1. UIComponent Trait

所有可接收事件的UI元素都应该实现这个trait：

```rust
pub trait UIComponent {
    // 基础属性
    fn contains_point(&self, x: f32, y: f32) -> bool;
    fn is_visible(&self) -> bool;
    fn is_modal(&self) -> bool;
    fn is_focusable(&self) -> bool;
    fn has_focus(&self) -> bool;
    fn set_focus(&mut self, focused: bool);
    
    // 鼠标事件
    fn on_mouse_move(&mut self, x: f32, y: f32) -> EventResult;
    fn on_mouse_down(&mut self, x: f32, y: f32) -> EventResult;
    fn on_mouse_up(&mut self, x: f32, y: f32) -> EventResult;
    fn on_click(&mut self, x: f32, y: f32) -> EventResult;
    
    // 键盘事件
    fn on_key_down(&mut self, keycode: KeyCode) -> EventResult;
    fn on_char_input(&mut self, ch: char) -> EventResult;
    
    // 焦点事件
    fn on_focus_gained(&mut self);
    fn on_focus_lost(&mut self);
}
```

### 2. EventResult

事件处理结果，控制事件传播行为：

```rust
pub enum EventResult {
    /// 事件已处理，停止传播
    Handled,
    
    /// 事件已处理，但允许继续传播
    /// 用例：虚拟键盘显示悬停效果时，底层登录对话框的按钮也需要显示悬停
    HandledContinue,
    
    /// 事件未处理，继续传播到下一层
    Unhandled,
}
```

### 3. UILayer

UI层，用于管理一组相关的UI组件：

```rust
pub struct UILayer {
    pub name: String,      // 层名称
    pub z_order: i32,      // Z轴顺序（越大越靠前）
    pub visible: bool,     // 是否可见
    pub modal: bool,       // 是否模态（阻止底层接收点击）
}
```

### 4. UIEventDispatcher

事件分发器，管理所有UI层并分发事件：

```rust
pub struct UIEventDispatcher {
    layers: Vec<UILayer>,          // 所有层（按Z-order排序）
    focused_layer: Option<usize>,  // 当前有焦点的层
}
```

## 使用方法

### 步骤1：初始化事件分发器

```rust
impl LoginScene {
    pub fn new() -> Self {
        let mut dispatcher = UIEventDispatcher::new();
        
        // 定义静态UI层（始终存在的UI）
        dispatcher.add_layer(UILayer::new("background", 0));
        dispatcher.add_layer(UILayer::new("login_dialog", 10));
        
        Self {
            event_dispatcher: dispatcher,
            login_dialog: LoginDialog::new(),
            // ... 其他字段
        }
    }
}
```

### 步骤2：动态添加/移除UI层

```rust
// 显示模态对话框
fn show_new_account_dialog(&mut self) {
    self.new_account_dialog = Some(NewAccountDialog::new());
    
    // 添加到事件系统，标记为模态
    self.event_dispatcher.add_layer(
        UILayer::new("new_account_dialog", 20).modal()
    );
}

// 关闭对话框
fn close_new_account_dialog(&mut self) {
    self.new_account_dialog = None;
    self.event_dispatcher.remove_layer("new_account_dialog");
}
```

### 步骤3：分发鼠标移动事件

```rust
fn on_mouse_move(&mut self, x: f32, y: f32) -> GameResult {
    let (vx, vy) = self.screen_to_virtual(x, y);
    
    self.event_dispatcher.dispatch_mouse_move(vx, vy, |layer_name| {
        match layer_name {
            "virtual_keyboard" => {
                if let Some(keyboard) = &mut self.virtual_keyboard {
                    keyboard.on_mouse_move(vx, vy);
                    // 允许底层UI显示悬停效果
                    EventResult::HandledContinue
                } else {
                    EventResult::Unhandled
                }
            }
            "login_dialog" => {
                self.login_dialog.on_mouse_move(vx, vy);
                EventResult::Handled
            }
            _ => EventResult::Unhandled
        }
    });
    
    Ok(())
}
```

### 步骤4：分发鼠标点击事件

```rust
fn on_mouse_down(&mut self, x: f32, y: f32) -> GameResult {
    let (vx, vy) = self.screen_to_virtual(x, y);
    
    // 事件分发器自动处理模态对话框的阻塞和焦点设置
    self.event_dispatcher.dispatch_mouse_down(vx, vy, |layer_name| {
        match layer_name {
            "new_account_dialog" => {
                if let Some(dialog) = &mut self.new_account_dialog {
                    let action = dialog.on_mouse_down(vx, vy);
                    self.handle_new_account_action(action);
                    EventResult::Handled
                } else {
                    EventResult::Unhandled
                }
            }
            "login_dialog" => {
                let action = self.login_dialog.on_mouse_down(vx, vy);
                self.handle_login_action(action);
                EventResult::Handled
            }
            _ => EventResult::Unhandled
        }
    });
    
    Ok(())
}
```

### 步骤5：分发键盘事件（仅发送给有焦点的层）

```rust
fn on_key_down(&mut self, keycode: KeyCode) -> GameResult {
    self.event_dispatcher.dispatch_key_down(keycode, |layer_name| {
        match layer_name {
            "new_account_dialog" => {
                if let Some(dialog) = &mut self.new_account_dialog {
                    dialog.on_key_down(keycode);
                    EventResult::Handled
                } else {
                    EventResult::Unhandled
                }
            }
            _ => EventResult::Unhandled
        }
    });
    
    Ok(())
}
```

## 事件传播机制

### 鼠标移动事件

```
高Z-order → 低Z-order

虚拟键盘(30) → HandledContinue → 继续传播
消息框(25)   → Handled          → 停止传播
对话框(20)   → (不会执行)
登录框(10)   → (不会执行)
```

### 鼠标点击事件

```
高Z-order → 低Z-order

模态对话框(20, modal) → Handled → 停止传播，底层不会收到点击
登录框(10)            → (被阻塞)
```

### 键盘事件

```
只发送给当前有焦点的层

有焦点的对话框 → 处理键盘输入
其他层         → 不接收键盘事件
```

## 优势对比

### 之前的代码

```rust
fn on_mouse_move(&mut self, x: f32, y: f32) -> GameResult {
    let (vx, vy) = self.screen_to_virtual(x, y);
    
    // 手动管理优先级，容易出错
    if let Some(keyboard) = &mut self.virtual_keyboard {
        keyboard.on_mouse_move(vx, vy);
        // 手动决定是否继续传播
        self.login_dialog.on_mouse_move(vx, vy);
        return Ok(());
    }
    
    if let Some(msg_box) = &mut self.message_box {
        msg_box.on_mouse_move(vx, vy);
        return Ok(());  // 手动return阻止传播
    }
    
    if let Some(dialog) = &mut self.change_password_dialog {
        dialog.on_mouse_move(vx, vy);
        return Ok(());
    }
    
    // ... 更多if-else嵌套
    
    Ok(())
}
```

**问题**：
- 每个事件方法都要重复优先级判断
- 添加新UI需要修改多处代码
- 事件传播逻辑不清晰
- 难以维护和测试

### 使用事件分发器后

```rust
fn on_mouse_move(&mut self, x: f32, y: f32) -> GameResult {
    let (vx, vy) = self.screen_to_virtual(x, y);
    
    // 统一的事件分发
    self.event_dispatcher.dispatch_mouse_move(vx, vy, |layer_name| {
        // 每个层独立处理，清晰明了
        match layer_name {
            "virtual_keyboard" => self.handle_keyboard_hover(vx, vy),
            "message_box" => self.handle_message_box_hover(vx, vy),
            "change_password_dialog" => self.handle_dialog_hover(vx, vy),
            _ => EventResult::Unhandled
        }
    });
    
    Ok(())
}
```

**优势**：
- ✅ 自动管理优先级（Z-order）
- ✅ 清晰的事件传播控制（EventResult）
- ✅ 易于添加新UI（只需添加layer）
- ✅ 代码更简洁、更易测试

## Z-order建议

推荐的Z-order层级规划：

```
0-9:    背景和静态UI
10-19:  主要对话框（登录、角色选择等）
20-29:  次要对话框（新建账号、修改密码等）
30-39:  辅助UI（虚拟键盘、tooltip等）
40-49:  消息框和确认框
50+:    系统级UI（加载屏幕、错误提示等）
```

## 最佳实践

### 1. 为每个UI组件定义明确的action

```rust
pub enum DialogAction {
    None,
    Submit,
    Cancel,
}

impl Dialog {
    fn on_mouse_down(&mut self, x: f32, y: f32) -> DialogAction {
        // 返回action而不是直接执行业务逻辑
        if self.ok_button.contains(x, y) {
            DialogAction::Submit
        } else {
            DialogAction::None
        }
    }
}
```

### 2. 在Scene中集中处理业务逻辑

```rust
fn handle_dialog_action(&mut self, action: DialogAction) {
    match action {
        DialogAction::Submit => {
            // 集中处理业务逻辑
            self.submit_form();
            self.close_dialog();
        }
        DialogAction::Cancel => self.close_dialog(),
        DialogAction::None => {}
    }
}
```

### 3. 使用modal标记阻止底层交互

```rust
// 确认对话框应该阻止底层点击
self.event_dispatcher.add_layer(
    UILayer::new("confirm_dialog", 40).modal()
);

// 虚拟键盘不应该阻止底层悬停效果
self.event_dispatcher.add_layer(
    UILayer::new("virtual_keyboard", 30)  // 不是modal
);
```

## 未来扩展

### 1. 拖拽支持

```rust
trait UIComponent {
    fn on_drag_start(&mut self, x: f32, y: f32) -> EventResult;
    fn on_drag_move(&mut self, dx: f32, dy: f32) -> EventResult;
    fn on_drag_end(&mut self, x: f32, y: f32) -> EventResult;
}
```

### 2. 动画和过渡

```rust
impl UILayer {
    pub fn show_animated(&mut self, duration: Duration) {
        // 淡入动画
    }
    
    pub fn hide_animated(&mut self, duration: Duration) {
        // 淡出动画
    }
}
```

### 3. 触摸支持（移动端）

```rust
impl UIEventDispatcher {
    pub fn dispatch_touch_start(&mut self, touches: &[Touch]) { ... }
    pub fn dispatch_touch_move(&mut self, touches: &[Touch]) { ... }
    pub fn dispatch_touch_end(&mut self, touches: &[Touch]) { ... }
    pub fn dispatch_pinch(&mut self, scale: f32) { ... }
}
```

### 4. 调试工具

```rust
impl UIEventDispatcher {
    pub fn debug_draw(&self, canvas: &mut Canvas) {
        // 绘制每个层的边界框
        // 显示Z-order和层名称
        // 高亮当前有焦点的层
    }
    
    pub fn debug_print_hierarchy(&self) {
        // 打印UI层级树
    }
}
```

## 总结

UI事件分发器提供了一个清晰、可维护的方式来管理UI事件。通过引入层级管理、事件传播控制和焦点管理，大大简化了UI事件处理逻辑。

**核心优势**：
- 🎯 自动管理UI优先级
- 🔄 清晰的事件传播机制
- 🎛️ 统一的焦点管理
- 🚫 模态对话框自动阻塞
- 🧪 易于测试和调试
- 🔧 易于扩展和维护
