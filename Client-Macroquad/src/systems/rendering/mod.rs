//! # 渲染系统模块 (rendering)
//!
//! ## 状态说明（重要）
//!
//! - 本目录是 **ECS 渲染系统草案/迁移中的实现**：目前项目的主渲染链路主要由 `scenes/`（如 GameScene）+
//!   `map_renderer/` 完成。
//! - `SystemScheduler` 的 draw 阶段尚未接入主循环；因此这些系统即使编译，也可能不会在正常运行时被调用。
//! - `MapRenderSystem`/`UIRenderSystem` 属于迁移草案，默认通过 feature `ecs_rendering` 关闭。
//! - 可运行验证入口：`src/bin/test_ecs_min.rs`（验证 systems 的 update 调度闭环）。
//!
//! **优先级范围**: 900-1999
//!
//! ## 模块职责
//!
//! 负责将游戏世界渲染到屏幕：
//! 1. 地图渲染（瓦片、Tile 动画）
//! 2. 实体渲染（玩家、怪物、NPC）
//! 3. 特效渲染（技能、粒子）
//! 4. UI 渲染（界面、文字）
//!
//! ## 基础渲染系统
//!
//! | 系统名称 | 优先级 | 依赖组件（读） | 启用场景 | 职责说明 |
//! |---------|--------|----------------|----------|----------|
//! | `MapRenderSystem` | 900 | MapData, MapTile, AnimatedTile, Camera | 游戏 | 地图渲染、Tile 动画 |
//! | `EntityRenderSystem` | 920 | Position, Sprite, Camera, Player, Monster | 游戏 | 实体渲染、深度排序 |
//! | `EffectRenderSystem` | 920 | Position, Sprite, SpellData | 游戏 | 特效渲染、ADD 混合 |
//! | `UIRenderSystem` | 930 | UI 组件 | 全部 | UI 界面渲染 |
//!
//! ## 渲染流程
//!
//! ```text
//! 逻辑更新完成
//!         ↓
//! MapRenderSystem: 渲染地图底层
//!         ↓
//! EntityRenderSystem: 渲染玩家/怪物（深度排序）
//!         ↓
//! EffectRenderSystem: 渲染特效（ADD 混合）
//!         ↓
//! UIRenderSystem: 渲染 UI（覆盖层）
//!         ↓
//! 画面输出
//! ```
//!
//! ## 使用示例
//!
//! ```rust
//! use crate::systems::rendering::{EntityRenderSystem, MapRenderSystem};
//! use crate::components::{Position, Sprite, Camera};
//!
//! // 创建可渲染实体
//! world.spawn((
//!     Position::new(100.0, 200.0),
//!     Sprite::new(2, 0),  // Objects.wil, index 0
//! ));
//!
//! // 创建相机
//! world.spawn((
//!     Position::new(0.0, 0.0),
//!     Camera { zoom: 1.0, ..Default::default() },
//! ));
//!
//! // EntityRenderSystem 会自动渲染它
//! ```
//!
//! ## 注意事项
//!
//! - 所有渲染系统都是 `DrawSystem`，只实现 `draw()` 方法
//! - 渲染顺序由优先级决定（地图 < 实体 < 特效 < UI）
//! - 深度排序在 EntityRenderSystem 中按 Y 坐标处理
//! - 特效使用 ADD 混合模式（在 Sprite 组件中指定）
// ============================================================================
pub mod sprite_system;
pub mod effect_system;
pub mod map_system;

pub mod ui_system;

pub use sprite_system::SpriteRenderSystem;
pub use effect_system::EffectRenderSystem;
pub use map_system::MapRenderSystem;

pub use ui_system::UIRenderSystem;


