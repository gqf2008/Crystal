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
pub mod game_world; // 🆕 全局游戏资源 (Resources)
                    // Map Loader 模块
pub mod game_context;
pub mod map_loader; // 🆕 GameContext - 零拷贝输入访问

// 坐标工具模块 - 统一地图/世界/屏幕坐标转换 (不是 ECS System)
pub mod coord;

// UI 模块
pub mod ui;

// 游戏主应用和场景系统
pub mod game_app;
pub mod scenes;

// IME 输入处理
pub mod ime_handler;

pub use game_context::{GameContext, InputContext};
pub use game_world::GameWorld;
pub use resources::*; // 🆕 导出全局资源
pub use systems::input::InputStateSystem; // 🆕 导出输入状态系统
pub use systems::logic::physics::{MapManager, MapUpdateSystem}; // 🆕 导出地图更新系统
pub use systems::*; // 🆕 导出 GameContext
                    // Map Loader 导出
pub use map_loader::MapLoader;

// 坐标工具导出
pub use coord::{
    CameraController, Coord, MapUtils, ObjectRenderer, ViewportConfig, CELL_HEIGHT, CELL_WIDTH,
};

// UI 导出
pub use ui::{CharacterStatus, ChatWindow, ExpBar, HealthBar, ManaBar, SkillBar};

// 游戏应用导出
pub use game_app::GameState;
pub use scenes::{GameScene, LoginScene, Scene, SceneType, SelectScene};

// 使用有效的Entity ID (高32位为generation=1, 低32位为index)
const SETTING_ENTITY_ID: hecs::Entity = hecs::Entity::from_bits(0x100000001).unwrap();
const NETWORK_ENTITY_ID: hecs::Entity = hecs::Entity::from_bits(0x100000002).unwrap();

/// 全局配置实体的常量（用于向后兼容 map_update_system.rs）
pub const SETTING_ENTITY: Option<hecs::Entity> = Some(SETTING_ENTITY_ID);
/// 网络上下文实体的常量（用于向后兼容 map_update_system.rs）
pub const NETWORK_ENTITY: Option<hecs::Entity> = Some(NETWORK_ENTITY_ID);
