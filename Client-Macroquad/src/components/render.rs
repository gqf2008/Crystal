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

/// 渲染阶段参数（用于多次渲染 pass，例如遮挡后的 ghost pass）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum RenderStage {
    /// 正常渲染（位于地图 Middle 与 Front 之间的对象/特效）
    #[default]
    Normal,
    /// 后置渲染（位于地图 Front 之后的世界叠加层：名字/血条/轮廓/漂浮字等）
    PostFront,
    /// UI 渲染（默认相机下的覆盖层 UI；必须在世界渲染完成后执行）
    Ui,
}


/// 渲染阶段参数（用于多次渲染 pass，例如遮挡后的 ghost pass）
#[derive(Debug, Clone, Copy)]
pub struct RenderPass {
    pub alpha: f32,
    pub local_only: bool,
    pub stage: RenderStage,
}

/// 地图前景遮挡信息（单例组件）。
///
/// 用于在 RenderStage::PostFront 中决定是否绘制“本地玩家 ghost”。
#[derive(Debug, Clone, Copy, Default)]
pub struct FrontOcclusion {
    pub local_player_occluded: bool,
}

/// 当前鼠标悬停对象（用于渲染高亮/轮廓等视觉效果）
#[derive(Debug, Clone, Copy, Default)]
pub struct HoverHighlight {
    pub npc_object_id: Option<u32>,
    pub monster_object_id: Option<u32>,
}

/// 当前正在交互的 NPC（对齐 C# 的 GameScene.NPCID）
///
/// 说明：服务器的 NPCResponse 可能不带 object id，因此需要客户端侧保留“当前 NPC”。
#[derive(Debug, Clone, Copy, Default)]
pub struct ActiveNpc {
    pub npc_object_id: Option<u32>,
}

/// CallNPC 发送节流（对齐 C# GameScene.NPCTime：5s 内只允许一次）
#[derive(Debug, Clone, Copy, Default)]
pub struct NpcCallCooldown {
    pub until: f64,
}

/// 漂浮文本（用于伤害数字等临时反馈）
#[derive(Debug, Clone)]
pub struct FloatingText {
    pub text: String,
    pub start_time: f64,
    pub duration: f64,
    /// 上浮速度（像素/秒）
    pub rise_speed: f32,
    /// 文本颜色（伤害类型/暴击等）
    pub color: Option<macroquad::prelude::Color>,
}

/// 生命条显示动画（用于“掉血”平滑过渡）。
///
/// 说明：真实血量由 `Health` 决定；该组件仅用于渲染层显示。
#[derive(Debug, Clone, Copy)]
pub struct HealthBarAnim {
    /// 当前显示的血量（允许为小数，用于平滑插值）。
    pub displayed: f32,
}

impl Default for RenderPass {
    fn default() -> Self {
        Self {
            alpha: 1.0,
            local_only: false,
            stage: RenderStage::Normal,
        }
    }
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

/// 相机模式 - 控制相机行为
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum CameraMode {
    /// 跟随玩家模式 - CameraFollowSystem 自动更新相机位置
    #[default]
    FollowPlayer,
    /// 手动控制模式 - 用户可以拖拽相机，不自动跟随
    Manual,
    /// 固定模式 - 相机位置固定，不响应任何输入
    Fixed,
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
    pub show_animations: bool,        // 是否播放动画
    pub show_static_tiles: bool,      // 是否显示静态瓦片
    pub show_animated_tiles: bool,    // 是否显示动画瓦片
    pub show_borders: bool,
    pub show_npc_borders: bool,      // NPC边框调试
    pub show_monster_borders: bool,  // Monster边框调试
    pub show_effect_borders: bool,   // 特效边框调试
    pub show_path: bool,
    pub show_player_debug: bool,     // 玩家调试信息（位置、速度等）
    pub max_fps: u32,
    pub enable_lod: bool,
    pub enable_camera_drag: bool,    // 是否允许鼠标拖拽相机（地图查看器专用）
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
            show_static_tiles: true,      // 默认显示静态瓦片
            show_animated_tiles: true,    // 默认显示动画瓦片
            show_borders: false,
            show_npc_borders: false,
            show_monster_borders: false,
            show_effect_borders: false,
            show_path: false,
            show_player_debug: false,     // 默认不显示玩家调试信息
            max_fps: 120,
            enable_lod: false,
            enable_camera_drag: false,  // 默认禁用（正常游戏不需要）
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

/// 对象装饰组件（来自 ObjectDecoReceived）
///
/// 存储实体当前激活的视觉装饰 ID（如结婚戒指光效、特殊特效等）。
/// 渲染系统可读取此组件来绘制对应的装饰效果。
#[derive(Debug, Clone, Copy)]
pub struct ObjectDeco {
    pub deco_id: u16,
}
