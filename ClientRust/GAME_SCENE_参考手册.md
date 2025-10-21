# GameScene 快速参考

## 🎮 功能概览

`GameScene` 是游戏主场景，基于 ECS 架构，集成了地图渲染、角色控制、UI 系统和网络通信。

## 📁 文件位置

```
ClientRust/src/ecs/scenes/game_scene.rs
```

## 🔑 核心组件

### 结构体字段
```rust
pub struct GameScene {
    camera_entity: Entity,           // 相机实体
    time_entity: Entity,              // 时间跟踪实体
    config_entity: Entity,            // 渲染配置实体
    visible_area_entity: Entity,      // 可见区域缓存实体
    network_system: NetworkSystem,    // 网络同步系统
    ui_font_name: String,            // UI字体名称
}
```

### 创建的 ECS 实体

| 实体 | 组件 | 用途 |
|------|------|------|
| 相机 | Position, Camera, Draggable | 视角控制 |
| 时间跟踪 | TimeTracker | FPS计算、帧计数 |
| 渲染配置 | RenderConfig | 渲染开关控制 |
| 可见区域 | VisibleArea | 裁剪优化缓存 |
| 玩家 | Player, Position | 角色状态和位置 |
| 鼠标输入 | MouseInput | 鼠标状态跟踪 |
| 角色状态 | CharacterStatus | HP/MP/EXP |
| 血条 | HealthBar | 血量显示 |
| 魔法条 | ManaBar | 魔法值显示 |
| 经验条 | ExpBar | 经验值显示 |
| 技能栏 | SkillBar | 技能快捷键 |
| 聊天窗口 | ChatWindow | 聊天消息 |

## 🎯 主要方法

### 初始化
```rust
pub fn new(ctx: &mut Context, world: &mut World) -> GameResult<Self>
```
- 初始化地图库
- 加载地图文件（0.map）
- 创建所有 ECS 实体
- 加载中文字体

### 网络事件处理
```rust
pub fn handle_network_event(&mut self, world: &mut World, event: &GameEvent)
```
- 处理服务器发来的游戏事件
- 更新世界状态

### Scene Trait 实现

#### 更新
```rust
fn update(&mut self, ctx: &mut Context, world: &mut World, ...) -> GameResult<Option<SceneType>>
```
- 帧率限制（默认 160 FPS）
- 更新动画系统
- 更新相机系统
- 更新角色系统

#### 渲染
```rust
fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas, world: &World) -> GameResult
```
- 渲染地图瓦片（多层）
- 渲染角色
- 渲染 UI（HP/MP/技能/聊天）
- 显示 FPS 和提示

#### 键盘输入
```rust
fn on_key_down(&mut self, ..., input: KeyInput, ...) -> GameResult<Option<SceneType>>
```

**支持的按键**:
- `Esc` → 返回选择角色场景
- `W / ↑` → 向上移动
- `S / ↓` → 向下移动
- `A / ←` → 向左移动
- `D / →` → 向右移动
- `Shift + WASD` → 跑步

#### 鼠标输入
```rust
fn on_mouse_down(&mut self, ..., button: MouseButton, x: f32, y: f32) -> GameResult
fn on_mouse_up(&mut self, ..., button: MouseButton, x: f32, y: f32) -> GameResult
fn on_mouse_move(&mut self, ..., x: f32, y: f32) -> GameResult
```

## 🔌 网络集成

### 发送的命令

```rust
// 普通移动
NetworkCommand::Walk { direction: MirDirection }

// 跑步
NetworkCommand::Run { direction: MirDirection }
```

### 处理的事件

通过 `NetworkSystem::process_event()` 处理所有 `GameEvent`

## 🎨 渲染层次

1. **地图瓦片**
   - Back 层（背景）
   - Middle 层（地面）
   - Front 层（前景）

2. **角色**
   - 玩家角色
   - NPC/怪物（待实现）

3. **UI 层**
   - 状态条（血、蓝、经验）
   - 技能栏
   - 聊天窗口
   - FPS 显示
   - 提示文字

## ⚙️ 配置项

通过 `RenderConfig` 组件控制：

```rust
pub struct RenderConfig {
    show_back: bool,           // 显示背景层
    show_middle: bool,         // 显示地面层
    show_front: bool,          // 显示前景层
    show_grid: bool,           // 显示网格
    show_obstacles: bool,      // 显示障碍物
    show_animations: bool,     // 显示动画
    show_borders: bool,        // 显示边界
    show_path: bool,          // 显示路径
    max_fps: usize,           // 最大FPS（默认160）
    enable_lod: bool,         // 启用LOD
}
```

## 📊 性能优化

1. **帧率限制**: 通过 `max_fps` 控制
2. **可见区域裁剪**: 只渲染屏幕内的瓦片
3. **LOD 支持**: 远处物体使用低精度渲染
4. **组件缓存**: 避免重复查询

## 🔍 调试信息

### FPS 显示
- 位置: 左上角 (10, 10)
- 颜色: 绿色
- 格式: `FPS: {fps:.1}`

### 操作提示
- 位置: 右上角
- 颜色: 灰色
- 内容: `[WASD/方向键] 移动  [Shift+WASD] 跑动  [鼠标] 点击移动  [Esc] 返回`

## 🚀 使用示例

```rust
// 在 GameApp 中创建
let game_scene = GameScene::new(ctx, world)?;

// 处理网络事件
if let Some(event) = network_rx.try_recv() {
    game_scene.handle_network_event(world, &event);
}

// 更新
game_scene.update(ctx, world, &network_tx)?;

// 渲染
game_scene.draw(ctx, canvas, world)?;
```

## 📌 注意事项

1. **初始化顺序**: 必须先调用 `initialize_all_libraries()` 再创建场景
2. **坐标系统**: 使用世界坐标，通过相机转换到屏幕坐标
3. **方向枚举**: 使用 `mir2_shared::enums::MirDirection`
4. **字体加载**: 自动尝试加载 Windows 系统中文字体

## 🔗 相关模块

- `crate::ecs::systems::*` - 各种系统实现
- `crate::ecs::components::*` - ECS 组件定义
- `crate::ecs::ui::*` - UI 组件和渲染器
- `crate::network::*` - 网络通信模块
- `crate::objects::MapReader` - 地图文件读取

---

**版本**: 1.0  
**最后更新**: 2025-10-21
