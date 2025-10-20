// GameScene Bevy 渲染层
// 
// 功能说明:
// 1. MLibrary 集成 - 将 MLibrary 纹理系统集成到 Bevy 资源系统
// 2. Sprite 渲染 - 批处理渲染游戏精灵
// 3. 地图渲染 - 渲染地图图层和对象
// 4. 摄像机系统 - 跟随玩家的摄像机
// 5. 地图加载 - 从文件加载地图数据
// 6. 渲染初始化 - 初始化摄像机和加载地图
//
// 子模块:
// - mlibrary_assets.rs - Bevy 资源加载 MLibrary 纹理
// - sprite_renderer.rs - Sprite 批处理渲染系统
// - map_renderer.rs - 地图渲染器 (Bevy ECS版本)
// - camera.rs - 摄像机系统
// - map_loader.rs - 地图加载系统
// - init.rs - 渲染初始化和清理

pub mod mlibrary_assets;
pub mod sprite_renderer;
pub mod map_renderer;
pub mod camera;
pub mod map_loader;
pub mod init;
pub mod debug_transforms;
pub mod grid_debug;  // 新增：网格调试系统

pub use mlibrary_assets::MLibraryAssets;
pub use sprite_renderer::SpriteRenderer;
pub use map_renderer::{MapRenderData, TileCache, TileEntity, TileLayer, DoorInfo, render_map_system, update_animation_system, setup_map_renderer};
pub use camera::{GameCamera, camera_follow_system as camera_follow_system_new, camera_zoom_system};
pub use map_loader::{MapLoadRequest, load_map_system as load_map_system_new, load_map_direct};
pub use init::{setup_game_rendering, cleanup_game_rendering};
pub use debug_transforms::debug_transforms_system;
pub use grid_debug::{GridLines, toggle_grid_system, render_grid_system};  // 导出网格系统

