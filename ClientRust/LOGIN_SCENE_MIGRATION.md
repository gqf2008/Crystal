# LoginScene 完整业务逻辑移植计划

## 当前状态 ✅
已完成基础功能：
- ✅ LoginDialog - 登录对话框（账号/密码输入）
- ✅ MessageBox - 消息框
- ✅ NewAccountDialog - 新建账号对话框（完整）
- ✅ ChangePasswordDialog - 修改密码对话框（完整）
- ✅ 基础网络事件处理（Connected, Disconnected, LoginResponse, LoginBanned）
- ✅ 背景动画
- ✅ 基础键盘鼠标输入

## 还需移植的功能 🔧

### 1. LoginScene结构扩展
需要添加的字段：
```rust
pub struct LoginScene {
    // 已有字段...
    connecting: bool,
    login_enabled: bool,
    background_frame: usize,
    animation_timer: f32,
    animation_paused: bool,
    login_dialog: LoginDialog,
    message_box: Option<MessageBox>,
    status_log: Vec<String>,
    
    // ❌ 需要添加：
    version_checked: bool,          // 客户端版本是否已验证
    version_valid: bool,             // 版本是否有效
    require_password_change: bool,   // 是否需要修改密码
    ready_for_character_select: bool,// 是否准备好进入选人界面
    
    // Ban信息
    login_ban_info: Option<BanInfo>,
    password_change_ban_info: Option<BanInfo>,
    
    // 角色列表（登录成功后）
    characters: Vec<CharacterSummary>,
    
    // 新建账号和修改密码对话框
    new_account_dialog: Option<NewAccountDialog>,
    change_password_dialog: Option<ChangePasswordDialog>,
    
    // 结果追踪
    last_login_result: Option<u8>,
    last_new_account_result: Option<u8>,
    last_change_password_result: Option<u8>,
}
```

### 2. BanInfo 结构
```rust
#[derive(Debug, Clone)]
pub struct BanInfo {
    pub reason: String,
    pub expiry_date: i64,
}
```

### 3. 完整网络事件处理
需要在 `handle_network_event` 中添加：
```rust
GameEvent::ClientVersionResponse { result } => {
    // 处理版本验证响应
}
GameEvent::LoginSuccess { characters } => {
    // 保存角色列表，GameApp会切换到SelectScene
}
GameEvent::NewAccountResponse { result } => {
    // 处理新建账号响应
}
GameEvent::ChangePasswordResponse { result } => {
    // 处理修改密码响应
}
GameEvent::ChangePasswordBanned { reason, expiry_date } => {
    // 处理修改密码封禁
}
```

### 4. 辅助方法
需要添加的静态方法：
```rust
impl LoginScene {
    fn login_result_message(result: u8) -> Option<&'static str> { ... }
    fn new_account_result_message(result: u8) -> Option<&'static str> { ... }
    fn change_password_result_message(result: u8) -> Option<&'static str> { ... }
    fn ban_message(prefix: &str, info: &BanInfo) -> String { ... }
    fn ban_duration_components(expiry_ticks: i64) -> Option<(i64, i64, i64)> { ... }
}
```

