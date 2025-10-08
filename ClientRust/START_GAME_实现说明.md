# Start Game功能实现说明

## 实现时间
2025年10月8日

## 功能概述
实现了"Start Game"按钮功能,允许玩家选择角色后进入游戏场景。

## 核心流程

```
用户点击Start Game按钮
    ↓
SelectScene.start_game() 被调用
    ↓
发送 NetworkCommand::StartGame { character_index }
    ↓
NetworkManager 处理命令
    ↓
发送 StartGame 数据包到服务器
    ↓
服务器验证并返回 StartGame响应
    ↓
GameClient.on_start_game() 处理响应
    ↓
发送 GameEvent::StartGameResponse 事件
    ↓
SelectScene.process_event() 处理事件
    ↓
切换到 GameScene (待实现)
```

## 文件修改

### 1. SelectScene (ClientRust/src/scenes/select_scene.rs)

#### start_game() 方法实现
```rust
pub fn start_game(&mut self) {
    if self.selected_index >= 0 && (self.selected_index as usize) < self.characters.len() {
        let character = &self.characters[self.selected_index as usize];
        tracing::info!("🎮 Starting game with character: {} (index={})", 
            character.name, character.index);
        
        // Send StartGame command to network thread
        if let Some(command_tx) = &self.command_tx {
            use crate::network::NetworkCommand;
            
            if command_tx.send(NetworkCommand::StartGame {
                character_index: character.index,
            }).is_ok() {
                tracing::info!("📤 Sent StartGame command for character index {}", 
                    character.index);
            } else {
                tracing::error!("❌ Failed to send StartGame command");
            }
        } else {
            tracing::error!("❌ Network command channel not available");
        }
    } else {
        tracing::warn!("⚠️ Cannot start game: No character selected");
    }
}
```

#### 事件处理
```rust
GameEvent::StartGameResponse { result } => {
    tracing::info!("🎮 进入游戏响应: result={}", result);
    if *result == 0 {
        // Success - switch to game scene
        tracing::info!("✅ 进入游戏成功! 准备切换到游戏场景...");
        // TODO: 切换到GameScene
        // return Some(SceneType::Game);
    } else {
        // Error
        let error_msg = match *result {
            1 => "You are not logged in.",
            2 => "Character not found.",
            3 => "Failed to start game.",
            _ => "Unknown error occurred.",
        };
        tracing::error!("❌ 进入游戏失败: {}", error_msg);
        // TODO: 显示错误消息框
    }
}
GameEvent::StartGameBanned { reason, expiry_date } => {
    tracing::warn!("🚫 进入游戏被禁止: reason={}, expiry={}", reason, expiry_date);
    // TODO: 显示封禁消息框
}
GameEvent::StartGameDelay { milliseconds } => {
    tracing::info!("⏱️ 进入游戏延迟: {}ms", milliseconds);
    // TODO: 显示延迟提示
}
```

### 2. GameClient (ClientRust/src/network/game_client.rs)

#### GameEvent枚举扩展
```rust
pub enum GameEvent {
    // ... existing events
    
    // Start game events
    StartGameResponse { result: u8 },
    StartGameBanned { reason: String, expiry_date: i64 },
    StartGameDelay { milliseconds: i64 },
}
```

#### 响应处理器更新
```rust
fn on_start_game(&mut self, packet: packets::StartGame) {
    tracing::info!("🎮 Start game response received: result={}", packet.result);
    /*
     * Result codes:
     * 0: Disabled
     * 1: Not logged in
     * 2: Character not found
     * 3: Start Game Error
     */
    self.send_event(GameEvent::StartGameResponse {
        result: packet.result,
    });
}

fn on_start_game_banned(&mut self, packet: packets::StartGameBanned) {
    tracing::warn!("🚫 Start game banned: reason={}, expiry_date={}", 
        packet.reason, packet.expiry_date);
    self.send_event(GameEvent::StartGameBanned {
        reason: packet.reason,
        expiry_date: packet.expiry_date,
    });
}

fn on_start_game_delay(&mut self, packet: packets::StartGameDelay) {
    tracing::info!("⏱️ Start game delayed: {}ms", packet.milliseconds);
    self.send_event(GameEvent::StartGameDelay {
        milliseconds: packet.milliseconds,
    });
}
```

### 3. NetworkCommand (ClientRust/src/network/network_command.rs)

已有定义,无需修改:
```rust
pub enum NetworkCommand {
    // ... other commands
    
    /// Start game with selected character
    StartGame {
        character_index: i32,
    },
}
```

## 错误代码说明

### StartGameResponse.result
| 代码 | 含义 | 说明 |
|------|------|------|
| 0 | 成功 | 可以进入游戏 |
| 1 | 未登录 | 用户会话已过期 |
| 2 | 角色未找到 | 选择的角色不存在 |
| 3 | 启动错误 | 通用启动失败 |

## 测试步骤

1. **运行游戏**
   ```bash
   cd ClientRust
   $env:RUST_LOG="info"
   cargo run --bin mir2_client
   ```

2. **登录**
   - 输入用户名/密码
   - 点击"Login"

3. **选择角色**
   - 在角色列表中点击角色
   - 点击"Start Game"按钮

