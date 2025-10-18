// GameScene Bevy 渲染层
// 
// 功能说明:
// 1. MLibrary 集成 - 将 MLibrary 纹理系统集成到 Bevy 资源系统
// 2. Sprite 渲染 - 批处理渲染游戏精灵
// 3. 地图渲染 - 渲染地图图层和对象
// 4. 摄像机系统 - 跟随玩家的摄像机
//
// 子模块:
// - mlibrary_assets.rs - Bevy 资源加载 MLibrary 纹理
// - sprite_renderer.rs - Sprite 批处理渲染系统
// - map_renderer.rs - 地图渲染器 (Bevy ECS版本)
// - camera.rs - 摄像机系统 (移植自 ggez 版本)

pub mod mlibrary_assets;
pub mod sprite_renderer;
pub mod map_renderer;
pub mod camera;

pub use mlibrary_assets::MLibraryAssets;
pub use sprite_renderer::SpriteRenderer;
pub use map_renderer::{MapRenderData, TileCache, TileEntity, TileLayer, DoorInfo};
pub use camera::GameCamera;

