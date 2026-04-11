//! # 系统模块 (ecs/systems)
//!
//! 本模块包含游戏的所有系统（Systems），按分层架构组织。
//!
//! ## 快速导航
//!
//! - **完整架构说明**：请查看本模块底部的详细注释和 `README.md`
//! - **输入系统**：[`input`] 模块
//! - **逻辑系统**：[`logic`] 模块（包含 physics, combat, decision）
//! - **表现系统**：[`presentation`] 模块
//! - **渲染系统**：[`rendering`] 模块
//! - **优先级常量**：[`priority`] 模块
//!
//! ## 三种系统类型
//!
//! 1. **System** - 纯逻辑系统（实现 `update()`）
//! 2. **DrawSystem** - 纯渲染系统（实现 `draw()`）
//! 3. **HybridSystem** - 混合系统（同时实现 `update()` 和 `draw()`）
//!
//! ## 使用示例
//!
//! ```rust
//! use crate::systems::{System, priority};
//!
//! struct MySystem;
//!
//! impl System for MySystem {
//!     fn priority(&self) -> u32 { priority::MOVEMENT }
//!     
//!     fn update(&mut self, world: &mut hecs::World, dt: f32) -> GameResult {
//!         // 处理逻辑
//!         Ok(())
//!     }
//! }
//! ```

// 系统优先级常量定义
// ============================================================================
/// 系统优先级常量，用于控制系统执行顺序（数字越小越优先）
///
/// ## 分层说明
///
/// - **0-99**: 基础设施（资源、场景、保存）
/// - **100-199**: 输入与网络
/// - **200-599**: 游戏逻辑（AI、战斗、物理）
/// - **600-899**: 表现层（动画、音效、相机）
/// - **900-1999**: 渲染层（地图、实体、特效、UI）
/// - **9000+**: 调试工具
pub mod priority {
    // 第0層：基礎設施 (0-99)
    pub const RESOURCE_PRELOAD: u32 = 10; // 资源预加载系统
    pub const SCENE: u32 = 20; // 场景管理系统
    pub const SAVE: u32 = 30; // 保存系统

    // 第1層：輸入網絡 (100-199)
    pub const INPUT: u32 = 100;
    pub const NETWORK: u32 = 110; // 网络系统
    /// NetworkSystem 拉取事件后，立刻落地到 ECS 状态（本地玩家/切图定位等）
    pub const NETWORK_APPLY: u32 = 115;
    /// 本地玩家自动喝药（自动回血）
    pub const AUTO_POTION: u32 = 116;
    /// 无网络事件时的地图启动兜底（创建默认 MapManager/MapData）
    pub const MAP_BOOTSTRAP: u32 = 117;
    /// 消费 MapInformation/MapChanged 并加载/更新 MapData
    pub const MAP_LOAD: u32 = 118;
    /// 本地玩家自动战斗/挂机 AI（在 PlayerControlSystem 之前注入 PlayerInput）
    pub const LOCAL_PLAYER_AI: u32 = 119;
    pub const PLAYER_CONTROL: u32 = 120;
    pub const GAME_EVENT: u32 = 130;
    // 第2层：游戏逻辑(200-599)
    // ├── AI 系统: MonsterAISystem → NPCInteractionSystem → PetAISystem
    // ├── 社交系统: GuildSystem → PartySystem → FriendSystem
    // ├── 传奇特色: PKSystem → DungeonSystem → BossSystem → SiegeWarSystem
    // ├── 任务系统: QuestSystem → DailySystem → AchievementSystem
    // ├── 职业系统: ClassSystem → TalentSystem → SummonSystem
    // ├── 经济系统: InventorySystem → EquipmentSystem → AuctionSystem → MarketSystem
    // ├── 战斗系统: CombatSystem → SkillSystem → BuffDebuffSystem → RegenSystem
    // ├── 移动&自动化系统: MovementSystem → CollisionSystem → TeleportSystem → AutoBattleSystem → AutoPathfindingSystem
    pub const MONSTER_AI: u32 = 200;
    pub const NPC_INTERACTION: u32 = 210;
    pub const PET_AI: u32 = 230;
    pub const COMBAT: u32 = 300;
    pub const SKILL: u32 = 310;
    pub const BUFF_DEBUFF: u32 = 320;
    pub const REGEN: u32 = 330;
    pub const ATTACK: u32 = 340;
    // 注意：调度器按 priority 从小到大执行。
    // 寻路必须在移动之前运行，否则会出现“晚一帧才开始走/跟随”的延迟。
    pub const PATHFINDING: u32 = 490;
    pub const MOVEMENT: u32 = 500;
    pub const COLLISION: u32 = 510; // 碰撞检测在移动之后，检查并修正位置

