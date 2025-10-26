// ============================================================================
// 渲染相关组件
// ============================================================================

use std::time::Instant;

/// 渲染层级 (用于排序)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RenderLayer {
    Ground = 0,      // 地面层
    GroundItem = 1,  // 地面物品
    Shadow = 2,      // 阴影
    Object = 3,      // 游戏对象 (玩家/怪物/NPC)
    Effect = 4,      // 特效 (技能/爆炸)
    UI = 5,          // UI元素
}

#[derive(Debug, Clone, Copy)]
pub struct RenderOrder {
    pub layer: RenderLayer,
    pub z_order: i32, // 同层内的排序 (Y坐标)
}

impl RenderOrder {
    pub fn new(layer: RenderLayer, z_order: i32) -> Self {
        Self { layer, z_order }
    }
}

/// 相机组件 - 视口控制
#[derive(Debug, Clone)]
pub struct Camera {
    pub zoom: f32,
    pub screen_width: f32,
    pub screen_height: f32,
}

impl Camera {
    pub fn new(screen_width: f32, screen_height: f32) -> Self {
        Self {
            zoom: 1.0,
            screen_width,
            screen_height,
        }
    }
}

/// 渲染配置组件
#[derive(Debug, Clone)]
pub struct RenderConfig {
    pub show_back: bool,
    pub show_middle: bool,
    pub show_front: bool,
    pub show_grid: bool,
    pub show_obstacles: bool,
    pub show_animations: bool,
    pub show_borders: bool,
    pub show_npc_borders: bool,      // NPC边框调试
    pub show_monster_borders: bool,  // Monster边框调试
    pub show_effect_borders: bool,   // 特效边框调试
    pub show_path: bool,
    pub max_fps: u32,
    pub enable_lod: bool,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            show_back: true,
            show_middle: true,
            show_front: true,
            show_grid: false,
            show_obstacles: false,
            show_animations: true,
            show_borders: false,
            show_npc_borders: false,
            show_monster_borders: false,
            show_effect_borders: false,
            show_path: false,
            max_fps: 60,
            enable_lod: false,
        }
    }
}

/// 可见区域缓存
#[derive(Debug, Clone)]
pub struct VisibleArea {
    pub start_x: i32,
    pub end_x: i32,
    pub start_y: i32,
    pub end_y: i32,
    pub front_end_y: i32,
    pub zoom: f32,
    pub camera_x: f32,
    pub camera_y: f32,
    pub visible_entities: Vec<hecs::Entity>,
    pub last_update: Instant,
}

impl Default for VisibleArea {
    fn default() -> Self {
        Self {
            start_x: -999999,
            end_x: -999999,
            start_y: -999999,
            end_y: -999999,
            front_end_y: -999999,
            zoom: -1.0,
            camera_x: -999999.0,
            camera_y: -999999.0,
            visible_entities: Vec::new(),
            last_update: Instant::now(),
        }
    }
}
