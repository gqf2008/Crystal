# 登录场景网络集成指南

**日期**: 2025年10月17日  
**目标**: 连接登录界面与网络模块，实现服务器验证

## 🎯 当前状态

### ✅ 已完成
- [x] LoginScene UI 完整实现（19帧背景动画）
- [x] 账号/密码输入和验证
- [x] 按钮交互和悬停效果
- [x] 新建账号/改密码对话框
- [x] 登录后的场景过渡逻辑（19帧动画 → Select 场景）
- [x] 模块合并（v2 替代 v1）

### 🔄 正在实现
- [~] 网络请求发送
- [~] 服务器响应处理
- [~] 登录状态管理

### ⏳ 待实现
- [ ] 服务器验证逻辑
- [ ] 错误处理（登录失败）
- [ ] 角色选择场景实现

## 📡 网络集成架构

### 当前架构

```
LoginScene UI
    ↓
handle_login_message
    ↓
TODO: Send LoginAuthPacket via GameClient
    ↓
Network Layer (NetworkStack + GameClient)
    ↓
Server Response
    ↓
TODO: Handle in GameEvent listener
    ↓
Update LoginState
    ↓
Transition to Select Scene
```

## 🔌 集成步骤

### 第1步：添加 GameClient 资源到 LoginScene

**位置**: `src/bevy/scenes/login_scene/mod.rs` - `setup_login_scene` 函数

```rust
// 未来改进：获取 GameClient 资源并存储
pub fn setup_login_scene(
    mut commands: Commands,
    // game_client: Option<Res<SharedGameClient>>,  // 待添加
    // ...
) {
    // 存储 GameClient 以便在 handle_login_message 中使用
}
```

### 第2步：修改 handle_login_message 发送网络包

**当前代码** (`src/bevy/scenes/login_scene/mod.rs`):
```rust
pub fn handle_login_message(
    events: Option<MessageReader<LoginButtonPressed>>,
    mut login_state: ResMut<LoginState>,
) {
    // 现在: 临时自动批准登录用于测试
    // TODO: 实现实际的网络请求
}
```

**需要改进为**:
```rust
pub fn handle_login_message(
    events: Option<MessageReader<LoginButtonPressed>>,
    mut login_state: ResMut<LoginState>,
    game_client: Option<Res<SharedGameClient>>,  // 获取网络客户端
) {
    let Some(mut events) = events else { return; };
    
    for event in events.read() {
        info!("🚀 Sending login request to server");
        
        login_state.connecting = true;
        login_state.connect_attempts += 1;
        
        // 发送登录认证包
        if let Some(client) = game_client {
            // 创建并发送登录包
            // let packet = LoginAuthPacket {
            //     account_id: event.account_id.clone(),
            //     password: event.password.clone(),
            // };
            // client.send_packet(&packet);  // 需要根据实际 API 调用
        }
    }
}
```

### 第3步：处理服务器响应

**需要添加新的处理系统**:
```rust
pub fn handle_login_response(
    game_client: Option<Res<SharedGameClient>>,
    mut login_state: ResMut<LoginState>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    // 检查 GameClient 中的登录响应
    // 如果成功 → 设置 login_success = true
    // 如果失败 → 显示错误信息
}
```

### 第4步：在 main_bevy.rs 注册系统

```rust
// 添加到 OnEnter(GameState::Login) 系统中
app.add_systems(OnEnter(GameState::Login), (
    setup_login_scene,
    // 初始化网络连接（如需要）
));

// 添加到 Update 系统中
app.add_systems(Update, 
    handle_login_response.run_if(in_state(GameState::Login))
);
```

## 📦 网络包格式

### 登录认证包

基于 `SharedRust/src/packets/client/` 中的定义，登录包应该包含：

```rust
struct LoginAuthPacket {
    account_id: String,      // 3-15个字母数字
    password: String,        // 5-15个字母数字
    // 可选：游戏版本、客户端ID等
}
```

## 🎮 服务器响应处理

### 可能的响应

1. **登录成功** (OK_RESPONSE)
   - 返回：角色列表、玩家信息
   - 操作：设置 `login_success = true` → 动画 → Select 场景

2. **账号不存在** (ACCOUNT_NOT_FOUND)
   - 返回：错误信息
   - 操作：显示错误提示，保持在 Login 场景

3. **密码错误** (INVALID_PASSWORD)
   - 返回：错误信息
   - 操作：显示错误提示，允许重新输入

4. **账号被禁用** (ACCOUNT_BANNED)
   - 返回：禁用原因、解禁时间
   - 操作：显示禁用通知

5. **服务器错误** (SERVER_ERROR)
   - 返回：错误代码
   - 操作：显示重试提示

## 🔄 登录流程状态机

```
┌─────────────────────────────────────────┐
│         Login State Ready               │
│  (UI显示, 等待用户输入账号/密码)          │
└──────────────────┬──────────────────────┘
                   │ 用户点击登录按钮
                   ↓
┌─────────────────────────────────────────┐
│      Connecting (connecting=true)       │
│  (发送 LoginAuthPacket 到服务器)         │
└──────────────────┬──────────────────────┘
                   │
        ┌──────────┴──────────┐
        ↓                     ↓
   ┌─────────────┐    ┌──────────────────┐
   │   成功      │    │   失败           │
   └──────┬──────┘    └─────────┬────────┘
          │                     │
          ↓                     ↓
  login_success=true   显示错误消息
  animation_paused=false  允许重试
  frames_after_login=0
          │
          ↓
  ┌──────────────────┐
  │  动画播放 (19帧)  │
  │  每帧0.1秒       │
  └──────────┬───────┘
             │
             ↓
  ┌──────────────────────┐
  │  进入 Select 场景     │
  │ (角色选择)           │
  └──────────────────────┘
```

## 📊 测试检查表

- [ ] 登录请求正确发送到服务器
- [ ] 服务器响应被正确接收
- [ ] 登录成功时进入 Select 场景
- [ ] 登录失败时显示错误消息
- [ ] 背景动画正确播放19帧
- [ ] 场景转移没有闪烁或延迟
- [ ] 多次登录尝试工作正常

## 🚀 下一步行动

### 立即
1. 查看现有的 GameClient 实现
2. 找到发送登录包的 API
3. 实现 `handle_login_message` 中的网络调用

### 短期
1. 实现登录响应处理
2. 处理各种错误情况
3. 添加超时重试机制

### 中期
1. 实现 Select 场景
2. 显示角色列表
3. 处理角色选择

## 📝 关键文件参考

| 文件 | 目的 |
|------|------|
| `src/bevy/scenes/login_scene/mod.rs` | 主登录场景逻辑 |
| `src/network/game_client.rs` | 网络客户端实现 |
| `src/network/network_manager.rs` | 网络管理器 |
| `src/bin/main_bevy.rs` | Bevy 系统注册 |
| `SharedRust/src/packets/` | 网络包定义 |

## 💡 架构建议

建议创建独立的网络事件处理系统，类似于当前的消息系统：

```rust
#[derive(Message)]
pub struct LoginResponse {
    pub success: bool,
    pub error_code: u32,
    pub error_message: String,
}

pub fn handle_login_response(
    mut events: MessageReader<LoginResponse>,
    mut login_state: ResMut<LoginState>,
) {
    for event in events.read() {
        if event.success {
            login_state.login_success = true;
            // ...
        } else {
            // 显示错误信息
        }
    }
}
```

---

**状态**: 🔄 正在进行 - 等待网络集成实现  
**负责人**: (当前用户)  
**优先级**: 🔴 高 - 必须完成才能进入下一阶段