### 5. 对话框管理方法
```rust
impl LoginScene {
    pub fn open_new_account_dialog(&mut self) {
        self.new_account_dialog = Some(NewAccountDialog::new());
        self.new_account_dialog.as_mut().unwrap().show();
    }
    
    pub fn open_change_password_dialog(&mut self, autofill_id: Option<String>, autofill_password: Option<String>) {
        self.change_password_dialog = Some(ChangePasswordDialog::new());
        self.change_password_dialog.as_mut().unwrap().show(autofill_id, autofill_password);
    }
    
    pub fn close_new_account_dialog(&mut self) {
        self.new_account_dialog = None;
    }
    
    pub fn close_change_password_dialog(&mut self) {
        self.change_password_dialog = None;
    }
    
    pub fn submit_new_account(&mut self, network_tx: &mpsc::UnboundedSender<NetworkCommand>) {
        if let Some(dialog) = &self.new_account_dialog {
            if dialog.can_submit() {
                let cmd = NetworkCommand::NewAccount {
                    account_id: dialog.registration.account_id.clone(),
                    password: dialog.registration.password.clone(),
                    email: dialog.registration.email.clone(),
                    // ... 其他字段
                };
                let _ = network_tx.send(cmd);
                self.connecting = true;
            }
        }
    }
    
    pub fn submit_change_password(&mut self, network_tx: &mpsc::UnboundedSender<NetworkCommand>) {
        if let Some(dialog) = &self.change_password_dialog {
            if dialog.can_submit() {
                let cmd = NetworkCommand::ChangePassword {
                    account_id: dialog.account_id.clone(),
                    current_password: dialog.current_password.clone(),
                    new_password: dialog.new_password.clone(),
                };
                let _ = network_tx.send(cmd);
                self.connecting = true;
            }
        }
    }
}
```

### 6. 更新Scene trait实现

#### update方法
需要更新所有对话框：
```rust
fn update(&mut self, ctx: &mut Context, ...) {
    let dt = ctx.time.delta().as_secs_f32();
    
    // 背景动画
    if !self.animation_paused {
        self.animation_timer += dt;
        if self.animation_timer >= 0.1 {
            self.background_frame = (self.background_frame + 1) % 10;
            self.animation_timer = 0.0;
        }
    }
    
    // 更新所有对话框
    self.login_dialog.update(dt);
    if let Some(dialog) = &mut self.new_account_dialog {
        dialog.update(dt);
    }
    if let Some(dialog) = &mut self.change_password_dialog {
        dialog.update(dt);
    }
    
    Ok(None)
}
```

#### draw方法
需要绘制所有对话框：
```rust
fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas, _world: &World) {
    let bg_index = 1740 + self.background_frame as i32;
    let _ = draw_sprite_at(ctx, canvas, &LibraryName::Prguse, bg_index, 0.0, 0.0);
    
    // 绘制登录对话框
    let _ = self.login_dialog.draw(ctx, canvas);
    
    // 绘制新建账号对话框
    if let Some(dialog) = &self.new_account_dialog {
        let _ = dialog.draw(ctx, canvas);
    }
    
    // 绘制修改密码对话框
    if let Some(dialog) = &self.change_password_dialog {
        let _ = dialog.draw(ctx, canvas);
    }
    
    // 绘制消息框（最上层）
    if let Some(msg_box) = &self.message_box {
        let _ = msg_box.draw(ctx, canvas);
    }
    
    Ok(())
}
```

#### on_mouse_down方法
需要处理所有对话框：
```rust
fn on_mouse_down(&mut self, ...) {
    // 1. 消息框优先级最高
    if let Some(msg_box) = &mut self.message_box {
        if msg_box.on_mouse_down(x, y) {
            self.message_box = None;
        }
        return Ok(());
    }
    
    // 2. 新建账号对话框
    if let Some(dialog) = &mut self.new_account_dialog {
        use dialogs::NewAccountAction;
        match dialog.on_mouse_down(x, y) {
            NewAccountAction::Submit => {
                self.submit_new_account(network_tx);
                return Ok(());
            }
            NewAccountAction::Cancel => {
                self.close_new_account_dialog();
                return Ok(());
            }
            NewAccountAction::None => {
                return Ok(()); // 点击在对话框内，不处理其他
            }
        }
    }
    
    // 3. 修改密码对话框
    if let Some(dialog) = &mut self.change_password_dialog {
        use dialogs::ChangePasswordAction;
        match dialog.on_mouse_down(x, y) {
            ChangePasswordAction::Submit => {
                self.submit_change_password(network_tx);
                return Ok(());
            }
            ChangePasswordAction::Cancel => {
                self.close_change_password_dialog();
                return Ok(());
            }
            ChangePasswordAction::None => {
                return Ok(()); // 点击在对话框内，不处理其他
            }
        }
    }
    
    // 4. 登录对话框
    let action = self.login_dialog.on_mouse_down(x, y);
    match action {
        DialogAction::Login => self.submit_login(network_tx),
        DialogAction::OpenNewAccount => self.open_new_account_dialog(),
        DialogAction::OpenChangePassword => {
            let (account, password) = self.login_dialog.get_credentials_raw();
            self.open_change_password_dialog(Some(account), Some(password));
        }
        DialogAction::Exit => {
            tracing::info!("🚪 退出游戏");
            // TODO: 发送退出事件
        }
        DialogAction::None => {}
    }
    
    Ok(())
}
```

