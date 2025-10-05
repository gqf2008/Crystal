// MirScenes - Game scene system
// Mirrors the structure of Client/MirScenes/

pub mod state;
pub mod scene_trait;
pub mod login_scene;
pub mod select_scene;
pub mod game_scene;
pub mod map_control;
pub mod dialogs;

// Re-export scene types
pub use state::ClientState;
pub use scene_trait::{Scene, SceneType, MouseButton, KeyCode};
pub use login_scene::{LoginScene, BanInfo};
pub use select_scene::{SelectScene, SelectCharacter};
pub use game_scene::GameScene;
pub use map_control::{MapControl, CellInfo, Door, LightSetting, WeatherSetting};
