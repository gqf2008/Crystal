// ============================================================================
// ECS Systems - 三类系统架构 (System / DrawSystem / HybridSystem)
// ============================================================================
//
// ## 系统分类
//
// ### 1. System - 纯逻辑系统（只有 update）
//    实现 `System` trait，只需提供 `update()` 方法。
//    用于 AI、物理、网络、战斗等纯逻辑处理。
//
//    示例：MovementSystem, AISystem, CombatSystem
//
// ### 2. DrawSystem - 纯渲染系统（只有 draw）
//    实现 `DrawSystem` trait，只需提供 `draw()` 方法。
//    用于地图渲染、UI渲染等不需要逻辑更新的渲染任务。
//
//    示例：MapRenderSystem, UIRenderSystem
//
// ### 3. HybridSystem - 混合系统（update + draw）
//    实现 `HybridSystem` trait，同时提供 `update()` 和 `draw()` 方法。
//    用于粒子系统、调试系统等需要逻辑更新和渲染的系统。
//
//    示例：ParticleSystem, DebugSystem
//
// ## 执行流程
//
// 每帧执行顺序：
// 1. **Update 阶段**：按优先级执行所有 System 和 HybridSystem 的 update()
// 2. **Draw 阶段**：按优先级执行所有 DrawSystem 和 HybridSystem 的 draw()
//
// ## 系统优先级设计（六阶段架构）
//
// ### Update 阶段 (50-699)
//
// **阶段 1: 输入与网络 (50-199)**
//   接收网络数据、处理输入、玩家控制、事件分发
//   - NetworkRecvSystem(50) → InputSystem(100) → PlayerControlSystem(110) → GameEventSystem(120)
//
// **阶段 2: AI 与决策 (200-299)**
//   AI 行为逻辑、对话处理
//   - MonsterAISystem(200) → NpcAISystem(210) → DialogueSystem(220)
//
// **阶段 3: 战斗与技能 (300-399)**
//   技能释放、战斗计算
//   - SkillSystem(300) → CombatSystem(310)
//
// **阶段 4: 移动与物理 (400-499)**
//   实体移动、碰撞检测、相机跟随（逻辑层）
//   - MovementSystem(400) → CollisionSystem(410) → CameraFollowSystem(420)
//
// **阶段 5: 状态更新 (500-599)**
//   动画状态、粒子更新、音效触发、相机矩阵计算（准备渲染）
//   - AnimationSystem(500) → ParticleSystem(510, Hybrid) → SoundSystem(520) → CameraSystem(530)
//
// **阶段 6: 事件清理 (900)**
//   每帧结束清理 GlobalEvents 中的临时事件，防止事件污染
//   - EventCleanupSystem(900)
//
// ### Draw 阶段 (1000-1999)
//
// **阶段 7: 渲染 (1000-1999)**
//   按层级顺序渲染
//   - MapRenderSystem(1000) → SpriteRenderSystem(1010) → EffectRenderSystem(1020)
//     → UIRenderSystem(1030) → DebugSystem(1100, Hybrid)
//
// ## 数据流
// 网络/输入 → AI决策 → 战斗技能 → 移动物理 → 状态更新 → 事件清理 → 渲染输出
//
// ## 关键设计
// 1. **InputSystem** 负责收集所有输入源（键盘/鼠标/网络）并写入 GlobalEvents 组件
// 2. **PlayerControlSystem** 从 GlobalEvents 读取输入和网络数据，更新玩家状态（包含位置修正逻辑）
// 3. **GameEventSystem** 统一管理系统间事件通信
// 4. **EventCleanupSystem** 在每帧最后清理临时事件，防止下一帧重复处理
// 5. **CameraFollowSystem(420)** 在移动阶段更新跟随逻辑
// 6. **CameraSystem(530)** 在状态更新阶段计算最终矩阵
// 7. **ParticleSystem(510)** 和 **DebugSystem(1100)** 使用 HybridSystem 同时处理逻辑和渲染
//
// ============================================================================

