//! ECS 模块（ClientRust/src/ecs）
//!
//! 更多文档请参见 `ClientRust/src/ecs/README.md`。
//!
//! 本模块使用 `hecs` 管理实体与组件，渲染与输入由 `ggez` 负责。
// ============================================================================

pub mod components;
pub mod resources;
pub mod runtime;
// pub mod system_scheduler;  // ✅ 使用经过测试的 SystemScheduler
pub mod systems;
// pub mod update_render_parallel_scheduler; // 🔒 暂时注释
pub mod world; // 🆕 全局游戏资源 (Resources)
               // Map Loader 模块
pub mod map_loader;
pub mod game_context; // 🆕 GameContext - 零拷贝输入访问

// 坐标工具模块 - 统一地图/世界/屏幕坐标转换 (不是 ECS System)
pub mod coord;

// UI 模块
pub mod ui;

// 游戏主应用和场景系统
pub mod game_app;
pub mod scenes;

// IME 输入处理
pub mod ime_handler;

pub use resources::*; // 🆕 导出全局资源
pub use systems::logic::physics::{MapManager, MapUpdateSystem}; // 🆕 导出地图更新系统
pub use systems::input::InputStateSystem; // 🆕 导出输入状态系统
pub use systems::*;
pub use world::GameWorld;
pub use game_context::{GameContext, InputContext}; // 🆕 导出 GameContext
// Map Loader 导出
pub use map_loader::MapLoader;

// 坐标工具导出
pub use coord::{
    CameraController, Coord, MapUtils, ObjectRenderer, ViewportConfig, CELL_HEIGHT,
    CELL_WIDTH,
};

// UI 导出
pub use ui::{CharacterStatus, ChatWindow, ExpBar, HealthBar, ManaBar, SkillBar};

// 游戏应用导出
pub use game_app::GameState;
pub use scenes::{GameScene, LoginScene, Scene, SceneType, SelectScene};

use crate::network::NetContext;
use crate::ClientSettings;

// ============================================================================
// WorldExt trait - 向后兼容的 World 扩展
// ============================================================================

/// WorldExt trait - ECS World 扩展方法（向后兼容）
///
/// **注意**: 此 trait 主要用于向后兼容现有代码。
/// 
/// **推荐**: 新代码应使用 `GameWorld` 而非直接操作 `hecs::World`。
/// 
/// `GameWorld` 提供了相同的方法，以及额外的实体工厂方法：
/// ```rust
/// let mut game_world = GameWorld::new();
/// game_world.spawn_settings(settings);
/// game_world.spawn_local_player("Player", class, gender, position);
/// ```
/// 
/// 此 trait 保留是因为 `GameContext` 仍然直接暴露 `hecs::World`
pub trait WorldExt {
    fn spawn_settings(&mut self, settings: ClientSettings) -> &mut Self;
    fn spawn_network(&mut self, net_ctx: NetContext) -> &mut Self;
    fn settings(&self) -> hecs::Ref<'_, ClientSettings>;
    fn network(&self) -> hecs::Ref<'_, NetContext>;
}

// 使用有效的Entity ID (高32位为generation=1, 低32位为index)
const SETTING_ENTITY_ID: hecs::Entity = hecs::Entity::from_bits(0x100000001).unwrap();
const NETWORK_ENTITY_ID: hecs::Entity = hecs::Entity::from_bits(0x100000002).unwrap();

/// 全局配置实体的常量（用于向后兼容 map_update_system.rs）
pub const SETTING_ENTITY: Option<hecs::Entity> = Some(SETTING_ENTITY_ID);
/// 网络上下文实体的常量（用于向后兼容 map_update_system.rs）
pub const NETWORK_ENTITY: Option<hecs::Entity> = Some(NETWORK_ENTITY_ID);

impl WorldExt for hecs::World {
    fn spawn_settings(&mut self, settings: ClientSettings) -> &mut Self {
        self.spawn_at(SETTING_ENTITY_ID, (settings,));
        self
    }
    
    fn spawn_network(&mut self, net_ctx: NetContext) -> &mut Self {
        self.spawn_at(NETWORK_ENTITY_ID, (net_ctx,));
        self
    }

    fn settings(&self) -> hecs::Ref<'_, ClientSettings> {
        self.get::<&ClientSettings>(SETTING_ENTITY_ID)
            .expect("ClientSettings not found in World")
    }

    fn network(&self) -> hecs::Ref<'_, NetContext> {
        self.get::<&NetContext>(NETWORK_ENTITY_ID)
            .expect("NetContext not found in World")
    }
}

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
