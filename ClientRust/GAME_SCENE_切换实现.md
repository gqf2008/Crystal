# GameScene 场景切换实现说明

## 📌 实现概述

本次实现完成了从 SelectScene 到 GameScene 的场景切换功能,使玩家点击 "Start Game" 按钮后能够真正进入游戏世界。

**实现时间**: 2025-10-08  
**涉及文件**: 
- `ClientRust/src/scenes/select_scene.rs`
- `ClientRust/src/main_ggez.rs`

---

## 🔄 核心流程

```
用户点击 "Start Game"
    ↓
SelectScene.start_game() 发送 NetworkCommand::StartGame
    ↓
GameClient 接收服务器响应 StartGame packet
    ↓
GameClient 发出 GameEvent::StartGameResponse
    ↓
SelectScene.process_event() 处理响应
    ↓
设置 pending_scene_change = Some(SceneType::Game)
    ↓
主循环检测到 pending_scene_change
    ↓
调用 SceneManager.switch_scene(SceneType::Game)
    ↓
进入 GameScene
```

---

## 📝 代码修改详情

### 1. SelectScene 结构体添加字段

**文件**: `ClientRust/src/scenes/select_scene.rs`

```rust
pub struct SelectScene {
    // ... 其他字段 ...
    
    // Scene transition
    pub pending_scene_change: Option<SceneType>,  // ✅ 新增
    
    // ... 其他字段 ...
}
```

**用途**: 标记场景需要切换到哪个场景类型

---

### 2. 构造函数初始化

**文件**: `ClientRust/src/scenes/select_scene.rs`

```rust
pub fn new(characters: Vec<SelectInfo>) -> Self {
    let mut scene = Self {
        // ... 其他字段 ...
        pending_scene_change: None,  // ✅ 初始化为 None
        // ... 其他字段 ...
    };
    scene
}
```

---

### 3. process_event 处理 StartGame 成功

**文件**: `ClientRust/src/scenes/select_scene.rs`  
**位置**: `process_event()` 方法中

```rust
GameEvent::StartGameResponse { result } => {
    tracing::info!("🎮 进入游戏响应: result={}", result);
    if *result == 0 {
        // ✅ 成功 - 设置场景切换标记
        tracing::info!("✅ 进入游戏成功! 切换到游戏场景...");
        self.pending_scene_change = Some(SceneType::Game);
    } else {
        // ❌ 失败 - 记录错误
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
```

**改动**: 
- 将 `// TODO: 切换到GameScene` 改为实际设置 `pending_scene_change` 字段

---

### 4. 主循环检测场景切换请求

**文件**: `ClientRust/src/main_ggez.rs`  
**位置**: `CrystalGame::update()` 方法中

```rust
// ✅ 新增: 检查 SelectScene 是否需要切换到 GameScene
let should_switch_to_game = {
    let scene_manager = self.scene_manager.read();
    if scene_manager.current_scene_type() == Some(SceneType::Select) {
        if let Some(scene) = scene_manager.current_scene() {
            if let Some(select_scene) = scene.as_any().downcast_ref::<SelectScene>() {
                select_scene.pending_scene_change == Some(SceneType::Game)
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    }
};

// ✅ 如果需要切换,执行场景切换
if should_switch_to_game {
    tracing::info!("✅ 开始游戏,切换到 GameScene");
    
    let mut scene_manager = self.scene_manager.write();
    if let Err(e) = scene_manager.switch_scene(SceneType::Game) {
        tracing::error!("❌ 切换到 GameScene 失败: {}", e);
    } else {
        tracing::info!("🎮 场景切换完成: Game");
    }
}
```

**实现逻辑**:
1. 每帧检查当前场景是否为 SelectScene
2. 检查 `pending_scene_change` 字段
3. 如果设置为 `Game`,调用 `SceneManager::switch_scene()`
4. 记录日志

---

## 🎮 GameScene 现状

GameScene 已经存在并实现了 Scene trait:

