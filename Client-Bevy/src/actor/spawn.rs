// ============================================================================
// actor 模块拆分（#72）
// ============================================================================

use bevy::prelude::*;
use bevy::sprite::Anchor;
use mir2_shared::{MirAction, MirClass, MirGender};
use crate::map_renderer::{GameData, GameLibraries, TILE_HEIGHT, TILE_WIDTH};
use crate::resources::libraries::{ArrayLibType, LibraryName};
use crate::ui::sprite_ui::{UiFont, UiImageCache};
use crate::network::{NetObject, NetObjectRemoved};
use super::components::*;
use super::frames::actor_frame;
use super::spawn_helpers::*;

pub(crate) fn spawn_net_objects_when_ready(
    mut commands: Commands,
    data: Res<GameData>,
    mut spawns: MessageReader<NetObject>,
    session: Res<crate::network::SessionState>,
    mut libs: ResMut<GameLibraries>,
    mut images: ResMut<Assets<Image>>,
    mut cache: ResMut<UiImageCache>,
    mut fonts: ResMut<Assets<Font>>,
    mut ui_font: ResMut<UiFont>,
    sound_bank: Res<crate::game::sound::SoundBank>,
    mut audio_assets: ResMut<Assets<bevy::audio::AudioSource>>,
    mut actors: Query<(Entity, &NetObjectId, Option<&MountState>)>,
    children: Query<&Children>,
    mut layers: Query<&mut SpriteLayer>,
) {
    if data.map.is_none() {
        return;
    }
    let pending: Vec<NetObject> = spawns.read().cloned().collect();
    if pending.is_empty() {
        return;
    }
    // mock 模式没有 UserInformation → local_player_id=None，第一个 ObjectPlayer 视为本地
    let mut local_spawned = session.local_player_id.is_some();
    for obj in &pending {
        let is_local = match obj {
            NetObject::Player { object_id, .. } => {
                if session.local_player_id == Some(*object_id) {
                    true
                } else if session.local_player_id.is_none() && !local_spawned {
                    local_spawned = true;
                    true
                } else {
                    false
                }
            }
            _ => false,
        };
        match obj {
            NetObject::GroundGold {
                object_id,
                gold,
                location_x,
                location_y,
            } => spawn_ground_gold(
                &mut commands,
                &mut images,
                &mut fonts,
                &mut ui_font,
                *gold,
                *location_x,
                *location_y,
                *object_id,
            ),
            NetObject::GroundItem {
                object_id,
                item,
                location_x,
                location_y,
            } => spawn_ground_item(
                &mut commands,
                &mut libs,
                &mut images,
                &mut cache,
                &mut fonts,
                &mut ui_font,
                item,
                *location_x,
                *location_y,
                *object_id,
            ),
            NetObject::Player {
                object_id,
                mount_type,
                is_mounted,
                guild_name,
                class,
                gender,
                hair,
                weapon,
                weapon_effect,
                armour,
                wing_effect,
                ..
            } => {
                // M60：已存在的玩家（骑乘/下马重发 ObjectPlayer）→ 只更新坐骑层，不重复生成
                // #1402：加退会/职位变化重发 ObjectPlayer → 即时更新行会名标签（#1374 续）
                let existing = actors
                    .iter()
                    .find(|(_, id, _)| id.0 == *object_id)
                    .map(|(e, _, _)| e);
                if let Some(ent) = existing {
                    let has_mount = actors
                        .iter()
                        .any(|(e, _, m)| e == ent && m.is_some());
                    if *is_mounted && *mount_type >= 0 && !has_mount {
                        commands.entity(ent).insert(MountState { mount_type: *mount_type });
                        commands.entity(ent).with_children(|p| {
                            p.spawn((
                                Sprite::default(),
                                Transform::default(),
                                SpriteLayer {
                                    lib: ArrayLibType::Mounts,
                                    slot: (*mount_type).max(0) as u32,
                                    frame: 0,
                                    is_effect: false,
                                    is_mount: true,
                                    alpha: 1.0,
                                },
                            ));
                        });
                        tracing::info!("🐴 玩家 {} 骑乘坐骑 type={}", object_id, mount_type);
                    } else if !*is_mounted && has_mount {
                        commands.entity(ent).remove::<MountState>();
                        if let Ok(children_of) = children.get(ent) {
                            for c in children_of.iter() {
                                if let Ok(l) = layers.get(c) {
                                    if l.is_mount {
                                        commands.entity(c).despawn();
                                    }
                                }
                            }
                        }
                        tracing::info!("🐴 玩家 {} 下马", object_id);
                    }
                    // #1402：重发 ObjectPlayer 时同步行会名标签
                    if guild_name.is_empty() {
                        commands.entity(ent).remove::<PlayerGuildName>();
                    } else {
                        commands
                            .entity(ent)
                            .insert(PlayerGuildName(guild_name.clone()));
                    }
                    // #1606：重发 ObjectPlayer（换装/发型）→ 即时更新 ActorAppearance（装备外观）
                    commands.entity(ent).insert(ActorAppearance {
                        class: *class,
                        gender: *gender,
                        armour: *armour as u16,
                        hair: *hair,
                        weapon: *weapon,
                        weapon_effect: *weapon_effect,
                        wing_effect: *wing_effect,
                    });
                    continue;
                }
                spawn_net_object_entity(&mut commands, obj, is_local, &sound_bank, &mut audio_assets);
            }
            _ => spawn_net_object_entity(&mut commands, obj, is_local, &sound_bank, &mut audio_assets),
        }
    }
    tracing::info!("🌐 网络对象生成完成: {} 个", pending.len());
}

