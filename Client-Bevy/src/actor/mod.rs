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
pub use spawn::depth_z;

use render::actor_sprite_render;
use spawn::{despawn_removed_objects, spawn_demo_actors_when_ready, spawn_net_objects_when_ready};
use systems::{advance_actor_animations, demo_drive, dump_depth_debug, log_player_walk, sync_actor_depth, sync_player_equipment, update_local_ghost};

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
    }
}