- **文件**: `ClientRust/src/scenes/game_scene.rs`
- **状态**: 基础框架已完成,但功能尚未完全实现
- **已实现**: 
  - ✅ Scene trait 实现 (scene_type, initialize, update, draw, process_event 等)
  - ✅ 基础数据结构 (user, hero, objects, items, monsters 等)
  - ✅ 游戏状态字段 (gold, attack_mode, pet_mode 等)
  - ✅ 事件处理框架
- **待实现**:
  - ⚠️ 地图渲染 (map_control)
  - ⚠️ 对象渲染 (角色、怪物、NPC、道具)
  - ⚠️ UI 对话框 (背包、技能、聊天等 40+ 对话框)
  - ⚠️ 移动/攻击/技能逻辑
  - ⚠️ 寻路系统

---

## 🧪 测试步骤

### 编译测试

```powershell
cd ClientRust
cargo build --bin mir2_client
```

**结果**: ✅ 编译成功,无错误

```
   Compiling mir2_client v0.1.0
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 7.83s
```

### 运行测试

```powershell
$env:RUST_LOG="info"
cargo run --bin mir2_client
```

**测试流程**:
1. 启动游戏 → 显示 LoginScene
2. 输入用户名/密码 → 登录成功
3. 显示 SelectScene → 选择角色
4. 点击 "Start Game" 按钮
5. **预期结果**: 
   - 日志显示 "🎮 进入游戏响应: result=0"
   - 日志显示 "✅ 进入游戏成功! 切换到游戏场景..."
   - 日志显示 "✅ 开始游戏,切换到 GameScene"
   - 日志显示 "🎮 场景切换完成: Game"
   - 日志显示 "GameScene::initialize"
   - 画面切换到 GameScene (目前是空白场景)

---

## 📊 错误处理

### StartGame 响应错误码

| 错误码 | 含义 | 处理方式 |
|--------|------|----------|
| 0 | 成功 | 切换到 GameScene |
| 1 | 未登录 | 显示错误消息,返回 LoginScene |
| 2 | 角色不存在 | 显示错误消息,刷新角色列表 |
| 3 | 启动游戏失败 | 显示错误消息 |

**当前实现**: 记录错误日志  
**TODO**: 实现 MessageBox 组件显示用户友好的错误提示

---

## 🔍 日志关键字

启动游戏时应该看到的日志:

```
🎮 Starting game with character: 角色名 (index=X)
📤 Sent StartGame command for character index X
🎮 Start game response received: result=0
🎮 进入游戏响应: result=0
✅ 进入游戏成功! 切换到游戏场景...
✅ 开始游戏,切换到 GameScene
🎮 场景切换完成: Game
GameScene::initialize
```

失败时的日志:

```
🎮 Start game response received: result=2
🎮 进入游戏响应: result=2
❌ 进入游戏失败: Character not found.
```

---

## 🚧 待实现功能 (GameScene)

### 高优先级

1. **地图渲染**
   - [ ] 实现 map_control 模块
   - [ ] 加载地图文件 (.map 格式)
   - [ ] 渲染地图瓦片
   - [ ] 相机跟随玩家

2. **玩家角色显示**
   - [ ] 根据 class/gender 加载角色精灵
   - [ ] 实现角色动画 (站立、走路、攻击)
   - [ ] 渲染玩家角色

3. **基础交互**
   - [ ] 处理 ObjectSpawned 事件 (其他玩家、怪物、NPC)
   - [ ] 处理 PlayerMoved 事件
   - [ ] 实现鼠标点击移动

### 中优先级

4. **UI 系统**
   - [ ] 实现 MainDialog (主界面)
   - [ ] 实现 ChatDialog (聊天窗口)
   - [ ] 实现 InventoryDialog (背包)
   - [ ] 实现 CharacterDialog (角色属性)

5. **战斗系统**
   - [ ] 实现攻击逻辑
   - [ ] 实现技能系统
   - [ ] 实现伤害显示