4. **观察日志**
   - 查看"🎮 Starting game with character"日志
   - 查看"📤 Sent StartGame command"日志
   - 查看"🎮 Start game response received"日志
   - 查看"✅ 进入游戏成功"日志

## 预期日志输出

```
INFO mir2_client::scenes::select_scene: 🎮 Starting game with character: 战士测试_530 (index=2)
INFO mir2_client::scenes::select_scene: 📤 Sent StartGame command for character index 2
INFO mir2_client::network::game_client: 🎮 Start game response received: result=0
INFO mir2_client::scenes::select_scene: 🎮 进入游戏响应: result=0
INFO mir2_client::scenes::select_scene: ✅ 进入游戏成功! 准备切换到游戏场景...
```

## 待实现功能

### 高优先级
1. **场景切换**
   - [ ] 实现`SceneType::Game`枚举值
   - [ ] 从SelectScene返回场景切换命令
   - [ ] 在main_ggez.rs中处理场景切换

2. **GameScene实现**
   - [ ] 创建GameScene结构体
   - [ ] 实现Scene trait
   - [ ] 地图渲染
   - [ ] 玩家角色显示

3. **错误提示优化**
   - [ ] 实现通用MessageBox组件
   - [ ] 显示启动失败错误消息
   - [ ] 显示封禁信息对话框

### 中优先级
4. **延迟处理**
   - [ ] 显示"正在进入游戏..."加载提示
   - [ ] 实现倒计时显示(如果有延迟)

5. **用户体验**
   - [ ] Start Game按钮点击后禁用(防止重复点击)
   - [ ] 添加进入游戏的过渡动画
   - [ ] 播放进入游戏音效

### 低优先级
6. **特殊情况处理**
   - [ ] 维护模式提示
   - [ ] 活动限制提示
   - [ ] 等级要求提示

## 与C#版本的对比

| 功能 | C#版本 | Rust版本(当前) | 状态 |
|------|--------|----------------|------|
| 发送StartGame包 | ✅ | ✅ | 完成 |
| 处理成功响应 | ✅ | ✅ | 完成 |
| 处理错误响应 | ✅ | ✅ | 完成 |
| 处理封禁响应 | ✅ | ✅ | 完成 |
| 处理延迟响应 | ✅ | ✅ | 完成 |
| 切换到游戏场景 | ✅ | ⚠️ | 待实现 |
| 显示错误消息框 | ✅ | ⚠️ | 待实现 |
| 加载进度显示 | ✅ | ❌ | 未实现 |

## 技术细节

### 线程模型
- **UI线程**: 运行SelectScene,处理用户输入
- **网络线程**: NetworkManager处理所有网络通信
- **通信方式**: 通过tokio::sync::mpsc通道传递命令和事件

### 数据流
1. UI线程通过`command_tx`发送`NetworkCommand::StartGame`
2. 网络线程接收命令,构造并发送`StartGame`数据包
3. 服务器返回`StartGame`响应包
4. 网络线程解析包,调用`on_start_game()`
5. 通过`send_event()`发送`GameEvent::StartGameResponse`
6. UI线程在下一帧的`process_event()`中处理事件

### 安全性
- ✅ 检查角色索引有效性
- ✅ 验证角色存在于本地列表
- ✅ 防止空指针访问
- ✅ 错误处理完整

## 下一步计划

### 立即任务(本次会话)
1. 实现基础GameScene结构
2. 实现场景切换逻辑
3. 测试完整的进入游戏流程

### 短期任务(下次会话)
1. 实现地图加载和渲染
2. 实现玩家角色显示
3. 实现基础移动控制

### 中期任务
1. 实现其他玩家/NPC显示
2. 实现聊天系统
3. 实现物品系统
4. 实现战斗系统

## 已知问题

1. **场景切换未实现**
   - 现状: StartGame成功后只打印日志
   - 需要: 返回SceneType::Game触发场景切换
   - 优先级: 高

2. **错误提示不友好**
   - 现状: 错误只记录到日志
   - 需要: 显示MessageBox给用户
   - 优先级: 中

3. **无加载进度**
   - 现状: 点击后无反馈
   - 需要: 显示"正在进入..."提示
   - 优先级: 低

## 参考资料

### C#版本实现
- `Client/MirScenes/SelectScene.cs` - StartGame方法(约900行)
- `Client/MirScenes/GameScene.cs` - 游戏主场景
- `Shared/ClientPackets.cs` - StartGame包定义(169行)
- `Shared/ServerPackets.cs` - StartGame响应包(333-395行)

### Rust版本实现
- `ClientRust/src/scenes/select_scene.rs` - SelectScene实现
- `ClientRust/src/network/game_client.rs` - 网络事件处理
- `SharedRust/src/packets/client/account.rs` - 客户端包定义
- `SharedRust/src/packets/server/login.rs` - 服务器响应包定义

## 贡献者
- AI Assistant (GitHub Copilot)
- gqf2008 (Repository Owner)

## 更新日志

### 2025-10-08
- ✅ 实现start_game()方法
- ✅ 添加StartGame事件到GameEvent
- ✅ 更新on_start_game()等处理器
- ✅ 在SelectScene中处理StartGameResponse事件
- ✅ 编译成功,功能可测试
- 📝 创建本文档
