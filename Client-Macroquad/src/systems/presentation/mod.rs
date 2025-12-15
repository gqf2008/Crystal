// ### 第3層：表現層系統 (600-899)

// #### 動畫和特效系統
// | 系統名稱 | 優先級 | 啟用場景 | 職責說明 |
// |---------|--------|----------|----------|
// | `AnimationSystem` | 600 | 遊戲 | 角色動畫、幀更新、**攻擊動畫** |
// | `ParticleSystem` | 610 | 遊戲 | 粒子效果、生命周期管理 |
// | `WeatherSystem` | 620 | 遊戲 | 天氣效果、日夜循環 |

// #### 音效系統
// | 系統名稱 | 優先級 | 啟用場景 | 職責說明 |
// |---------|--------|----------|----------|
// | `SoundSystem` | 630 | 全部 | 3D音效、背景音樂管理 |
// | `VoiceChatSystem` | 640 | 遊戲 | 語音聊天、音量控制 |

// #### 攝像機系統
// | 系統名稱 | 優先級 | 啟用場景 | 職責說明 |
// |---------|--------|----------|----------|
// | `CameraFollowSystem` | 650 | 遊戲 | 攝像機跟隨玩家 |
// | `CameraSystem` | 700 | 遊戲 | 攝像機矩陣計算、特效 |

// #### UI 系統
// | 系統名稱 | 優先級 | 啟用場景 | 職責說明 |
// |---------|--------|----------|----------|
// | `UISystem` | 800 | 全部 | UI狀態更新、事件處理 |
// | `HUDSystem` | 810 | 遊戲 | 血條、狀態欄、快捷欄 |
// | `MinimapSystem` | 820 | 遊戲 | 小地圖、坐標顯示 |
// | `DialogSystem` | 830 | 遊戲 | 對話框、劇情文本 |
mod camera_follow_system;
mod camera_system;
mod camera_space_gate_system;
mod camera_bounds_system;
mod ui_system;
mod hud_system;
mod minimap_system;
mod dialog_system;
mod floating_text_system;
mod particle_system;
mod sound_system;
mod animation_system;
mod mount_state_sync_system;
mod position_interpolation_system;
pub use camera_system::CameraSystem;
pub use camera_follow_system::CameraFollowSystem;
pub use camera_space_gate_system::CameraSpaceGateSystem;
pub use camera_bounds_system::CameraBoundsSystem;
pub use floating_text_system::FloatingTextSystem;
pub use particle_system::ParticleSystem;
pub use sound_system::SoundSystem;
pub use animation_system::AnimationSystem;
pub use mount_state_sync_system::MountStateSyncSystem;
pub use position_interpolation_system::PositionInterpolationSystem;
pub use ui_system::UISystem;
pub use hud_system::HUDSystem;
pub use minimap_system::MinimapSystem;
pub use dialog_system::DialogSystem;
