// ============================================================================
// ECS Systems - 六阶段架构系统
// ============================================================================
//
// 架构设计：
// 
// 阶段 1: 输入与网络 (50-199)
//   - 接收网络数据、处理输入、玩家控制、事件分发
//   - 系统：NetworkRecvSystem(50) → InputSystem(100) → PlayerControlSystem(110) → GameEventSystem(120)
//
// 阶段 2: AI 与决策 (200-299)
//   - AI 行为逻辑、对话处理
//   - 系统：MonsterAISystem(200) → NpcAISystem(210) → DialogueSystem(220)
//
// 阶段 3: 战斗与技能 (300-399)
//   - 技能释放、战斗计算
//   - 系统：SkillSystem(300) → CombatSystem(310)
//
// 阶段 4: 移动与物理 (400-499)
//   - 实体移动、碰撞检测、相机跟随（逻辑层）
//   - 系统：MovementSystem(400) → CollisionSystem(410) → CameraFollowSystem(420)
//
// 阶段 5: 状态更新 (500-599)
//   - 动画状态、粒子更新、音效触发、相机矩阵计算（准备渲染）
//   - 系统：AnimationSystem(500) → ParticleSystem(510) → SoundSystem(520) → CameraSystem(530)
//
// 阶段 6: 网络同步 (600-699)
//   - 状态收集与发送
//   - 系统：NetworkSendSystem(600) → SyncSystem(610)
//
// 阶段 7: 渲染 (1000-1999) [DrawSystem]
//   - 纯渲染输出，按层级顺序
//   - 系统：MapRenderSystem(1000) → SpriteRenderSystem(1010) → EffectRenderSystem(1020) → UIRenderSystem(1030) → DebugSystem(1100)
//
// 数据流：网络/输入 → AI决策 → 战斗技能 → 移动物理 → 状态更新 → 网络发送 → 渲染输出
//
// 关键改进：
// 1. CameraFollowSystem(420) 在移动阶段更新跟随逻辑
// 2. CameraSystem(530) 在状态更新阶段计算最终矩阵
// 3. GameEventSystem(120) 统一管理系统间事件通信
// 4. PlayerControlSystem(110) 专门处理玩家控制转换
//
// ============================================================================

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

    // 阶段 6: 网络同步 (600-699)
    pub const NETWORK_SEND: u32 = 600;
    pub const SYNC: u32 = 610;

    // 阶段 7: 渲染 (1000-1999)
    pub const MAP_RENDER: u32 = 1000;
    pub const SPRITE_RENDER: u32 = 1010;
    pub const EFFECT_RENDER: u32 = 1020;
    pub const UI_RENDER: u32 = 1030;
    pub const DEBUG_RENDER: u32 = 1100;
}

// 六阶段架构模块
pub mod layer1_input;
pub mod layer2_logic;
pub mod layer3_presentation;
pub mod layer4_rendering;
pub mod layer5_ui;
// pub mod update;
// pub mod render;

use ggez::graphics::Canvas;
use ggez::GameResult;
// 重新导出各层系统
pub use layer1_input::{
    ClientNetworkSystem, InputCollectingSystem, MockNetworkConfig, MockNetworkSystem,
};
pub use layer2_logic::{
    CombatSystem, InterpolationSystem, LocalPredictionSystem, MagicCastSystem, MonsterSystem,
    MovementSystemV2, NPCSystem, ReconciliationSystem,
};
pub use layer3_presentation::{
    AnimationStateSystem, MonsterAnimationStateSystem, NPCActionSystem, PlayerAnimationSystem,
};
pub use layer4_rendering::{
    AnimationPlaybackSystem, CameraSystem, MovementInterpolationSystem, OcclusionSystem,
    RenderSystem, TileAnimationSystem,
};
pub use layer5_ui::{
    DialogManagerSystem,
    ItemSystem,
    KeyboardShortcutSystem,
    MagicLearningSystem,
    MouseEventSystem, // 🆕 输入事件处理系统
    QuestSystem,
    TradeSystem,
    UIEventDispatcher, // 🆕 UI系统拆分后的子系统
    UISystem,
};

// 重新导出其他系统的特殊类型
pub use layer2_logic::combat_system::{CombatResult, DamageType, SkillEffectSystem};
pub use layer5_ui::quest_system::{Quest, QuestObjective, QuestReward, QuestState};
pub use layer5_ui::trade_system::{ShopData, ShopItem, ShopSystem, TradeData, TradeState};

