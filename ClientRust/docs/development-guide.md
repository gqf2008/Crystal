# ClientRust 快速开发指南

## 当前状态总结

### ✅ 已完成的模块
1. **图形框架**: egui + eframe (wgpu 后端)
2. **场景系统**: Scene trait, LoginScene, SelectScene, GameScene
3. **对话框框架**: DialogManager, 40+ dialog 模块(框架已就绪)
4. **网络层**: 276 个服务器包处理器(结构完整,逻辑待实现)
5. **对象系统**: MapObject 及派生类型(UserObject, MonsterObject 等)
6. **事件系统**: GameEvent 枚举,事件通道架构

### ⏳ 进行中/待完善
- **网络逻辑**: 95% 的 PacketHandler 仅记录日志,需填充状态更新逻辑
- **对话框渲染**: Dialog::draw() 方法为空,需实现 egui UI
- **资源加载**: graphics 模块需实现纹理/音频加载
- **场景渲染**: LoginScene/SelectScene/GameScene 占位符 UI 需替换

## 开发优先级路线图

### 第一阶段: 可视化基础 (1-2 周)
**目标**: 让玩家能看到游戏界面和基本交互

#### 任务 1.1: 资源加载系统
```rust
// 需要实现的文件:
src/graphics/texture_loader.rs   // 加载 .lib 图像库
src/graphics/sprite_renderer.rs  // 渲染精灵
src/sounds/sound_loader.rs       // 加载音效文件
```

**参考 C# 代码**:
- `Client/MirGraphics/MLibrary.cs` - 图像库加载
- `Client/MirSounds/SoundManager.cs` - 音效管理

#### 任务 1.2: LoginScene 完善
```rust
// 需要修改: src/scenes/login_scene.rs
- 添加真实的网络连接逻辑 (connect_to_server)
- 实现 submit_login() 发送 Login 包
- 显示服务器消息(SystemMessage event)
- 加载登录背景图
- 播放背景音乐
```

#### 任务 1.3: 基础 UI 控件
```rust
// 需要创建: src/controls/
mir_button.rs      // 游戏风格按钮
mir_textbox.rs     // 文本输入框
mir_label.rs       // 标签
mir_imagebox.rs    // 图片框
```

**参考**: `Client/MirControls/`

### 第二阶段: 核心游戏循环 (2-3 周)
**目标**: 玩家能进入游戏,看到角色和地图

#### 任务 2.1: SelectScene 实现
```rust
// 需要修改: src/scenes/select_scene.rs
- 渲染角色列表(使用 CharacterSummary)
- 实现角色创建对话框
- 发送 SelectCharacter/NewCharacter 包
- 显示角色预览(装备、外观)
```

#### 任务 2.2: GameScene 地图渲染
```rust
// 需要创建: src/graphics/map_renderer.rs
- 加载地图数据(.map 文件)
- 渲染地图图块(Map Tiles)
- 实现相机跟随玩家
- 渲染对象层(玩家、怪物、NPC)
```

**参考**: `Client/MirScenes/GameScene.cs` (Lines 1-1000)

#### 任务 2.3: 对象渲染
```rust
// 需要修改: src/objects/
- user_object.rs: 添加 render() 方法
- monster_object.rs: 添加 render() 方法
- npc_object.rs: 添加 render() 方法
- spell_object.rs: 添加 render() 方法
```

#### 任务 2.4: 网络层核心包实现
优先实现以下 PacketHandler (按重要性排序):
1. `on_object_player()` - 玩家对象同步
2. `on_object_monster()` - 怪物对象同步
3. `on_object_turn()` / `on_object_walk()` - 移动同步
4. `on_object_remove()` - 对象移除
5. `on_user_location()` - 玩家位置更新
6. `on_map_information()` - 地图信息

**参考**: `Client/MirNetwork/MirNetwork.cs` PacketHandler 方法

### 第三阶段: 游戏系统 (3-4 周)
**目标**: 完整的游戏功能(战斗、物品、技能)

#### 任务 3.1: 主对话框实现
```rust
// 需要修改: src/scenes/dialogs/main_dialog.rs
- render() 方法实现 egui UI
- 显示血条、魔法条、经验条
- 显示小地图
- 显示快捷栏
- 按钮点击处理(背包、技能、角色等)
```

**参考**: `Client/MirScenes/Dialogs/MainDialog.cs` (4166 lines)

#### 任务 3.2: 库存系统
```rust
// 需要修改: src/scenes/dialogs/
inventory_dialog.rs   // 背包界面
character_dialog.rs   // 角色装备界面
storage_dialog.rs     // 仓库界面

// 需要实现网络包:
on_refresh_item()     // 物品刷新
on_new_item()         // 获得新物品
on_delete_item()      // 删除物品
```

#### 任务 3.3: 技能系统
```rust
// 需要修改: src/scenes/dialogs/
skillbar_dialog.rs    // 技能栏
magic_dialog.rs       // 技能书

// 需要实现网络包:
on_magic_learned()    // 学习技能
on_magic_cast()       // 释放技能
on_object_magic()     // 其他玩家释放技能
```

#### 任务 3.4: 战斗系统
```rust
// 需要实现网络包:
on_object_attack()    // 攻击动作
on_damage_indicator() // 伤害显示
on_object_struck()    // 受击动作
on_object_died()      // 死亡动作
on_spell_animation()  // 技能特效
```

### 第四阶段: 高级功能 (4+ 周)
**目标**: 完整的 MMO 体验

