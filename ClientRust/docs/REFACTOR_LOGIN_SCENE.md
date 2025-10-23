# LoginScene 重构指南

## 重构策略：渐进式迁移

为了降低风险，建议采用渐进式重构策略：

### 阶段1：保留原有代码，添加事件分发器（当前建议）

**优势**：
- 风险最低，可以逐步验证
- 新旧代码可以并存
- 出问题可以快速回退

**步骤**：

1. **在LoginScene中添加事件分发器字段**
```rust
pub struct LoginScene {
    // 新增：事件分发器（先不使用）
    event_dispatcher: UIEventDispatcher,
    
    // 保持原有所有字段不变
    login_dialog: LoginDialog,
    new_account_dialog: Option<NewAccountDialog>,
    // ... 其他字段
}
```

2. **在new()中初始化分发器**
```rust
impl LoginScene {
    pub fn new() -> Self {
        let mut dispatcher = UIEventDispatcher::new();
        
        // 定义UI层级
        dispatcher.add_layer(UILayer::new("background", 0));
        dispatcher.add_layer(UILayer::new("login_dialog", 10));
        
        Self {
            event_dispatcher: dispatcher,
            // ... 原有初始化代码不变
        }
    }
}
```

3. **选择一个简单的事件方法进行试点**（推荐从on_mouse_move开始）

### 阶段2：逐步替换事件处理方法

一次替换一个事件方法，验证无误后再继续下一个。

### 阶段3：清理旧代码

当所有事件方法都使用事件分发器后，进行代码清理。

---

## 详细重构步骤

### Step 1: 修改LoginScene结构体

**文件**: `src/ecs/scenes/login_scene/mod.rs`

```rust
pub struct LoginScene {
    // ============ 新增：事件分发器 ============
    event_dispatcher: UIEventDispatcher,
    
    // ============ 保持原有字段 ============
    login_dialog: LoginDialog,
    new_account_dialog: Option<NewAccountDialog>,
    change_password_dialog: Option<ChangePasswordDialog>,
    virtual_keyboard: Option<VirtualKeyboard>,
    message_box: Option<MessageBox>,
    
    // UI状态
    scale: f32,
    offset_x: f32,
    offset_y: f32,
    
    // 动画
    background_frame: u32,
    frame_timer: f32,
}
```

### Step 2: 修改new()方法

```rust
impl LoginScene {
    pub fn new() -> Self {
        // ============ 新增：初始化事件分发器 ============
        let mut event_dispatcher = UIEventDispatcher::new();
        
        // 定义静态UI层（始终存在的UI）
        event_dispatcher.add_layer(UILayer::new("background", 0));
        event_dispatcher.add_layer(UILayer::new("login_dialog", 10));
        
        // ============ 保持原有初始化代码 ============
        let login_dialog = LoginDialog::new(1280.0, 720.0);
        
        Self {
            event_dispatcher,  // 新增
            login_dialog,
            new_account_dialog: None,
            change_password_dialog: None,
            virtual_keyboard: None,
            message_box: None,
            scale: 1.0,
            offset_x: 0.0,
            offset_y: 0.0,
            background_frame: 0,
            frame_timer: 0.0,
        }
    }
}
```

### Step 3: 添加辅助方法