### 低优先级

6. **高级功能**
   - [ ] 寻路系统
   - [ ] 组队系统
   - [ ] 行会系统
   - [ ] 交易系统

---

## 💡 与 C# 版本对比

### C# 版本流程
```csharp
// Client/MirScenes/SelectScene.cs
private void StartGameButton_Click(object sender, EventArgs e)
{
    Network.Enqueue(new C.StartGame { CharacterIndex = Characters[Selected].Index });
}

// 收到响应后
private void StartGame()
{
    GameScene.Enabled = true;
    Enabled = false;  // 禁用 SelectScene
}
```

### Rust 版本流程
```rust
// ClientRust/src/scenes/select_scene.rs
pub fn start_game(&mut self) {
    command_tx.send(NetworkCommand::StartGame { character_index }).ok();
}

// process_event 中
GameEvent::StartGameResponse { result } => {
    if *result == 0 {
        self.pending_scene_change = Some(SceneType::Game);
    }
}

// ClientRust/src/main_ggez.rs 主循环
if should_switch_to_game {
    scene_manager.switch_scene(SceneType::Game)?;
}
```

**主要区别**:
- C# 使用 `Enabled` 属性直接控制场景显示
- Rust 使用 `SceneManager` 统一管理场景切换
- Rust 采用事件驱动 + 标记位模式 (pending_scene_change)

---

## ✅ 完成检查清单

- [x] SelectScene 添加 `pending_scene_change` 字段
- [x] 构造函数初始化字段
- [x] process_event 处理 StartGameResponse 成功
- [x] 主循环检测场景切换请求
- [x] 主循环执行场景切换
- [x] GameScene 实现 Scene trait (已存在)
- [x] 编译通过,无错误
- [ ] 运行测试验证场景切换
- [ ] 实现 GameScene 基础渲染

---

## 📚 相关文档

- [START_GAME_实现说明.md](./START_GAME_实现说明.md) - StartGame 网络通信实现
- [GGEZ_MIGRATION_COMPLETED.md](./GGEZ_MIGRATION_COMPLETED.md) - Ggez 迁移完成文档
- C# 参考: `Client/MirScenes/GameScene.cs` (12,297 lines)

---

## 🎯 下一步计划

**立即执行**:
1. **运行测试** - 验证场景切换是否正常工作
2. **实现 GameScene 基础渲染** - 至少显示一个空白的游戏世界界面

**短期计划**:
3. **实现地图加载** - 加载并显示地图瓦片
4. **实现玩家角色显示** - 显示玩家角色精灵
5. **实现基础移动** - 鼠标点击移动

**中期计划**:
6. **实现其他对象渲染** - 怪物、NPC、道具
7. **实现基础 UI** - 聊天、背包、角色属性
8. **实现战斗系统** - 攻击、技能

---

## 🎉 总结

本次实现完成了从角色选择到游戏世界的完整场景切换流程:

1. ✅ 网络通信 - StartGame packet 发送和响应处理
2. ✅ 事件系统 - GameEvent 传递和处理
3. ✅ 场景管理 - 场景切换标记和执行
4. ✅ 代码质量 - 编译通过,无错误

**用户体验**:
- 点击 "Start Game" 按钮
- 看到日志提示 "✅ 进入游戏成功!"
- 场景切换到 GameScene
- (目前显示空白场景,待实现渲染)

**技术亮点**:
- 使用 `pending_scene_change` 字段实现延迟切换
- 主循环统一处理场景切换,避免在事件处理中直接修改场景
- 线程安全的场景管理 (Arc<RwLock<SceneManager>>)

**与 C# 版本对比**:
- 更清晰的场景生命周期管理
- 更安全的多线程场景切换
- 更容易扩展的事件驱动架构

---

**Created**: 2025-10-08  
**Author**: GitHub Copilot  
**Status**: ✅ 实现完成,待测试验证
