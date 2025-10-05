# 下一步开发建议

## 🎯 即将开始: LoginDialog 交互功能

现在你已经有了完美显示的登录界面,下一步是让它可以交互!

---

## 优先级 1: 文本输入 (推荐首先实现)

### 1.1 AccountIDTextBox 实现

**需要实现:**
```rust
pub struct TextBox {
    pub text: String,
    pub cursor_position: usize,
    pub max_length: usize,
    pub focused: bool,
    pub password: bool, // 是否是密码框
}
```

**绘制:**
- 在对话框的输入框区域绘制文本
- C# 位置: (85, 85) - 账号框
- C# 位置: (85, 108) - 密码框
- 大小: 136x15

**实现要点:**
```rust
// 在 LoginScene::draw() 中
if let Some(text) = &self.account_text {
    let text_render = Text::new(text);
    let params = DrawParam::default()
        .dest([dialog_x + 85.0, dialog_y + 85.0])
        .color(GgezColor::WHITE);
    canvas.draw(&text_render, params);
}

// 密码框显示 ***
if let Some(password) = &self.password_text {
    let masked = "*".repeat(password.len());
    let text_render = Text::new(&masked);
    let params = DrawParam::default()
        .dest([dialog_x + 85.0, dialog_y + 108.0])
        .color(GgezColor::WHITE);
    canvas.draw(&text_render, params);
}
```

### 1.2 键盘输入处理

**在 LoginScene::on_key_down() 中:**
```rust
fn on_key_down(&mut self, key: VirtualKeyCode, modifiers: ModifiersState) {
    if !self.account_focused && !self.password_focused {
        return;
    }
    
    match key {
        VirtualKeyCode::Back => {
            // 删除字符
            if self.account_focused {
                self.account_text.pop();
            } else if self.password_focused {
                self.password_text.pop();
            }
        }
        VirtualKeyCode::Return => {
            // 回车键 - 提交登录
            self.attempt_login();
        }
        VirtualKeyCode::Tab => {
            // Tab 键 - 切换焦点
            if self.account_focused {
                self.account_focused = false;
                self.password_focused = true;
            } else {
                self.account_focused = true;
                self.password_focused = false;
            }
        }
        _ => {}
    }
}

fn on_text_input(&mut self, character: char) {
    if self.account_focused {
        if self.account_text.len() < 20 {
            self.account_text.push(character);
        }
    } else if self.password_focused {
        if self.password_text.len() < 20 {
            self.password_text.push(character);
        }
    }
}
```

---

## 优先级 2: 按钮交互

### 2.1 按钮状态管理

**需要添加到 LoginScene:**
```rust
pub struct LoginScene {
    // ... 现有字段 ...
    
    // 按钮状态
    ok_button_hovered: bool,
    account_button_hovered: bool,
    pass_button_hovered: bool,
    close_button_hovered: bool,
    
    // 输入框焦点
    account_focused: bool,
    password_focused: bool,
    
    // 输入内容
    account_text: String,
    password_text: String,
}
```

### 2.2 鼠标悬停检测

**在 LoginScene::on_mouse_move() 中:**
```rust
fn on_mouse_move(&mut self, x: i32, y: i32) {
    let center_x = 1024.0 / 2.0;
    let center_y = 768.0 / 2.0;
    let dialog_x = center_x - 164.0;
    let dialog_y = center_y - 110.0;
    
    // OK 按钮区域: (227, 81), 大小 42x42
    let ok_x = dialog_x + 227.0;
    let ok_y = dialog_y + 81.0;
    self.ok_button_hovered = 
        x as f32 >= ok_x && x as f32 <= ok_x + 42.0 &&
        y as f32 >= ok_y && y as f32 <= ok_y + 42.0;
    
    // ... 其他按钮类似 ...
}
```

### 2.3 按钮绘制 (不同状态)

**更新 LoginScene::draw():**
```rust
// OK 按钮 - 根据状态选择索引
let ok_index = if self.ok_button_hovered { 321 } else { 320 };
let _ = lib.draw_to_canvas(ctx, canvas, ok_index, 
    dialog_x + 227.0, dialog_y + 81.0, false);

// 新建账号按钮
let account_index = if self.account_button_hovered { 324 } else { 323 };
let _ = lib.draw_to_canvas(ctx, canvas, account_index,
    dialog_x + 60.0, dialog_y + 163.0, false);

// ... 其他按钮类似 ...
```

### 2.4 按钮点击处理

**在 LoginScene::on_mouse_click() 中:**
```rust
fn on_mouse_click(&mut self, x: i32, y: i32, button: MouseButton) {
    if button != MouseButton::Left {
        return;
    }
    
    let center_x = 1024.0 / 2.0;
    let center_y = 768.0 / 2.0;
    let dialog_x = center_x - 164.0;
    let dialog_y = center_y - 110.0;
    
    // 检查 OK 按钮点击
    if self.ok_button_hovered {
        self.attempt_login();
        return;
    }
    
    // 检查新建账号按钮
    if self.account_button_hovered {
        // TODO: 打开新建账号对话框
        println!("新建账号按钮被点击");
        return;
    }
    
    // 检查关闭按钮
    if self.close_button_hovered {
        // TODO: 关闭程序
        println!("关闭按钮被点击");
        return;
    }
    
    // 检查输入框点击 (设置焦点)
    let account_box_clicked = 
        x as f32 >= dialog_x + 85.0 && x as f32 <= dialog_x + 221.0 &&
        y as f32 >= dialog_y + 85.0 && y as f32 <= dialog_y + 100.0;
    
    let password_box_clicked =
        x as f32 >= dialog_x + 85.0 && x as f32 <= dialog_x + 221.0 &&
        y as f32 >= dialog_y + 108.0 && y as f32 <= dialog_y + 123.0;
    
    self.account_focused = account_box_clicked;
    self.password_focused = password_box_clicked;
}
```