```rust
impl LoginScene {
    // ============ 新增：UI层管理辅助方法 ============
    
    /// 显示新建账号对话框
    fn show_new_account_dialog(&mut self) {
        tracing::info!("🆕 打开新建账号对话框");
        let base_w = 1280.0;
        let base_h = 720.0;
        let mut dialog = NewAccountDialog::new(base_w, base_h);
        dialog.show();
        self.new_account_dialog = Some(dialog);
        
        // 添加到事件分发器（模态层，阻止底层点击）
        self.event_dispatcher.add_layer(
            UILayer::new("new_account_dialog", 20).modal()
        );
    }
    
    /// 关闭新建账号对话框
    fn close_new_account_dialog(&mut self) {
        self.new_account_dialog = None;
        self.event_dispatcher.remove_layer("new_account_dialog");
    }
    
    /// 显示修改密码对话框
    fn show_change_password_dialog(&mut self, account_id: Option<String>, password: Option<String>) {
        tracing::info!("🔑 打开修改密码对话框");
        let mut dialog = ChangePasswordDialog::new();
        dialog.show(account_id, password);
        self.change_password_dialog = Some(dialog);
        
        self.event_dispatcher.add_layer(
            UILayer::new("change_password_dialog", 20).modal()
        );
    }
    
    /// 关闭修改密码对话框
    fn close_change_password_dialog(&mut self) {
        self.change_password_dialog = None;
        self.event_dispatcher.remove_layer("change_password_dialog");
    }
    
    /// 显示虚拟键盘
    fn show_virtual_keyboard(&mut self, focused: FocusedInput) {
        tracing::info!("⌨️ 打开虚拟键盘");
        let base_w = 1280.0;
        let base_h = 720.0;
        let mut keyboard = VirtualKeyboard::new(base_w, base_h);
        keyboard.show(focused);
        self.virtual_keyboard = Some(keyboard);
        
        // 虚拟键盘不是模态的，允许底层显示悬停效果
        self.event_dispatcher.add_layer(
            UILayer::new("virtual_keyboard", 30)
        );
    }
    
    /// 关闭虚拟键盘
    fn close_virtual_keyboard(&mut self) {
        self.virtual_keyboard = None;
        self.event_dispatcher.remove_layer("virtual_keyboard");
    }
    
    /// 显示消息框
    fn show_message_box(&mut self, message: &str) {
        let base_w = 1280.0;
        let base_h = 720.0;
        let mut msg_box = MessageBox::new(base_w, base_h);
        msg_box.show(message);
        self.message_box = Some(msg_box);
        
        self.event_dispatcher.add_layer(
            UILayer::new("message_box", 40).modal()
        );
    }
    
    /// 关闭消息框
    fn close_message_box(&mut self) {
        self.message_box = None;
        self.event_dispatcher.remove_layer("message_box");
    }
}
```

### Step 4: 重构on_mouse_move（试点）

**原代码**（复杂的if-else嵌套）：
```rust
fn on_mouse_move(&mut self, _ctx: &mut Context, _world: &mut World, x: f32, y: f32) -> GameResult {
    let (vx, vy) = self.screen_to_virtual(x, y);
    
    // 虚拟键盘优先级最高
    if let Some(keyboard) = &mut self.virtual_keyboard {
        keyboard.on_mouse_move(vx, vy);
        self.login_dialog.on_mouse_move(vx, vy);
        return Ok(());
    }
    
    if let Some(msg_box) = &mut self.message_box {
        msg_box.on_mouse_move(vx, vy);
        return Ok(());
    }
    
    if let Some(dialog) = &mut self.change_password_dialog {
        dialog.on_mouse_move(vx, vy);
        return Ok(());
    }
    
    if let Some(dialog) = &mut self.new_account_dialog {
        dialog.on_mouse_move(vx, vy);
        return Ok(());
    }
    
    self.login_dialog.on_mouse_move(vx, vy);
    Ok(())
}
```

