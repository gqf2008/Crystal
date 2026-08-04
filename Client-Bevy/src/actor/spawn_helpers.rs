// ============================================================================
// actor 模块拆分（#72）
// ============================================================================

use bevy::prelude::*;
use mir2_shared::{MirAction, MirClass, MirGender};
use crate::resources::libraries::{ArrayLibType, LibraryName};
use crate::ui::sprite_ui::{UiFont, UiImageCache};
use super::components::*;
use super::spawn::depth_z;

pub(crate) fn spawn_player(commands: &mut Commands, x: f32, y: f32) {
    spawn_player_with(
        commands,
        x,
        y,
        MirClass::Warrior,
        MirGender::Male,
        0,
        0,
        0,
        0,
        0,
    );
}

/// 按外观生成本地玩家实体
#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_player_with(
    commands: &mut Commands,
    x: f32,
    y: f32,
    class: MirClass,
    gender: MirGender,
    armour: i16,
    hair: u8,
    weapon: i16,
    weapon_effect: i16,
    wing_effect: u8,
) -> Entity {
    let z = depth_z(-y); // y 是 Bevy 负坐标
    let root = commands
        .spawn((
            LocalPlayer,
            Player,
            ActorAppearance {
                class,
                gender,
                armour: armour.max(0) as u16,
                hair,
                weapon,
                weapon_effect,
                wing_effect,
            },
            ActorAnim::default(),
            DemoBehavior::Walk {
                side_len: 6,
                side_progress: 0,
                direction: 0,
                step_progress: 0.0,
                from_x: x,
                from_y: y,
                to_x: x,
                to_y: y,
                started: false,
            },
            Transform::from_xyz(x, -y, z),
            Visibility::default(),
        ))
        .id();
    commands.entity(root).with_children(|p| {
        p.spawn((
            Sprite::default(),
            Transform::default(),
            SpriteLayer {
                lib: ArrayLibType::CArmours,
                slot: armour.max(0) as u32,
                frame: 0,
                is_effect: false,
                is_mount: false,
                alpha: 1.0,
            },
        ));
        p.spawn((
            Sprite::default(),
            Transform::default(),
            SpriteLayer {
                lib: ArrayLibType::CHair,
                slot: hair as u32,
                frame: 0,
                is_effect: false,
                is_mount: false,
                alpha: 1.0,
            },
        ));
        p.spawn((
            Sprite::default(),
            Transform::default(),
            SpriteLayer {
                lib: ArrayLibType::CWeapons,
                slot: weapon.max(0) as u32,
                frame: 0,
                is_effect: false,
                is_mount: false,
                alpha: 1.0,
            },
        ));
        // M62：武器特效（C# DrawWeapon：WeaponEffectLibrary1.DrawBlend(DrawFrame, 0.4F)）
        if weapon_effect > 0 {
            tracing::debug!("⚔️ 武器特效层: type={}", weapon_effect);
            p.spawn((
                Sprite::default(),
                Transform::default(),
                SpriteLayer {
                    lib: ArrayLibType::CWeaponEffect,
                    slot: weapon_effect.max(0) as u32,
                    frame: 0,
                    is_effect: false,
                    is_mount: false,
                    alpha: 0.4,
                },
            ));
        }
        // M62：翅膀特效（C# DrawWings：WingLibrary.DrawBlend(DrawWingFrame)）
        if wing_effect > 0 && wing_effect < 100 {
            tracing::debug!("🪽 翅膀特效层: type={}", wing_effect);
            p.spawn((
                Sprite::default(),
                Transform::default(),
                SpriteLayer {
                    lib: ArrayLibType::CHumEffect,
                    slot: wing_effect.saturating_sub(1).max(0) as u32,
                    frame: 0,
                    is_effect: true,
                    is_mount: false,
                    alpha: 1.0,
                },
            ));
        }
        // ghost 残影层（遮挡时显示，镜像对应图层）
        for lib in [
            ArrayLibType::CArmours,
            ArrayLibType::CHair,
            ArrayLibType::CWeapons,
        ] {
            p.spawn((
                Sprite::default(),
                Transform::from_xyz(0.0, 0.0, 0.5),
                Visibility::Hidden,
                GhostLayer { lib },
            ));
        }
    });
    root
}

