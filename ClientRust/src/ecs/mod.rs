// ============================================================================
// ECS Architecture for Crystal Mir2 Client
// 使用 hecs 轻量级 ECS 库重构游戏逻辑
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
pub use systems::logic::update::{MapManager, MapUpdateSystem}; // 🆕 导出地图更新系统
pub use systems::*;
pub use world::GameWorld;

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

use crate::ecs::components::GlobalEvents;
use crate::network::NetContext;
use crate::ClientSettings;

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

pub trait WorldExt {
    fn spawn_settings(&mut self, settings: ClientSettings) -> &mut Self;
    fn spawn_network(&mut self, net_ctx: NetContext) -> &mut Self;
    fn spawn_global_events(&mut self, events: GlobalEvents) -> &mut Self;
    fn settings(&self) -> hecs::Ref<'_, ClientSettings>;
    fn network(&self) -> hecs::Ref<'_, crate::network::NetContext>;
    fn global_events(&self) -> hecs::Ref<'_, GlobalEvents>;
    fn global_events_mut(&mut self) -> &mut GlobalEvents;
}

// 使用有效的Entity ID (高32位为generation=1, 低32位为index)
pub const SETTING_ENTITY: Option<hecs::Entity> = hecs::Entity::from_bits(0x100000001);
pub const NETWORK_ENTITY: Option<hecs::Entity> = hecs::Entity::from_bits(0x100000002);
pub const GAME_EVENTS_ENTITY: Option<hecs::Entity> = hecs::Entity::from_bits(0x100000003);

impl WorldExt for hecs::World {
    fn spawn_settings(&mut self, settings: ClientSettings) -> &mut Self {
        self.spawn_at(
            SETTING_ENTITY.unwrap_or(hecs::Entity::DANGLING),
            (settings,),
        );
        self
    }
    fn spawn_network(&mut self, net_ctx: NetContext) -> &mut Self {
        self.spawn_at(NETWORK_ENTITY.unwrap_or(hecs::Entity::DANGLING), (net_ctx,));
        self
    }

    fn spawn_global_events(&mut self, events: GlobalEvents) -> &mut Self {
        self.spawn_at(
            GAME_EVENTS_ENTITY.unwrap_or(hecs::Entity::DANGLING),
            (events,),
        );
        self
    }

    fn settings(&self) -> hecs::Ref<'_, ClientSettings> {
        self.get::<&ClientSettings>(SETTING_ENTITY.unwrap_or(hecs::Entity::DANGLING))
            .expect("GameWorld ClientSettings not found")
    }

    fn network(&self) -> hecs::Ref<'_, crate::network::NetContext> {
        self.get::<&NetContext>(NETWORK_ENTITY.unwrap_or(hecs::Entity::DANGLING))
            .expect("GameWorld NetContext not found")
    }

    fn global_events(&self) -> hecs::Ref<'_, GlobalEvents> {
        self.get::<&GlobalEvents>(GAME_EVENTS_ENTITY.unwrap_or(hecs::Entity::DANGLING))
            .expect("GameWorld GlobalEvents not found")
    }

    fn global_events_mut(&mut self) -> &mut GlobalEvents {
        self.query_one_mut::<&mut GlobalEvents>(
            GAME_EVENTS_ENTITY.unwrap_or(hecs::Entity::DANGLING),
        )
        .expect("GameWorld GlobalEvents not found")
    }
}