---

## 优先级 3: 登录逻辑

### 3.1 实现登录验证

**添加方法:**
```rust
impl LoginScene {
    fn attempt_login(&mut self) {
        // 验证账号格式
        if self.account_text.len() < 3 {
            self.record_status("账号至少需要3个字符".to_string());
            return;
        }
        
        // 验证密码格式
        if self.password_text.len() < 5 {
            self.record_status("密码至少需要5个字符".to_string());
            return;
        }
        
        // 发送登录请求
        self.send_login_request();
    }
    
    fn send_login_request(&mut self) {
        use crate::network::NetworkCommand;
        use mir2_shared::packets::client::Login;
        
        let login_packet = Login {
            account_id: self.account_text.clone(),
            password: self.password_text.clone(),
        };
        
        // 发送到网络线程
        if let Err(e) = self.network_tx.send(NetworkCommand::Login(login_packet)) {
            self.record_status(format!("发送登录请求失败: {}", e));
        } else {
            self.record_status("正在登录...".to_string());
            self.connecting = true;
        }
    }
}
```

---

## 优先级 4: 输入框光标和选择

### 4.1 光标闪烁效果

**添加字段:**
```rust
pub struct LoginScene {
    // ... 现有字段 ...
    cursor_visible: bool,
    cursor_blink_timer: f32,
}
```

**在 update() 中:**
```rust
fn update(&mut self, delta_time: f32) {
    // 光标闪烁 (每 0.5 秒切换)
    self.cursor_blink_timer += delta_time;
    if self.cursor_blink_timer >= 0.5 {
        self.cursor_visible = !self.cursor_visible;
        self.cursor_blink_timer = 0.0;
    }
}
```

**在 draw() 中:**
```rust
// 绘制光标
if self.account_focused && self.cursor_visible {
    let text_width = measure_text(&self.account_text);
    let cursor_x = dialog_x + 85.0 + text_width;
    let cursor_y = dialog_y + 85.0;
    
    // 绘制一条竖线作为光标
    let cursor_mesh = Mesh::new_rectangle(
        ctx,
        DrawMode::fill(),
        Rect::new(cursor_x, cursor_y, 1.0, 15.0),
        Color::WHITE,
    )?;
    canvas.draw(&cursor_mesh, DrawParam::default());
}
```

---

## 优先级 5: 背景动画

### 5.1 实现登录背景动画

**添加字段:**
```rust
pub struct LoginScene {
    // ... 现有字段 ...
    background_frame: usize,
    background_timer: f32,
    background_animating: bool,
}
```

**在 initialize() 中:**
```rust
self.background_frame = 0;
self.background_timer = 0.0;
self.background_animating = true; // 或 false (C# 原版默认不动画)
```

**在 update() 中:**
```rust
fn update(&mut self, delta_time: f32) {
    // 背景动画 (100ms 每帧)
    if self.background_animating {
        self.background_timer += delta_time;
        if self.background_timer >= 0.1 {
            self.background_frame = (self.background_frame + 1) % 19; // 0-18 共 19 帧
            self.background_timer = 0.0;
        }
    }
}
```

**在 draw() 中:**
```rust
// 绘制动画背景
if let Some(lib_arc) = get_library(LibraryName::ChrSel) {
    if let Ok(mut lib) = lib_arc.try_lock() {
        let frame = if self.background_animating {
            self.background_frame
        } else {
            0 // 静态使用第一帧
        };
        let _ = lib.draw_to_canvas(ctx, canvas, frame, 0.0, 0.0, false);
    }
}
```

---

## 📋 完整实现清单

按照这个顺序实现,每完成一项就有一个可测试的功能:

- [ ] 1. 文本输入框 (1-2小时)
  - [ ] 显示文本
  - [ ] 键盘输入
  - [ ] 密码遮挡
  
- [ ] 2. 按钮交互 (1-2小时)
  - [ ] 悬停效果
  - [ ] 点击检测
  - [ ] 状态切换

- [ ] 3. 输入框焦点 (30分钟)
  - [ ] 点击设置焦点
  - [ ] Tab 切换
  - [ ] 视觉反馈

- [ ] 4. 登录逻辑 (1小时)
  - [ ] 输入验证
  - [ ] 发送请求
  - [ ] 错误处理

- [ ] 5. 光标显示 (30分钟)
  - [ ] 光标绘制
  - [ ] 闪烁动画

- [ ] 6. 背景动画 (30分钟)
  - [ ] 帧切换
  - [ ] 时间控制

---

## 💡 开发建议

1. **一次实现一个功能** - 每完成一项就测试,确保正常工作
2. **参考 C# 代码** - 遇到问题看 `Client/MirScenes/LoginScene.cs`
3. **使用 println! 调试** - 在关键点打印信息
4. **保存设置** - 记住上次登录的账号 (Settings.AccountID)

---

## 🚀 开始吧!

建议从 **文本输入框** 开始,因为这是最基础的交互功能。

需要我帮你实现其中的某一项吗?比如:
- "帮我实现文本输入"
- "帮我实现按钮悬停效果"
- "帮我实现登录逻辑"

随时告诉我! 😊