// ============================================================================
// 系统 Trait 设计
// ============================================================================
//
// 本 ECS 系统基于 Rust nightly 的 specialization 特性实现了优雅的双 trait 架构：
//
// 1. System trait - 更新系统
//    - 必须实现 update() 方法
//    - 提供默认的元数据方法：name、is_enabled、priority
//
// 2. DrawSystem trait - 绘制系统
//    - 必须实现 draw() 方法
//    - 提供默认的元数据方法：name、is_enabled、priority
//    - 通过 specialization blanket impl 自动获得 System trait 实现
//    - blanket impl 会桥接元数据方法并提供默认空 update()
//
// 核心特性：
// - 单一入口：scheduler.add_system() 自动判断系统类型并路由到正确的队列
// - 类型安全：基于 trait bound 的编译期检查，DrawSystem 和 System 分离
// - 职责分离：update_systems 和 draw_systems 独立存储和调度
// - 灵活扩展：DrawSystem 可选择性覆盖 update() 实现自定义逻辑
// - 方法覆盖：可以只覆盖需要的元数据方法，其他使用默认实现
//
// 使用示例：
//
// 1. 纯更新系统（逻辑系统）：
//    ```rust
//    struct MovementSystem;
//    impl System for MovementSystem {
//        fn update(&mut self, world: &mut hecs::World, dt: f32) -> GameResult {
//            // 移动逻辑
//            Ok(())
//        }
//    }
//    scheduler.add_system(MovementSystem); // 自动归入 update_systems
//    ```
//
// 2. 纯绘制系统（无更新逻辑）：
//    ```rust
//    struct RenderSystem;
//    impl DrawSystem for RenderSystem {
//        fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas, world: &hecs::World) -> GameResult {
//            // 绘制逻辑
//            Ok(())
//        }
//    }
//    scheduler.add_system(RenderSystem); // 自动归入 draw_systems
//    // System trait 自动实现，update 为空 ✅
//    ```
//
// 3. 带自定义更新的绘制系统（如粒子系统）：
//    ```rust
//    struct ParticleSystem;
//    impl DrawSystem for ParticleSystem {
//        fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas, world: &hecs::World) -> GameResult {
//            // 绘制粒子
//            Ok(())
//        }
//    }
//    // 显式实现 System 覆盖默认的空 update
//    impl System for ParticleSystem {
//        fn update(&mut self, world: &mut hecs::World, dt: f32) -> GameResult {
//            // 更新粒子状态
//            Ok(())
//        }
//    }
//    scheduler.add_system(ParticleSystem); // 自动归入 draw_systems
//    // 调度器会在 draw 阶段调用 DrawSystem::draw()
//    // 如果实现了自定义 update，不会被自动调用（需要手动在 draw 中调用或在 update 队列中注册）
//    ```
//
// 4. 自定义元数据（只覆盖需要的方法）：
//    ```rust
//    use crate::ecs::systems::priority;
//    
//    struct HighPriorityRenderer;
//    impl DrawSystem for HighPriorityRenderer {
//        fn priority(&self) -> u32 { priority::MAP_RENDER }  // 使用预定义优先级
//        // name 和 is_enabled 使用默认实现 ✅
//        
//        fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas, world: &hecs::World) -> GameResult {
//            Ok(())
//        }
//    }
//    
//    struct CustomNameSystem;
//    impl System for CustomNameSystem {
//        fn name(&self) -> &'static str { "MyCustomSystem" }
//        fn priority(&self) -> u32 { priority::MOVEMENT }  // 使用预定义优先级
//        // is_enabled 使用默认实现 ✅
//        
//        fn update(&mut self, world: &mut hecs::World, dt: f32) -> GameResult {
//            Ok(())
//        }
//    }
//    ```
//
// 5. 使用优先级常量（推荐）：
//    ```rust
//    use crate::ecs::systems::priority;
//    
//    struct NetworkRecvSystem;
//    impl System for NetworkRecvSystem {
//        fn priority(&self) -> u32 { priority::NETWORK_RECV }  // 50
//        fn update(&mut self, world: &mut hecs::World, dt: f32) -> GameResult {
//            Ok(())
//        }
//    }
//    
//    struct MapRenderSystem;
//    impl DrawSystem for MapRenderSystem {
//        fn priority(&self) -> u32 { priority::MAP_RENDER }  // 1000
//        fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas, world: &hecs::World) -> GameResult {
//            Ok(())
//        }
//    }
//    
//    // 添加系统时会自动按优先级排序
//    scheduler.add_system(NetworkRecvSystem);
//    scheduler.add_system(MapRenderSystem);
//    ```
//
// 调度流程：
// - update 阶段：遍历 update_systems，调用 System::update()
// - draw 阶段：遍历 draw_systems，调用 DrawSystem::draw()
//
// 实现细节：
// - DrawSystem 的元数据方法（name/is_enabled/priority）通过 blanket impl 桥接到 System
// - 如果在 DrawSystem 中覆盖了元数据方法，System 会自动使用覆盖后的值
// - DrawSystem 的默认 update 实现为空操作，不会影响性能
//
// 注意事项：
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

