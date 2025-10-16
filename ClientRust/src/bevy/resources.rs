// Bevy Resources - 全局资源定义
use bevy::prelude::*;
use std::collections::HashMap;

/// MLibrary 资源包装器
#[derive(Resource)]
pub struct MLibraryAssets {
    // 暂时使用 HashMap 存储,后续会改为 Bevy Asset 系统
    pub textures: HashMap<String, Handle<Image>>,
}

impl MLibraryAssets {
    pub fn new() -> Self {
        Self {
            textures: HashMap::new(),
        }
    }
}

/// 地图资源
#[derive(Resource)]
pub struct MapAssets {
    pub current_map: Option<String>,
}

impl MapAssets {
    pub fn new() -> Self {
        Self {
            current_map: None,
        }
    }
}

/// 游戏配置
#[derive(Resource, Debug)]
pub struct GameConfig {
    pub cell_width: f32,
    pub cell_height: f32,
    pub screen_width: f32,
    pub screen_height: f32,
}

impl Default for GameConfig {
    fn default() -> Self {
        Self {
            cell_width: 48.0,
            cell_height: 32.0,
            screen_width: 1024.0,
            screen_height: 768.0,
        }
    }
}