/// 生成本地受控玩家（真实网络；无 DemoBehavior，由玩家控制系统驱动）
#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_local_player_with(
    commands: &mut Commands,
    x: f32,
    y: f32,
    class: MirClass,
    gender: MirGender,
    armour: i16,
    hair: u8,
    weapon: i16,
    weapon_effect: i16,
    wing_effect: u8,
    object_id: u32,
    mount_type: i16,
    is_mounted: bool,
) -> Entity {
    let z = depth_z(-y); // y 是 Bevy 负坐标
    let root = commands
        .spawn((
            LocalPlayer,
            NetObjectId(object_id),
            Player,
            ActorAppearance {
                class,
                gender,
                armour: armour.max(0) as u16,
                hair,
                weapon,
                weapon_effect,
                wing_effect,
            },
            ActorAnim::default(),
            Transform::from_xyz(x, y, z),
            Visibility::default(),
        ))
        .id();
    attach_player_layers(commands, root, armour, hair, weapon, weapon_effect, wing_effect);
    if is_mounted && mount_type >= 0 {
        commands.entity(root).insert(MountState { mount_type });
        attach_mount_layer(commands, root, mount_type);
    }
    root
}

/// 生成远端玩家（其他玩家；无 LocalPlayer、无 DemoBehavior）
#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_remote_player_with(
    commands: &mut Commands,
    x: f32,
    y: f32,
    class: MirClass,
    gender: MirGender,
    armour: i16,
    hair: u8,
    weapon: i16,
    weapon_effect: i16,
    wing_effect: u8,
    object_id: u32,
    mount_type: i16,
    is_mounted: bool,
) -> Entity {
    let z = depth_z(-y); // y 是 Bevy 负坐标
    let root = commands
        .spawn((
            NetObjectId(object_id),
            Player,
            ActorAppearance {
                class,
                gender,
                armour: armour.max(0) as u16,
                hair,
                weapon,
                weapon_effect,
                wing_effect,
            },
            ActorAnim::default(),
            Transform::from_xyz(x, y, z),
            Visibility::default(),
        ))
        .id();
    attach_player_layers(commands, root, armour, hair, weapon, weapon_effect, wing_effect);
    if is_mounted && mount_type >= 0 {
        commands.entity(root).insert(MountState { mount_type });
        attach_mount_layer(commands, root, mount_type);
    }
    root
}

/// 坐骑子精灵（Mount/xx.Lib，帧号由动画系统按坐骑动作写入）
pub(crate) fn attach_mount_layer(commands: &mut Commands, root: Entity, mount_type: i16) {
    commands.entity(root).with_children(|p| {
        p.spawn((
            Sprite::default(),
            Transform::default(),
            SpriteLayer {
                lib: ArrayLibType::Mounts,
                slot: mount_type.max(0) as u32,
                frame: 0,
                is_effect: false,
                is_mount: true,
                alpha: 1.0,
            },
        ));
    });
}

