// ============================================================================
// actor 模块拆分（#72）
// ============================================================================

use super::components::*;
use super::spawn::depth_z;
use crate::resources::libraries::{ArrayLibType, LibraryName};
use crate::ui::sprite_ui::{UiFont, UiImageCache};
use bevy::prelude::*;
use mir2_shared::{MirAction, MirClass, MirGender};

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
        // 武器层：照 C# 职业武器规则（`weapon_layer_plan`）——刺客 `AWeapon/{i} R`+`L`、弓箭手 `ARWeapon/{i}`，
        // 默认武器仍 `CWeapon/{shape}`。此前这里**只挂 CWeapons**，于是库里 63 个 `shape 100..199` 的职业武器
        // 会去开不存在的 `CWeapon/100..152`（目录只有 `00..78`）⇒ 武器贴图直接缺失（owner 报的那一类）。
        for (lib, slot) in crate::resources::libraries::weapon_layer_plan(class, weapon, false) {
            p.spawn((
                Sprite::default(),
                Transform::default(),
                SpriteLayer {
                    lib,
                    slot,
                    frame: 0,
                    is_effect: false,
                    is_mount: false,
                    alpha: 1.0,
                },
            ));
        }
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
    // #2633 批次4：挂本地玩家状态组件默认值（Vitals/Inventory/…，设计 §7 挂载 A）。
    // 各 ServerEvent 写系统按 LocalPlayer 定位就地更新；实体未生成时写系统跳过（§12 R1）。
    commands
        .entity(root)
        .insert(crate::game::player_state::LocalPlayerStateBundle::default());
    attach_player_layers(
        commands,
        root,
        class,
        armour,
        hair,
        weapon,
        weapon_effect,
        wing_effect,
    );
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
    attach_player_layers(
        commands,
        root,
        class,
        armour,
        hair,
        weapon,
        weapon_effect,
        wing_effect,
    );
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
        // ghost 残影层（遮挡时显示，镜像坐骑层）——与 attach_player_layers 的身体
        // ghost 同构；缺它时骑乘走过建筑/树背后，身体有残影而坐骑直接消失
        p.spawn((
            Sprite::default(),
            Transform::from_xyz(0.0, 0.0, 0.5),
            Visibility::Hidden,
            GhostLayer {
                lib: ArrayLibType::Mounts,
            },
        ));
    });
}

/// 移除 root 的坐骑层与坐骑 ghost 残影层（下马路径）。
/// ghost 无 SpriteLayer，须按 GhostLayer.lib 单独匹配。
pub(crate) fn detach_mount_layers(
    commands: &mut Commands,
    children: &Query<&Children>,
    layers: &Query<&mut SpriteLayer>,
    ghost_layers: &Query<&GhostLayer>,
    root: Entity,
) {
    if let Ok(children_of) = children.get(root) {
        for c in children_of.iter() {
            if let Ok(l) = layers.get(c) {
                if l.is_mount {
                    commands.entity(c).despawn();
                }
            }
            if let Ok(g) = ghost_layers.get(c) {
                if g.lib == ArrayLibType::Mounts {
                    commands.entity(c).despawn();
                }
            }
        }
    }
}