**新代码**（使用事件分发器）：
```rust
fn on_mouse_move(&mut self, _ctx: &mut Context, _world: &mut World, x: f32, y: f32) -> GameResult {
    let (vx, vy) = self.screen_to_virtual(x, y);
    
    // 使用事件分发器自动处理优先级和传播
    self.event_dispatcher.dispatch_mouse_move(vx, vy, |layer_name| {
        match layer_name {
            "virtual_keyboard" => {
                if let Some(keyboard) = &mut self.virtual_keyboard {
                    keyboard.on_mouse_move(vx, vy);
                    // 返回HandledContinue允许底层UI显示悬停效果
                    EventResult::HandledContinue
                } else {
                    EventResult::Unhandled
                }
            }
            "message_box" => {
                if let Some(msg_box) = &mut self.message_box {
                    msg_box.on_mouse_move(vx, vy);
                    EventResult::Handled
                } else {
                    EventResult::Unhandled
                }
            }
            "change_password_dialog" => {
                if let Some(dialog) = &mut self.change_password_dialog {
                    dialog.on_mouse_move(vx, vy);
                    EventResult::Handled
                } else {
                    EventResult::Unhandled
                }
            }
            "new_account_dialog" => {
                if let Some(dialog) = &mut self.new_account_dialog {
                    dialog.on_mouse_move(vx, vy);
                    EventResult::Handled
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

**对比优势**：
- ✅ 不需要手动判断优先级顺序
- ✅ 事件传播逻辑清晰（HandledContinue vs Handled）
- ✅ 模态层自动阻塞（message_box会阻止底层接收事件）
- ✅ 易于添加新UI层

### Step 5: 重构on_mouse_down

**原代码中的问题**：
- 大量重复的if-else判断
- 手动return来阻止事件传播
- 业务逻辑和事件分发混在一起

**新代码**：
```rust
fn on_mouse_down(&mut self, _ctx: &mut Context, _world: &mut World, _button: MouseButton, 
                 x: f32, y: f32, network_tx: &mpsc::UnboundedSender<NetworkCommand>) -> GameResult {
    let (vx, vy) = self.screen_to_virtual(x, y);
    
    // 事件分发器自动处理模态对话框的阻塞
    self.event_dispatcher.dispatch_mouse_down(vx, vy, |layer_name| {
        match layer_name {
            "virtual_keyboard" => {
                if let Some(keyboard) = &mut self.virtual_keyboard {
                    let action = keyboard.on_mouse_down(vx, vy);
                    self.handle_virtual_keyboard_action(action);
                    EventResult::Handled
                } else {
                    EventResult::Unhandled
                }
            }
            "message_box" => {
                if let Some(_msg_box) = &mut self.message_box {
                    // 点击消息框任意位置关闭
                    self.close_message_box();
                    EventResult::Handled
                } else {
                    EventResult::Unhandled
                }
            }
            "change_password_dialog" => {
                if let Some(dialog) = &mut self.change_password_dialog {
                    let action = dialog.on_mouse_down(vx, vy);
                    self.handle_change_password_action(action, network_tx);
                    EventResult::Handled
                } else {
                    EventResult::Unhandled
                }
            }
            "new_account_dialog" => {
                if let Some(dialog) = &mut self.new_account_dialog {
                    let action = dialog.on_mouse_down(vx, vy);
                    self.handle_new_account_action(action, network_tx);
                    EventResult::Handled
                } else {
                    EventResult::Unhandled
                }
            }
            "login_dialog" => {
                let action = self.login_dialog.on_mouse_down(vx, vy);
                self.handle_login_action(action, network_tx);
                EventResult::Handled
            }
            _ => EventResult::Unhandled
        }
    });
    
    Ok(())
}

// 将业务逻辑提取到独立方法
fn handle_virtual_keyboard_action(&mut self, action: VirtualKeyboardAction) {
    match action {
        VirtualKeyboardAction::Close => {
            self.close_virtual_keyboard();
        }
        VirtualKeyboardAction::Delete => {
            if let Some(keyboard) = &self.virtual_keyboard {
                match keyboard.focused_input {
                    FocusedInput::Account => {
                        self.login_dialog.account_input.backspace();
                    }
                    FocusedInput::Password => {
                        self.login_dialog.password_input.backspace();
                    }
                }
            }
        }
        VirtualKeyboardAction::Input(ch) => {
            if ch.is_ascii_alphanumeric() {
                if let Some(keyboard) = &self.virtual_keyboard {
                    match keyboard.focused_input {
                        FocusedInput::Account => {
                            self.login_dialog.account_input.add_char(ch.to_ascii_lowercase());
                        }
                        FocusedInput::Password => {
                            self.login_dialog.password_input.add_char(ch.to_ascii_lowercase());
                        }
                    }
                }
            }
        }
        VirtualKeyboardAction::None => {}
    }
}

fn handle_login_action(&mut self, action: DialogAction, network_tx: &mpsc::UnboundedSender<NetworkCommand>) {
    match action {
        DialogAction::Login => self.submit_login(network_tx),
        DialogAction::OpenNewAccount => self.show_new_account_dialog(),
        DialogAction::OpenChangePassword => {
            let (account_id, password) = self.login_dialog.get_credentials()
                .map(|(id, pwd)| (Some(id), Some(pwd)))
                .unwrap_or((None, None));
            self.show_change_password_dialog(account_id, password);
        }
        DialogAction::OpenViewKey => {
            let focused = if self.login_dialog.account_input.focused {
                FocusedInput::Account
            } else {
                FocusedInput::Password
            };
            self.show_virtual_keyboard(focused);
        }
        DialogAction::Exit => tracing::info!("🚪 退出游戏"),
        DialogAction::None => {}
    }
}

