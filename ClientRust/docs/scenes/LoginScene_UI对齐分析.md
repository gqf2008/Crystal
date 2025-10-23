# LoginScene UI纹理与位置对齐分析

**分析日期**: 2025-10-23  
**参考标准**: `Client/MirScenes/LoginScene.cs` (C# 原版)  
**分析对象**: `ClientRust/src/ecs/scenes/login_scene/` (Rust 移植版)

---

## 一、LoginDialog 组件详细对比

### 1.1 对话框主体

| 项目 | C# 原版 | Rust 当前实现 | 差异 | 优先级 |
|------|---------|--------------|------|--------|
| **背景纹理** | Index=1084, Prguse | Index=944, Prguse | ❌ **索引错误** | 🔴 P0 |
| **尺寸** | 328x220 | 未明确 | ⚠️ **需确认** | 🟡 P1 |
| **位置计算** | `(ScreenW-328)/2, (ScreenH-220)/2` | 固定 `(274, 244)` | ❌ **需动态居中** | 🔴 P0 |
| **PixelDetect** | false | - | ⚠️ **未知** | 🟢 P2 |

**C# 代码**:
```csharp
Index = 1084;
Library = Libraries.Prguse;
Location = new Point((Settings.ScreenWidth - Size.Width)/2, 
                     (Settings.ScreenHeight - Size.Height)/2);
Size = new Size(328, 220);
```

**Rust 当前代码**:
```rust
let (x, y) = (274.0, 244.0);  // 硬编码!
// ...
draw_sprite_at(ctx, canvas, &LibraryName::Prguse, 944, self.x, self.y)?;  // 错误索引!
```

---

### 1.2 标签组件

| 组件 | C# 原版 | Rust 实现 | 状态 |
|------|---------|-----------|------|
| **Title标签** | Index=30, Title库 | ❌ 未绘制 | ❌ **完全缺失** |
| Title位置 | 居中: `(Size.Width - TitleLabel.Size.Width)/2, 12` | - | - |
| **AccountID标签** | Index=31, Title库, (52, 83) | ❌ 未绘制 | ❌ **完全缺失** |
| **Password标签** | Index=32, Title库, (43, 105) | ❌ 未绘制 | ❌ **完全缺失** |

**C# 代码**:
```csharp
TitleLabel = new MirImageControl {
    Index = 30,
    Library = Libraries.Title,
    Parent = this,
};
TitleLabel.Location = new Point((Size.Width - TitleLabel.Size.Width)/2, 12);

AccountIDLabel = new MirImageControl {
    Index = 31,
    Library = Libraries.Title,
    Parent = this,
    Location = new Point(52, 83),
};

PassLabel = new MirImageControl {
    Index = 32,
    Library = Libraries.Title,
    Parent = this,
    Location = new Point(43, 105)
};
```

---

### 1.3 按钮组件对比

#### OK按钮

| 项目 | C# 原版 | Rust 实现 | 差异 |
|------|---------|-----------|------|
| 纹理库 | Title | Prguse | ❌ **库错误** |
| Normal索引 | 320 | 984 | ❌ **索引错误** |
| Hover索引 | 321 | - | ⚠️ **可能缺失** |
| Pressed索引 | 322 | - | ⚠️ **可能缺失** |
| 位置 | (227, 81) | (87, 185) | ❌ **位置错误** |
| 尺寸 | 42x42 | - | ⚠️ **未明确** |

**C# 代码**:
```csharp
OKButton = new MirButton {
    Enabled = false,
    Size = new Size(42, 42),
    HoverIndex = 321,
    Index = 320,
    Library = Libraries.Title,
    Location = new Point(227, 81),
    Parent = this,
    PressedIndex = 322
};
```

**Rust 当前代码**:
```rust
ok_button: Button::new(x + 87.0, y + 185.0, LibraryName::Prguse, 984),
```

#### New Account按钮

| 项目 | C# 原版 | Rust 实现 | 差异 |
|------|---------|-----------|------|
| 纹理库 | Title | Prguse | ❌ **库错误** |
| Normal索引 | 323 | 982 | ❌ **索引错误** |
| Hover索引 | 324 | - | ⚠️ **可能缺失** |
| Pressed索引 | 325 | - | ⚠️ **可能缺失** |
| 位置 | (60, 163) | (193, 185) | ❌ **位置错误** |

**C# 代码**:
```csharp
AccountButton = new MirButton {
    HoverIndex = 324,
    Index = 323,
    Library = Libraries.Title,
    Location = new Point(60, 163),
    Parent = this,
    PressedIndex = 325,
};
```

#### Change Password按钮

| 项目 | C# 原版 | Rust 实现 | 差异 |
|------|---------|-----------|------|
| 纹理库 | Title | Prguse | ❌ **库错误** |
| Normal索引 | 326 | 986 | ❌ **索引错误** |
| Hover索引 | 327 | - | ⚠️ **可能缺失** |
| Pressed索引 | 328 | - | ⚠️ **可能缺失** |
| 位置 | (166, 163) | (299, 185) | ❌ **位置错误** |

**C# 代码**:
```csharp
PassButton = new MirButton {
    HoverIndex = 327,
    Index = 326,
    Library = Libraries.Title,
    Location = new Point(166, 163),
    Parent = this,
    PressedIndex = 328,
};
```

#### View Key按钮 (虚拟键盘)

| 项目 | C# 原版 | Rust 实现 | 状态 |
|------|---------|-----------|------|
| 功能 | 打开虚拟键盘输入密码 | ❌ 未实现 | ❌ **完全缺失** |
| 纹理库 | Title | - | - |
| Normal索引 | 332 | - | - |
| Hover索引 | 333 | - | - |
| Pressed索引 | 334 | - | - |
| 位置 | (60, 189) | - | - |

**C# 代码**:
```csharp
ViewKeyButton = new MirButton {
    HoverIndex = 333,
    Index = 332,
    Library = Libraries.Title,
    Location = new Point(60, 189),
    Parent = this,
    PressedIndex = 334,
};
```

#### Close按钮

| 项目 | C# 原版 | Rust 实现 | 差异 |
|------|---------|-----------|------|
| 纹理库 | Title | Prguse | ❌ **库错误** |
| Normal索引 | 329 | 360 | ❌ **索引错误** |
| Hover索引 | 330 | - | ⚠️ **可能缺失** |
| Pressed索引 | 331 | - | ⚠️ **可能缺失** |
| 位置 | (166, 189) | (427, 12) | ❌ **位置错误** |

**C# 代码**:
```csharp
CloseButton = new MirButton {
    HoverIndex = 330,
    Index = 329,
    Library = Libraries.Title,
    Location = new Point(166, 189),
    Parent = this,
    PressedIndex = 331,
};
```

---

### 1.4 输入框组件

#### 账号输入框

| 项目 | C# 原版 | Rust 实现 | 差异 |
|------|---------|-----------|------|
| 位置 | (85, 85) | (115, 86) | ⚠️ **X偏移+30, Y偏移+1** |
| 尺寸 | 136x15 | 180x20 | ⚠️ **宽+44, 高+5** |
| MaxLength | Globals.MaxAccountIDLength | 20 | ⚠️ **需确认全局配置** |
| Password模式 | false | false | ✅ **一致** |

**C# 代码**:
```csharp
AccountIDTextBox = new MirTextBox {
    Location = new Point(85, 85),
    Parent = this,
    Size = new Size(136, 15),
    MaxLength = Globals.MaxAccountIDLength
};
```

**Rust 当前代码**:
```rust
account_input: TextInput::new(x + 115.0, y + 86.0, 180.0, 20),
```

#### 密码输入框

| 项目 | C# 原版 | Rust 实现 | 差异 |
|------|---------|-----------|------|
| 位置 | (85, 108) | (115, 132) | ⚠️ **X偏移+30, Y偏移+24** |
| 尺寸 | 136x15 | 180x20 | ⚠️ **宽+44, 高+5** |
| MaxLength | Globals.MaxPasswordLength | - | ⚠️ **需确认** |
| Password模式 | true | true | ✅ **一致** |

**C# 代码**:
```csharp
PasswordTextBox = new MirTextBox {
    Location = new Point(85, 108),
    Parent = this,
    Password = true,
    Size = new Size(136, 15),
    MaxLength = Globals.MaxPasswordLength
};
```

**Rust 当前代码**:
```rust
password_input: TextInput::new(x + 115.0, y + 132.0, 180.0, 20).password(),
```

---

## 二、背景动画对比

| 项目 | C# 原版 | Rust 实现 | 状态 |
|------|---------|-----------|------|
| 纹理库 | Libraries.ChrSel | ❓ 未明确 | ⚠️ **需确认** |
| 起始索引 | Index=0 | background_frame=0 | ✅ **一致** |
| 帧数 | AnimationCount=19 | 检测19帧 | ✅ **一致** |
| 帧延迟 | AnimationDelay=100ms | animation_timer >= 0.1 | ✅ **一致** |
| 初始状态 | Animated=false | animation_paused=true | ✅ **一致** |
| 循环播放 | Loop=false | 单次播放 | ✅ **一致** |
| 触发时机 | LoginSuccess设置Animated=true | animation_paused=false | ✅ **一致** |
| 结束检测 | AfterAnimation事件 | background_frame >= 19 | ✅ **一致** |

**C# 代码**:
```csharp
_background = new MirAnimatedControl {
    Animated = false,
    AnimationCount = 19,
    AnimationDelay = 100,
    Index = 0,
    Library = Libraries.ChrSel,
    Loop = false,
    Parent = this,
};

// 登录成功后触发
_background.Animated = true;
_background.AfterAnimation += (o, e) => {
    Dispose();
    ActiveScene = new SelectScene(p.Characters);
};
```

---

## 三、场景级UI元素

### 3.1 版本标签

| 项目 | C# 原版 | Rust 实现 | 状态 |
|------|---------|-----------|------|
| 类型 | MirLabel | ❌ 未实现 | ❌ **缺失** |
| 位置 | (5, ScreenHeight - 20) | - | - |
| 内容格式 | `"Build: {Codename}.{Debug/Release}.{Version}"` | - | - |
| AutoSize | true | - | - |
| 背景色 | `Color.FromArgb(200, 50, 50, 50)` (半透明灰) | - | - |
| 边框 | true, Black | - | - |

**C# 代码**:
```csharp
Version = new MirLabel {
    AutoSize = true,
    BackColour = Color.FromArgb(200, 50, 50, 50),
    Border = true,
    BorderColour = Color.Black,
    Location = new Point(5, Settings.ScreenHeight - 20),
    Parent = _background,
    Text = string.Format("Build: {0}.{1}.{2}", 
                        Globals.ProductCodename, 
                        Settings.UseTestConfig ? "Debug" : "Release", 
                        Application.ProductVersion),
};
```

### 3.2 测试标签 (Debug模式)

| 项目 | C# 原版 | Rust 实现 | 状态 |
|------|---------|-----------|------|
| 类型 | MirImageControl | ❌ 未实现 | ❌ **缺失** |
| 纹理 | Index=79, Prguse | - | - |
| 位置 | (ScreenWidth - 116, 10) | - | - |
| 显示条件 | Settings.UseTestConfig | - | - |

**C# 代码**:
```csharp
TestLabel = new MirImageControl {
    Index = 79,
    Library = Libraries.Prguse,
    Parent = this,
    Location = new Point(Settings.ScreenWidth - 116, 10),
    Visible = Settings.UseTestConfig
};
```

---

## 四、连接提示框

| 项目 | C# 原版 | Rust 实现 | 状态 |
|------|---------|-----------|------|
| 消息 | "Attempting to connect to the server." | ✅ 已实现 | ✅ **对齐** |
| 按钮类型 | Cancel | ✅ Cancel | ✅ **一致** |
| Cancel行为 | `Program.Form.Close()` | 退出程序 | ✅ **一致** |
| 动态更新 | 显示连接次数 | ✅ connect_attempts | ✅ **一致** |
| 更新时机 | `Process()`检查`!Network.Connected` | ⚠️ 待确认 | ⚠️ **需验证** |

**C# 代码**:
```csharp
_connectBox = new MirMessageBox("Attempting to connect to the server.", 
                               MirMessageBoxButtons.Cancel);
_connectBox.CancelButton.Click += (o, e) => Program.Form.Close();

// Process()中动态更新
if (!Network.Connected && _connectBox.Label != null)
    _connectBox.Label.Text = string.Format(GameLanguage.AttemptingConnect,
                                          "\n\n", 
                                          Network.ConnectAttempt);
```

---

## 五、音效与音乐

### 5.1 背景音乐

| 项目 | C# 原版 | Rust 实现 | 状态 |
|------|---------|-----------|------|
| 音乐文件 | SoundList.IntroMusic | ❌ 未播放 | ❌ **缺失** |
| 循环播放 | true | - | - |
| 停止时机 | Disposing事件 | - | ❌ **缺失** |

**C# 代码**:
```csharp
public LoginScene() {
    SoundManager.PlayMusic(SoundList.IntroMusic, true);
    Disposing += (o, e) => SoundManager.StopMusic();
    // ...
}
```

### 5.2 登录成功音效

| 项目 | C# 原版 | Rust 实现 | 状态 |
|------|---------|-----------|------|
| 音效文件 | SoundList.LoginEffect | ❌ 未播放 | ❌ **缺失** |
| 触发时机 | LoginSuccess时 | - | - |

**C# 代码**:
```csharp
private void Login(S.LoginSuccess p) {
    // ...
    SoundManager.PlaySound(SoundList.LoginEffect);
    _background.Animated = true;
    // ...
}
```

---

## 六、严重问题汇总

### 🔴 P0 - 关键错误（必须立即修复）

1. **LoginDialog背景纹理索引错误**
   - 错误: Index=944
   - 正确: Index=1084
   - 影响: 显示错误的对话框背景图

2. **所有按钮纹理库错误**
   - 错误: 使用Prguse库
   - 正确: 应使用Title库
   - 影响: 所有按钮显示错误

3. **所有按钮索引全部错误**
   - OK: 984 → 应为320/321/322
   - NewAccount: 982 → 应为323/324/325
   - ChangePass: 986 → 应为326/327/328
   - Close: 360 → 应为329/330/331

4. **所有按钮位置全部错误**
   - OK: (87, 185) → 应为(227, 81)
   - NewAccount: (193, 185) → 应为(60, 163)
   - ChangePass: (299, 185) → 应为(166, 163)
   - Close: (427, 12) → 应为(166, 189)

5. **对话框位置硬编码**
   - 错误: 固定(274, 244)
   - 正确: 应根据屏幕尺寸动态居中

### 🟡 P1 - 重要缺失

6. **缺失3个标签组件**
   - Title标签 (Index=30, 居中)
   - AccountID标签 (Index=31)
   - Password标签 (Index=32)

7. **输入框位置和尺寸偏差**
   - 账号框: 位置偏移(+30, +1), 尺寸偏差(+44, +5)
   - 密码框: 位置偏移(+30, +24), 尺寸偏差(+44, +5)

8. **ViewKey按钮完全缺失**
   - 无虚拟键盘功能
   - Index=332/333/334, 位置(60, 189)

9. **版本标签未显示**
   - 位置: (5, screen_height - 20)
   - 需要半透明背景和边框

10. **TestLabel未实现**
    - Index=79, 位置(screen_width - 116, 10)
    - Debug模式显示

### 🟢 P2 - 次要问题

11. **背景动画纹理库未明确**
    - 应确认使用Libraries.ChrSel

12. **音效系统未集成**
    - 背景音乐未播放
    - 登录成功音效未播放

---

## 七、修复建议

### 优先级1: 修复LoginDialog所有纹理和位置

```rust
// login.rs - 完全重构

pub struct LoginDialog {
    // 对话框参数
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    visible: bool,
    
    // 标签
    title_pos: (f32, f32),      // 需居中计算
    account_label_pos: (f32, f32),
    pass_label_pos: (f32, f32),
    
    // 输入框
    account_input: TextInput,
    password_input: TextInput,
    
    // 按钮 (需要支持3态: normal/hover/pressed)
    ok_button: ButtonWithStates,
    new_account_button: ButtonWithStates,
    change_password_button: ButtonWithStates,
    view_key_button: ButtonWithStates,
    exit_button: ButtonWithStates,
}

impl LoginDialog {
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        // 对话框尺寸
        let width = 328.0;
        let height = 220.0;
        
        // 动态居中
        let x = (screen_width - width) / 2.0;
        let y = (screen_height - height) / 2.0;
        
        Self {
            x, y, width, height, visible: true,
            
            // 标签位置 (相对对话框)
            title_pos: (x + (width - TITLE_WIDTH) / 2.0, y + 12.0),  // 居中
            account_label_pos: (x + 52.0, y + 83.0),
            pass_label_pos: (x + 43.0, y + 105.0),
            
            // 输入框 (精确坐标)
            account_input: TextInput::new(x + 85.0, y + 85.0, 136.0, 15),
            password_input: TextInput::new(x + 85.0, y + 108.0, 136.0, 15).password(),
            
            // 按钮 (Title库, 正确索引)
            ok_button: ButtonWithStates::new(
                x + 227.0, y + 81.0,
                LibraryName::Title,
                320, 321, 322  // normal, hover, pressed
            ),
            new_account_button: ButtonWithStates::new(
                x + 60.0, y + 163.0,
                LibraryName::Title,
                323, 324, 325
            ),
            change_password_button: ButtonWithStates::new(
                x + 166.0, y + 163.0,
                LibraryName::Title,
                326, 327, 328
            ),
            view_key_button: ButtonWithStates::new(
                x + 60.0, y + 189.0,
                LibraryName::Title,
                332, 333, 334
            ),
            exit_button: ButtonWithStates::new(
                x + 166.0, y + 189.0,
                LibraryName::Title,
                329, 330, 331
            ),
        }
    }
    
    pub fn draw(&self, ctx: &mut Context, canvas: &mut Canvas) -> anyhow::Result<()> {
        if !self.visible { return Ok(()); }
        
        // 1. 绘制对话框背景 (正确索引)
        draw_sprite_at(ctx, canvas, &LibraryName::Prguse, 1084, self.x, self.y)?;
        
        // 2. 绘制标签
        draw_sprite_at(ctx, canvas, &LibraryName::Title, 30, 
                      self.title_pos.0, self.title_pos.1)?;
        draw_sprite_at(ctx, canvas, &LibraryName::Title, 31,
                      self.account_label_pos.0, self.account_label_pos.1)?;
        draw_sprite_at(ctx, canvas, &LibraryName::Title, 32,
                      self.pass_label_pos.0, self.pass_label_pos.1)?;
        
        // 3. 绘制输入框
        self.account_input.draw(ctx, canvas)?;
        self.password_input.draw(ctx, canvas)?;
        
        // 4. 绘制按钮
        self.ok_button.draw(ctx, canvas)?;
        self.new_account_button.draw(ctx, canvas)?;
        self.change_password_button.draw(ctx, canvas)?;
        self.view_key_button.draw(ctx, canvas)?;
        self.exit_button.draw(ctx, canvas)?;
        
        Ok(())
    }
}

// 新增: 支持3态的按钮
pub struct ButtonWithStates {
    x: f32, y: f32,
    library: LibraryName,
    normal_index: i32,
    hover_index: i32,
    pressed_index: i32,
    is_hovered: bool,
    is_pressed: bool,
}

impl ButtonWithStates {
    pub fn draw(&self, ctx: &mut Context, canvas: &mut Canvas) -> anyhow::Result<()> {
        let index = if self.is_pressed {
            self.pressed_index
        } else if self.is_hovered {
            self.hover_index
        } else {
            self.normal_index
        };
        
        draw_sprite_at(ctx, canvas, &self.library, index, self.x, self.y)
    }
}
```

### 优先级2: 添加版本和测试标签

```rust
// LoginScene - 添加场景级UI
pub struct LoginScene {
    // ... 现有字段
    
    // 版本标签
    version_text: String,
    version_pos: (f32, f32),
    
    // 测试标签
    show_test_label: bool,
    test_label_pos: (f32, f32),
}

impl LoginScene {
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        Self {
            // ...
            
            version_text: format!("Build: {}.{}.{}", 
                env!("CARGO_PKG_NAME"),
                if cfg!(debug_assertions) { "Debug" } else { "Release" },
                env!("CARGO_PKG_VERSION")
            ),
            version_pos: (5.0, screen_height - 20.0),
            
            show_test_label: cfg!(debug_assertions),
            test_label_pos: (screen_width - 116.0, 10.0),
        }
    }
    
    pub fn draw(&self, ctx: &mut Context, canvas: &mut Canvas) -> anyhow::Result<()> {
        // ... 现有绘制
        
        // 绘制版本标签 (带半透明背景)
        // TODO: 实现带背景的文本绘制
        
        // 绘制测试标签 (Debug模式)
        if self.show_test_label {
            draw_sprite_at(ctx, canvas, &LibraryName::Prguse, 79,
                         self.test_label_pos.0, self.test_label_pos.1)?;
        }
        
        Ok(())
    }
}
```

---

## 八、测试验证清单

### 视觉验证

- [ ] LoginDialog背景正确显示 (Index=1084)
- [ ] Title标签居中显示
- [ ] AccountID/Password标签正确位置
- [ ] 所有按钮显示正确纹理
- [ ] 按钮hover效果正常
- [ ] 按钮pressed效果正常
- [ ] 输入框位置对齐标签
- [ ] 版本标签左下角显示
- [ ] TestLabel右上角显示(Debug模式)

### 功能验证

- [ ] 对话框在不同分辨率下居中
- [ ] ViewKey按钮可点击
- [ ] 虚拟键盘可打开
- [ ] 背景动画使用正确纹理库
- [ ] 连接提示框显示连接次数

---

**分析人**: AI Assistant  
**完成日期**: 2025-10-23