/// 玩家分层子精灵（护甲/发型/武器 + 武器特效/翅膀 + ghost 层）
#[allow(clippy::too_many_arguments)]
pub(crate) fn attach_player_layers(
    commands: &mut Commands,
    root: Entity,
    class: MirClass,
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
        // 武器层：照 C# 职业武器规则（`weapon_layer_plan`）——刺客 `AWeapon/{i} R`+`L`、弓箭手 `ARWeapon/{i}`，
        // 默认武器仍 `CWeapon/{shape}`。此前这里**只挂 CWeapons**，于是库里 63 个 `shape 100..199` 的职业武器
        // 会去开不存在的 `CWeapon/100..152`（目录只有 `00..78`）⇒ 武器贴图直接缺失（owner 报的那一类）。
        for (lib, slot) in crate::resources::libraries::weapon_layer_plan(class, weapon, false) {
            p.spawn((
                Sprite::default(),
                Transform::default(),
                SpriteLayer {
                    lib,
                    slot,
                    frame: 0,
                    is_effect: false,
                    is_mount: false,
                    alpha: 1.0,
                },
            ));
        }
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

pub(crate) fn spawn_monster(
    commands: &mut Commands,
    monster_type: u16,
    x: f32,
    y: f32,
    direction: u8,
) -> Entity {
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

pub(crate) fn spawn_npc(
    commands: &mut Commands,
    npc_index: u16,
    x: f32,
    y: f32,
    direction: u8,
) -> Entity {
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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::system::RunSystemOnce;
    use bevy::ecs::world::CommandQueue;

    /// 坐骑层必须带 ghost 残影层（遮挡半透明）——缺它时骑乘走过建筑/树背后，
    /// 身体有残影而坐骑直接消失（2026-09-18 用户实机报告）
    #[test]
    fn attach_mount_layer_spawns_layer_and_ghost() {
        let mut world = World::new();
        let root = world.spawn_empty().id();
        let mut queue = CommandQueue::default();
        {
            let mut commands = Commands::new(&mut queue, &world);
            attach_mount_layer(&mut commands, root, 3);
        }
        queue.apply(&mut world);
        let children = world.get::<Children>(root).expect("root 应有子实体");
        let mut has_mount_layer = false;
        let mut has_mount_ghost = false;
        for c in children.iter() {
            if let Some(l) = world.get::<SpriteLayer>(c) {
                if l.is_mount && l.lib == ArrayLibType::Mounts && l.slot == 3 {
                    has_mount_layer = true;
                }
            }
            if let Some(g) = world.get::<GhostLayer>(c) {
                if g.lib == ArrayLibType::Mounts {
                    has_mount_ghost = true;
                    // ghost 初始 Hidden（遮挡时才由 update_local_ghost 翻 Visible）
                    assert_eq!(world.get::<Visibility>(c), Some(&Visibility::Hidden));
                }
            }
        }
        assert!(has_mount_layer, "应有坐骑 SpriteLayer");
        assert!(has_mount_ghost, "应有坐骑 GhostLayer（遮挡残影）");
    }

    /// 下马必须连坐骑 ghost 一起移除，身体层保留——否则下马后残留 ghost 实体
    #[test]
    fn detach_mount_layers_removes_layer_and_ghost_keeps_body() {
        let mut world = World::new();
        let root = world.spawn_empty().id();
        let mut queue = CommandQueue::default();
        {
            let mut commands = Commands::new(&mut queue, &world);
            attach_mount_layer(&mut commands, root, 0);
            // 身体层（应保留）
            commands.entity(root).with_children(|p| {
                p.spawn((
                    Sprite::default(),
                    Transform::default(),
                    SpriteLayer {
                        lib: ArrayLibType::CArmours,
                        slot: 1,
                        frame: 0,
                        is_effect: false,
                        is_mount: false,
                        alpha: 1.0,
                    },
                ));
            });
        }
        queue.apply(&mut world);
        world
            .run_system_once(
                move |mut commands: Commands,
                      children: Query<&Children>,
                      layers: Query<&mut SpriteLayer>,
                      ghost_layers: Query<&GhostLayer>| {
                    detach_mount_layers(&mut commands, &children, &layers, &ghost_layers, root);
                },
            )
            .expect("detach 应运行");
        let children = world.get::<Children>(root).expect("root 应有子实体");
        let mut remaining: Vec<String> = Vec::new();
        for c in children.iter() {
            if let Some(l) = world.get::<SpriteLayer>(c) {
                remaining.push(format!("layer:{:?}:mount={}", l.lib, l.is_mount));
            }
            if world.get::<GhostLayer>(c).is_some() {
                remaining.push("ghost".to_string());
            }
        }
        assert_eq!(
            remaining,
            vec!["layer:CArmours:mount=false".to_string()],
            "下马后应只剩身体层，实际: {remaining:?}"
        );
    }
}