fn handle_new_account_action(&mut self, action: NewAccountAction, network_tx: &mpsc::UnboundedSender<NetworkCommand>) {
    match action {
        NewAccountAction::Submit => {
            if let Some(dialog) = &self.new_account_dialog {
                let cmd = dialog.build_network_command();
                if let Err(e) = network_tx.send(cmd) {
                    tracing::error!("❌ 发送注册命令失败: {}", e);
                    self.show_message_box("网络错误，无法发送注册请求");
                }
            }
        }
        NewAccountAction::ValidationFailed(error_msg) => {
            self.show_message_box(&error_msg);
        }
        NewAccountAction::Cancel => {
            self.close_new_account_dialog();
        }
        NewAccountAction::None => {}
    }
}

fn handle_change_password_action(&mut self, action: ChangePasswordAction, network_tx: &mpsc::UnboundedSender<NetworkCommand>) {
    match action {
        ChangePasswordAction::Submit => {
            if let Some(dialog) = &self.change_password_dialog {
                let cmd = dialog.build_network_command();
                if let Err(e) = network_tx.send(cmd) {
                    tracing::error!("❌ 发送修改密码命令失败: {}", e);
                    self.show_message_box("网络错误，无法发送修改密码请求");
                }
            }
        }
        ChangePasswordAction::ValidationFailed(error_msg) => {
            self.show_message_box(&error_msg);
        }
        ChangePasswordAction::Cancel => {
            self.close_change_password_dialog();
        }
        ChangePasswordAction::None => {}
    }
}
```

---

## 重构检查清单

### ✅ 编译检查
- [ ] 代码编译通过
- [ ] 没有警告信息
- [ ] 所有依赖正确导入

### ✅ 功能测试
- [ ] 登录对话框正常显示和交互
- [ ] 新建账号对话框可以打开和关闭
- [ ] 修改密码对话框可以打开和关闭
- [ ] 虚拟键盘可以打开和关闭
- [ ] 虚拟键盘显示时，底层按钮悬停效果正常
- [ ] 消息框显示时，底层UI不接收点击
- [ ] 对话框打开时，底层UI不接收点击

### ✅ 事件传播测试
- [ ] 虚拟键盘：HandledContinue - 底层悬停效果正常
- [ ] 消息框：modal - 底层不接收点击
- [ ] 对话框：modal - 底层不接收点击
- [ ] 点击空白区域：焦点正确清除

### ✅ 性能测试
- [ ] FPS稳定
- [ ] 内存占用正常
- [ ] 事件响应及时

---

## 常见问题和解决方案

### Q1: 事件分发器和现有代码冲突？
**A**: 使用渐进式迁移，先添加分发器但不使用，逐步替换一个事件方法。

### Q2: 如何处理动态UI层？
**A**: 使用辅助方法（show_xxx/close_xxx）统一管理UI层的添加和移除。

### Q3: 事件传播不符合预期？
**A**: 检查EventResult返回值：
- `Handled` - 拦截事件
- `HandledContinue` - 处理但继续传播
- `Unhandled` - 跳过该层

### Q4: 模态对话框没有阻塞底层？
**A**: 确保添加层时使用`.modal()`：
```rust
UILayer::new("dialog", 20).modal()
```

### Q5: 虚拟键盘显示时底层悬停效果失效？
**A**: 虚拟键盘应该返回`EventResult::HandledContinue`而不是`Handled`。

---

## 总结

重构后的代码具有以下优势：

1. **清晰的层级管理**：Z-order自动排序
2. **明确的事件传播**：EventResult控制
3. **模态支持**：自动阻塞底层
4. **易于维护**：业务逻辑分离
5. **易于扩展**：添加新UI层不影响现有代码

建议从on_mouse_move开始试点，验证无误后再继续重构其他方法。
