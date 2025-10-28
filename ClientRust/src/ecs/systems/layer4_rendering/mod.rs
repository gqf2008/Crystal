// ============================================================================
// Layer 4: 渲染层
// ============================================================================
//
// 职责：
// - 纯渲染逻辑，不包含游戏逻辑
// - 从组件读取数据，绘制到屏幕
// - Y-sorting（深度排序）
// - 相机变换
// - 遮挡透明度
// - 音效播放（读取Layer 3的音效触发决策）
//
// 输入组件（只读）：
// - Position
// - AnimationState（Layer 3 写入）
// - SoundTrigger（Layer 3 写入）
// - Camera
// - MapData
//
// 输出：屏幕图像、音频播放
//
// ============================================================================

pub mod render_system;
pub mod camera_system;
pub mod occlusion_system;
pub mod animation_playback_system;
pub mod tile_animation_system;
pub mod movement_interpolation_system;
pub mod sound_playback_system;
pub mod hud_render_system;
pub mod ui_render_system;

pub use render_system::RenderSystem;
pub use camera_system::CameraSystem;
pub use occlusion_system::OcclusionSystem;
pub use animation_playback_system::AnimationPlaybackSystem;
pub use tile_animation_system::TileAnimationSystem;
pub use movement_interpolation_system::MovementInterpolationSystem;
pub use sound_playback_system::SoundPlaybackSystem;
pub use hud_render_system::HUDRenderSystem;
pub use ui_render_system::UIRenderSystem;