    /// 远程玩家/对象位置插值（消除“瞬移感”）
    pub const REMOTE_INTERPOLATION: u32 = 585;

    /// 远程 Walk/Run 动作到期后自动回 Stand（避免“原地跑/走”）
    pub const REMOTE_MOVE_ANIM: u32 = 586;

    /// 基于 Lifetime 组件的实体清理（死亡尸体/特效等）
    pub const LIFETIME_CLEANUP: u32 = 598;

    // 第3层：表现层(600-899)
    // ├── 动画特效: AnimationSystem → ParticleSystem → WeatherSystem
    // ├── 音效系统: SoundSystem → VoiceChatSystem
    // ├── 摄像机系统: CameraFollowSystem → CameraSystem
    // └── UI 系统: UISystem → HUDSystem → MinimapSystem → DialogSystem
    // 装备/外观派生状态（渲染/动作切换）
    pub const MOUNT_STATE_SYNC: u32 = 590;
    pub const ANIMATION: u32 = 600;
    /// 怪物血条显示值平滑过渡（掉血动画）
    pub const HEALTH_BAR_ANIM: u32 = 605;
    pub const PARTICLE: u32 = 610;
    /// 浮动文字（伤害数字等）在粒子之后渲染
    pub const FLOATING_TEXT: u32 = 615;
    pub const WEATHER: u32 = 620;
    pub const SOUND: u32 = 630;
    pub const VOICE_CHAT: u32 = 640;
    pub const CAMERA_FOLLOW: u32 = 700;
    /// Space 按住时启用拖拽/缩放并切到 Manual
    pub const CAMERA_SPACE_GATE: u32 = 695;
    pub const CAMERA: u32 = 710;
    /// 相机边界限制（防止拖出地图）
    pub const CAMERA_BOUNDS: u32 = 720;
    pub const UI: u32 = 800;
    pub const HUD: u32 = 810;
    pub const MINIMAP: u32 = 820;
    pub const DIALOG: u32 = 830;

    // ↓
    // 第4层：渲染层(900-1999)
    // ├── 基础渲染: MapRenderSystem → SpriteRenderSystem → EffectRenderSystem → UIRenderSystem
    // └── 高级渲染: LightingRenderSystem → PostProcessSystem → TextRenderSystem
    pub const MAP_RENDER: u32 = 900;
    pub const SPRITE_RENDER: u32 = 910;
    pub const ENTITY_RENDER: u32 = 920; // EntityRenderSystem: 实体渲染（玩家/怪物）
    pub const EFFECT_RENDER: u32 = 925;
    pub const UI_RENDER: u32 = 930;
    pub const LIGHTING_RENDER: u32 = 940;
    pub const POST_PROCESS: u32 = 1000;
    pub const TEXT_RENDER: u32 = 1100;
    // ↓
    // 第5层：调试工具(9000+)
    // ├── 作弊系统 → 个人资料系统 → 调试系统 → 记录系统
    pub const CHEAT: u32 = 9000;
    pub const PROFILE: u32 = 9100;
    pub const DEBUG: u32 = 9200;
    pub const LOGGING: u32 = 9300;
    // ↓
    // 帧结束
    pub const FRAME_END: u32 = 20000;
}