/// 玩家分层子精灵（护甲/发型/武器 + 武器特效/翅膀 + ghost 层）
#[allow(clippy::too_many_arguments)]
pub(crate) fn attach_player_layers(
    commands: &mut Commands,
    root: Entity,
    armour: i16,
    hair: u8,
    weapon: i16,
    weapon_effect: i16,
    wing_effect: u8,
) {
    commands.entity(root).with_children(|p| {
        p.spawn((
            Sprite::default(),
            Transform::default(),
            SpriteLayer {
                lib: ArrayLibType::CArmours,
                slot: armour.max(0) as u32,
                frame: 0,
                is_effect: false,
                is_mount: false,
                alpha: 1.0,
            },
        ));
        p.spawn((
            Sprite::default(),
            Transform::default(),
            SpriteLayer {
                lib: ArrayLibType::CHair,
                slot: hair as u32,
                frame: 0,
                is_effect: false,
                is_mount: false,
                alpha: 1.0,
            },
        ));
        p.spawn((
            Sprite::default(),
            Transform::default(),
            SpriteLayer {
                lib: ArrayLibType::CWeapons,
                slot: weapon.max(0) as u32,
                frame: 0,
                is_effect: false,
                is_mount: false,
                alpha: 1.0,
            },
        ));
        // M62：武器特效（C# DrawWeapon：WeaponEffectLibrary1.DrawBlend(DrawFrame, 0.4F)）
        if weapon_effect > 0 {
            tracing::debug!("⚔️ 武器特效层: type={}", weapon_effect);
            p.spawn((
                Sprite::default(),
                Transform::default(),
                SpriteLayer {
                    lib: ArrayLibType::CWeaponEffect,
                    slot: weapon_effect.max(0) as u32,
                    frame: 0,
                    is_effect: false,
                    is_mount: false,
                    alpha: 0.4,
                },
            ));
        }
        // M62：翅膀特效（C# DrawWings：WingLibrary.DrawBlend(DrawWingFrame)）
        if wing_effect > 0 && wing_effect < 100 {
            tracing::debug!("🪽 翅膀特效层: type={}", wing_effect);
            p.spawn((
                Sprite::default(),
                Transform::default(),
                SpriteLayer {
                    lib: ArrayLibType::CHumEffect,
                    slot: wing_effect.saturating_sub(1).max(0) as u32,
                    frame: 0,
                    is_effect: true,
                    is_mount: false,
                    alpha: 1.0,
                },
            ));
        }
        for lib in [
            ArrayLibType::CArmours,
            ArrayLibType::CHair,
            ArrayLibType::CWeapons,
        ] {
            p.spawn((
                Sprite::default(),
                Transform::from_xyz(0.0, 0.0, 0.5),
                Visibility::Hidden,
                GhostLayer { lib },
            ));
        }
    });
}

pub(crate) fn spawn_monster(commands: &mut Commands, monster_type: u16, x: f32, y: f32, direction: u8) -> Entity {
    let z = depth_z(-y); // y 是 Bevy 负坐标
    let root = commands
        .spawn((
            Monster,
            MonsterAppearance {
                monster_type,
                stage: 0,
            },
            ActorAnim {
                action: MirAction::Standing,
                direction,
                frame_index: 0,
                elapsed_ms: 0.0,
            },
            if monster_type.is_multiple_of(3) {
                DemoBehavior::Attack {
                    timer: 0.0,
                    interval: 4.0,
                    attacking: false,
                    attack_timer: 0.0,
                }
            } else {
                DemoBehavior::Idle {
                    timer: 0.0,
                    interval: 1.5,
                }
            },
            Transform::from_xyz(x, -y, z),
            Visibility::default(),
        ))
        .id();
    commands.entity(root).with_children(|p| {
        p.spawn((
            Sprite::default(),
            Transform::default(),
            SpriteLayer {
                lib: ArrayLibType::Monsters,
                slot: monster_type as u32,
                frame: 0,
                is_effect: false,
                is_mount: false,
                alpha: 1.0,
            },
        ));
    });
    root
}

pub(crate) fn spawn_npc(commands: &mut Commands, npc_index: u16, x: f32, y: f32, direction: u8) -> Entity {
    let z = depth_z(-y); // y 是 Bevy 负坐标
    let root = commands
        .spawn((
            Npc,
            NpcAppearance { npc_index },
            ActorAnim {
                action: MirAction::Standing,
                direction,
                frame_index: 0,
                elapsed_ms: 0.0,
            },
            DemoBehavior::Idle {
                timer: 0.0,
                interval: 3.0,
            },
            Transform::from_xyz(x, -y, z),
            Visibility::default(),
        ))
        .id();
    commands.entity(root).with_children(|p| {
        p.spawn((
            Sprite::default(),
            Transform::default(),
            SpriteLayer {
                lib: ArrayLibType::Npcs,
                slot: npc_index as u32,
                frame: 0,
                is_effect: false,
                is_mount: false,
                alpha: 1.0,
            },
        ));
    });
    root
}
