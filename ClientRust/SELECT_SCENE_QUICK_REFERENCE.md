# SelectScene 快速参考指南

## 🎯 场景概览

**SelectScene** (角色选择场景) 是登录后的第一个交互场景，用户在此选择要进行游戏的角色。

```
用户流程: 登录 → 选择角色 → 进入游戏
游戏状态: Login → Select → Game
```

## 📍 文件位置

```
src/bevy/scenes/select_scene/
├── mod.rs              # 主模块（系统和 UI 逻辑）
└── components.rs       # 数据结构和组件
```

## 🔧 快速集成步骤

### 1. 状态转移到 SelectScene

```rust
// 在任何需要进入角色选择的地方
next_state.set(GameState::Select);
```

### 2. 触发按钮事件

```rust
// 选择角色
if let Some(ref mut events) = events {
    events.write(SelectCharacterMessage { index: 0 });
}

// 开始游戏
events.write(StartGameMessage { character_index: 1 });

// 返回登录
events.write(BackToLoginMessage);
```

### 3. 访问场景状态

```rust
pub fn my_system(state: Res<SelectSceneState>) {
    if let Some(idx) = state.selected_index {
        println!("选中的角色索引: {}", idx);
    }
    println!("可用角色数: {}", state.characters.len());
}
```

## 📊 UI 组件树

```
SelectSceneRoot (根节点)
│
├─ Title (标题)
│  └─ Text "选择角色"
│
├─ CharacterListContainer (角色列表)
│  ├─ CharacterItem #1
│  ├─ CharacterItem #2
│  └─ CharacterItem #3 (动态)
│
└─ ButtonPanel (按钮面板)
   ├─ StartGameButton "开始游戏"
   ├─ CreateCharacterButton "创建角色"
   └─ BackToLoginButton "返回登录"
```

## 🎨 样式配置

所有颜色常量定义在 `components.rs` 中：

```rust
pub const BACKGROUND_COLOR: Color = Color::srgba(0.1, 0.1, 0.15, 1.0);
pub const BUTTON_COLOR: Color = Color::srgba(0.2, 0.2, 0.3, 1.0);
pub const BUTTON_HOVER_COLOR: Color = Color::srgba(0.3, 0.3, 0.4, 1.0);
pub const BUTTON_PRESSED_COLOR: Color = Color::srgba(0.15, 0.15, 0.25, 1.0);
pub const TEXT_COLOR: Color = Color::srgba(1.0, 1.0, 1.0, 1.0);
pub const SELECTED_COLOR: Color = Color::srgba(1.0, 1.0, 0.0, 1.0);
pub const ERROR_COLOR: Color = Color::srgba(1.0, 0.0, 0.0, 1.0);
```

修改这些常量即可改变 UI 样式。

## 📨 消息系统

SelectScene 定义了 5 个消息类型：

| 消息 | 用途 | 数据 |
|------|------|------|
| `SelectCharacterMessage` | 选择角色 | `index: usize` |
| `DeleteCharacterMessage` | 删除角色 | `index: usize` |
| `CreateCharacterMessage` | 创建新角色 | `name, class, gender` |
| `StartGameMessage` | 开始游戏 | `character_index: i32` |
| `BackToLoginMessage` | 返回登录 | 无数据 |

### 发送消息示例

```rust
// 需要 MessageWriter 参数
pub fn my_handler(
    mut events: Option<MessageWriter<SelectCharacterMessage>>,
) {
    if let Some(ref mut events) = events {
        events.write(SelectCharacterMessage { index: 0 });
    }
}
```

## 🎛️ 系统功能速查表

| 函数 | 用途 | 调用时机 |
|------|------|---------|
| `setup_select_scene` | 初始化场景 UI | `OnEnter(GameState::Select)` |
| `cleanup_select_scene` | 清理场景资源 | `OnExit(GameState::Select)` |
| `update_character_list` | 更新角色列表显示 | `Update` (Select 状态) |
| `handle_button_hover` | 按钮悬停效果 | `Update` (Select 状态) |
| `handle_character_select` | 处理角色选择 | `Update` (Select 状态) |
| `handle_start_game` | 处理开始游戏 | `Update` (Select 状态) |
| `message_handle_*` | 消息处理 | `Update` (Select 状态) |

## 🔄 状态机整合

SelectScene 与游戏状态机的关系：

```rust
// 从 Login 进入 Select
if login_success {
    next_state.set(GameState::Select);
}

// 从 Select 进入 Game
if start_game_clicked {
    next_state.set(GameState::Game);
}

// 从 Select 回到 Login
if back_to_login_clicked {
    next_state.set(GameState::Login);
}
```

## 📝 SelectSceneState 结构

场景全局状态资源：