// ============================================================================
//
// # 热血传奇 ECS 系统职责说明表
//
// | 阶段 | 系统名称 | 类型 | 优先级 | 职责说明 |
// |------|----------|------|--------|----------|
// | **第一阶段：输入和网络** | InputSystem | System | 100 | [Input] 收集所有输入源（键盘/鼠标/网络数据包）并写入 GlobalEvents 组件 |
// | | PlayerControlSystem | System | 110 | [Update] 从 GlobalEvents 读取输入和网络数据，更新玩家状态，包含位置修正算法 |
// | | GameEventSystem | System | 120 | [Update] 管理游戏事件分发（任务、系统、UI事件），协调系统间通信 |
// | **第二阶段：AI和决策** | MonsterAISystem | System | 200 | 怪物AI逻辑（巡逻、追击、攻击决策、状态切换） |
// | | NpcAISystem | System | 210 | NPC行为逻辑（对话触发、任务发放、商店交互） |
// | | DialogueSystem | System | 220 | 处理对话树、选项分支、对话进度管理 |
// | **第三阶段：战斗和技能** | SkillSystem | System | 300 | 技能释放逻辑、冷却计算、技能效果应用 |
// | | CombatSystem | System | 310 | 战斗伤害计算、命中判定、暴击处理、死亡判断 |
// | **第四阶段：移动和物理** | MovementSystem | System | 400 | 实体移动更新、路径追踪、速度方向计算 |
// | | CollisionSystem | System | 410 | 碰撞检测与响应、障碍物判断、实体间碰撞 |
// | | CameraFollowSystem | System | 420 | 相机跟随逻辑、目标追踪、平滑移动、边界限制 |
// | **第五阶段：状态更新** | AnimationSystem | System | 500 | 动画状态机更新、帧切换、动画混合 |
// | | ParticleSystem | **Hybrid** | 510 | **[Update]** 粒子生命期管理、位置速度计算<br>**[Draw]** 粒子效果渲染 |
// | | SoundSystem | System | 520 | 音效触发管理、3D音效位置计算、音量控制 |
// | | CameraSystem | System | 530 | 相机矩阵计算、震动效果、过场动画、最终视图矩阵 |
// | **第六阶段：事件清理** | EventCleanupSystem | System | 900 | 清理 GlobalEvents 中的临时事件，防止下一帧重复处理 |
// | **第七阶段：渲染** | MapRenderSystem | DrawSystem | 1000 | 地图图层渲染、地形绘制、遮罩处理 |
// | | SpriteRenderSystem | DrawSystem | 1010 | 精灵实体渲染、排序、批处理优化 |
// | | EffectRenderSystem | DrawSystem | 1020 | 特效渲染（技能特效、光影、后处理） |
// | | UIRenderSystem | DrawSystem | 1030 | UI界面渲染、HUD、文字显示 |
// | | DebugSystem | **Hybrid** | 1100 | **[Update]** 性能统计收集、数据采样<br>**[Draw]** 调试信息显示、开发工具 |
//
// ============================================================================
//
// ## 系统类型说明
//
// - **System**: 纯逻辑系统，只实现 `update()` 方法，用于游戏逻辑处理
// - **DrawSystem**: 纯渲染系统，只实现 `draw()` 方法，用于图形绘制
// - **Hybrid**: 混合系统，同时实现 `update()` 和 `draw()` 方法，用于需要状态更新和渲染的系统
//
// ## 系统依赖关系说明
//
// 1. **数据流动**：
//    网络接收 → 输入处理 → 控制响应 → 事件触发 → AI决策 → 战斗计算
//    → 移动物理 → 状态更新 → 网络发送 → 渲染显示
//
// 2. **关键依赖**：
//    - PlayerControlSystem 依赖 InputSystem 的输入数据
//    - CameraFollowSystem 依赖 MovementSystem 的位置更新
//    - CameraSystem 依赖 CameraFollowSystem 的跟随逻辑
//    - 所有战斗相关系统依赖 GameEventSystem 的事件通知
//    - HybridSystem 的 update 在逻辑阶段执行，draw 在渲染阶段执行
//
// 3. **渲染顺序**：地图 → 实体 → 特效 → UI → 调试信息（从底层到顶层）
//
// ============================================================================
// 系统优先级常量定义
// ============================================================================
/// 系统优先级常量，用于控制系统执行顺序（数字越小越优先）
pub mod priority {
    // 阶段 1: 输入与网络 (50-199)
    pub const NETWORK_RECV: u32 = 50;
    pub const INPUT: u32 = 100;
    pub const PLAYER_CONTROL: u32 = 110;
    pub const GAME_EVENT: u32 = 120;

    // 阶段 2: AI 与决策 (200-299)
    pub const MONSTER_AI: u32 = 200;
    pub const NPC_AI: u32 = 210;
    pub const DIALOGUE: u32 = 220;

    // 阶段 3: 战斗与技能 (300-399)
    pub const SKILL: u32 = 300;
    pub const COMBAT: u32 = 310;

    // 阶段 4: 移动与物理 (400-499)
    pub const MOVEMENT: u32 = 400;
    pub const COLLISION: u32 = 410;
    pub const CAMERA_FOLLOW: u32 = 420;