/// 按网络对象生成实体；is_local_player 时生成受控本地玩家（无 DemoBehavior）
fn spawn_net_object_entity(
    commands: &mut Commands,
    obj: &NetObject,
    is_local_player: bool,
    sound_bank: &crate::game::sound::SoundBank,
    audio_assets: &mut Assets<bevy::audio::AudioSource>,
) {
    // 瓦片坐标 → 世界像素（脚点）
    let wx = |tx: i32| tx as f32 * TILE_WIDTH + TILE_WIDTH / 2.0;
    let wy = |ty: i32| ty as f32 * TILE_HEIGHT + TILE_HEIGHT;

    match obj {
        NetObject::Player {
            object_id,
            name,
            guild_name,
            guild_rank_name: _,
            class,
            gender,
            location_x,
            location_y,
            direction: _,
            hair,
            weapon,
            weapon_effect,
            armour,
            wing_effect,
            mount_type,
            is_mounted,
        } => {
            tracing::debug!("🧍 NetObject::Player id={} name={} loc=({},{}) local={}", object_id, name, location_x, location_y, is_local_player);
            // 注意：世界坐标 y 向下取负（与地图/怪物/NPC 一致），此前玩家未取负导致镜像位置
            let e = if is_local_player {
                tracing::debug!("🧍 生成本地玩家 world=({:.0},{:.0})", wx(*location_x), -wy(*location_y));
                spawn_local_player_with(
                    commands,
                    wx(*location_x),
                    -wy(*location_y),
                    *class,
                    *gender,
                    *armour,
                    *hair,
                    *weapon,
                    *weapon_effect,
                    *wing_effect,
                    *object_id,
                    *mount_type,
                    *is_mounted,
                )
            } else {
                spawn_remote_player_with(
                    commands,
                    wx(*location_x),
                    -wy(*location_y),
                    *class,
                    *gender,
                    *armour,
                    *hair,
                    *weapon,
                    *weapon_effect,
                    *wing_effect,
                    *object_id,
                    *mount_type,
                    *is_mounted,
                )
            };
            commands.entity(e).insert(PlayerName(name.clone()));
            // #1374：行会名标签（非空才插）
            if !guild_name.is_empty() {
                commands
                    .entity(e)
                    .insert(PlayerGuildName(guild_name.clone()));
            }
        }
        NetObject::GroundGold { .. } => {}
        NetObject::Monster {
            object_id,
            name,
            location_x,
            location_y,
            image,
            direction,
        } => {
            let e = spawn_monster(
                commands,
                *image,
                wx(*location_x),
                wy(*location_y),
                *direction,
            );
            commands
                .entity(e)
                .insert((NetObjectId(*object_id), MonsterName(name.clone())));
            // #1631：怪物出现音（C# SetAction(Standing) → PlayAppearSound，MonsterObject.cs:284-296）
            if let Some(sound_id) = crate::game::sound::monster_appear_sound(*image as u16, false) {
                crate::game::sound::play_sound(commands, audio_assets, sound_bank, sound_id);
            }
        }
        NetObject::Npc {
            object_id,
            name,
            image,
            location_x,
            location_y,
            direction,
        } => {
            let e = spawn_npc(
                commands,
                *image,
                wx(*location_x),
                wy(*location_y),
                *direction,
            );
            commands
                .entity(e)
                .insert((NetObjectId(*object_id), NpcName(name.clone())));
        }
        // 地面物品在 spawn_net_objects_when_ready 中带资源生成，此处不处理
        NetObject::GroundItem { .. } => {}
    }
}

/// 生成地面物品实体：Items 库图标 + 名称标签（原版 C# ItemObject 地面渲染）
fn spawn_ground_item(
    commands: &mut Commands,
    libs: &mut GameLibraries,
    images: &mut Assets<Image>,
    cache: &mut UiImageCache,
    fonts: &mut Assets<Font>,
    ui_font: &mut UiFont,
    item: &crate::game::dialogs::inventory::InvItem,
    tx: i32,
    ty: i32,
    object_id: u32,
) {
    libs.0.ensure_initialized();
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(fonts);
    }
    let font = ui_font.0.clone();
    let wx = tx as f32 * TILE_WIDTH + TILE_WIDTH / 2.0;
    let wy = ty as f32 * TILE_HEIGHT + TILE_HEIGHT;
    let z = depth_z(wy);
    let e = commands
        .spawn((
            GroundItem {
                name: item.name.clone(),
            },
            NetObjectId(object_id),
            Transform::from_xyz(wx, -wy + 8.0, z),
            Visibility::default(),
        ))
        .id();
    if let Some(h) = crate::ui::sprite_ui::ui_image(
        libs,
        images,
        cache,
        LibraryName::Items,
        item.image as usize,
    ) {
        commands.entity(e).with_children(|p| {
            // 物品图标（原版 ItemObject.Draw 用 Items 库帧）
            p.spawn((
                Sprite::from_image(h),
                Anchor::CENTER,
                Transform::from_xyz(0.0, 0.0, 0.1),
            ));
            // 名称标签（白字，图标上方）
            p.spawn((
                Text2d::new(item.name.clone()),
                Anchor::TOP_LEFT,
                TextFont {
                    font: FontSource::Handle(font),
                    font_size: FontSize::Px(10.0),
                    ..default()
                },
                TextColor(Color::WHITE),
                Transform::from_xyz(-22.0, -22.0, 0.2),
            ));
        });
    }
}