pub mod dbug;
pub mod infra;
pub mod input;
pub mod logic;
pub mod presentation;
pub mod rendering;

// 重新导出派生宏
pub use ecs_macros::{LogicSystem, RenderSystem};

// 重新导出各层系统（保持向后兼容）
// 注意：新代码应使用 update:: 和 render:: 模块

pub use input::{AutoPotionSystem, LocalPlayerAiSystem, PlayerControlSystem};
pub use infra::NetworkSystem;
pub use infra::NetworkApplySystem;
pub use infra::{FrameEndSystem, TimeTickSystem};
pub use infra::MapBootstrapSystem;
pub use logic::combat::{
    CombatResult, CombatSystem, DamageType, HealthRegenSystem, SkillSystem, SpellInputSystem,
};
pub use logic::lifetime_cleanup_system::LifetimeCleanupSystem;
pub use logic::decision::{MonsterAISystem, NpcAISystem, NpcDialogueSystem};
pub use logic::physics::{CollisionSystem, MapLoadSystem, MapManager, MovementSystem, PathfindingSystem};
pub use presentation::{
    AnimationSystem, CameraBoundsSystem, CameraFollowSystem, CameraSpaceGateSystem, CameraSystem,
    FloatingTextSystem, MountStateSyncSystem, ParticleSystem, PositionInterpolationSystem,
    RemoteMoveAnimSystem,
};

// ============================================================================
// 系统 Trait 设计
// ============================================================================
//
// 本 ECS 系统实现了三类系统架构：
//
// ## 1. System - 纯逻辑系统
//    - 只需实现 `update()` 方法
//    - 用于 AI、物理、网络、战斗等纯逻辑处理
//    - 提供默认的元数据方法：name、is_enabled、priority
//
// ## 2. DrawSystem - 纯渲染系统
//    - 只需实现 `draw()` 方法
//    - 用于地图渲染、UI渲染等不需要逻辑更新的渲染任务
//    - 提供默认的元数据方法：name、is_enabled、priority
//
// ## 3. HybridSystem - 混合系统
//    - 同时实现 `update()` 和 `draw()` 方法
//    - 用于粒子系统、调试系统等需要逻辑更新和渲染的系统
//    - 提供默认的元数据方法：name、is_enabled、priority
//    - update() 在逻辑阶段执行，draw() 在渲染阶段执行
//
// ## 核心特性
// - **类型安全**：三种系统类型在编译期严格区分
// - **职责分离**：System 专注逻辑，DrawSystem 专注渲染，HybridSystem 同时处理两者
// - **自动调度**：调度器根据系统类型自动在正确的阶段调用相应方法
// - **灵活扩展**：所有元数据方法都有默认实现，只需覆盖需要的方法
// - **优先级控制**：通过 priority 常量精确控制系统执行顺序
//
// ## 使用示例
//
// ### 1. System - 纯逻辑系统
//    ```rust
//    use crate::systems::priority;
//
//    struct MovementSystem;
//
//    impl System for MovementSystem {
//        fn priority(&self) -> u32 { priority::MOVEMENT }  // 400
//
//        fn update(&mut self, world: &mut hecs::World, dt: f32) -> GameResult {
//            // 更新实体位置
//            Ok(())
//        }
//    }
//
//    // 使用声明宏注册
//    logic_system!(MovementSystem);
//    ```
//
// ### 2. DrawSystem - 纯渲染系统
//    ```rust
//    use crate::systems::priority;
//
//    struct MapRenderSystem;
//
//    impl DrawSystem for MapRenderSystem {
//        fn priority(&self) -> u32 { priority::MAP_RENDER }  // 1000
//
//        fn draw(
//            &mut self,
//            ctx: &mut Context,
//            canvas: &mut Canvas,
//            world: &hecs::World
//        ) -> GameResult {
//            // 绘制地图
//            Ok(())
//        }
//    }
//
//    // 使用声明宏注册
//    draw_system!(MapRenderSystem);
//    ```
//
// ### 3. HybridSystem - 混合系统
//    ```rust
//    use crate::systems::priority;
//
//    struct ParticleSystem;
//
//    impl HybridSystem for ParticleSystem {
//        fn priority(&self) -> u32 { priority::PARTICLE }  // 510
//
//        fn update(&mut self, world: &mut hecs::World, dt: f32) -> GameResult {
//            // 更新粒子生命周期和位置
//            Ok(())
//        }
//
//        fn draw(
//            &mut self,
//            ctx: &mut Context,
//            canvas: &mut Canvas,
//            world: &hecs::World
//        ) -> GameResult {
//            // 绘制粒子效果
//            Ok(())
//        }
//    }
//
//    // 使用声明宏注册
//    hybrid_system!(ParticleSystem);
//    ```
//
// ### 4. 批量注册系统
//    ```rust
//    // 批量注册多个纯逻辑系统
//    logic_system!(
//        MovementSystem,
//        CollisionSystem,
//        AISystem,
//    );
//
//    // 批量注册多个纯渲染系统
//    draw_system!(
//        MapRenderSystem,
//        SpriteRenderSystem,
//        UIRenderSystem,
//    );
//
//    // 批量注册多个混合系统
//    hybrid_system!(
//        ParticleSystem,
//        DebugSystem,
//    );
//    ```
//
// ## 调度流程
//
// 每帧执行顺序：
// 1. **Update 阶段**：按优先级执行所有 System 和 HybridSystem 的 update()
// 2. **Draw 阶段**：按优先级执行所有 DrawSystem 和 HybridSystem 的 draw()
//
// ## 设计要点
// - 需要 Rust nightly 工具链：`rustup default nightly`
// - 需要在 crate root 添加：`#![feature(specialization)]` 和 `#![allow(incomplete_features)]`
// - System 和 DrawSystem 的元数据方法需要分别定义（代码重复但功能独立）
// - 可以只覆盖需要的元数据方法，其他方法自动使用 trait 中的默认实现
// ============================================================================