#### on_key_down方法
需要处理所有对话框的键盘输入：
```rust
fn on_key_down(&mut self, ...) {
    if let ggez::winit::event::KeyEvent {
        physical_key: PhysicalKey::Code(keycode),
        text,
        ..
    } = input.event
    {
        // 1. 消息框处理Escape/Enter关闭
        if self.message_box.is_some() {
            if matches!(keycode, KeyCode::Escape | KeyCode::Enter) {
                self.message_box = None;
            }
            return Ok(None);
        }
        
        // 2. 新建账号对话框
        if let Some(dialog) = &mut self.new_account_dialog {
            match keycode {
                KeyCode::Tab => dialog.on_tab(),
                KeyCode::Enter => {
                    if dialog.can_submit() {
                        self.submit_new_account(network_tx);
                    }
                }
                KeyCode::Backspace => dialog.on_backspace(),
                KeyCode::Escape => {
                    self.close_new_account_dialog();
                }
                _ => {
                    if let Some(text) = text {
                        for ch in text.chars() {
                            dialog.on_char(ch);
                        }
                    }
                }
            }
            return Ok(None);
        }
        
        // 3. 修改密码对话框
        if let Some(dialog) = &mut self.change_password_dialog {
            match keycode {
                KeyCode::Tab => dialog.on_tab(),
                KeyCode::Enter => {
                    if dialog.can_submit() {
                        self.submit_change_password(network_tx);
                    }
                }
                KeyCode::Backspace => dialog.on_backspace(),
                KeyCode::Escape => {
                    self.close_change_password_dialog();
                }
                _ => {
                    if let Some(text) = text {
                        for ch in text.chars() {
                            dialog.on_char(ch);
                        }
                    }
                }
            }
            return Ok(None);
        }
        
        // 4. 登录对话框
        match keycode {
            KeyCode::Tab => self.login_dialog.on_tab(),
            KeyCode::Enter => {
                let action = self.login_dialog.on_enter();
                if action == DialogAction::Login {
                    self.submit_login(network_tx);
                }
            }
            KeyCode::Backspace => self.login_dialog.on_backspace(),
            _ => {
                if let Some(text) = text {
                    for ch in text.chars() {
                        self.login_dialog.on_char(ch);
                    }
                }
            }
        }
    }
    
    Ok(None)
}
```

## 实现优先级

1. ✅ **高** - 新建账号和修改密码对话框（已完成）
2. **高** - 扩展LoginScene结构体字段
3. **高** - 完善网络事件处理逻辑
4. **中** - 添加对话框管理方法
5. **中** - 更新Scene trait的事件处理方法
6. **低** - 辅助方法（ban_message等）

## 测试计划

1. ✅ 编译通过
2. 运行游戏，测试登录对话框
3. 测试新建账号对话框打开/关闭
4. 测试修改密码对话框打开/关闭
5. 测试网络事件（连接、登录响应、ban等）
6. 测试键盘输入（Tab切换、Enter提交、Escape关闭）
7. 测试鼠标交互（点击按钮、输入框等）

## 代码统计

- 原始 login_scene.rs: **2192行**
- 当前 ECS版本: **208行** (mod.rs)
- 新增对话框:
  - new_account.rs: ~400行
  - change_password.rs: ~350行
- 预估最终: **~1000行** (相比原版减少50%以上，因为UI组件复用)
