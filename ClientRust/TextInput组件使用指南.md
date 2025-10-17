# 文本输入组件使用指南

## 概述

已实现基础的文本输入组件 `TextInput`,支持键盘输入、光标控制、字符过滤等功能。

## 组件特性

### TextInput 组件
```rust
#[derive(Component)]
pub struct TextInput {
    pub text: String,              // 当前文本
    pub max_length: usize,         // 最大长度
    pub is_password: bool,         // 密码模式
    pub focused: bool,             // 是否获得焦点
    pub cursor_position: usize,    // 光标位置
    pub allowed_chars: CharFilter, // 字符过滤器
}
```

### 功能列表

✅ **基础功能**
- 文本输入/删除
- 光标移动 (←→ Home End)
- 最大长度限制
- 焦点管理

✅ **高级功能**
- 密码模式 (显示星号)
- 字符过滤 (All, AlphaNumeric, Custom)
- Backspace/Delete 支持
- 光标位置显示

## 使用方法

### 1. 创建文本输入框

```rust
use bevy::prelude::*;
use crate::bevy::components::TextInput;

// 普通输入框
let account_input = TextInput::new(15)  // 最大15字符
    .with_filter(CharFilter::AlphaNumeric);

// 密码输入框
let password_input = TextInput::new(15)
    .password()
    .with_text("default".to_string());
```

### 2. 添加到实体

```rust
commands.spawn((
    TextInput::new(15)
        .with_filter(CharFilter::AlphaNumeric),
    Text::new(""),
    TextFont {
        font_size: 14.0,
        ..default()
    },
    Node {
        position_type: PositionType::Absolute,
        left: Val::Px(85.0),
        top: Val::Px(85.0),
        width: Val::Px(136.0),
        height: Val::Px(15.0),
        ..default()
    },
    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
    // 你的组件标记
    AccountIdInput,
));
```

### 3. 注册系统

在 `App` 中添加文本输入系统:

```rust
.add_systems(Update, (
    text_input_system,         // 处理键盘输入
    text_input_focus_system,   // 处理焦点切换
    text_input_render_system,  // 更新显示文本
))
```

## 键盘快捷键

| 按键 | 功能 |
|------|------|
| `字母/数字` | 输入字符 |
| `Backspace` | 删除光标前字符 |
| `Delete` | 删除光标后字符 |
| `←` | 光标左移 |
| `→` | 光标右移 |
| `Home` | 光标移到开头 |
| `End` | 光标移到结尾 |

## API 方法

### 构造方法
```rust
TextInput::new(max_length)                    // 创建输入框
    .password()                                // 设置为密码框
    .with_filter(CharFilter::AlphaNumeric)    // 设置字符过滤
    .with_text("initial".to_string())         // 设置初始文本
```

### 操作方法
```rust
input.insert_char('a')      // 插入字符
input.delete_char()         // 删除 (Backspace)
input.delete_char_forward() // 删除 (Delete)
input.move_cursor_left()    // 光标左移
input.move_cursor_right()   // 光标右移
input.move_cursor_home()    // 光标到开头
input.move_cursor_end()     // 光标到结尾
input.clear()               // 清空文本
```

### 查询方法
```rust
input.display_text()  // 获取显示文本 (密码框显示星号)
```

## 字符过滤器

### 内置过滤器
```rust
// 1. 允许所有字符
CharFilter::All

// 2. 仅字母和数字
CharFilter::AlphaNumeric

// 3. 自定义过滤函数
CharFilter::Custom(|c| c.is_ascii_alphabetic())
```

### 自定义过滤器示例
```rust
// 仅允许小写字母
let lowercase_filter = CharFilter::Custom(|c| {
    c.is_ascii_lowercase()
});

// 仅允许数字和特定符号
let special_filter = CharFilter::Custom(|c| {
    c.is_numeric() || c == '@' || c == '.'
});
```

## 集成到 LoginScene

### 修改 LoginScene 以使用 TextInput

```rust
// 在 spawn_dialog_contents 中
fn spawn_dialog_contents(...) {
    // ... 其他代码 ...
    
    // 账号输入框
    parent.spawn((
        TextInput::new(15)
            .with_filter(CharFilter::AlphaNumeric),
        Text::new(""),
        TextFont { font_size: 14.0, ..default() },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(85.0),
            top: Val::Px(85.0),
            width: Val::Px(136.0),
            height: Val::Px(15.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
        AccountIdInput,
    ));
    
    // 密码输入框
    parent.spawn((
        TextInput::new(15).password(),
        Text::new(""),
        TextFont { font_size: 14.0, ..default() },
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(85.0),
            top: Val::Px(108.0),
            width: Val::Px(136.0),
            height: Val::Px(15.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
        PasswordInput,
    ));
}
```

### 读取输入内容

```rust
// 在登录按钮处理中
pub fn handle_login_button(
    mut events: MessageReader<LoginButtonPressed>,
    account_query: Query<&TextInput, With<AccountIdInput>>,
    password_query: Query<&TextInput, With<PasswordInput>>,
    mut login_state: ResMut<LoginState>,
) {
    for _event in events.read() {
        // 获取输入内容
        let account_id = account_query
            .get_single()
            .map(|input| input.text.clone())
            .unwrap_or_default();
            
        let password = password_query
            .get_single()
            .map(|input| input.text.clone())
            .unwrap_or_default();
        
        info!("Login: account={}, password={}", account_id, password);
        
        // 验证和处理登录...
    }
}
```

## 待改进功能

- [ ] 文本选择 (Shift + 箭头)
- [ ] 复制/粘贴 (Ctrl+C / Ctrl+V)
- [ ] 撤销/重做 (Ctrl+Z / Ctrl+Y)
- [ ] 双击选择单词
- [ ] 鼠标拖拽选择
- [ ] 输入法支持 (IME)
- [ ] 自动完成提示

## 注意事项

1. **焦点管理**: 同一时间只能有一个输入框获得焦点
2. **坐标转换**: 当前焦点检测使用简单边界框,复杂布局可能需要改进
3. **性能**: 每帧更新显示文本,适合少量输入框
4. **中文输入**: 当前不支持 IME,仅支持 ASCII 字符
5. **事件**: 使用 Bevy 的 `ReceivedCharacter` 事件接收字符输入

## 测试建议

```bash
# 编译并运行
cargo run --bin mir2_bevy --release

# 测试步骤:
# 1. 点击账号输入框
# 2. 输入字符 (仅字母数字)
# 3. 测试光标移动 (←→ Home End)
# 4. 测试删除 (Backspace Delete)
# 5. 点击密码输入框
# 6. 输入字符 (应显示星号)
# 7. 按回车或点击登录按钮
```

---

**创建日期**: 2025-10-17  
**状态**: ✅ 基础功能完成
