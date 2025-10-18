# 网络集成快速实现指南

## 当前状态 ✅

登录请求已可发送，缺少的是：
1. **NetworkManager 启动** - 需要在主应用中创建
2. **通道注入** - 需要将 command_tx 传递给 LoginState
3. **响应监听** - 需要创建系统监听 GameEvent::LoginSuccess

## 快速检查清单

### 1. 验证登录命令发送工作

**测试代码** (在 handle_login_message 中):
```rust
// 这段代码已经在 mod.rs:1142-1188 实现
if let Some(tx) = &login_state.command_tx {
    let command = crate::network::NetworkCommand::Login {
        username: event.account_id.clone(),
        password: event.password.clone(),
    };
    match tx.send(command) {
        Ok(_) => info!("✅ Login command sent"),
        Err(e) => error!("❌ Failed to send: {}", e),
    }
}
```

**编译验证**:
```bash
cargo check
# Expected: Finished `dev` profile ... in 0.49s
```

### 2. 设置网络管理器 (下一步)

**目标**: 在应用启动时创建网络管理器

```rust
// 在 src/bin/main_bevy.rs 中添加:

use mir2_client::network::{NetworkManager, NetworkCommand};

fn main() {
    // ... 现有代码 ...
    
    // 创建网络命令通道
    let (command_tx, command_rx) = tokio::sync::mpsc::unbounded_channel::<NetworkCommand>();
    
    // 创建事件通道 (用于从网络层接收事件)
    let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel::<GameEvent>();
    
    // 在 Bevy 中创建资源以存储这些通道
    app.insert_resource(NetworkCommandChannel(command_tx.clone()));
    app.insert_resource(NetworkEventReceiver(event_rx));
    
    // 在后台线程中启动网络管理器
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut manager = NetworkManager::new(
                settings,
                event_tx,
                command_rx,
            );
            manager.run().await;
        });
    });
    
    // ... 继续运行应用 ...
}
```

### 3. 在 LoginScene 中注入通道

**修改 `init_network_channel` 系统**:

```rust
// 在 src/bevy/scenes/login_scene/mod.rs 中更新:

pub fn init_network_channel(
    mut login_state: ResMut<LoginState>,
    network_command_channel: Res<NetworkCommandChannel>,
) {
    // 将网络命令通道注入到 LoginState
    login_state.set_command_sender(network_command_channel.0.clone());
    info!("📡 Network command channel initialized");
}

// 需要定义资源类型
#[derive(Resource)]
pub struct NetworkCommandChannel(pub tokio::sync::mpsc::UnboundedSender<crate::network::NetworkCommand>);
```

### 4. 添加响应监听系统

**创建新文件或在 mod.rs 中添加**:

```rust
/// 监听来自网络线程的登录成功事件
pub fn handle_login_success_event(
    // 接收来自网络事件的通道
    mut event_receiver: ResMut<NetworkEventReceiver>,
    mut login_state: ResMut<LoginState>,
) {
    // 尝试从通道接收事件
    while let Ok(event) = event_receiver.try_recv() {
        match event {
            GameEvent::LoginSuccess { characters } => {
                info!("🎉 Login successful! Received {} characters", characters.len());
                
                // 存储角色列表供后续使用
                // TODO: 存储到某个资源中供 Select 场景使用
                
                // 触发动画和转场
                login_state.login_success = true;
                login_state.frames_after_login = 0;
                login_state.animation_paused = false;
            }
            GameEvent::Disconnected { reason } => {
                error!("❌ Disconnected: {}", reason);
                login_state.connecting = false;
            }
            _ => {
                // 忽略其他事件
            }
        }
    }
}

// 资源定义
#[derive(Resource)]
pub struct NetworkEventReceiver(pub tokio::sync::mpsc::UnboundedReceiver<GameEvent>);
```

### 5. 在 main_bevy.rs 中注册系统

```rust
// 添加到 Update 系统中，在 Login 状态下运行

app.add_systems(Update,
    handle_login_success_event.run_if(in_state(GameState::Login))
);
```

## 实现顺序

1. **第一步** - 编译检查 ✅ (已完成)
   ```bash
   cargo check  # 应该成功
   ```

2. **第二步** - 启动 NetworkManager
   - 修改 `main_bevy.rs` 以创建 NetworkManager
   - 将通道作为 Bevy 资源注入

3. **第三步** - 注入命令通道到 LoginScene
   - 更新 `init_network_channel` 以接收和设置 command_tx
   - 在 LoginState 中存储通道

4. **第四步** - 添加事件监听
   - 创建系统来监听来自网络的 GameEvent
   - 在接收到 LoginSuccess 时更新 LoginState

5. **第五步** - 测试完整流程
   - 运行应用
   - 输入凭证并点击登录
   - 验证网络请求被发送
   - 验证响应被处理
   - 验证动画播放并转移到 Select 场景

## 关键代码位置参考

| 功能 | 文件 | 行号 |
|------|------|------|
| 登录消息处理 | `src/bevy/scenes/login_scene/mod.rs` | 1142-1188 |
| 动画和转场 | `src/bevy/scenes/login_scene/mod.rs` | 346-375 |
| 网络通道初始化 | `src/bevy/scenes/login_scene/mod.rs` | 286-300 |
| LoginState 定义 | `src/bevy/scenes/login_scene/mod.rs` | 50-82 |
| 主应用入口 | `src/bin/main_bevy.rs` | 1-204 |

## 调试命令

```bash
# 查看编译错误
cargo check 2>&1

# 运行并显示登录日志
RUST_LOG=mir2_client::bevy::scenes::login_scene=debug cargo run --bin main_bevy

# 运行并显示所有网络日志
RUST_LOG=mir2_client::network=debug cargo run --bin main_bevy

# 清理和重新编译
cargo clean && cargo check
```

## 常见问题

**Q: command_tx 在哪里初始化？**
A: 目前还没有。需要在 main_bevy.rs 中创建 NetworkManager 并获取 command_tx

**Q: 如何测试而不需要真实的游戏服务器？**
A: handle_login_message 中已有测试模式回退，当 command_tx 为 None 时自动批准

**Q: 为什么登录后没有转移到 Select 场景？**
A: 因为 login_state.login_success 还没有被设置为 true。需要先：
   1. 创建 NetworkManager
   2. 添加事件监听系统
   3. 在接收到 LoginSuccess 时设置 login_success = true

**Q: 如何验证登录请求被发送了？**
A: 查看日志中 "Login command sent to network thread successfully" 消息