    // 阶段 5: 状态更新 (500-599)
    pub const ANIMATION: u32 = 500;
    pub const PARTICLE: u32 = 510;
    pub const SOUND: u32 = 520;
    pub const CAMERA: u32 = 530;

    // 阶段 6: 网络同步与事件清理 (600-699)
    pub const EVENT_CLEANUP: u32 = 900;

    // 阶段 7: 渲染 (1000-1999)
    pub const MAP_RENDER: u32 = 1000;
    pub const SPRITE_RENDER: u32 = 1010;
    pub const EFFECT_RENDER: u32 = 1020;
    pub const UI_RENDER: u32 = 1030;
    pub const DEBUG_RENDER: u32 = 1100;
}

// ✅ update/render 架构（推荐）
pub mod logic;
pub mod render;

use ggez::graphics::Canvas;
use ggez::GameResult;

// 重新导出派生宏
pub use ecs_macros::{HybridSystem, LogicSystem, RenderSystem};

// 重新导出各层系统（保持向后兼容）
// 注意：新代码应使用 update:: 和 render:: 模块

// Layer 1 (Input) - 向后兼容导出
pub use logic::input::PlayerControlSystem;

// Layer 2 (Decision) - 向后兼容导出
pub use logic::decision::{MonsterAISystem, NpcAISystem, NpcDialogueSystem};

// Layer 3 (Combat & Skills) - 向后兼容导出
pub use logic::combat_skill::{
    CombatResult, CombatSystem as CombatSystemV2, DamageType, SkillSystem,
};

// Layer 4 (Physics & Movement) - 向后兼容导出
pub use logic::physics::{CollisionSystem, MovementSystem};

// Layer 5 (State Update) - 向后兼容导出
pub use logic::update::{
    AnimationSystem, CameraSystem, HealthRegenSystem, ParticleSystem, SoundSystem,
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
//    use crate::ecs::systems::priority;
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
//    use crate::ecs::systems::priority;
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
//    use crate::ecs::systems::priority;
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

/// ECS 更新系统抽象
///
/// 所有需要在逻辑更新阶段执行的系统都应实现此 trait。
pub trait System {
    /// 系统名称，默认使用类型全名
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// 是否启用，默认为 true
    fn is_enabled(&self) -> bool {
        true
    }

    /// 优先级（数字越小越优先），默认为 100
    fn priority(&self) -> u32 {
        100
    }
    /// 更新方法，每帧在逻辑阶段调用
    fn update(&mut self, world: &mut hecs::World, delay_time: f32) -> GameResult;
}

/// ECS 绘制系统抽象（纯渲染，无逻辑更新）
///
/// 所有**纯渲染**系统应实现此 trait（如 MapRenderSystem、UIRenderSystem）。
/// 如需同时执行更新和渲染，请使用 `HybridSystem` trait。
pub trait DrawSystem {
    /// 系统名称，默认使用类型全名
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// 是否启用，默认为 true
    fn is_enabled(&self) -> bool {
        true
    }

    /// 优先级（数字越小越优先），默认为 100
    fn priority(&self) -> u32 {
        100
    }
    /// 绘制方法，每帧在渲染阶段调用
    fn draw(
        &mut self,
        ctx: &mut ggez::Context,
        canvas: &mut ggez::graphics::Canvas,
        world: &hecs::World,
    ) -> GameResult;
}

/// ECS 混合系统抽象（同时需要更新和渲染）
///
/// 用于需要在逻辑阶段更新状态、在渲染阶段绘制的系统（如粒子系统、调试系统）。
///
/// # 与 DrawSystem 的区别
/// - `DrawSystem`: 纯渲染，无逻辑更新（如地图渲染、UI渲染）
/// - `HybridSystem`: 需要逻辑更新 + 渲染（如粒子效果、调试信息）
/// - `System`: 纯逻辑，无渲染（如 AI、物理、网络）
///
/// # 用法
/// ```rust
/// #[derive(HybridSystem)]
/// struct ParticleSystem;
///
/// impl HybridSystem for ParticleSystem {
///     fn update(&mut self, world: &mut hecs::World, dt: f32) -> GameResult {
///         // 更新粒子状态（位置、生命周期等）
///         Ok(())
///     }
///     
///     fn draw(
///         &mut self,
///         ctx: &mut ggez::Context,
///         canvas: &mut ggez::graphics::Canvas,
///         world: &hecs::World,
///     ) -> GameResult {
///         // 绘制粒子
///         Ok(())
///     }
/// }
/// ```
pub trait HybridSystem {
    /// 系统名称，默认使用类型全名
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    /// 是否启用，默认为 true
    fn is_enabled(&self) -> bool {
        true
    }

    /// 优先级（数字越小越优先），默认为 100
    fn priority(&self) -> u32 {
        100
    }
    /// 更新方法，每帧在逻辑阶段调用（必须实现）
    fn update(&mut self, world: &mut hecs::World, delay_time: f32) -> GameResult;

    /// 绘制方法，每帧在渲染阶段调用（必须实现）
    fn draw(
        &mut self,
        ctx: &mut ggez::Context,
        canvas: &mut ggez::graphics::Canvas,
        world: &hecs::World,
    ) -> GameResult;
}

pub enum SystemKind {
    Update(Box<dyn System>),
    Draw(Box<dyn DrawSystem>),
    Hybrid(Box<dyn HybridSystem>),
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

/// 为纯渲染系统批量实现 IntoSystemKind
///
/// 等价于为每个类型添加 `#[derive(RenderSystem)]`
///
/// # 用法
/// ```rust
/// draw_system!(MapRenderer, UIRenderer);
/// ```
#[macro_export]
macro_rules! draw_system {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl $crate::ecs::systems::IntoSystemKind for $ty {
                fn into_kind(self: Box<Self>) -> $crate::ecs::systems::SystemKind {
                    $crate::ecs::systems::SystemKind::Draw(self)
                }
            }
        )+
    };
}

