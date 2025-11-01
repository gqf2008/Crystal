// ============================================================================
// Layer 5: 状态更新层 (State Update Layer)
// 优先级范围: 500-599
// ============================================================================
//
// ## 模块职责
//
// 负责更新游戏中的各种表现状态和辅助系统：
// 1. 动画播放和状态切换
// 2. 粒子特效更新
// 3. 生命值自动恢复
// 4. 音效播放
// 5. 相机控制（边缘滚动、缩放）
// 6. 地图更新（动画瓦片、光照）
//
// ## 设计原则
//
// - **表现更新**: 只更新表现状态，不影响核心逻辑
// - **独立系统**: 各系统职责单一，互不干扰
// - **可关闭**: 这些系统可以禁用而不影响游戏逻辑
//
// ## 系统列表
//
// | 系统 | 优先级 | 职责 |
// |------|--------|------|
// | AnimationSystem | 500 | 动画帧更新、状态切换 |
// | ParticleSystem | 510 | 粒子特效更新（位置、生命周期） |
// | HealthRegenSystem | 515 | 生命值/魔法值自动恢复 |
// | SoundSystem | 520 | 音效播放（读取音效触发事件） |
// | CameraSystem | 530 | 相机边缘滚动、缩放控制 |
// | MapUpdateSystem | 540 | 地图动画瓦片、光照更新 |
//
// ## 输入组件
//
// - **Animation**: 动画状态（当前帧、播放速度）
// - **ParticleEmitter**: 粒子发射器
// - **Health/Mana**: 生命值/魔法值
// - **SoundTrigger**: 音效触发事件（由 Layer 3 写入）
// - **Camera**: 相机配置
// - **MapData**: 地图数据
//
// ## 输出组件
//
// - **Animation.current_frame**: 当前动画帧
// - **Particle.position/lifetime**: 粒子位置和生命
// - **Health/Mana.current**: 恢复后的生命值/魔法值
// - **Camera.position/zoom**: 相机位置和缩放
// - **MapData.animated_tiles**: 动画瓦片状态
//
// ## 数据流
//
// ```
// Layer 3 决策 → AnimationState
//                    ↓
//              AnimationSystem 播放动画
//                    ↓
//              Layer 6 (Render) 渲染
//
// SoundTrigger 事件 → SoundSystem 播放音效
// ```
//
// ## 注意事项
//
// ⚠️ **CameraSystem 职责重复**: 
//    - 与 `physics_movement/camera_follow_system.rs` 存在职责重叠
//    - 建议合并为一个系统（见 ARCHITECTURE_REVIEW.md）
//
// ⚠️ **SoundSystem vs Layer 4 的 SoundPlaybackSystem**:
//    - SoundSystem (Layer 5): 决定播放什么音效
//    - SoundPlaybackSystem (Layer 4 Render): 实际播放音频
//    - 需要明确职责边界
//
// ============================================================================

pub mod animation_system;
pub mod health_regen_system;
pub mod particle_system;
pub mod sound_system;
pub mod camera_system;
pub mod map_update_system;
pub mod map_load_system;

pub use animation_system::AnimationSystem;
pub use health_regen_system::HealthRegenSystem;
pub use particle_system::ParticleSystem;
pub use sound_system::SoundSystem;
pub use camera_system::CameraSystem;
pub use map_update_system::{MapUpdateSystem, MapManager as OldMapManager};
pub use map_load_system::{MapLoadSystem, MapManager};

