// ============================================================================
// ActorPlugin - 角色/NPC/怪物精灵渲染与帧动画（里程碑 2）
// ============================================================================
//
// 对应 Client-Macroquad:
// - objects/frames.rs（帧表，原样复用）
// - systems/presentation/animation_system.rs（帧号计算）
// - systems/rendering/sprite_system/character.rs（分层绘制）
//
// 帧号公式（与 C#/macroquad 一致）:
//   DrawFrame     = Frame.Start + Direction * (Count+Skip) + FrameIndex
//   EffectFrame   = Frame.EffectStart + Direction * (EffectCount+EffectSkip) + FrameIndex
//
// 渲染方式: 每个角色实体挂多层子实体（身体/发型/武器/特效），
// 每帧按帧号从对应 .Lib 取图并缓存为 Bevy Image 资产。
//
// #72 拆分：components.rs（组件）/ frames.rs（帧号计算）/ render.rs（取图渲染）/
// spawn.rs + spawn_helpers.rs（实体生成）/ systems.rs（深度/幽灵/演示驱动）
// ============================================================================

mod components;
mod frames;
mod render;
mod spawn;
mod spawn_helpers;
mod systems;

pub use components::*;
pub(crate) use spawn_helpers::attach_mount_layer;
pub(crate) use render::ActorNameLabel;
pub use spawn::depth_z;

use render::{actor_sprite_render, apply_poison_tint};
use spawn::{despawn_removed_objects, spawn_demo_actors_when_ready, spawn_net_objects_when_ready};
use systems::{actor_hover_tooltip_system, advance_actor_animations, demo_drive, dump_depth_debug, log_player_walk, sync_actor_depth, sync_player_equipment, update_local_ghost};

use bevy::prelude::*;

pub struct ActorPlugin;

impl Plugin for ActorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActorImageCache>();
        app.add_systems(
            Update,
            (
                spawn_demo_actors_when_ready,
                spawn_net_objects_when_ready,
                despawn_removed_objects,
                advance_actor_animations,
                actor_sprite_render,
                apply_poison_tint,
                update_local_ghost,
                dump_depth_debug,
            )
                .chain()
                .after(crate::network::network_system)
                .run_if(in_state(crate::scenes::AppState::Game)),
        );
        app.add_systems(
            Update,
            (demo_drive, sync_actor_depth, sync_player_equipment, log_player_walk)
                .run_if(in_state(crate::scenes::AppState::Game)),
        );
        // #152 头顶名字（新角色生成时挂载，跟随移动）
        app.add_systems(
            Update,
            render::actor_name_label_system.run_if(in_state(crate::scenes::AppState::Game)),
        );

        // 悬停玩家/怪物/NPC → 目标头顶显示名字
        app.add_systems(
            Update,
            actor_hover_tooltip_system.run_if(in_state(crate::scenes::AppState::Game)),
        );
        // #1402：行会名标签即时更新（加退会/职位变化重发 ObjectPlayer）。
        // 排在 sync_outline_system 之前：变更检测按 tick 严格比较，同帧晚于
        // sync 的零散写入永不可见（描边副本陈旧）；副本同步另有直接写兜底
        app.add_systems(
            Update,
            render::actor_guild_label_system
                .before(crate::ui::outlined_text::sync_outline_system)
                .run_if(in_state(crate::scenes::AppState::Game)),
        );
        // #178 PK 名字染色（ObjectColourChanged）
        app.add_systems(
            Update,
            (render::object_colour_server_events, render::actor_name_colour_system)
                .after(crate::network::network_system)
                .run_if(in_state(crate::scenes::AppState::Game)),
        );
        // 登出/ReturnToLogin 回登录界面时清掉玩家实体（评审 CRITICAL-1）：
        // 此前 dialog 侧只清 DialogManager/session，LocalPlayer 残留——同进程换角色重登
        // 会出现双实体 → 全库 Query::single() 静默 MultipleEntities，状态冻结无日志。
        // Bevy 0.16+ 关系层级 .despawn() 自动带子树（渲染层/名字标签），与全仓 idiom 一致。
        app.add_systems(OnExit(crate::scenes::AppState::Game), despawn_local_player);
        // 评审 MINOR：--demo 下 OnExit 清掉演示玩家后重进 Game 须重新生成（done 复位；
        // 若用 Local<bool> 则永不重建，见 spawn::DemoActorsSpawned）。
        app.init_resource::<spawn::DemoActorsSpawned>();
        app.add_systems(
            OnExit(crate::scenes::AppState::Game),
            |mut done: ResMut<spawn::DemoActorsSpawned>| done.0 = false,
        );
    }
}

/// 登出/ReturnToLogin 回登录界面时清掉本地玩家实体（评审 CRITICAL-1）。
/// 一并清掉全部怪物/NPC：spawn_monster/spawn_npc **无条件**挂 DemoBehavior
///（在役行为组件——待机转向/周期攻击，非 --demo 专属；demo_drive 驱动全部挂接者，
/// 网络对象路径 spawn.rs NetObject::Monster/Npc 同样经过），故本过滤器实际覆盖
/// 网络怪物/NPC——此前它们登出后无任何清理、靠 #1813 去重兜底（幽灵实体泄漏），
/// 登出清理由此顺带消除泄漏；重登服务端全量重发。已知不对称：远端玩家/地面物品
/// 仍残留（复审 FINDING 2 登记，另案处理）。
pub(crate) fn despawn_local_player(
    mut commands: Commands,
    q: Query<Entity, Or<(With<LocalPlayer>, With<DemoBehavior>)>>,
) {
    for e in &q {
        commands.entity(e).despawn();
    }
}