/// 为混合系统批量实现 IntoSystemKind
///
/// 等价于为每个类型添加 `#[derive(HybridSystem)]`
///
/// # 用法
/// ```rust
/// hybrid_system!(ParticleSystem, DebugSystem);
/// ```
#[macro_export]
macro_rules! hybrid_system {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl $crate::ecs::systems::IntoSystemKind for $ty {
                fn into_kind(self: Box<Self>) -> $crate::ecs::systems::SystemKind {
                    $crate::ecs::systems::SystemKind::Hybrid(self)
                }
            }
        )+
    };
}

/// 系统调度器的内部存储结构
enum SystemEntry {
    Update {
        system: Box<dyn System>,
        priority: u32,
    },
    Draw {
        system: Box<dyn DrawSystem>,
        priority: u32,
    },
    Hybrid {
        system: Box<dyn HybridSystem>,
        priority: u32,
    },
}

impl SystemEntry {
    fn priority(&self) -> u32 {
        match self {
            SystemEntry::Update { priority, .. } => *priority,
            SystemEntry::Draw { priority, .. } => *priority,
            SystemEntry::Hybrid { priority, .. } => *priority,
        }
    }

    fn is_enabled(&self) -> bool {
        match self {
            SystemEntry::Update { system, .. } => system.is_enabled(),
            SystemEntry::Draw { system, .. } => system.is_enabled(),
            SystemEntry::Hybrid { system, .. } => system.is_enabled(),
        }
    }
}

pub struct SystemScheduler {
    systems: Vec<SystemEntry>,
}

impl SystemScheduler {
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
        }
    }

    /// 添加系统（自动判断类型）
    pub fn add_system<S>(&mut self, system: S) -> &mut Self
    where
        S: IntoSystemKind + 'static,
    {
        match IntoSystemKind::into_kind(Box::new(system)) {
            SystemKind::Update(sys) => {
                let priority = sys.priority();
                self.systems.push(SystemEntry::Update {
                    system: sys,
                    priority,
                });
            }
            SystemKind::Draw(sys) => {
                let priority = sys.priority();
                self.systems.push(SystemEntry::Draw {
                    system: sys,
                    priority,
                });
            }
            SystemKind::Hybrid(sys) => {
                let priority = sys.priority();
                self.systems.push(SystemEntry::Hybrid {
                    system: sys,
                    priority,
                });
            }
        }

        // 按优先级排序（数字越小越优先）
        self.systems.sort_by_key(|entry| entry.priority());
        self
    }

    /// 逻辑更新阶段 - 按优先级统一调度所有系统的 update
    pub fn update(&mut self, world: &mut hecs::World, delay_time: f32) -> GameResult {
        for entry in &mut self.systems {
            if !entry.is_enabled() {
                continue;
            }

            match entry {
                SystemEntry::Update { system, .. } => {
                    system.update(world, delay_time)?;
                }
                SystemEntry::Hybrid { system, .. } => {
                    system.update(world, delay_time)?;
                }
                SystemEntry::Draw { .. } => {
                    // 纯渲染系统无需更新
                }
            }
        }

        Ok(())
    }

    /// 渲染阶段 - 调度 DrawSystem 和 HybridSystem 的 draw 方法
    pub fn draw(&mut self, ctx: &mut ggez::Context, canvas: &mut Canvas, world: &hecs::World) -> GameResult {
        for entry in &mut self.systems {
            if !entry.is_enabled() {
                continue;
            }

            match entry {
                SystemEntry::Draw { system, .. } => {
                    system.draw(ctx, canvas, world)?;
                }
                SystemEntry::Hybrid { system, .. } => {
                    system.draw(ctx, canvas, world)?;
                }
                SystemEntry::Update { .. } => {
                    // 纯逻辑系统无需渲染
                }
            }
        }

        Ok(())
    }
}