/// ECS 更新系统抽象（V1 - 稳定版本）
///
/// 所有需要在逻辑更新阶段执行的系统都应实现此 trait。
pub trait LogicSystem {
    /// 系统名称，默认使用类型全名
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// 是否启用，默认为 true
    fn is_enabled(&self) -> bool {
        true
    }
    /// 更新方法，每帧在逻辑阶段调用
    fn update(&mut self, ctx: &mut crate::game::GameContext, delay_time: f32) -> crate::game::GameResult;
}

pub trait RenderSystem {
    /// 系统名称，默认使用类型全名
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// 是否启用，默认为 true
    fn is_enabled(&self) -> bool {
        true
    }

    /// 更新方法，每帧在逻辑阶段调用（可选实现）
    fn update(&mut self, _ctx: &mut crate::game::GameContext, _delay_time: f32) -> crate::game::GameResult {
        Ok(())
    }

    /// 绘制方法，每帧在渲染阶段调用（必须实现）
    /// 
    /// macroquad 使用全局函数渲染，不需要 ctx 和 canvas 参数
    /// 使用 macroquad::prelude::* 中的函数：
    /// - draw_texture()
    /// - draw_text()
    /// - draw_rectangle()
    /// 等等
    fn draw(&mut self, _world: &hecs::World) -> crate::game::GameResult;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderStageMask(u8);

impl RenderStageMask {
    pub const NONE: Self = Self(0);
    pub const NORMAL: Self = Self(1 << 0);
    pub const POST_FRONT: Self = Self(1 << 1);
    pub const UI: Self = Self(1 << 2);
    pub const ALL: Self = Self(Self::NORMAL.0 | Self::POST_FRONT.0 | Self::UI.0);

