// ============================================================================
// Render Layer: 渲染系统层
// 优先级范围: 1000-1999
// ============================================================================
//
// ## 模块职责
//
// 负责游戏的所有渲染工作：
// 1. 地图渲染（瓦片、地形）
// 2. 精灵渲染（玩家、怪物、NPC）
// 3. 特效渲染（粒子、技能特效）
// 4. UI渲染（界面、对话框）
// 5. 调试渲染（网格、碰撞框、FPS）
//
// ## 设计原则
//
// - **纯渲染**: 只读取组件，不修改游戏逻辑状态
// - **DrawSystem trait**: 所有渲染系统实现 DrawSystem trait
// - **Y-sorting**: 在渲染前对实体按 Y 坐标排序（正确遮挡）
// - **相机变换**: 世界坐标 → 屏幕坐标转换
//
// ## 系统列表
//
// | 系统 | 优先级 | 类型 | 职责 |
// |------|--------|------|------|
// | MapRenderSystem | 1000 | DrawSystem | 渲染地图瓦片（背景层、中间层、前景层） |
// | SpriteRenderSystem | 1100 | DrawSystem | 渲染所有精灵（玩家、怪物、NPC、物品） |
// | EffectRenderSystem | 1200 | DrawSystem | 渲染粒子特效、技能特效 |
// | UIRenderSystem | 1300 | DrawSystem | 渲染UI界面（背包、对话框、技能栏） |
// | DebugSystem | u32::MAX-1 | HybridSystem | 渲染调试信息（网格、坐标、FPS） |
//
// ## 输入组件（只读）
//
// - **Position**: 实体位置（世界坐标）
// - **Sprite**: 精灵图片数据
// - **Animation**: 当前动画帧
// - **Camera**: 相机配置（位置、缩放）
// - **MapData**: 地图数据（瓦片、光照）
// - **ParticleEmitter**: 粒子发射器
// - **UIComponent**: UI 数据（对话框、背包）
//
// ## 输出
//
// - **屏幕图像**: 绘制到 ggez::Canvas
// - **无状态修改**: 不写入任何组件
//
// ## 渲染流程
//
// ```
// 1. MapRenderSystem: 渲染地图背景
//      ↓
// 2. SpriteRenderSystem: 
//      - 收集所有可见实体
//      - Y-sorting（按 Y 坐标排序）
//      - 依次渲染精灵
//      ↓
// 3. EffectRenderSystem: 渲染粒子特效（Alpha混合）
//      ↓
// 4. UIRenderSystem: 渲染UI界面（屏幕坐标）
//      ↓
// 5. DebugSystem: 渲染调试信息（可关闭）
// ```
//
// ## 系统类型说明
//
// ### DrawSystem (纯渲染系统)
// ```rust
// pub trait DrawSystem {
//     fn draw(&mut self, ctx: &mut Context, canvas: &mut Canvas, world: &World) -> GameResult;
// }
// ```
// - 只实现 `draw()` 方法
// - 只读取组件，不修改状态
// - 用于纯渲染逻辑
//
// ### HybridSystem (混合系统)
// ```rust
// pub trait HybridSystem: System + DrawSystem {
//     // 同时实现 update() 和 draw()
// }
// ```
// - 同时实现 `update()` 和 `draw()` 方法
// - DebugSystem 需要在 update() 中收集性能数据，在 draw() 中渲染
// - **谨慎使用**: 大部分渲染系统应该是纯 DrawSystem
//
// ## 注意事项
//
// ⚠️ **实现状态**: 当前所有渲染系统都是空实现（框架已就位）
// ⚠️ **DebugSystem**: 使用 HybridSystem，需要明确是否真的需要 update()
// ⚠️ **Y-sorting**: SpriteRenderSystem 负责排序，其他系统不要重复排序
// ⚠️ **相机变换**: 所有世界坐标渲染都要经过相机变换
//
// ## 宏说明
//
// - `draw_system!`: 批量导出 DrawSystem 类型的系统
// - `hybrid_system!`: 批量导出 HybridSystem 类型的系统
//
// ============================================================================

pub mod map_system;
pub mod sprite_system;
pub mod entity_render_system;
pub mod character_system;
pub mod effect_system;
pub mod ui_system;
pub mod debug_system;  // ✅ V2 版本 - 使用 GameContext

pub use map_system::MapRenderSystem;
pub use sprite_system::SpriteRenderSystem;
pub use entity_render_system::EntityRenderSystem;
pub use character_system::CharacterRenderSystem;
pub use effect_system::EffectRenderSystem;
pub use ui_system::UIRenderSystem;
pub use debug_system::DebugSystem;  // ✅ V2 版本（V1 已删除）

// ============================================================================
// 为所有渲染系统批量实现 IntoSystemKind
// ============================================================================

// 为纯渲染系统实现 IntoSystemKind
crate::draw_system!(
    MapRenderSystem,
    SpriteRenderSystem,
    EntityRenderSystem,
    CharacterRenderSystem,
    EffectRenderSystem,
    UIRenderSystem,
);
crate::hybrid_system!(
    DebugSystem,
);