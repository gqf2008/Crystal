// ============================================================================
// actor 模块拆分（#72）
// ============================================================================

use bevy::prelude::*;
use mir2_shared::{MirAction, MirDirection};
use crate::objects::frames::{get_default_npc_frame, get_monster_frame, get_player_frame, Frame};
use super::components::{ActorAnim, ActorAppearance, MonsterAppearance, NpcAppearance};

pub(crate) fn mount_lib_frames(action: MirAction) -> (i32, i32) {
    match action {
        MirAction::Walking => (32, 8),
        MirAction::Running => (96, 6),
        MirAction::Struck => (144, 3),
        MirAction::Attack1
        | MirAction::Attack2
        | MirAction::Attack3
        | MirAction::Attack4
        | MirAction::AttackRange1
        | MirAction::AttackRange2
        | MirAction::AttackRange3 => (168, 6),
        _ => (0, 4), // Standing 等
    }
}

/// 坐骑动作 → 玩家 Hum 帧表动作（C# Frames.cs Mounts 段）
pub(crate) fn mount_player_action(action: MirAction) -> MirAction {
    match action {
        MirAction::Walking => MirAction::MountWalking,
        MirAction::Running => MirAction::MountRunning,
        MirAction::Struck => MirAction::MountStruck,
        MirAction::Attack1
        | MirAction::Attack2
        | MirAction::Attack3
        | MirAction::Attack4
        | MirAction::AttackRange1
        | MirAction::AttackRange2
        | MirAction::AttackRange3 => MirAction::MountAttack,
        _ => MirAction::MountStanding,
    }
}

/// 演示行为（统一枚举，挂在演示角色上）
pub(crate) fn actor_frame(
    player: Option<&ActorAppearance>,
    monster: Option<&MonsterAppearance>,
    npc: Option<&NpcAppearance>,
    anim: &ActorAnim,
) -> Option<&'static Frame> {
    if let Some(_p) = player {
        return get_player_frame(anim.action);
    }
    if let Some(m) = monster {
        let dir = MirDirection::try_from(anim.direction).unwrap_or(MirDirection::Up);
        return get_monster_frame(m.monster_type, anim.action, dir, m.stage);
    }
    if let Some(_n) = npc {
        return get_default_npc_frame(anim.action);
    }
    None
}