    pub fn contains(self, stage: crate::components::RenderStage) -> bool {
        match stage {
            crate::components::RenderStage::Normal => (self.0 & Self::NORMAL.0) != 0,
            crate::components::RenderStage::PostFront => (self.0 & Self::POST_FRONT.0) != 0,
            crate::components::RenderStage::Ui => (self.0 & Self::UI.0) != 0,
        }
    }

    pub fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

pub enum SystemKind {
    Update(Box<dyn LogicSystem>),
    Render(Box<dyn RenderSystem>),
}

/// 系统类型标记 trait - 用于在编译期判断系统应归入哪个队列
pub trait IntoSystemKind {
    fn into_kind(self: Box<Self>) -> SystemKind;
}

// ============================================================================
// 声明式宏 - 可选的批量实现方式（与派生宏功能相同）
// ============================================================================
//
// 推荐使用派生宏（`#[derive(LogicSystem)]` 等）为单个系统添加支持。
// 声明式宏适合批量为多个系统实现，两种方式可以混用。
//
// 示例对比：
// - 派生宏（推荐）：`#[derive(LogicSystem)] struct MySystem;`
// - 声明式宏（批量）：`impl_system!(System1, System2, System3);`
// ============================================================================

/// 为纯逻辑系统批量实现 IntoSystemKind
///
/// 等价于为每个类型添加 `#[derive(LogicSystem)]`
///
/// # 用法
/// ```rust
/// logic_system!(MovementSystem, CollisionSystem, AISystem);
/// ```
#[macro_export]
macro_rules! logic_system {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl $crate::ecs::systems::IntoSystemKind for $ty {
                fn into_kind(self: Box<Self>) -> $crate::ecs::systems::SystemKind {
                    $crate::ecs::systems::SystemKind::Update(self)
                }
            }
        )+
    };
}

/// 为混合系统批量实现 IntoSystemKind
///
/// 等价于为每个类型添加 `#[derive(RenderSystem)]`
///
/// # 用法
/// ```rust
/// render_system!(ParticleSystem, DebugSystem);
/// ```
#[macro_export]
macro_rules! render_system {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl $crate::ecs::systems::IntoSystemKind for $ty {
                fn into_kind(self: Box<Self>) -> $crate::ecs::systems::SystemKind {
                    $crate::ecs::systems::SystemKind::Render(self)
                }
            }
        )+
    };
}

/// 系统调度器的内部存储结构
enum SystemEntry {
    Update {
        system: Box<dyn LogicSystem>,
        priority: u32,
        order: u64,
    },
    Hybrid {
        system: Box<dyn RenderSystem>,
        priority: u32,
        order: u64,
        draw_mask: RenderStageMask,
    },
}

impl SystemEntry {
    fn priority(&self) -> u32 {
        match self {
            SystemEntry::Update { priority, .. } => *priority,
            // SystemEntry::Draw { priority, .. } => *priority,
            SystemEntry::Hybrid { priority, .. } => *priority,
        }
    }

    fn order(&self) -> u64 {
        match self {
            SystemEntry::Update { order, .. } => *order,
            SystemEntry::Hybrid { order, .. } => *order,
        }
    }

    fn is_enabled(&self) -> bool {
        match self {
            SystemEntry::Update { system, .. } => system.is_enabled(),
            // SystemEntry::Draw { system, .. } => system.is_enabled(),
            SystemEntry::Hybrid { system, .. } => system.is_enabled(),
        }
    }
}

pub struct SystemScheduler {
    systems: Vec<SystemEntry>,
    next_order: u64,
}

