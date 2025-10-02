// MirScenes - Game scene system
// Mirrors the structure of Client/MirScenes/

pub mod scene_trait;
pub mod login_scene;
pub mod select_scene;
pub mod game_scene;
pub mod dialogs;

// Re-export scene types
pub use scene_trait::{Scene, SceneType, MouseButton, KeyCode};
pub use login_scene::LoginScene;
pub use select_scene::SelectScene;
pub use game_scene::GameScene;
