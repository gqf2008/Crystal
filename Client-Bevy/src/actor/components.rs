// ============================================================================
// actor 模块拆分（#72）
// ============================================================================

use bevy::prelude::*;
use mir2_shared::{MirAction, MirClass, MirGender};
use crate::resources::libraries::ArrayLibType;
use std::collections::HashMap;

#[derive(Component, Default)]
pub struct Player;

/// 怪物类型标签
#[derive(Component, Default)]
pub struct Monster;

/// NPC 类型标签
#[derive(Component, Default)]
pub struct Npc;

/// 玩家外观数据（决定用哪套库）
#[derive(Component, Clone)]
pub struct ActorAppearance {
    pub class: MirClass,
    pub gender: MirGender,
    pub armour: u16,
    pub hair: u8,
    pub weapon: i16,
    pub weapon_effect: i16,
    pub wing_effect: u8,
}

/// 怪物外观数据
#[derive(Component, Clone)]
pub struct MonsterAppearance {
    pub monster_type: u16,
    pub stage: u8,
}

/// NPC 外观数据
#[derive(Component, Clone)]
pub struct NpcAppearance {
    pub npc_index: u16,
}

/// 本地玩家标记（用于遮挡 ghost 效果）
#[derive(Component)]
pub struct LocalPlayer;

/// 服务器对象 ID（ObjectRemove 用它删除实体）
#[derive(Component, Clone, Copy)]
pub struct NetObjectId(pub u32);

/// 地面物品（ObjectItem）：世界坐标精灵，随 ObjectRemove 清除
#[derive(Component)]
pub struct GroundItem {
    pub name: String,
}

/// NPC 名称（测试驱动/调试用）
#[derive(Component)]
pub struct NpcName(pub String);

/// 怪物名称（测试驱动/调试用；real-verify 用于排除守卫等非猎杀目标）
#[derive(Component)]
pub struct MonsterName(pub String);

/// 玩家名称（右键邀请组队/交易等交互用）
#[derive(Component)]
pub struct PlayerName(pub String);

/// 动画状态（动作/朝向/当前帧）
#[derive(Component)]
pub struct ActorAnim {
    pub action: MirAction,
    pub direction: u8,
    pub frame_index: i32,
    pub elapsed_ms: f32,
}

impl Default for ActorAnim {
    fn default() -> Self {
        Self {
            action: MirAction::Standing,
            direction: 0,
            frame_index: 0,
            elapsed_ms: 0.0,
        }
    }
}

/// 角色身上的单个渲染层（子实体）
#[derive(Component)]
pub struct SpriteLayer {
    /// 使用哪个数组库
    pub lib: ArrayLibType,
    /// 库槽位（护甲索引/怪物类型/NPC 索引等）
    pub slot: u32,
    /// 当前绘制帧号（由动画系统写入）
    pub frame: i32,
    /// true = 特效层（用 effect 帧段，如翅膀）
    pub is_effect: bool,
    /// true = 坐骑层（M60：帧号按坐骑库布局计算，非玩家帧表）
    pub is_mount: bool,
    /// 透明度（M62：武器特效 DrawBlend 0.4）
    pub alpha: f32,
}

/// M60 坐骑状态（挂玩家实体；mount_type>=0 且骑乘时显示坐骑层）
#[derive(Component)]
pub struct MountState {
    pub mount_type: i16,
}

#[derive(Component)]
pub enum DemoBehavior {
    /// 玩家：绕方块行走（平滑插值，一格 0.6s）
    Walk {
        side_len: i32,
        side_progress: i32,
        direction: u8,
        step_progress: f32,
        from_x: f32,
        from_y: f32,
        to_x: f32,
        to_y: f32,
        started: bool,
    },
    /// 原地待机并缓慢转向
    Idle { timer: f32, interval: f32 },
    /// 周期性攻击
    Attack {
        timer: f32,
        interval: f32,
        attacking: bool,
        attack_timer: f32,
    },
}

// ============================================================================
// 帧表 + 精灵图缓存
// ============================================================================

pub(crate) struct CachedSprite {
    pub(crate) handle: Handle<Image>,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) offset_x: i32,
    pub(crate) offset_y: i32,
}

#[derive(Resource, Default)]
pub struct ActorImageCache {
    pub(crate) map: HashMap<(u8, u32, u32), CachedSprite>,
}

// ============================================================================
// 系统
// ============================================================================

#[derive(Component, Clone, Copy)]
pub struct GhostLayer {
    pub lib: crate::resources::libraries::ArrayLibType,
}