/// #244 生成地面金币实体：金币块（金色彩块）+ "{N} 金币"标签（原版 ItemObject.Load(S.ObjectGold)）
fn spawn_ground_gold(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    fonts: &mut Assets<Font>,
    ui_font: &mut UiFont,
    gold: u32,
    tx: i32,
    ty: i32,
    object_id: u32,
) {
    if !ui_font.0.is_strong() {
        ui_font.0 = crate::ui::sprite_ui::load_ui_font(fonts);
    }
    let font = ui_font.0.clone();
    let wx = tx as f32 * TILE_WIDTH + TILE_WIDTH / 2.0;
    let wy = ty as f32 * TILE_HEIGHT + TILE_HEIGHT;
    let z = depth_z(wy);
    let white = images.add(crate::map_renderer::make_image(
        vec![255, 255, 255, 255],
        1,
        1,
    ));
    let e = commands
        .spawn((
            GroundGold { gold },
            NetObjectId(object_id),
            Transform::from_xyz(wx, -wy + 8.0, z),
            Visibility::default(),
        ))
        .id();
    commands.entity(e).with_children(|p| {
        // 金币块（金色彩块，占位 FloorItems 图标）
        p.spawn((
            Sprite {
                image: white.clone(),
                color: Color::srgb(1.0, 0.85, 0.2),
                custom_size: Some(Vec2::splat(14.0)),
                ..default()
            },
            Anchor::CENTER,
            Transform::from_xyz(0.0, 0.0, 0.1),
        ));
        // 金币标签
        p.spawn((
            Text2d::new(format!("{} 金币", gold)),
            Anchor::TOP_LEFT,
            TextFont {
                font: FontSource::Handle(font),
                font_size: FontSize::Px(10.0),
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.9, 0.3)),
            Transform::from_xyz(-22.0, -22.0, 0.2),
        ));
    });
}

/// 处理 ObjectRemove：按 NetObjectId 删除对应实体
pub(crate) fn despawn_removed_objects(
    mut commands: Commands,
    mut removals: MessageReader<NetObjectRemoved>,
    query: Query<(Entity, &NetObjectId)>,
) {
    let to_remove: Vec<u32> = removals.read().map(|r| r.0).collect();
    if to_remove.is_empty() {
        return;
    }
    for (e, id) in &query {
        if to_remove.contains(&id.0) {
            tracing::debug!("🗑️ 移除对象实体 id={}", id.0);
            commands.entity(e).despawn();
        }
    }
}

/// 等待地图加载完成后生成演示角色（只跑一次）
pub(crate) fn spawn_demo_actors_when_ready(
    mut commands: Commands,
    data: Res<GameData>,
    mut done: Local<bool>,
) {
    // 演示角色只在 --demo 模式下生成（默认走网络 mock 对象）
    if !std::env::args().any(|a| a == "--demo") {
        return;
    }
    if *done {
        return;
    }
    let Some(map) = data.map.as_ref() else {
        return;
    };
    *done = true;

    let cx = (map.width as f32 * TILE_WIDTH) / 2.0;
    let cy = (map.height as f32 * TILE_HEIGHT) / 2.0;

    spawn_player(&mut commands, cx, cy - 4.0 * TILE_HEIGHT);
    spawn_monster(
        &mut commands,
        1,
        cx - 4.0 * TILE_WIDTH,
        cy + 2.0 * TILE_HEIGHT,
        0,
    );
    spawn_monster(
        &mut commands,
        5,
        cx + 4.0 * TILE_WIDTH,
        cy - 3.0 * TILE_HEIGHT,
        0,
    );
    spawn_monster(
        &mut commands,
        9,
        cx - 3.0 * TILE_WIDTH,
        cy + 4.0 * TILE_HEIGHT,
        0,
    );
    spawn_npc(
        &mut commands,
        0,
        cx + 3.0 * TILE_WIDTH,
        cy + 3.0 * TILE_HEIGHT,
        0,
    );
}

/// 实现角色与建筑/树的经典交错遮挡）
pub fn depth_z(world_y: f32) -> f32 {
    crate::map_renderer::depth_y(world_y)
}