#### 任务 4.1: 社交系统
- 聊天系统(ChatDialog 完善)
- 好友系统(FriendDialog)
- 组队系统(GroupDialog)
- 公会系统(GuildDialog)
- 交易系统(TradeDialog)

#### 任务 4.2: 任务系统
- QuestListDialog 实现
- NPC 对话系统(NPCDialog)
- 任务追踪显示

#### 任务 4.3: 特殊系统
- 坐骑系统(MountDialog)
- 宠物系统
- 钓鱼系统(FishingDialog)
- 制作系统(CraftDialog)
- 精炼系统(RefineDialog)

## 开发工具和技巧

### 调试网络包
```rust
// 在 game_client.rs 中启用详细日志:
tracing::debug!("Received packet: {:?}", packet);
```

### 查看场景状态
```rust
// 在 app.rs update() 中:
if ui.input(|i| i.key_pressed(egui::Key::F3)) {
    tracing::info!("Current scene: {:?}", self.current_scene);
    tracing::info!("Player: {:?}", self.login_scene.username);
}
```

### 性能分析
```bash
# 使用 release 模式运行
cargo run --release

# 启用帧率限制(在 app.rs 中):
ctx.request_repaint_after(std::time::Duration::from_millis(16)); // 60 FPS
```

### 对比 C# 代码
当不确定如何实现某个功能时:
1. 在 `Client/` 目录搜索相关代码
2. 查看对应的 `MirScenes/`, `MirObjects/`, `MirNetwork/` 文件
3. 理解 C# 逻辑
4. 转换为 Rust 实现(注意所有权和生命周期)

### 常用参考文件映射

| C# 文件 | Rust 对应文件 | 说明 |
|---------|---------------|------|
| `GameScene.cs` (12297 lines) | `game_scene.rs` (552 lines) | 游戏主场景 |
| `MainDialog.cs` (4166 lines) | `main_dialog.rs` | 主界面 UI |
| `UserObject.cs` (822 lines) | `user_object.rs` (436 lines) | 玩家对象 |
| `MirNetwork.cs` | `game_client.rs` (2719 lines) | 网络包处理 |
| `MLibrary.cs` | `texture_loader.rs` (待创建) | 资源加载 |
| `SoundManager.cs` | `sound_manager.rs` (待创建) | 音频管理 |

## 代码风格指南

### Rust 命名约定
```rust
// 结构体: PascalCase
pub struct LoginScene { ... }

// 方法: snake_case
fn process_event(&mut self, event: &GameEvent) { ... }

// 常量: SCREAMING_SNAKE_CASE
const MAX_CHAT_MESSAGES: usize = 100;

// 模块: snake_case
mod login_scene;
```

### 错误处理
```rust
// 优先使用 Result<T, E>
pub fn load_texture(&self, path: &Path) -> Result<Texture, LoadError> {
    let data = std::fs::read(path)?;
    Ok(Texture::from_bytes(&data)?)
}

// 避免 unwrap(),使用 ? 或 match
```

### 异步代码
```rust
// 网络操作使用 tokio
async fn connect_to_server(&mut self) -> Result<()> {
    let stream = TcpStream::connect("127.0.0.1:7000").await?;
    // ...
    Ok(())
}
```

## 测试策略

### 单元测试
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_scene_creation() {
        let scene = LoginScene::new();
        assert_eq!(scene.scene_type(), SceneType::Login);
    }
}
```

### 集成测试
```rust
// tests/integration_test.rs
#[tokio::test]
async fn test_server_connection() {
    let mut client = GameClient::new();
    // 连接到测试服务器
    // 发送包
    // 验证响应
}
```

## 常见问题解答

### Q: 为什么选择 egui 而不是其他 UI 框架?
A: egui 是 immediate-mode UI,非常适合游戏开发。它:
- 与 Rust 生态系统集成良好
- 支持 wgpu 后端(现代 GPU API)
- 代码简洁,易于调试
- 性能优异

### Q: 如何处理 C# 的 WinForms 控件?
A: 使用 egui 的等价组件:
- `Button` → `ui.button()`
- `TextBox` → `ui.text_edit_singleline()`
- `Label` → `ui.label()`
- `PictureBox` → `ui.image(texture_id, size)`

### Q: 网络层的 95% TODO 怎么办?
A: 按优先级逐步实现:
1. 先实现登录流程(ClientVersion, Login, LoginSuccess)
2. 然后角色选择(SelectCharacter, StartGame)
3. 再实现游戏内核心包(ObjectPlayer, ObjectMonster, UserLocation)
4. 最后补充边缘功能(Trade, Guild, Quest 等)

### Q: 如何加载 MIR2 的 .lib 图像文件?
A: 参考 C# 的 `MLibrary.cs`:
1. 读取文件头(版本、图像数量)
2. 读取索引表(每个图像的偏移和大小)
3. 解压图像数据(zlib)
4. 转换为 egui 的 TextureHandle

## 资源链接

### 官方文档
- [egui 文档](https://docs.rs/egui/)
- [eframe 文档](https://docs.rs/eframe/)
- [wgpu 文档](https://docs.rs/wgpu/)
- [Tokio 文档](https://tokio.rs/)

### 示例代码
- [egui 示例](https://github.com/emilk/egui/tree/master/examples)
- [Rust 游戏开发](https://arewegameyet.rs/)

### Crystal 项目
- C# 客户端: `Client/` 目录
- C# 服务端: `Server/` 目录
- 共享协议: `Shared/` 目录

---

**最后更新**: 2025-01-04
**当前版本**: ClientRust v0.1.0 (egui 集成完成)