mod tests {
    use super::*;
    #[derive(RenderSystem)]
    pub struct TestDrawSystem;

    impl DrawSystem for TestDrawSystem {
        fn priority(&self) -> u32 {
            50
        }
        fn draw(
            &mut self,
            _ctx: &mut ggez::Context,
            _canvas: &mut ggez::graphics::Canvas,
            _world: &hecs::World,
        ) -> GameResult {
            Ok(())
        }
    }

    /// 测试系统
    #[derive(LogicSystem)]
    pub struct TestSystem;

    impl System for TestSystem {
        fn update(&mut self, _world: &mut hecs::World, _delay_time: f32) -> GameResult {
            println!("TestSystem::update called");
            Ok(())
        }
    }

    #[derive(HybridSystem)]
    pub struct TestDebugSystem;

    impl HybridSystem for TestDebugSystem {
        fn priority(&self) -> u32 {
            50
        }
        fn update(&mut self, _world: &mut hecs::World, _dt: f32) -> GameResult {
            // 混合系统的更新逻辑（必须实现）
            Ok(())
        }

        fn draw(
            &mut self,
            _ctx: &mut ggez::Context,
            _canvas: &mut ggez::graphics::Canvas,
            _world: &hecs::World,
        ) -> GameResult {
            Ok(())
        }
    }

    #[test]
    fn test_add_system() {
        let mut scheduler = SystemScheduler::new();

        // 这应该能编译通过
        scheduler.add_system(TestSystem);
        scheduler.add_system(TestDrawSystem);
        scheduler.add_system(TestDebugSystem); // 添加绘制系统
        println!("✅ Test passed!");
    }

    #[test]
    fn test_system_execution_order() {
        use std::sync::{Arc, Mutex};

        // 测试系统执行顺序
        #[derive(LogicSystem)]
        struct EarlySystem(Arc<Mutex<Vec<String>>>);

        impl System for EarlySystem {
            fn update(&mut self, _world: &mut hecs::World, _dt: f32) -> GameResult {
                self.0.lock().unwrap().push("Early(100)".to_string());
                Ok(())
            }
        }

        #[derive(HybridSystem)]
        struct LateHybridSystem(Arc<Mutex<Vec<String>>>);

        impl HybridSystem for LateHybridSystem {
            fn priority(&self) -> u32 {
                200
            }
            fn update(&mut self, _world: &mut hecs::World, _dt: f32) -> GameResult {
                self.0.lock().unwrap().push("LateHybrid(200)".to_string());
                Ok(())
            }
            fn draw(
                &mut self,
                _ctx: &mut ggez::Context,
                _canvas: &mut ggez::graphics::Canvas,
                _world: &hecs::World,
            ) -> GameResult {
                Ok(())
            }
        }

        #[derive(LogicSystem)]
        struct MiddleSystem(Arc<Mutex<Vec<String>>>);

        impl System for MiddleSystem {
            fn priority(&self) -> u32 {
                150
            }
            fn update(&mut self, _world: &mut hecs::World, _dt: f32) -> GameResult {
                self.0.lock().unwrap().push("Middle(150)".to_string());
                Ok(())
            }
        }

        let execution_order = Arc::new(Mutex::new(Vec::new()));

        let mut scheduler = SystemScheduler::new();
        scheduler.add_system(LateHybridSystem(execution_order.clone()));
        scheduler.add_system(EarlySystem(execution_order.clone()));
        scheduler.add_system(MiddleSystem(execution_order.clone()));

        let mut world = hecs::World::new();
        scheduler.update(&mut world, 0.016).unwrap();

        let order = execution_order.lock().unwrap();
        println!("执行顺序: {:?}", *order);

        // 验证按优先级顺序执行：100 -> 150 -> 200
        assert_eq!(order.len(), 3);
        assert_eq!(order[0], "Early(100)");
        assert_eq!(order[1], "Middle(150)");
        assert_eq!(order[2], "LateHybrid(200)");

        println!("✅ 系统按优先级正确排序执行！");
    }
}
