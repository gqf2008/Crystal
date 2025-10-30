// ============================================================================
// ECS Architecture for Crystal Mir2 Client
// 使用 hecs 轻量级 ECS 库重构游戏逻辑
// ============================================================================

pub mod components;
pub mod systems;
pub mod system_scheduler;
pub mod game_scene_scheduler;  // 🆕 GameScene专用调度器
pub mod parallel_scheduler;    // 🆕 并行系统调度器
pub mod world;
pub mod runtime;
// Map Loader 模块
pub mod map_loader;

// 坐标工具模块 - 统一地图/世界/屏幕坐标转换 (不是 ECS System)
pub mod coordinates;

// UI 模块
pub mod ui;

// 游戏主应用和场景系统
pub mod game_app;
pub mod scenes;

// IME 输入处理
pub mod ime_handler;

pub use components::*;
pub use systems::*;
pub use system_scheduler::{SystemScheduler, SystemStats};
pub use game_scene_scheduler::GameSceneScheduler;  // 🆕 导出GameScene调度器
pub use parallel_scheduler::{ParallelScheduler, ExecutionMode, ParallelSystemStats};  // 🆕 导出并行调度器
pub use world::GameWorld;

// Map Loader 导出
pub use map_loader::MapLoader;

// 坐标工具导出
pub use coordinates::{
    Coordinates, ViewportConfig, ObjectRenderer,
    CELL_WIDTH, CELL_HEIGHT, MapUtils, CameraController
};

// UI 导出
pub use ui::{CharacterStatus, HealthBar, ManaBar, ExpBar, SkillBar, ChatWindow};

// 游戏应用导出
pub use game_app::GameState;
pub use scenes::{Scene, SceneType, LoginScene, SelectScene, GameScene};

// ============================================================================
// 架构说明
// ============================================================================
//
// GGEZ + hecs 分工:
// - GGEZ: 负责渲染、音频、输入处理、资源管理
// - hecs: 负责游戏实体(Entity)和逻辑(System)
//
// 优势:
// 1. 实体管理更清晰 (Player/Monster/NPC/Spell 都是 Entity)
// 2. 逻辑解耦 (Movement/Combat/AI 等系统独立)
// 3. 性能优异 (hecs 的缓存友好设计)
// 4. 保留 GGEZ 的简单渲染 (ADD 混合等特性)
//
// 实体类型:
// - Player Entity: Position + Velocity + Sprite + PlayerData + Health
// - Monster Entity: Position + Velocity + Sprite + MonsterData + AI + Health
// - NPC Entity: Position + Sprite + NPCData + Dialogue
// - Spell Entity: Position + Velocity + Sprite + SpellData + Lifetime
// - Item Entity: Position + Sprite + ItemData
//
// 系统类型:
// - MovementSystem: 处理实体移动
// - CombatSystem: 处理战斗逻辑
// - AISystem: 处理怪物AI
// - AnimationSystem: 处理动画状态
// - RenderSystem: 渲染所有可见实体 (调用 GGEZ)
// - NetworkSystem: 同步远程玩家
//
// ============================================================================
