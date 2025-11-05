# ECS 模块（ClientRust/src/ecs）概览

本 README 说明重构后的 ECS（Entity-Component-System）架构，帮助开发者快速理解各子模块职责、核心 API 和迁移注意事项。

## 目标

- 使用 `hecs` 作为轻量 ECS 实现，职责清晰、性能良好。
- 将渲染与底层窗口交由 `ggez` 处理，ECS 专注于游戏逻辑与实体管理。

## 主要概念与导出（重要）

- `GameContext`：集成 ggez 核心组件（fs/gfx/keyboard/mouse/gamepad/time）与游戏资源（`hecs::World`、`NetContext`、`ClientSettings`），并提供零拷贝输入/网络事件访问。
- `WorldExt`：为 `hecs::World` 提供便捷单例访问（settings、network）的扩展方法（`spawn_settings`, `spawn_network`, `settings()`, `network()`）。
- `GameWorld`：包含一组工厂方法用于创建玩家/怪物/NPC/特效/掉落等实体，以及常用查询与清理方法。
- `resources.rs`：全局唯一的数据结构（`CurrentMap`、`GroupData`、`GuildData`、`FriendList`、`ActiveQuests`、`TradingState`、`HeroData` 等），以普通 Rust 类型管理（非 hecs 组件）。

## 目录结构（高层）

- `mod.rs` - 模块顶层导出与架构说明。导出常用类型和 re-exports（例如 `GameWorld`、`GameContext`）。
- `resources.rs` - 全局资源定义（不是组件）。
- `world.rs` - `GameWorld`: hecs 世界管理与实体工厂。
- `game_context.rs` - `GameContext` 与 `InputContext` 的实现与帮助方法。
- `runtime.rs` - 客户端运行时初始化（图形库、字体、日志等）。
- `components/` - 所有 ECS 组件按子模块组织（player, actor, item, map, render, input, movement 等）。
- `systems/` - 游戏逻辑系统（Movement, Combat, AI, MapUpdateSystem 等）。
- `scenes/` - 场景与 Scene 管理（`GameScene`, `LoginScene`, `SelectScene` 等）。
- `ui/` - UI 组件（聊天、血条、技能栏等）。

## 快速使用示例

1. 创建 `GameContext`（通过 builder）：

```rust
let (mut ctx, event_loop) = GameContext::builder(game_id, conf, fs, settings)?;
```

2. 在系统/逻辑中访问 World、Network、Settings：

```rust
// 从 GameContext
let world = ctx.world_mut();
let net = ctx.network();
let settings = ctx.settings();

// 或者直接使用 hecs::World 的扩展
let local_player = ctx.world.get_local_player();
```

3. 使用 `GameWorld` 工厂创建实体（示例）：

```rust
let mut gw = GameWorld::new();
let player = gw.spawn_local_player("hero".into(), MirClass::Warrior, MirGender::Male, Point::new(10, 10));
```

## 迁移/重构注意事项（给维护者）

- 输入与网络事件现在应通过 `GameContext` 的事件缓冲访问（`frame_input_events` 与 `net_events`），避免直接作为单例组件广播，优先使用 `InputContext`/`ctx.input()`。
- 全局数据（好友、公会、任务等）迁移至 `resources.rs` 的类型：这些不是 hecs 组件。如需跨线程共享，请在调用处使用 `Arc<RwLock<T>>`。
- 避免使用 `as_ggez_context()`（unsafe transmute），优先使用 `split_gfx_world()` 以获取 `GraphicsContext` 与 `World` 的可变引用。

## 建议的下一步

- 为 `systems/` 中的关键系统（例如 `MapUpdateSystem`、`MovementSystem`）补充 README 或 Rust doc 注释，列出依赖组件与资源。
- 为 `components/` 内的主要子模块（player、actor、map、render）生成简要 API 摘要，便于快速查阅字段与用途。

----

若需要，我可以继续为 `components/`、`systems/`、`scenes/`、`ui/` 生成更详细的子目录 README（包含文件列表、关键类型与示例）。