```rust
pub struct SelectSceneState {
    // 角色数据
    pub characters: Vec<CharacterInfo>,
    pub selected_index: Option<usize>,
    
    // 对话框状态
    pub show_create_dialog: bool,
    pub show_delete_dialog: bool,
    pub delete_confirm_index: Option<usize>,
    
    // 创建新角色的数据
    pub new_character_name: String,
    pub new_character_class: u8,
    pub new_character_gender: u8,
    
    // 动画状态
    pub animation_timer: f32,
    pub is_animating: bool,
}
```

访问方式：

```rust
pub fn check_state(state: Res<SelectSceneState>) {
    // 检查是否有选中的角色
    match state.selected_index {
        Some(idx) => println!("已选中: {}", idx),
        None => println!("未选中任何角色"),
    }
    
    // 访问角色列表
    for character in &state.characters {
        println!("角色: {}, 等级: {}", character.name, character.level);
    }
}
```

## ⚙️ 配置常量

在 `components.rs` 中定义的其他常量：

```rust
pub const MAX_CHARACTERS: usize = 3;  // 最多创建 3 个角色

pub const CLASSES: &[(&str, u8)] = &[
    ("战士 (Warrior)", 0),
    ("道士 (Taoist)", 1),
    ("法师 (Wizard)", 2),
];

pub const GENDERS: &[(&str, u8)] = &[
    ("男 (Male)", 0),
    ("女 (Female)", 1),
];
```

## 🚀 常见任务

### 任务 1: 添加新的按钮

在 `setup_select_scene` 函数中的 ButtonPanel 子节点处添加：

```rust
parent.spawn((
    Button,
    Node {
        width: Val::Px(150.0),
        height: Val::Px(50.0),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    },
    BackgroundColor(BUTTON_COLOR),
    MyNewButton,  // 自定义组件
)).with_children(|parent| {
    parent.spawn((
        Text::new("按钮标签"),
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(TEXT_COLOR),
    ));
});
```

### 任务 2: 添加新的系统

1. 定义系统函数
2. 在 `main_bevy.rs` 中导入
3. 添加到 Update 系统链：

```rust
app.add_systems(Update, (
    my_new_system,  // 添加这里
).run_if(in_state(GameState::Select)));
```

### 任务 3: 响应消息

定义消息处理系统：

```rust
pub fn message_handle_my_event(
    mut events: Option<MessageReader<MyEventMessage>>,
) {
    let Some(mut events) = events else { return; };
    
    for event in events.read() {
        // 处理事件
        println!("收到消息: {:?}", event);
    }
}
```

### 任务 4: 修改 UI 样式

修改 `components.rs` 中的常量：

```rust
// 改变按钮颜色
pub const BUTTON_COLOR: Color = Color::srgba(0.5, 0.2, 0.2, 1.0);  // 更红

// 改变背景
pub const BACKGROUND_COLOR: Color = Color::srgba(0.0, 0.0, 0.0, 1.0);  // 纯黑
```

## 🐛 调试技巧

### 打印状态信息

```rust
pub fn debug_state(state: Res<SelectSceneState>) {
    println!("=== SelectScene 状态 ===");
    println!("角色数: {}", state.characters.len());
    println!("选中: {:?}", state.selected_index);
    println!("创建对话框: {}", state.show_create_dialog);
    println!("删除对话框: {}", state.show_delete_dialog);
}

// 在 main_bevy.rs 中添加
app.add_systems(Update, debug_state.run_if(in_state(GameState::Select)));
```

### 监听消息

```rust
pub fn debug_messages(
    mut msg1: Option<MessageReader<SelectCharacterMessage>>,
    mut msg2: Option<MessageReader<StartGameMessage>>,
) {
    if let Some(mut msg1) = msg1 {
        for event in msg1.read() {
            println!("📨 选择角色: {}", event.index);
        }
    }
    
    if let Some(mut msg2) = msg2 {
        for event in msg2.read() {
            println!("📨 开始游戏: {}", event.character_index);
        }
    }
}
```

## 📦 编译检查

在修改代码后，运行：

```bash
cargo check
```

确保输出类似：

```
    Finished `dev` profile [optimized + debuginfo] target(s) in X.XXs
```

## 🔗 相关文件链接

- 📄 [完整实现报告](./SELECT_SCENE_IMPLEMENTATION_COMPLETE.md)
- 📂 主模块: `src/bevy/scenes/select_scene/mod.rs`
- 📂 组件定义: `src/bevy/scenes/select_scene/components.rs`
- 📂 场景管理: `src/bevy/scenes/mod.rs`
- 📂 主应用: `src/bin/main_bevy.rs`

---

**版本**: 1.0
**最后更新**: 现在
**状态**: ✅ 完成并编译成功