impl Default for SystemScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemScheduler {
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
            next_order: 0,
        }
    }

    /// 添加系统（自动判断类型）
    pub fn add_system<S>(&mut self, system: S, priority: u32) -> &mut Self
    where
        S: IntoSystemKind + 'static,
    {
        let order = self.next_order;
        self.next_order = self.next_order.wrapping_add(1);

        // 运行时检查优先级唯一性，避免不同系统意外共享同一优先级
        if let Some(existing) = self.systems.iter().find(|e| e.priority() == priority) {
            tracing::warn!(
                "⚠️ System priority collision: priority {} already used (order {}). New system will run after it.",
                priority,
                existing.order(),
            );
        }

        match IntoSystemKind::into_kind(Box::new(system)) {
            SystemKind::Update(sys) => {
                // let priority = sys.priority();
                self.systems.push(SystemEntry::Update {
                    system: sys,
                    priority,
                    order,
                });
            }
            // SystemKind::Draw(sys) => {
            //     let priority = sys.priority();
            //     self.systems.push(SystemEntry::Draw {
            //         system: sys,
            //         priority,
            //     });
            // }
            SystemKind::Render(sys) => {
                // let priority = sys.priority();
                self.systems.push(SystemEntry::Hybrid {
                    system: sys,
                    priority,
                    order,
                    draw_mask: RenderStageMask::ALL,
                });
            }
        }

        // 按优先级排序（数字越小越优先）；相同优先级保持插入顺序稳定
        self.systems
            .sort_by_key(|entry| (entry.priority(), entry.order()));
        self
    }

    /// 添加渲染系统（可指定仅在某些 RenderStage 下执行 draw）
    ///
    /// 注意：stage 过滤只影响 draw，不影响 update。
    pub fn add_render_system_staged<S>(
        &mut self,
        system: S,
        priority: u32,
        draw_mask: RenderStageMask,
    ) -> &mut Self
    where
        S: RenderSystem + 'static,
    {
        let order = self.next_order;
        self.next_order = self.next_order.wrapping_add(1);

        self.systems.push(SystemEntry::Hybrid {
            system: Box::new(system),
            priority,
            order,
            draw_mask,
        });

        self.systems
            .sort_by_key(|entry| (entry.priority(), entry.order()));
        self
    }

    /// 逻辑更新阶段 - 按优先级统一调度所有系统的 update
    pub fn update(&mut self, ctx: &mut crate::game::GameContext, delay_time: f32) -> crate::game::GameResult {
        for entry in &mut self.systems {
            if !entry.is_enabled() {
                continue;
            }

            match entry {
                SystemEntry::Update { system, .. } => {
                    tracing::trace!("🔄 Executing system update: {}", system.name());
                    system.update(ctx, delay_time)?;
                    tracing::trace!("✅ System update completed: {}", system.name());
                }
                SystemEntry::Hybrid { system, .. } => {
                    system.update(ctx, delay_time)?;
                } 
            }
        }

        Ok(())
    }

    pub fn draw(
        &mut self,
        world: &hecs::World,
    ) -> crate::game::GameResult {
        tracing::trace!("🎨 Starting draw phase");

        let stage = world
            .query::<&crate::components::RenderPass>()
            .iter()
            .next()
            .map(|p| p.stage)
            .unwrap_or(crate::components::RenderStage::Normal);

        for entry in &mut self.systems {
            if !entry.is_enabled() {
                continue;
            }

            match entry {
                // SystemEntry::Draw { system, .. } => {
                //     tracing::trace!("🎨 Drawing system: {}", system.name());
                //     system.draw(world)?;
                //     tracing::trace!("✅ System draw completed: {}", system.name());
                // }
                SystemEntry::Hybrid {
                    system,
                    draw_mask,
                    ..
                } => {
                    if !draw_mask.contains(stage) {
                        continue;
                    }
                    tracing::trace!("🎨 Drawing hybrid system: {}", system.name());
                    system.draw(world)?;
                    tracing::trace!("✅ Hybrid system draw completed: {}", system.name());
                }
                SystemEntry::Update { .. } => {
                    // 纯逻辑系统无需渲染
                }
            }
        }

        Ok(())
    }
}
