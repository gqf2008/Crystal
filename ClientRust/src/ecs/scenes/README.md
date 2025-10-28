# ecs/scenes - 游戏场景系统

**文件数**: 多个  
**代码行数**: ~3,500  
**状态**: ✅ 核心完成

---

## 📚 目录

1. [模块概述](#-模块概述)
2. [场景架构](#-场景架构)
3. [场景详解](#-场景详解)
4. [使用指南](#-使用指南)

---

## 📖 模块概述

`scenes` 目录包含游戏的所有场景实现。场景是游戏状态的最高层抽象，每个场景对应一个游戏阶段。

### 场景类型

| 场景 | 用途 | 状态 |
|------|------|------|
| **LoginScene** | 账号登录 | ✅ 完成 |
| **SelectScene** | 角色选择 | ✅ 完成 |
| **GameScene** | 游戏主场景 | ✅ 完成 |

### 文件结构

```
scenes/
├── mod.rs                  # 场景模块入口
├── game_scene.rs           # 游戏主场景
├── login_scene/            # 登录场景
│   ├── mod.rs
│   ├── components.rs       # 登录相关组件
│   ├── systems.rs          # 登录相关系统
│   └── ui.rs               # 登录UI
├── select_scene/           # 角色选择场景
│   ├── mod.rs
│   ├── components.rs
│   ├── systems.rs
│   └── ui.rs
└── ui/                     # 共享UI组件
    ├── button.rs
    ├── text_input.rs
    └── dialog.rs
```

---

## 🏗 场景架构

### Scene Trait

所有场景都实现 `Scene` trait：

```rust
pub trait Scene {
    /// 向下转型支持
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    
    /// 更新场景逻辑（返回场景切换）
    fn update(
        &mut self,
        ctx: &mut Context,
        world: &mut World,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> GameResult<Option<SceneType>>;
    
    /// 绘制场景
    fn draw(
        &mut self, 
        ctx: &mut Context, 
        canvas: &mut Canvas, 
        world: &World
    ) -> GameResult;
    
    // 输入事件
    fn on_mouse_down(&mut self, ...) -> GameResult;
    fn on_mouse_up(&mut self, ...) -> GameResult;
    fn on_mouse_move(&mut self, ...) -> GameResult;
    fn on_key_down(&mut self, ...) -> GameResult;
    fn on_key_up(&mut self, ...) -> GameResult;
    fn on_text_input(&mut self, ...) -> GameResult;
}
```

### 场景生命周期

```
游戏启动
    ↓
LoginScene::new()
    ↓
LoginScene::update() ← 每帧更新
    ↓
用户登录成功
    ↓
返回 Some(SceneType::Select)
    ↓
场景切换
    ↓
SelectScene::new()
    ↓
SelectScene::update()
    ↓
选择角色进入游戏
    ↓
返回 Some(SceneType::Game)
    ↓
场景切换
    ↓
GameScene::new()
    ↓
GameScene::update() ← 主游戏循环
```

---

## 📦 场景详解

### 1. LoginScene - 登录场景

**职责**: 处理用户登录

#### 核心结构

```rust
pub struct LoginScene {
    /// ECS世界（场景独立）
    world: World,
    
    /// UI状态
    ui_state: LoginUIState,
    
    /// 网络状态
    network_state: LoginNetworkState,
}

pub struct LoginUIState {
    /// 账号输入框
    account_input: TextInput,
    
    /// 密码输入框
    password_input: TextInput,
    
    /// 登录按钮
    login_button: Button,
    
    /// 新账号按钮
    new_account_button: Button,
    
    /// 错误消息
    error_message: Option<String>,
}

pub enum LoginNetworkState {
    Idle,
    Connecting,
    Connected,
    LoggingIn,
    Success,
    Failed(String),
}
```

#### 主要功能

```rust
impl Scene for LoginScene {
    fn update(
        &mut self,
        ctx: &mut Context,
        world: &mut World,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> GameResult<Option<SceneType>> {
        // 1. 更新网络状态
        self.update_network_state();
        
        // 2. 更新UI状态
        self.update_ui(ctx)?;
        
        // 3. 处理登录逻辑
        if self.network_state == LoginNetworkState::Success {
            return Ok(Some(SceneType::Select));
        }
        
        Ok(None)
    }
    
    fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas, world: &World) -> GameResult {
        // 1. 绘制背景
        self.draw_background(ctx, canvas)?;
        
        // 2. 绘制UI元素
        self.ui_state.account_input.draw(ctx, canvas)?;
        self.ui_state.password_input.draw(ctx, canvas)?;
        self.ui_state.login_button.draw(ctx, canvas)?;
        self.ui_state.new_account_button.draw(ctx, canvas)?;
        
        // 3. 绘制错误消息
        if let Some(error) = &self.ui_state.error_message {
            self.draw_error(ctx, canvas, error)?;
        }
        
        Ok(())
    }
    
    fn on_mouse_down(
        &mut self,
        ctx: &mut Context,
        world: &mut World,
        button: MouseButton,
        x: f32,
        y: f32,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> GameResult {
        // 检查按钮点击
        if self.ui_state.login_button.contains(x, y) {
            self.handle_login(network_tx)?;
        } else if self.ui_state.new_account_button.contains(x, y) {
            self.handle_new_account(network_tx)?;
        }
        
        Ok(())
    }
}
```

#### 特性

- ✅ 账号密码输入
- ✅ 登录/注册
- ✅ 错误提示
- ✅ 网络连接管理
- 🚧 记住密码
- 🚧 自动登录

### 2. SelectScene - 角色选择场景

**职责**: 角色创建和选择

#### 核心结构

```rust
pub struct SelectScene {
    /// ECS世界
    world: World,
    
    /// 角色列表
    characters: Vec<SelectInfo>,
    
    /// 当前选中角色
    selected_index: Option<usize>,
    
    /// UI状态
    ui_state: SelectUIState,
}

pub struct SelectUIState {
    /// 角色卡片
    character_cards: Vec<CharacterCard>,
    
    /// 新建角色按钮
    new_character_button: Button,
    
    /// 删除角色按钮
    delete_character_button: Button,
    
    /// 开始游戏按钮
    start_game_button: Button,
    
    /// 角色创建对话框
    create_dialog: Option<CreateCharacterDialog>,
}

pub struct CharacterCard {
    pub name: String,
    pub class: MirClass,
    pub gender: MirGender,
    pub level: u16,
    pub position: (f32, f32),
    pub size: (f32, f32),
}
```

#### 主要功能

```rust
impl Scene for SelectScene {
    fn update(
        &mut self,
        ctx: &mut Context,
        world: &mut World,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> GameResult<Option<SceneType>> {
        // 1. 更新角色列表
        self.update_characters();
        
        // 2. 更新UI
        self.update_ui(ctx)?;
        
        // 3. 检查是否开始游戏
        if self.should_start_game() {
            self.send_start_game(network_tx)?;
            return Ok(Some(SceneType::Game));
        }
        
        Ok(None)
    }
    
    fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas, world: &World) -> GameResult {
        // 1. 绘制背景
        self.draw_background(ctx, canvas)?;
        
        // 2. 绘制角色卡片
        for (i, card) in self.ui_state.character_cards.iter().enumerate() {
            let selected = Some(i) == self.selected_index;
            card.draw(ctx, canvas, selected)?;
        }
        
        // 3. 绘制按钮
        self.ui_state.new_character_button.draw(ctx, canvas)?;
        if self.selected_index.is_some() {
            self.ui_state.delete_character_button.draw(ctx, canvas)?;
            self.ui_state.start_game_button.draw(ctx, canvas)?;
        }
        
        // 4. 绘制创建角色对话框
        if let Some(dialog) = &self.ui_state.create_dialog {
            dialog.draw(ctx, canvas)?;
        }
        
        Ok(())
    }
}
```

#### 特性

- ✅ 显示角色列表
- ✅ 选择角色
- ✅ 创建角色
- ✅ 删除角色
- ✅ 职业选择（战士/法师/道士）
- ✅ 性别选择
- 🚧 角色预览动画
- 🚧 装备预览

### 3. GameScene - 游戏主场景

**职责**: 游戏主逻辑和渲染

#### 核心结构

```rust
pub struct GameScene {
    /// ECS世界（游戏主世界）
    world: World,
    
    /// 玩家实体ID
    player_entity: Option<Entity>,
    
    /// 相机
    camera: Camera,
    
    /// UI管理器
    ui_manager: UIManager,
    
    /// 网络状态
    network_state: GameNetworkState,
}
```

#### 主要功能

```rust
impl Scene for GameScene {
    fn update(
        &mut self,
        ctx: &mut Context,
        world: &mut World,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> GameResult<Option<SceneType>> {
        // 1. 运行所有ECS系统（五层架构）
        self.run_layer1_systems(world, network_tx)?;  // 输入层
        self.run_layer2_systems(world)?;              // 逻辑层
        self.run_layer3_systems(world)?;              // 表现层
        self.run_layer4_systems(world, ctx)?;         // 渲染层
        self.run_layer5_systems(world, ctx)?;         // UI层
        
        // 2. 更新相机
        self.update_camera(world)?;
        
        // 3. 检查是否退出游戏
        if self.should_exit() {
            return Ok(Some(SceneType::Select));
        }
        
        Ok(None)
    }
    
    fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas, world: &World) -> GameResult {
        // 1. 设置相机变换
        self.camera.apply(canvas);
        
        // 2. 渲染地图
        self.render_map(ctx, canvas, world)?;
        
        // 3. 渲染对象（Y-sorting）
        self.render_objects(ctx, canvas, world)?;
        
        // 4. 渲染特效
        self.render_effects(ctx, canvas, world)?;
        
        // 5. 渲染UI（不受相机影响）
        canvas.reset_transform();
        self.ui_manager.draw(ctx, canvas, world)?;
        
        Ok(())
    }
    
    fn on_mouse_down(
        &mut self,
        ctx: &mut Context,
        world: &mut World,
        button: MouseButton,
        x: f32,
        y: f32,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> GameResult {
        // 1. 检查UI点击
        if self.ui_manager.handle_click(x, y) {
            return Ok(());
        }
        
        // 2. 世界交互（移动、攻击等）
        let world_pos = self.camera.screen_to_world(x, y);
        self.handle_world_click(world_pos, button, network_tx)?;
        
        Ok(())
    }
}
```

#### 特性

- ✅ 完整的ECS系统集成
- ✅ 地图渲染
- ✅ 对象渲染（Y-sorting）
- ✅ 相机系统
- ✅ UI管理
- ✅ 输入处理
- ✅ 网络同步
- 🚧  小地图
- 🚧  完整的UI对话框

---

## 📖 使用指南

### 场景创建

```rust
use crate::ecs::scenes::*;

// 创建登录场景
let mut login_scene = LoginScene::new(ctx)?;

// 创建角色选择场景
let mut select_scene = SelectScene::new(ctx, characters)?;

// 创建游戏场景
let mut game_scene = GameScene::new(ctx, character_info)?;
```

### 场景切换

```rust
// 游戏主循环
let mut current_scene: Box<dyn Scene> = Box::new(LoginScene::new(ctx)?);

loop {
    // 更新场景
    if let Some(next_scene_type) = current_scene.update(ctx, &mut world, &network_tx)? {
        // 切换场景
        current_scene = match next_scene_type {
            SceneType::Login => Box::new(LoginScene::new(ctx)?),
            SceneType::Select => Box::new(SelectScene::new(ctx, characters)?),
            SceneType::Game => Box::new(GameScene::new(ctx, character)?),
        };
    }
    
    // 绘制场景
    canvas.clear();
    current_scene.draw(ctx, &mut canvas, &world)?;
    canvas.finish(ctx)?;
}
```

### 场景间数据传递

```rust
// LoginScene → SelectScene: 传递角色列表
impl LoginScene {
    fn on_login_success(&self) -> Vec<SelectInfo> {
        self.characters.clone()
    }
}

// SelectScene → GameScene: 传递角色信息
impl SelectScene {
    fn on_start_game(&self) -> CharacterInfo {
        self.selected_character.clone()
    }
}
```

### 自定义场景

```rust
use crate::ecs::scenes::Scene;

pub struct CustomScene {
    world: World,
    // ... 场景特有数据
}

impl Scene for CustomScene {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    
    fn update(
        &mut self,
        ctx: &mut Context,
        world: &mut World,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> GameResult<Option<SceneType>> {
        // 实现更新逻辑
        Ok(None)
    }
    
    fn draw(
        &mut self, 
        ctx: &mut Context, 
        canvas: &mut Canvas, 
        world: &World
    ) -> GameResult {
        // 实现绘制逻辑
        Ok(())
    }
    
    // 实现其他方法...
}
```

---

## 🎯 场景设计原则

### 1. 场景独立性

每个场景应该是独立的，有自己的ECS世界：

```rust
// ✅ 正确：每个场景有独立的World
pub struct LoginScene {
    world: World,  // 登录场景的World
    // ...
}

pub struct GameScene {
    world: World,  // 游戏场景的World
    // ...
}

// ❌ 错误：共享World可能导致数据污染
```

### 2. 明确的生命周期

场景切换时应该正确清理资源：

```rust
impl Drop for GameScene {
    fn drop(&mut self) {
        // 清理资源
        self.world.clear();
        self.ui_manager.cleanup();
    }
}
```

### 3. 状态机模式

使用状态机管理场景内的状态：

```rust
pub enum LoginState {
    EnteringCredentials,
    Connecting,
    Authenticating,
    Success,
    Failed,
}

impl LoginScene {
    fn update_state(&mut self) {
        match self.state {
            LoginState::EnteringCredentials => {
                // 等待用户输入
            }
            LoginState::Connecting => {
                // 连接服务器
            }
            // ...
        }
    }
}
```

### 4. 解耦UI和逻辑

UI和游戏逻辑应该分离：

```rust
// ✅ 正确：UI和逻辑分离
pub struct SelectScene {
    ui_state: SelectUIState,     // UI状态
    game_state: SelectGameState, // 游戏状态
}

impl SelectScene {
    fn update(&mut self) {
        // 1. 更新游戏逻辑
        self.game_state.update();
        
        // 2. 同步UI状态
        self.ui_state.sync(&self.game_state);
    }
}
```

---

## 📊 开发状态

### 完成度统计

| 场景 | 完成度 | 说明 |
|------|--------|------|
| **LoginScene** | 90% | 核心功能完成，记住密码待实现 |
| **SelectScene** | 85% | 主要功能完成，动画待完善 |
| **GameScene** | 95% | 游戏主循环完成，部分UI待实现 |

### 已实现功能

#### ✅ LoginScene

- [x] 账号密码输入
- [x] 登录验证
- [x] 新账号注册
- [x] 错误提示
- [x] 网络连接

#### ✅ SelectScene

- [x] 角色列表显示
- [x] 角色选择
- [x] 创建角色
- [x] 删除角色
- [x] 职业/性别选择
- [x] 开始游戏

#### ✅ GameScene

- [x] ECS系统集成
- [x] 地图渲染
- [x] 对象渲染
- [x] 相机系统
- [x] 输入处理
- [x] 网络同步

### 未实现功能

#### ⏳ LoginScene

- [ ] 记住密码
- [ ] 自动登录
- [ ] 服务器选择
- [ ] 公告显示

#### ⏳ SelectScene

- [ ] 角色动画预览
- [ ] 装备预览
- [ ] 角色属性显示
- [ ] 背景音乐

#### ⏳ GameScene

- [ ] 完整的UI对话框系统
- [ ] 小地图
- [ ] 任务追踪UI
- [ ] 聊天窗口优化

---

## 🔗 相关文档

- **ECS系统**: `../systems/README.md` - 场景使用的系统
- **UI组件**: `../ui/README.md` - 场景使用的UI组件
- **组件定义**: `../components/README.md` - 场景使用的组件

---

**文档版本**: v1.0  
**最后更新**: 2025-10-28  
**维护者**: Crystal Mir2 Team