/// ECS 绘制系统抽象
/// 
/// 所有需要在渲染阶段执行的系统都应实现此 trait。
/// System 会自动实现（通过 blanket impl，带默认空 update）。
/// 
/// 如需在绘制系统中执行更新逻辑（如粒子系统），可显式实现 System trait 覆盖默认行为。
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

// ============================================================================
// Specialization Blanket Implementation
// ============================================================================
//
// 使用 specialization 为所有 DrawSystem 自动实现 System trait。
// 提供默认空 update() 实现，允许纯绘制系统无需手动实现 update。
// 元数据方法从 DrawSystem 桥接到 System。
// 如果 DrawSystem 需要自定义 update 逻辑，可显式实现 System trait 覆盖此默认实现。
// ============================================================================
default impl<T> System for T
where
    T: DrawSystem + ?Sized,
{
    fn name(&self) -> &'static str {
        DrawSystem::name(self)
    }
    
    fn is_enabled(&self) -> bool {
        DrawSystem::is_enabled(self)
    }
    
    fn priority(&self) -> u32 {
        DrawSystem::priority(self)
    }
    
    fn update(&mut self, _world: &mut hecs::World, _delay_time: f32) -> GameResult {
        Ok(())
    }
}

pub enum SystemKind {
    Update(Box<dyn System>),
    Draw(Box<dyn DrawSystem>),
}

pub trait Schedulable: System {
    fn into_kind(self: Box<Self>) -> SystemKind;
}

// 默认实现：所有 System 都归为 Update
default impl<T> Schedulable for T
where
    T: System + 'static,
{
    fn into_kind(self: Box<Self>) -> SystemKind {
        SystemKind::Update(self)
    }
}

// 特化：DrawSystem 归为 Draw
default impl<T> Schedulable for T
where
    T: DrawSystem + 'static,
{
    fn into_kind(self: Box<Self>) -> SystemKind {
        SystemKind::Draw(self)
    }
}

pub struct SystemScheduler {
    update_systems: Vec<Box<dyn System>>,
    draw_systems: Vec<Box<dyn DrawSystem>>,
}

impl SystemScheduler {
    pub fn new() -> Self {
        Self {
            update_systems: Vec::new(),
            draw_systems: Vec::new(),
        }
    }

    /// 添加系统（自动判断类型）
    pub fn add_system<S>(&mut self, system: S)
    where
        S: Schedulable + 'static,
    {
        match Schedulable::into_kind(Box::new(system)) {
            SystemKind::Update(sys) => {
                self.update_systems.push(sys);
                self.update_systems.sort_by_key(|s| s.priority());
            }
            SystemKind::Draw(sys) => {
                self.draw_systems.push(sys);
                self.draw_systems.sort_by_key(|s| s.priority());
            }
        }
    }    pub fn update(&mut self, world: &mut hecs::World, delay_time: f32) -> GameResult {
        for system in &mut self.update_systems {
            if system.is_enabled() {
                system.update(world, delay_time)?;
            }
        }
        Ok(())
    }

    pub fn draw(&mut self, ctx: &mut ggez::Context, world: &hecs::World) -> GameResult {
        let mut canvas = Canvas::from_frame(ctx, ggez::graphics::Color::BLACK);
        for system in &mut self.draw_systems {
            if system.is_enabled() {
                system.draw(ctx, &mut canvas, world)?;
            }
        }
        Ok(())
    }
}

// pub struct A;

// // A 只需实现 DrawSystem，System 会通过 specialization 自动实现
// impl DrawSystem for A {
//     // 只覆盖 priority，其他使用默认值
//     fn priority(&self) -> u32 {
//         50
//     }
    
//     fn draw(
//         &mut self,
//         _ctx: &mut ggez::Context,
//         _canvas: &mut ggez::graphics::Canvas,
//         _world: &hecs::World,
//     ) -> GameResult {
//         Ok(())
//     }
// }

// // 如果想自定义 update，可以单独实现 System 覆盖默认实现
// impl System for A {
//     fn update(&mut self, world: &mut hecs::World, dt: f32) -> GameResult {
//         // 自定义更新逻辑
//         Ok(())
//     }
// }
