use super::*;

pub fn update(ctx: &mut GameContext, _delay_time: f32) -> GameResult {
        if !ctx.events().has_network_events() {
            return Ok(());
        }

        // 同一帧内 mock/网络层可能会对同一对象推送多条 walk/run/attack。
        // 这里做一次"按 object_id 合并取最后一条"，避免插值/动画被频繁打断导致瞬移/抽风。
        use std::collections::HashMap;

        enum RemoteMovePacket {
            Walk(mir2_shared::packets::server::ObjectWalk),
            Run(mir2_shared::packets::server::ObjectRun),
        }

        let mut user_info: Option<mir2_shared::packets::server::UserInformation> = None;
        let mut map_changed: Option<mir2_shared::packets::server::MapChanged> = None;
        let mut start_game: Option<mir2_shared::packets::server::StartGame> = None;
        let mut start_game_delay: Option<mir2_shared::packets::server::StartGameDelay> = None;
        let mut start_game_banned: Option<mir2_shared::packets::server::StartGameBanned> = None;

        let mut mock_spawns: Vec<(u32, crate::network::handlers::ObjectType, crate::resources::LibraryName, i32, i32, i32)> = Vec::new();
        let mut mock_despawns: Vec<u32> = Vec::new();

        // 地面物品/金币（收集后循环外落地）
        let mut ground_items: Vec<mir2_shared::packets::server::ObjectItem> = Vec::new();
        let mut ground_golds: Vec<mir2_shared::packets::server::ObjectGold> = Vec::new();

        let mut object_monsters: Vec<mir2_shared::packets::server::ObjectMonster> = Vec::new();
        let mut object_npcs: Vec<mir2_shared::packets::server::ObjectNpc> = Vec::new();
        let mut object_players: Vec<mir2_shared::packets::server::ObjectPlayer> = Vec::new();
        let mut object_heroes: Vec<u32> = Vec::new(); // spawned hero object_ids
        let mut object_removes: Vec<u32> = Vec::new();
        let mut object_moves: HashMap<u32, RemoteMovePacket> = HashMap::new();
        let mut object_turns: Vec<mir2_shared::packets::server::ObjectTurn> = Vec::new();
        let mut object_attacks: HashMap<u32, ObjectAttackData> = HashMap::new();

        // 本地玩家：server-driven 状态（对齐真服）
        let mut player_location_changed: Option<(i32, i32)> = None;
        let mut gold_delta_sum: i32 = 0;
        let mut credit_delta_sum: i32 = 0;
        let mut items_gained: Vec<mir2_shared::data::item::UserItem> = Vec::new();
        let mut items_lost: Vec<(u64, u32)> = Vec::new();
        let mut items_dropped: Vec<(u64, u32)> = Vec::new();
        let mut items_moved: Vec<(u32, u32)> = Vec::new();
        let mut items_equipped: Vec<(u64, u8)> = Vec::new();
        let mut items_stored: Vec<(i32, i32)> = Vec::new();
        let mut items_taken_back: Vec<(i32, i32)> = Vec::new();
        let mut user_inventory_received: Option<Vec<mir2_shared::data::item::UserItem>> = None;
        let mut user_equipment_received: Option<Vec<mir2_shared::data::item::UserItem>> = None;
        let mut user_storage_received: Option<Vec<mir2_shared::data::item::UserItem>> = None;

        // combat feedback
        let mut player_died: Option<(u32, u32, u8)> = None;
        let mut player_health_changed: Option<(u32, u32)> = None;
        let mut player_mana_changed: Option<(u32, u32)> = None;
        let mut object_struck: Vec<(u32, u32, i32)> = Vec::new();
        let mut object_died: Vec<u32> = Vec::new();

        // combat aux packets
        let mut damage_indicators: Vec<(u32, i32, u8)> = Vec::new();
        let mut object_health_percents: Vec<(u32, u8, u16)> = Vec::new();

        // spell effects (collected for post-loop visual spawning)
        let mut spell_casts: Vec<(u32, u8, u32)> = Vec::new(); // (object_id, spell, target_id)
        let mut effect_received: Vec<(u32, u8, u32)> = Vec::new(); // (object_id, effect, effect_type)

        // spell cooldowns (collected for post-loop application)
        let mut spell_delays: Vec<(u8, u32)> = Vec::new(); // (spell_id, delay_ms)

        // UI / presentation feedback
        let mut play_sounds: Vec<i32> = Vec::new();
        let mut mount_updates: Vec<(u32, i16, bool)> = Vec::new();
        let mut player_inspects: Vec<mir2_shared::packets::server::PlayerInspect> = Vec::new();

        // New Phase 1-4 collections
        let mut buff_adds: Vec<(u32, u32, i64, bool, bool)> = Vec::new(); // (object_id, buff_id, expire_time, infinite, paused)
        let mut buff_removes: Vec<(u32, u32)> = Vec::new(); // (object_id, buff_id)
        let mut buff_pauses: Vec<(u32, u32, bool)> = Vec::new(); // (object_id, buff_id, paused)
        let mut hidden_objects: Vec<u32> = Vec::new();
        let mut shown_objects: Vec<u32> = Vec::new();
        let mut dash_failed: Vec<u32> = Vec::new();
        let mut sat_down: Vec<u32> = Vec::new();
        let mut backsteps: Vec<(u32, i32, i32)> = Vec::new();
        let mut dashes: Vec<(u32, i32, i32)> = Vec::new();
        let mut pushed: Vec<(u32, i32, i32)> = Vec::new();
        let mut dash_attacked: Vec<(u32, i32, i32)> = Vec::new();
        let mut attack_moved: Vec<(u32, i32, i32)> = Vec::new();
        let mut attack_mode_changes: Vec<(hecs::Entity, u8)> = Vec::new();
        let mut pet_mode_changes: Vec<(hecs::Entity, u8)> = Vec::new();
        let mut poisoned_objects: Vec<(u32, u8)> = Vec::new();
        let mut revived: Vec<u32> = Vec::new();
        let mut harvested: Vec<(u32, i32, i32, u8)> = Vec::new();
        let mut object_level_ups: Vec<(u32, u16)> = Vec::new();
        let mut range_attacks: Vec<(u32, u8, i32, i32)> = Vec::new();
        let mut range_projectiles: Vec<(u32, u32, u32, u32, u16)> = Vec::new(); // (from_id, target_id, target_x, target_y, spell)
        // Experience/level
        let mut player_exp_gains: Vec<i64> = Vec::new();
        let mut player_level_ups: Vec<u16> = Vec::new();
        let mut hero_exp_gains: Vec<i64> = Vec::new();
        let mut hero_level_ups: Vec<u16> = Vec::new();
        let mut hero_health_changes: Vec<(i32, i32)> = Vec::new();
        // Hero magic updates (deferred to avoid E0502 in loop)
        let mut hero_magic_learned: Vec<(u8, u8, u32, u8)> = Vec::new(); // (spell, level, experience, key)
        let mut hero_magic_removed: Vec<u8> = Vec::new();
        let mut hero_magic_leveled_up: Vec<(u8, u8)> = Vec::new();
        let mut hero_spell_toggled: Vec<(u8, bool)> = Vec::new();
        // Name/colour/guild changes
        let mut player_colour: Option<u32> = None;
        let mut object_colours: Vec<(u32, u32)> = Vec::new();
        let mut object_guild_names: Vec<(u32, String)> = Vec::new();
        let mut object_names: Vec<(u32, String)> = Vec::new();
        let mut player_appearance_updates: Vec<(u32, i16, i16, i16, u8)> = Vec::new();
        // Object mana
        let mut object_mana_percents: Vec<(u32, u8)> = Vec::new();
        // Sneaking / visibility
        let mut object_sneaking: Vec<(u32, bool)> = Vec::new();
        // Level effects flags
        let mut object_level_effects: Vec<(u32, u32)> = Vec::new();
        // Object spell (for MonsterAnimState)
        let mut object_spells: Vec<(u32, u16)> = Vec::new();
        // Durability / repairs
        let mut dura_changes: Vec<(u64, i32)> = Vec::new();
        let mut item_repairs: Vec<(u64, u16, u16)> = Vec::new(); // (unique_id, current_dura, max_dura)
        // Trap rock state
        let mut trap_rock_state: Option<bool> = None;
        // Fishing status (本地玩家钓鱼状态，post-loop 应用动画)
        let mut fishing_status: Option<(u8, bool)> = None; // (state, success)
        // Base stats (BaseStatsReceived)
        let mut base_stats_received: Option<Vec<i32>> = None;
        // Elemental state updates
        let mut elemental_updates: Vec<(u32, bool, u32, u8, i64)> = Vec::new();
        // Object decoration updates
        let mut deco_updates: Vec<(u32, u16, bool)> = Vec::new();
        // NPC image updates
        let mut npc_image_updates: Vec<(u32, u16)> = Vec::new();
        // Projectile effects (spell, source_object_id, destination_object_id)
        let mut projectiles: Vec<(u8, u32, u32)> = Vec::new();
        // Delayed explosion removals (object_id)
        let mut delayed_explosions: Vec<u32> = Vec::new();
        // Map effects (effect_type, x, y, value)
        let mut map_effects: Vec<(u8, i32, i32, i32)> = Vec::new();
        // Guild state (joined = Some(GuildInfo), left = None trigger)
        let mut guild_joined: Option<crate::components::GuildInfo> = None;
        let mut guild_left = false;
        let mut guild_name_received: Option<String> = None;
        // Social state (lover/mentor updates)
        let mut lover_updated: Option<(String, i64)> = None;
        let mut mentor_updated: Option<(String, i32, bool)> = None;
        // Observe state
        let mut observe_allowed: Option<bool> = None;
        // Group state
        let mut group_allow_join: Option<bool> = None;
        let mut group_members_added: Vec<String> = Vec::new();
        let mut group_members_removed: Vec<String> = Vec::new();
        let mut group_disbanded = false;
        // Trade state
        let mut trade_started: Option<String> = None;
        let mut trade_completed = false;

        // 无事件时提前返回，跳过 O(N) entity_index 建立
        if ctx.events().network_events().next().is_none() {
            return Ok(());
        }

        // 提前建立 entity_index（供事件循环和延迟处理使用，O(1) 查找）
        let entity_index = NetworkApplySystem::build_object_index(&ctx.world);
        // 从索引中提取 local_player_entity
        let local_player_entity = {
            use crate::components::LocalPlayer;
            ctx.world.iter().find_map(|e| e.get::<&LocalPlayer>().map(|_| e.entity()))
        };
        let (local_player_object_id, local_player_name) = local_player_entity
            .and_then(|e| {
                ctx.world.get::<&crate::components::PlayerData>(e).ok()
                    .map(|pd| (Some(pd.object_id), Some(pd.name.clone())))
            })
            .unwrap_or((None, None));

        for event in ctx.events().network_events() {
            match event {
                NetworkEvent::StartGame { packet } => {
                    start_game = Some(packet.clone());
                }
                NetworkEvent::StartGameDelay { packet } => {
                    start_game_delay = Some(packet.clone());
                }
                NetworkEvent::StartGameBanned { packet } => {
                    start_game_banned = Some(packet.clone());
                }
                NetworkEvent::UserInformation { packet } => {
                    user_info = Some(packet.clone());
                }
                NetworkEvent::MapChanged { packet } => {
                    map_changed = Some(packet.clone());
                }
                NetworkEvent::MockLibrarySpriteSpawn {
                    object_id,
                    object_type,
                    library,
                    index,
                    location_x,
                    location_y,
                } => {
                    mock_spawns.push((*object_id, *object_type, *library, *index, *location_x, *location_y));
                }
                NetworkEvent::MockLibrarySpriteDespawn { object_id } => {
                    mock_despawns.push(*object_id);
                }

                NetworkEvent::ObjectMonster { packet } => {
                    object_monsters.push(packet.clone());
                }
                NetworkEvent::ObjectNpc { packet } => {
                    object_npcs.push(packet.clone());
                }
                NetworkEvent::ObjectPlayer { packet } => {
                    object_players.push(packet.clone());
                }
                NetworkEvent::ObjectRemove { object_id } => {
                    object_removes.push(*object_id);
                }
                NetworkEvent::ObjectWalk { packet } => {
                    object_moves.insert(packet.object_id, RemoteMovePacket::Walk(packet.clone()));
                }
                NetworkEvent::ObjectRun { packet } => {
                    object_moves.insert(packet.object_id, RemoteMovePacket::Run(packet.clone()));
                }
                NetworkEvent::ObjectTurn { packet } => {
                    object_turns.push(packet.clone());
                }
                NetworkEvent::ObjectAttack { object_id, location_x, location_y, direction, spell, attack_type, .. } => {
                    object_attacks.insert(*object_id, ObjectAttackData {
                        location_x: *location_x,
                        location_y: *location_y,
                        direction: *direction,
                        spell: *spell,
                        attack_type: *attack_type,
                    });
                }

                // ===== server-driven: local player state =====
                NetworkEvent::PlayerLocationChanged { x, y } => {
                    player_location_changed = Some((*x, *y));
                }
                NetworkEvent::GoldChanged { delta } => {
                    gold_delta_sum = gold_delta_sum.saturating_add(*delta);
                }
                NetworkEvent::ItemGained { item } => {
                    items_gained.push(item.clone());
                }
                NetworkEvent::ItemLost { unique_id, count } => {
                    tracing::trace!("📦 Item lost: uid={} count={}", unique_id, count);
                    items_lost.push((*unique_id, *count));
                }
                NetworkEvent::ItemMoved { grid: _, from, to, success: _ } => {
                    items_moved.push((*from, *to));
                }

                // ===== combat feedback =====
                NetworkEvent::PlayerDied { x, y, direction } => {
                    player_died = Some((*x, *y, *direction));
                }
                NetworkEvent::HealthChanged { current, max } => {
                    player_health_changed = Some((*current, *max));
                }
                NetworkEvent::ManaChanged { current, max } => {
                    player_mana_changed = Some((*current, *max));
                }
                NetworkEvent::DamageIndicator {
                    object_id,
                    damage,
                    damage_type,
                } => {
                    damage_indicators.push((*object_id, *damage, *damage_type));
                }
                NetworkEvent::ObjectHealthPercent {
                    object_id,
                    percent,
                    expire,
                } => {
                    object_health_percents.push((*object_id, *percent, *expire));
                }
                NetworkEvent::PlayerStruck { attacker_id, damage } => {
                    if let Some(oid) = local_player_object_id {
                        object_struck.push((oid, *attacker_id, *damage));
                    }
                }
                NetworkEvent::ObjectStruck {
                    object_id,
                    attacker_id,
                    damage,
                    ..
                } => {
                    object_struck.push((*object_id, *attacker_id, *damage));
                }
                NetworkEvent::ObjectDied { object_id, location_x, location_y, direction, death_type } => {
                    tracing::trace!("💀 Object {} died at ({},{}) dir={} type={}", object_id, location_x, location_y, direction, death_type);
                    object_died.push(*object_id);
                }

                NetworkEvent::PlaySound { sound_id } => {
                    play_sounds.push(*sound_id);
                }
                NetworkEvent::MountUpdated {
                    object_id,
                    mount_type,
                    riding_mount,
                } => {
                    mount_updates.push((*object_id, *mount_type, *riding_mount));
                }

                NetworkEvent::PlayerInspect { packet } => {
                    player_inspects.push(packet.clone());
                }

                // ===== 魔法/技能 =====
                NetworkEvent::MagicListReceived { spell, target_id, target_x, target_y, cast, level } => {
                    tracing::trace!("✨ Magic: spell={:?} target={} ({},{}) cast={} level={}", spell, target_id, target_x, target_y, cast, level);
                    if *cast {
                        if let Some(oid) = local_player_object_id {
                            spell_casts.push((oid, *spell as u8, *target_id));
                        }
                    }
                }
                NetworkEvent::MagicLearned { magic, hero } => {
                    tracing::debug!("✨ Magic learned: {:?} level={} hero={}", magic.spell, magic.level, hero);
                    if *hero {
                        hero_magic_learned.push((magic.spell as u8, magic.level, magic.experience as u32, magic.key));
                    } else if let Some(e) = local_player_entity {
                        if let Ok(mut magic_list) = ctx.world.get::<&mut crate::components::spell::MagicList>(e) {
                            NetworkApplySystem::update_learned_magic(&mut magic_list, magic.spell as u8, magic.level, magic.experience as u32, magic.key);
                        }
                    }
                }
                NetworkEvent::MagicRemoved { spell, hero } => {
                    tracing::debug!("📜 Magic removed: {:?} hero={}", spell, hero);
                    if *hero {
                        hero_magic_removed.push(*spell as u8);
                    } else if let Some(e) = local_player_entity {
                        if let Ok(mut magic_list) = ctx.world.get::<&mut crate::components::spell::MagicList>(e) {
                            NetworkApplySystem::remove_magic(&mut magic_list, *spell as u8);
                        }
                    }
                }
                NetworkEvent::MagicLeveledUp { spell, level, hero } => {
                    tracing::debug!("📈 Magic leveled up: {:?} level={} hero={}", spell, level, hero);
                    if *hero {
                        hero_magic_leveled_up.push((*spell as u8, *level));
                    } else if let Some(e) = local_player_entity {
                        if let Ok(mut magic_list) = ctx.world.get::<&mut crate::components::spell::MagicList>(e) {
                            NetworkApplySystem::update_magic_level(&mut magic_list, *spell as u8, *level);
                        }
                    }
                }
                NetworkEvent::MagicDelayReceived { object_id: _, spell, delay } => {
                    // 收集到循环外处理，避免借用冲突
                    spell_delays.push((*spell as u8, *delay));
                    tracing::trace!("⏳ Magic cooldown: {:?} delay={}ms", spell, delay);
                }
                NetworkEvent::MagicCastEvent { spell } => {
                    tracing::trace!("🪄 Magic cast: {:?}", spell);
                    if let Some(oid) = local_player_object_id {
                        spell_casts.push((oid, *spell as u8, 0));
                    }
                }
                NetworkEvent::ObjectMagicCast { object_id, spell, target_id, .. } => {
                    spell_casts.push((*object_id, *spell as u8, *target_id));
                }
                NetworkEvent::ObjectEffectReceived { object_id, effect, effect_type, delay_time, time } => {
                    tracing::trace!("✨ Object {} effect={} type={} delay={} duration={}", object_id, effect, effect_type, delay_time, time);
                    effect_received.push((*object_id, *effect as u8, *effect_type as u32));
                }
                NetworkEvent::ObjectProjectileReceived { spell, source, destination } => {
                    tracing::trace!("🪄 Projectile {:?} from {} to {}", spell, source, destination);
                    projectiles.push(((*spell).into(), *source, *destination));
                }
                NetworkEvent::SpellToggled { spell, can_use, hero } => {
                    tracing::trace!("🔄 Spell toggle: {:?} can_use={} hero={}", spell, can_use, hero);
                    if *hero {
                        hero_spell_toggled.push(((*spell).into(), *can_use));
                    } else if let Some(e) = local_player_entity {
                        if let Ok(mut magic_list) = ctx.world.get::<&mut crate::components::spell::MagicList>(e) {
                            NetworkApplySystem::update_spell_toggle(&mut magic_list, (*spell).into(), *can_use);
                        }
                    }
                }

                // ===== Buff =====
                NetworkEvent::BuffAdded { object_id, buff_id, visible, expire_time, infinite, paused } => {
                    tracing::trace!("➕ BuffAdded: object={} buff={} visible={} expire={} infinite={} paused={}", object_id, buff_id, visible, expire_time, infinite, paused);
                    buff_adds.push((*object_id, *buff_id, *expire_time, *infinite, *paused));
                }
                NetworkEvent::BuffRemoved { object_id, buff_id } => {
                    buff_removes.push((*object_id, *buff_id));
                }
                NetworkEvent::BuffPaused { object_id, buff_id, paused } => {
                    buff_pauses.push((*object_id, *buff_id, *paused));
                }

                // ===== 移动扩展 =====
                NetworkEvent::ObjectHeroSpawned { packet } => {
                    tracing::trace!("🦸 Object hero spawned: owner={} id={}", packet.owner_name, packet.player.object_id);
                    object_players.push(packet.player.clone());
                    object_heroes.push(packet.player.object_id);
                    // 若属于本地玩家，记录英雄 object_id 到 HeroState
                    if local_player_name.as_ref().is_some_and(|n| *n == packet.owner_name) {
                        if let Some(e) = local_player_entity {
                            if let Ok(mut hero) = ctx.world.get::<&mut crate::components::HeroState>(e) {
                                hero.hero_object_id = packet.player.object_id;
                            }
                        }
                    }
                }
                NetworkEvent::ObjectHidden { object_id, hidden } => {
                    if *hidden {
                        hidden_objects.push(*object_id);
                    }
                }
                NetworkEvent::ObjectShown { object_id } => {
                    shown_objects.push(*object_id);
                }
                NetworkEvent::ObjectTeleportingOut { object_id, teleport_type: _ } => {
                    hidden_objects.push(*object_id);
                }
                NetworkEvent::ObjectTeleportingIn { object_id, teleport_type: _ } => {
                    shown_objects.push(*object_id);
                }
                NetworkEvent::PlayerTeleportedIn => {
                    // 本地玩家传送进入：由地图切换系统处理
                }
                NetworkEvent::ObjectBackStepped { object_id, location_x, location_y, direction: _, distance: _ } => {
                    backsteps.push((*object_id, *location_x, *location_y));
                }
                NetworkEvent::PlayerBackStepped { x, y } => {
                    if let Some(oid) = local_player_object_id {
                        backsteps.push((oid, *x, *y));
                    }
                }
                NetworkEvent::ObjectDashing { object_id, location_x, location_y, direction: _ } => {
                    dashes.push((*object_id, *location_x as i32, *location_y as i32));
                }
                NetworkEvent::PlayerDashing { x, y } => {
                    if let Some(oid) = local_player_object_id {
                        dashes.push((oid, *x, *y));
                    }
                }
                NetworkEvent::ObjectDashFailed { object_id, location_x, location_y, direction } => {
                    tracing::trace!("💨 Object {} dash failed at ({},{}) dir={}", object_id, location_x, location_y, direction);
                    dash_failed.push(*object_id);
                }
                NetworkEvent::PlayerDashFailed { location_x, location_y, direction } => {
                    tracing::trace!("💨 Player dash failed: loc=({},{}) dir={}", location_x, location_y, direction);
                    if let Some(oid) = local_player_object_id {
                        dash_failed.push(oid);
                    }
                }
                NetworkEvent::ObjectSatDown { object_id, direction, location } => {
                    tracing::trace!("🪑 Object {} sat down at ({},{}) dir={}", object_id, location.0, location.1, direction);
                    sat_down.push(*object_id);
                }
                NetworkEvent::NewMapInfoReceived { packet } => {
                    tracing::trace!("🗺️ New map info received: idx={} title={}", packet.map_index, packet.title);
                }
                NetworkEvent::WorldMapSetupReceived { icons } => {
                    tracing::trace!("🗺️ World map setup received: {} icons", icons.len());
                }
                NetworkEvent::SearchMapResultReceived { map_index, location_x, location_y } => {
                    tracing::trace!("🗺️ Search map result: map={} loc=({}, {})", map_index, location_x, location_y);
                }
                NetworkEvent::TimeOfDayChanged { time_of_day } => {
                    tracing::trace!("🌅 Time of day changed: {}", time_of_day);
                    use crate::components::TimeOfDay;
                    if let Some((entity, _)) = ctx.world.query::<(hecs::Entity, &TimeOfDay)>().iter().next() {
                        if let Ok(mut td) = ctx.world.get::<&mut TimeOfDay>(entity) {
                            td.hour = *time_of_day;
                        }
                    }
                }

                // ===== 玩家状态 =====
                NetworkEvent::PlayerUpdated { object_id, light: _, weapon, weapon_effect, armor, wings_effect } => {
                    player_appearance_updates.push((*object_id, *weapon, *weapon_effect, *armor, *wings_effect));
                    tracing::trace!("👤 Player {} updated: weapon={} armor={}", object_id, weapon, armor);
                }
                NetworkEvent::AttackModeChanged { mode } => {
                    // 本地玩家的攻击模式变化
                    if let Some(e) = local_player_entity {
                        attack_mode_changes.push((e, *mode));
                    }
                }
                NetworkEvent::PetModeChanged { mode } => {
                    // 本地玩家的宠物模式变化
                    if let Some(e) = local_player_entity {
                        pet_mode_changes.push((e, *mode));
                    }
                }
                NetworkEvent::PlayerColourChanged { colour } => {
                    player_colour = Some(*colour);
                }
                NetworkEvent::ObjectColourChanged { object_id, colour } => {
                    object_colours.push((*object_id, *colour));
                }
                NetworkEvent::ObjectGuildNameChanged2 { object_id, guild_name } => {
                    object_guild_names.push((*object_id, guild_name.clone()));
                }
                NetworkEvent::PlayerNameUpdated { object_id, name } => {
                    object_names.push((*object_id, name.clone()));
                }
                NetworkEvent::UserNameUpdated { object_id, name } => {
                    object_names.push((*object_id, name.clone()));
                }

                // ===== 战斗扩展 =====
                NetworkEvent::DuraChanged { unique_id, durability } => {
                    dura_changes.push((*unique_id, *durability));
                }
                NetworkEvent::PlayerPoisoned { object_id, poison_type } => {
                    poisoned_objects.push((*object_id, *poison_type));
                }
                NetworkEvent::ObjectPoisonedEvent { object_id, poison_type } => {
                    poisoned_objects.push((*object_id, *poison_type));
                }
                NetworkEvent::RangeAttacked { target_id, target_x, target_y, spell, spell_level } => {
                    tracing::trace!("🏹 RangeAttack: target={} loc=({},{}) spell={} level={}", target_id, target_x, target_y, spell, spell_level);
                    let from_id = local_player_object_id.unwrap_or(0);
                    range_projectiles.push((from_id, *target_id, *target_x, *target_y, *spell));
                }
                NetworkEvent::ObjectRangeAttacked { object_id, location_x, location_y, direction, target_id, target_x, target_y, spell, spell_level: _ } => {
                    range_attacks.push((*object_id, (*direction), *location_x as i32, *location_y as i32));
                    range_projectiles.push((*object_id, *target_id, *target_x, *target_y, *spell));
                }
                NetworkEvent::PushedEvent { object_id, x, y, direction: _ } => {
                    pushed.push((*object_id, *x, *y));
                }
                NetworkEvent::ObjectPushedEvent { object_id, x, y, direction: _ } => {
                    pushed.push((*object_id, *x, *y));
                }
                NetworkEvent::UserDashAttacked { x, y, direction: _ } => {
                    if let Some(oid) = local_player_object_id {
                        dash_attacked.push((oid, *x, *y));
                    }
                }
                NetworkEvent::ObjectDashAttacked { object_id, location_x, location_y, direction: _, distance: _ } => {
                    dash_attacked.push((*object_id, *location_x, *location_y));
                }
                NetworkEvent::UserAttackMoved { x, y } => {
                    if let Some(oid) = local_player_object_id {
                        attack_moved.push((oid, *x, *y));
                    }
                }
                NetworkEvent::PlayerRevived => {
                    if let Some(e) = local_player_entity {
                        if let Ok(ns) = ctx.world.get::<&crate::components::NetworkSync>(e) {
                            revived.push(ns.object_id);
                        }
                    }
                }
                NetworkEvent::ObjectRevivedEvent { object_id, effect } => {
                    tracing::trace!("🔄 Object {} revived (effect={})", object_id, effect);
                    revived.push(*object_id);
                }
                NetworkEvent::ObjectLeveled { object_id, level } => {
                    object_level_ups.push((*object_id, *level));
                }
                NetworkEvent::ObjectManaPercent { object_id, percent } => {
                    object_mana_percents.push((*object_id, *percent));
                }
                NetworkEvent::ExperienceGained { amount } => {
                    player_exp_gains.push(*amount);
                }
                NetworkEvent::LevelUp { new_level } => {
                    player_level_ups.push(*new_level);
                }
                NetworkEvent::HeroExperienceGained { amount } => {
                    hero_exp_gains.push(*amount);
                }
                NetworkEvent::HeroLevelUp { new_level } => {
                    hero_level_ups.push(*new_level);
                }
                NetworkEvent::HeroHealthChanged { hp, mp } => {
                    hero_health_changes.push((*hp, *mp));
                }

                // ===== 物品扩展 =====
                NetworkEvent::ItemEquipped { grid: _, unique_id, slot, success } => {
                    if *success {
                        tracing::debug!("装备成功: uid={} slot={}", unique_id, slot);
                        items_equipped.push((*unique_id, *slot));
                    }
                }
                NetworkEvent::ItemMerged { grid_from: _, grid_to: _, id_from, id_to, success } => {
                    if *success {
                        tracing::debug!("物品合并成功: from={} to={}", id_from, id_to);
                    }
                    // 实际物品数据由后续 UserSlotsRefresh 更新
                }
                NetworkEvent::ItemRemoved { grid: _, unique_id, to: _, success: _ } => {
                    if let Some(e) = local_player_entity {
                        if let Ok(mut inv) = ctx.world.get::<&mut crate::components::Inventory>(e) {
                            inv.remove_by_unique_id(*unique_id, 0);
                        }
                    }
                }
                NetworkEvent::ItemSlotRemoved { grid: _, grid_to: _, slot, unique_id: _, success: _ } => {
                    if let Some(e) = local_player_entity {
                        if let Ok(mut inv) = ctx.world.get::<&mut crate::components::Inventory>(e) {
                            let s = *slot as usize;
                            if s < inv.items.len() {
                                inv.items[s] = None;
                            }
                        }
                    }
                }
                NetworkEvent::ItemTakenBack { from, to, success } => {
                    if *success {
                        tracing::debug!("物品取回成功: {} -> {}", from, to);
                        items_taken_back.push((*from, *to));
                    }
                }
                NetworkEvent::ItemStored { from, to, success } => {
                    if *success {
                        tracing::debug!("物品存入仓库: {} -> {}", from, to);
                        items_stored.push((*from, *to));
                    }
                }
                NetworkEvent::ItemSplit { grid: _, unique_id, count } => {
                    if let Some(e) = local_player_entity {
                        if let Ok(mut inv) = ctx.world.get::<&mut crate::components::Inventory>(e) {
                            if let Some(it) = inv.find_mut_by_id(*unique_id) {
                                it.count -= *count as u16;
                            }
                        }
                    }
                }
                NetworkEvent::ItemUsed { unique_id: _ } => {
                    // 物品使用：服务器确认已使用，客户端可播放使用特效
                    tracing::trace!("🧪 Item used");
                }
                NetworkEvent::ItemDropped { unique_id, count, success } => {
                    tracing::trace!("📦 Item dropped: uid={} count={} success={}", unique_id, count, success);
                    if *success {
                        items_dropped.push((*unique_id, *count));
                    }
                }
                NetworkEvent::ItemRefreshed { item } => {
                    // 刷新物品数据：更新背包中对应物品
                    if let Some(e) = local_player_entity {
                        if let Ok(mut inv) = ctx.world.get::<&mut crate::components::Inventory>(e) {
                            for slot in inv.items.iter_mut() {
                                if let Some(ref it) = slot {
                                    if it.unique_id == item.unique_id {
                                        *slot = Some(item.clone());
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                NetworkEvent::ItemSlotSizeChanged { slot, size } => {
                    if let Some(e) = local_player_entity {
                        if let Ok(mut inv) = ctx.world.get::<&mut crate::components::Inventory>(e) {
                            let s = *slot as usize;
                            let new_size = *size as usize;
                            if s < inv.items.len() && new_size != inv.items.len() {
                                // 调整背包大小
                                inv.items.resize(new_size, None);
                                inv.capacity = new_size;
                            }
                        }
                    }
                }
                NetworkEvent::ItemSealed { unique_id, expiry_date } => {
                    tracing::trace!("🔒 Item sealed: uid={} expiry={}", unique_id, expiry_date);
                    let Some(e) = local_player_entity else { continue };

                    let sealed = mir2_shared::data::item::SealedInfo {
                        expiry_date_binary: *expiry_date,
                        next_seal_date_binary: 0,
                    };

                    if let Ok(mut inv) = ctx.world.get::<&mut crate::components::Inventory>(e) {
                        if let Some(item) = inv.find_mut_by_id(*unique_id) {
                            item.sealed_info = Some(sealed.clone());
                        }
                    }
                    if let Ok(mut eq) = ctx.world.get::<&mut crate::components::Equipment>(e) {
                        if let Some(item) = eq.find_mut_by_id(*unique_id) {
                            item.sealed_info = Some(sealed);
                        }
                    }
                }
                NetworkEvent::ItemSlotEquipped { grid: _, grid_to: _, slot, unique_id, success } => {
                    if *success {
                        tracing::debug!("槽位装备成功: slot={} uid={}", slot, unique_id);
                        items_equipped.push((*unique_id, *slot as u8));
                    }
                }
                NetworkEvent::ItemCombined { grid: _, id_from, id_to, success, destroy } => {
                    if *success {
                        tracing::debug!("物品合成成功: from={} to={} destroy={}", id_from, id_to, destroy);
                    }
                    // 实际物品数据由后续 UserSlotsRefresh 更新
                }
                NetworkEvent::ItemUpgraded { item } => {
                    // 升级后的物品替换原物品
                    if let Some(e) = local_player_entity {
                        if let Ok(mut inv) = ctx.world.get::<&mut crate::components::Inventory>(e) {
                            for slot in inv.items.iter_mut() {
                                if let Some(ref it) = slot {
                                    if it.unique_id == item.unique_id {
                                        *slot = Some(item.clone());
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                NetworkEvent::GroundItem { packet } => {
                    // 地面上的物品（可拾取）- 收集到循环外落地
                    ground_items.push(packet.clone());
                }
                NetworkEvent::CreditChanged { delta } => {
                    tracing::trace!("💎 Credit changed: {}", delta);
                    credit_delta_sum += *delta;
                }
                NetworkEvent::ObjectHarvested { object_id, location_x, location_y, direction } => {
                    harvested.push((*object_id, *location_x, *location_y, *direction as u8));
                }
                NetworkEvent::RefineItemDeposited { from, to, success } => {
                    tracing::trace!("🔨 Refine item deposited: {}→{} success={}", from, to, success);
                }
                NetworkEvent::RefineItemRetrieved { from, to, success } => {
                    tracing::trace!("🔨 Refine item retrieved: {}→{} success={}", from, to, success);
                }
                NetworkEvent::RefineCancelled { unlock } => {
                    tracing::trace!("🔨 Refine cancelled: unlock={}", unlock);
                }
                NetworkEvent::RefineItemCompleted { unique_id } => {
                    tracing::trace!("🔨 Refine completed: unique_id={}", unique_id);
                }
                NetworkEvent::TradeItemDeposited { from_slot, success } => {
                    tracing::trace!("🤝 Trade item deposited: from={} success={}", from_slot, success);
                }
                NetworkEvent::TradeItemRetrieved { from_slot, success } => {
                    tracing::trace!("🤝 Trade item retrieved: from={} success={}", from_slot, success);
                }
                NetworkEvent::HeroItemTakenBack { from, to, success } => {
                    tracing::trace!("🦸 Hero item taken back: {}→{} success={}", from, to, success);
                }
                NetworkEvent::HeroItemTransferred { from, to, success } => {
                    tracing::trace!("🦸 Hero item transferred: {}→{} success={}", from, to, success);
                }
                NetworkEvent::NewItemInfoReceived { item_index, item_name } => {
                    tracing::trace!("📋 New item info: idx={} name={}", item_index, item_name);
                }
                NetworkEvent::NewChatItemReceived { item_id } => {
                    tracing::trace!("📋 New chat item: id={}", item_id);
                }
                NetworkEvent::ObjectGoldReceived { packet } => {
                    // 地面金币 - 收集到循环外落地
                    ground_golds.push(packet.clone());
                }

                // ===== 其他（交易/任务/好友/公会/NPC/英雄/邮件/市场/社交等）=====
                NetworkEvent::TradeGoldAdded { amount } => {
                    tracing::trace!("💰 Trade gold added: {}", amount);
                }
                NetworkEvent::TradeItemAdded { items } => {
                    tracing::trace!("📦 Trade item added: {} items", items.iter().filter(|i| i.is_some()).count());
                }
                NetworkEvent::TradeConfirmedEvent { locked } => {
                    tracing::trace!("🤝 Trade confirmed (locked={})", locked);
                }
                NetworkEvent::TradeCancelledEvent { unlock } => {
                    tracing::trace!("🤝 Trade cancelled (unlock={})", unlock);
                }
                NetworkEvent::QuestListUpdated => {
                    tracing::trace!("📋 Quest list updated");
                }
                NetworkEvent::QuestItemGained { item_id } => {
                    tracing::trace!("📋 Quest item gained: item_id={}", item_id);
                }
                NetworkEvent::QuestItemLost { unique_id } => {
                    // 失去任务物品
                    if let Some(e) = local_player_entity {
                        if let Ok(mut qinv) = ctx.world.get::<&mut crate::components::QuestInventory>(e) {
                            for slot in qinv.items.iter_mut() {
                                if let Some(ref it) = slot {
                                    if it.unique_id == *unique_id {
                                        *slot = None;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    tracing::trace!("📋 Quest item lost");
                }
                NetworkEvent::QuestShared { quest_id } => {
                    tracing::trace!("📋 Quest {} shared", quest_id);
                }
                NetworkEvent::QuestProgressUpdated { quest_id, progress } => {
                    tracing::trace!("📋 Quest {} progress: {}", quest_id, progress);
                }
                NetworkEvent::QuestInfoReceived { quest_id, name, .. } => {
                    tracing::trace!("📋 Quest info received: #{} {}", quest_id, name);
                }
                NetworkEvent::FriendUpdated { .. } => {
                    tracing::trace!("👥 Friend updated");
                }
                NetworkEvent::GuildJoined { guild_name, rank_name, level, experience, max_experience, gold, spare_points, member_count, max_members, my_rank_id, .. } => {
                    tracing::info!("🏰 Guild joined: {} (rank: {}) Lv.{}", guild_name, rank_name, level);
                    guild_joined = Some(crate::components::GuildInfo {
                        name: guild_name.clone(),
                        rank: rank_name.clone(),
                        level: *level as u16,
                        experience: *experience,
                        max_experience: *max_experience,
                        gold: *gold,
                        spare_points: *spare_points as u32,
                        member_count: *member_count as u16,
                        max_members: *max_members as u16,
                        my_rank_id: *my_rank_id as u8,
                    });
                }
                NetworkEvent::GuildLeft => {
                    tracing::info!("🏰 Guild left");
                    guild_left = true;
                }
                NetworkEvent::GuildNoticeUpdated { notice } => {
                    tracing::trace!("🏰 Guild notice updated: {}", notice);
                }
                NetworkEvent::GuildMemberUpdated { name, rank, online } => {
                    tracing::trace!("🏰 Guild member updated: {} rank={} online={}", name, rank, online);
                }
                NetworkEvent::GuildExpGained { amount } => {
                    tracing::trace!("🏰 Guild exp gained: {}", amount);
                }
                NetworkEvent::GuildNameReceived { name } => {
                    tracing::trace!("🏰 Guild name received: {}", name);
                    guild_name_received = Some(name.clone());
                }
                NetworkEvent::GuildStorageGoldChanged { delta, total } => {
                    tracing::trace!("🏰 Guild storage gold changed: delta={} total={}", delta, total);
                }
                NetworkEvent::GuildStorageItemChanged { change_type, slot } => {
                    tracing::trace!("🏰 Guild storage item changed: type={} slot={}", change_type, slot);
                }
                NetworkEvent::GuildStorageListReceived { items } => {
                    tracing::trace!("🏰 Guild storage list received: {} slots", items.len());
                }
                NetworkEvent::GuildWarRequested { guild_name } => {
                    tracing::trace!("🏰 Guild war requested by {}", guild_name);
                }
                NetworkEvent::GuildBuffListReceived { buff_ids } => {
                    tracing::trace!("🏰 Guild buff list received: {} buffs", buff_ids.len());
                }
                NetworkEvent::GuildTerritoryPageReceived { territories } => {
                    tracing::trace!("🏰 Guild territory page received: {} territories", territories.len());
                }
                NetworkEvent::GuildTerritoryPurchased { success } => {
                    tracing::trace!("🏰 Guild territory purchased: success={}", success);
                }
                NetworkEvent::NPCSellReceived => {
                    tracing::trace!("🏪 NPC sell received");
                }
                NetworkEvent::NPCRepairReceived { rate } => {
                    tracing::trace!("🔧 NPC repair received (rate={})", rate);
                }
                NetworkEvent::NPCSRepairReceived { rate } => {
                    tracing::trace!("🔧 NPC special repair received (rate={})", rate);
                }
                NetworkEvent::NPCRefineReceived { rate, refining } => {
                    tracing::trace!("🔨 NPC refine received (rate={}, refining={})", rate, refining);
                }
                NetworkEvent::NPCCheckRefineReceived => {
                    tracing::trace!("🔨 NPC check refine received");
                }
                NetworkEvent::NPCCollectRefineReceived { success } => {
                    tracing::trace!("🔨 NPC collect refine received (success={})", success);
                }
                NetworkEvent::NPCReplaceWedRingReceived { rate } => {
                    tracing::trace!("💍 NPC replace wedding ring received (rate={})", rate);
                }
                NetworkEvent::NPCStorageReceived => {
                    tracing::trace!("📦 NPC storage received");
                }
                NetworkEvent::NPCConsignReceived => {
                    tracing::trace!("🏪 NPC consign received");
                }
                NetworkEvent::SellItemReceived { unique_id, count, success } => {
                    tracing::trace!("💰 Sell item: id={}, count={}, success={}", unique_id, count, success);
                    if *success {
                        items_lost.push((*unique_id, *count as u32));
                    }
                }
                NetworkEvent::CraftItemReceived { unique_id, count, success } => {
                    tracing::trace!("🔨 Craft item: id={}, count={}, success={}", unique_id, count, success);
                }
                NetworkEvent::RepairItemReceived { unique_id } => {
                    tracing::trace!("🔧 Repair item: id={}", unique_id);
                }
                NetworkEvent::ItemRepairedEvent { unique_id, max_dura, current_dura } => {
                    tracing::trace!("🔧 Item repaired: id={}, max={}, cur={}", unique_id, max_dura, current_dura);
                    item_repairs.push((*unique_id, *current_dura, *max_dura));
                }
                NetworkEvent::DefaultNPCReceived { npc_id, message } => {
                    tracing::trace!("🗣️ NPC {} dialog: {}", npc_id, message);
                }
                NetworkEvent::NPCUpdated { npc_id } => {
                    tracing::trace!("🗣️ NPC updated: id={}", npc_id);
                }
                NetworkEvent::NPCImageUpdated { npc_id, image } => {
                    tracing::trace!("🖼️ NPC image updated: id={} image={}", npc_id, image);
                    npc_image_updates.push((*npc_id, *image));
                }
                NetworkEvent::NPCAwakeningReceived => {
                    tracing::trace!("🌟 NPC awakening received");
                }
                NetworkEvent::NPCDisassembleReceived => {
                    tracing::trace!("🔧 NPC disassemble received");
                }
                NetworkEvent::NPCDowngradeReceived => {
                    tracing::trace!("📉 NPC downgrade received");
                }
                NetworkEvent::NPCResetReceived => {
                    tracing::trace!("🔄 NPC reset received");
                }
                NetworkEvent::AwakeningNeedMaterialsReceived { item_id, materials } => {
                    tracing::trace!("🌟 Awakening need materials: item_id={} mats={}", item_id, materials.len());
                }
                NetworkEvent::AwakeningLockedItemReceived { unique_id, locked } => {
                    tracing::trace!("🌟 Awakening locked item: uid={} locked={}", unique_id, locked);
                }
                NetworkEvent::AwakeningReceived { result, remove_id } => {
                    tracing::trace!("🌟 Awakening received: result={} remove_id={}", result, remove_id);
                }
                NetworkEvent::NPCPearlGoodsReceived { rate, item_list } => {
                    tracing::trace!("🔮 NPC pearl goods received: rate={} items={}", rate, item_list.len());
                }
                NetworkEvent::NPCRequestInputReceived { npc_id, prompt, max_length } => {
                    tracing::trace!("🗣️ NPC {} requests input: {} (max={})", npc_id, prompt, max_length);
                }
                NetworkEvent::HeroCreateRequested { can_create_class } => {
                    tracing::trace!("🦸 Hero create requested: {} classes", can_create_class.len());
                }
                NetworkEvent::NewHeroCreated { result } => {
                    tracing::trace!("🦸 New hero result: {}", result);
                }
                NetworkEvent::HeroInfoReceived { hero_id } => {
                    tracing::trace!("🦸 Hero info received: hero_id={}", hero_id);
                }
                NetworkEvent::HeroSpawnStateUpdated { state } => {
                    tracing::trace!("🦸 Hero spawn state updated: {}", state);
                }
                NetworkEvent::HeroAutoPotUnlocked { unlocked } => {
                    tracing::trace!("🦸 Hero auto pot unlock: {}", unlocked);
                }
                NetworkEvent::HeroAutoPotSet { pot_type, value } => {
                    tracing::trace!("🦸 Hero auto pot set: type={} value={}", pot_type, value);
                }
                NetworkEvent::HeroAutoPotItemSet { slot, item_id } => {
                    tracing::trace!("🦸 Hero auto pot item set: slot={} item_id={}", slot, item_id);
                }
                NetworkEvent::HeroBehaviourSet { behaviour } => {
                    tracing::trace!("🦸 Hero behaviour set: behaviour={}", behaviour);
                    if let Some(e) = local_player_entity {
                        if let Ok(mut hero) = ctx.world.get::<&mut crate::components::HeroState>(e) {
                            hero.behaviour = match *behaviour {
                                0 => mir2_shared::enums::HeroBehaviour::Attack,
                                1 => mir2_shared::enums::HeroBehaviour::CounterAttack,
                                2 => mir2_shared::enums::HeroBehaviour::Follow,
                                _ => mir2_shared::enums::HeroBehaviour::Custom,
                            };
                        }
                    }
                }
                NetworkEvent::HeroManageReceived { heroes } => {
                    tracing::trace!("🦸 Hero manage received: {} heroes", heroes.len());
                }
                NetworkEvent::HeroChanged { success } => {
                    tracing::trace!("🦸 Hero changed: success={}", success);
                }
                NetworkEvent::HeroBaseStatsReceived { stats: _ } => {
                    tracing::trace!("🦸 Hero base stats received");
                }
                NetworkEvent::NewHeroInfoReceived { info: _ } => {
                    tracing::trace!("🦸 New hero info received");
                }
                NetworkEvent::MailReceived { mails } => {
                    tracing::trace!("📬 Mail received: {} mails", mails.len());
                }
                NetworkEvent::MailLockedItemReceived { unique_id, locked } => {
                    tracing::trace!("📬 Mail locked item: unique_id={} locked={}", unique_id, locked);
                }
                NetworkEvent::MailSendRequestReceived { mail_id } => {
                    tracing::trace!("📬 Mail send request: mail_id={}", mail_id);
                }
                NetworkEvent::MailSentEvent { result } => {
                    tracing::trace!("📬 Mail sent: result={}", result);
                }
                NetworkEvent::ParcelCollectedEvent { result } => {
                    tracing::trace!("📦 Parcel collected result: {}", result);
                }
                NetworkEvent::MailCostReceived { cost } => {
                    tracing::trace!("📬 Mail cost: {}", cost);
                }
                NetworkEvent::NPCConsignEvent => { tracing::trace!("🏪 NPC consign event"); }
                NetworkEvent::NPCMarketEvent2 { .. } => { tracing::trace!("🏪 NPC market event 2"); }
                NetworkEvent::NPCMarketPageEvent2 { .. } => { tracing::trace!("🏪 NPC market page event 2"); }
                NetworkEvent::ConsignItemEvent { .. } => { tracing::trace!("📦 Consign item event"); }
                NetworkEvent::MarketFailedEvent2 { reason } => { tracing::warn!("🏪 Market failed: {}", reason); }
                NetworkEvent::MarketSuccessEvent2 { .. } => { tracing::trace!("🏪 Market success"); }
                NetworkEvent::NewIntelligentCreatureReceived { creature_type: _ } => { tracing::trace!("🐾 New intelligent creature"); }
                NetworkEvent::IntelligentCreatureListUpdated { creatures } => { tracing::trace!("🐾 Creature list updated: {} creatures", creatures.len()); }
                NetworkEvent::IntelligentCreatureRenameEnabled { can_rename } => { tracing::trace!("🐾 Creature rename enabled: {}", can_rename); }
                NetworkEvent::IntelligentCreaturePickupReceived { enabled } => { tracing::trace!("🐾 Creature pickup: {}", enabled); }
                NetworkEvent::MarriageRequested2 { requester } => { tracing::trace!("💒 Marriage requested by {}", requester); }
                NetworkEvent::DivorceRequested2 { lover_name } => { tracing::trace!("💔 Divorce requested by {}", lover_name); }
                NetworkEvent::MentorRequested2 { mentor_name } => { tracing::trace!("🎓 Mentor requested by {}", mentor_name); }
                NetworkEvent::LoverUpdated { lover_name, date } => {
                    tracing::trace!("💒 Lover updated: {}", lover_name);
                    lover_updated = Some((lover_name.clone(), *date));
                }
                NetworkEvent::MentorUpdated { mentor_name, mentor_level, mentor_online } => {
                    tracing::trace!("🎓 Mentor updated: {}", mentor_name);
                    mentor_updated = Some((mentor_name.clone(), *mentor_level, *mentor_online));
                }
                NetworkEvent::RentalItemsReceived { items: _ } => { tracing::trace!("📦 Rental items received"); }
                NetworkEvent::ItemRentalRequested => { tracing::trace!("📦 Item rental requested"); }
                NetworkEvent::ItemRentalFeeReceived { fee } => { tracing::trace!("📦 Rental fee: {}", fee); }
                NetworkEvent::ItemRentalPeriodReceived { period } => { tracing::trace!("📦 Rental period: {}", period); }
                NetworkEvent::RentalItemDeposited { unique_id: _, success: _ } => { tracing::trace!("📦 Rental item deposited"); }
                NetworkEvent::RentalItemRetrieved { unique_id: _, success: _ } => { tracing::trace!("📦 Rental item retrieved"); }
                NetworkEvent::RentalItemUpdated { fee, period } => { tracing::trace!("📦 Rental item updated: fee={} period={}", fee, period); }
                NetworkEvent::ItemRentalCancelled { success } => { tracing::trace!("📦 Item rental cancelled: success={}", success); }
                NetworkEvent::ItemRentalLocked { locked } => { tracing::trace!("📦 Item rental locked: {}", locked); }
                NetworkEvent::ItemRentalPartnerLocked { locked } => { tracing::trace!("📦 Rental partner locked: {}", locked); }
                NetworkEvent::ItemRentalConfirmable { can_confirm } => { tracing::trace!("📦 Item rental confirmable: {}", can_confirm); }
                NetworkEvent::ItemRentalConfirmed { success } => { tracing::trace!("📦 Item rental confirmed: success={}", success); }
                NetworkEvent::FishingStatusUpdated { state, success } => {
                    tracing::trace!("🎣 Fishing status updated: {} success={}", state, success);
                    fishing_status = Some((*state, *success));
                }
                NetworkEvent::ReincarnationRequested => { tracing::trace!("🔄 Reincarnation requested"); }
                NetworkEvent::ReincarnationCancelled => { tracing::trace!("🔄 Reincarnation cancelled"); }
                NetworkEvent::RankingsReceived { rankings } => { tracing::trace!("🏆 Rankings received: {} entries", rankings.len()); }
                NetworkEvent::GameShopInfoReceived { items: _, credit: _, gold: _ } => {
                    tracing::trace!("🛒 Game shop info received");
                }
                NetworkEvent::GameShopStockReceived { item_index: _, stock: _ } => {
                    tracing::trace!("🛒 Game shop stock updated");
                }
                NetworkEvent::TimerSet { timer_id, seconds } => { tracing::trace!("⏱️ Timer {} set: {}s", timer_id, seconds); }
                NetworkEvent::TimerExpired { timer_id } => { tracing::trace!("⏱️ Timer {} expired", timer_id); }
                NetworkEvent::NoticeUpdated { notice } => { tracing::trace!("📢 Notice updated: {}", notice); }
                NetworkEvent::RollReceivedEvent { value } => { tracing::trace!("🎲 Roll received: {}", value); }
                NetworkEvent::CompassUpdated { location } => { tracing::trace!("🧭 Compass updated: {:?}", location); }
                NetworkEvent::BrowserOpened { url } => { tracing::trace!("🌐 Browser opened: {}", url); }
                NetworkEvent::DoorOpened { door_id, close } => {
                    tracing::trace!("🚪 Door {} {}", door_id, if *close { "closed" } else { "opened" });
                }
                NetworkEvent::TrapRockEntered { in_trap } => {
                    tracing::trace!("🪤 Trap rock: in_trap={}", in_trap);
                    trap_rock_state = Some(*in_trap);
                }
                NetworkEvent::BaseStatsReceived { stats } => {
                    tracing::trace!("📊 Base stats received: {} values", stats.len());
                    base_stats_received = Some(stats.clone());
                }
                NetworkEvent::InventoryResized { new_size } => {
                    if let Some(e) = local_player_entity {
                        if let Ok(mut inv) = ctx.world.get::<&mut crate::components::Inventory>(e) {
                            inv.items.resize(*new_size as usize, None);
                            inv.capacity = *new_size as usize;
                        }
                    }
                }
                NetworkEvent::StorageResized { new_size } => {
                    if let Some(e) = local_player_entity {
                        if let Ok(mut st) = ctx.world.get::<&mut crate::components::Storage>(e) {
                            st.items.resize(*new_size as usize, None);
                            st.capacity = *new_size as usize;
                        }
                    }
                }
                NetworkEvent::TransformUpdated { form } => {
                    tracing::trace!("🔄 Transform updated: {}", form);
                }
                NetworkEvent::MapEffectReceived { effect, location_x, location_y, value } => {
                    tracing::trace!("🌈 Map effect received: effect={} loc=({},{}) value={}", effect, location_x, location_y, value);
                    map_effects.push((*effect, *location_x, *location_y, *value));
                }
                NetworkEvent::ObserveAllowed { allowed } => {
                    tracing::trace!("👁️ Observe allowed: {}", allowed);
                    observe_allowed = Some(*allowed);
                }
                NetworkEvent::ObjectSpellReceived { object_id, location_x, location_y, spell } => {
                    tracing::trace!("✨ Object {} spell {:?} at ({},{})", object_id, spell, location_x, location_y);
                    object_spells.push((*object_id, *spell as u16));
                }
                NetworkEvent::ObjectDecoReceived { object_id, deco, remove } => {
                    tracing::trace!("🎭 Object {} deco={} remove={}", object_id, deco, remove);
                    deco_updates.push((*object_id, *deco, *remove));
                }
                NetworkEvent::ObjectSneakingReceived { object_id, sneaking } => {
                    tracing::trace!("🥷 Object {} sneaking={}", object_id, sneaking);
                    object_sneaking.push((*object_id, *sneaking));
                }
                NetworkEvent::ObjectLevelEffectsReceived { object_id, level_effects } => {
                    tracing::trace!("⭐ Object {} level effects={}", object_id, level_effects);
                    object_level_effects.push((*object_id, (*level_effects).into()));
                }
                NetworkEvent::BindingShotSet { enabled: _ } => {
                    tracing::trace!("🎯 Binding shot set");
                }
                NetworkEvent::OutputMessageReceived { message, message_type } => { tracing::trace!("💬 Message (type={}): {}", message_type, message); }
                NetworkEvent::UserStorageReceived { items } => {
                    tracing::trace!("📦 User storage received");
                    user_storage_received = Some(items.clone());
                }
                // 服务器下发的完整背包刷新（UserSlotsRefresh / UserInventory）
                NetworkEvent::UserInventoryReceived { items } => {
                    tracing::debug!("📦 User inventory received: {} items", items.len());
                    user_inventory_received = Some(items.clone());
                }
                // 服务器下发的完整装备刷新（UserSlotsRefresh / UserEquipment）
                NetworkEvent::UserEquipmentReceived { items } => {
                    tracing::debug!("📦 User equipment received: {} items", items.len());
                    user_equipment_received = Some(items.clone());
                }
                NetworkEvent::NewRecipeInfoReceived { recipe_id } => { tracing::trace!("📜 New recipe info received: id={}", recipe_id); }
                NetworkEvent::ChatItemStatsReceived { .. } => { tracing::trace!("💬 Chat item stats received"); }
                NetworkEvent::ConcentrationSet { object_id: _, enabled: _, interrupted: _ } => {
                    tracing::trace!("🎯 Concentration set");
                }
                NetworkEvent::ElementalSet { object_id, enabled, value, element, expire_time } => {
                    tracing::trace!("🔥 Elemental set: object={} enabled={} element={} value={} expire={}", object_id, enabled, element, value, expire_time);
                    elemental_updates.push((*object_id, *enabled, *value, *element, *expire_time));
                }
                NetworkEvent::DelayedExplosionRemoved { object_id } => {
                    tracing::trace!("💥 Delayed explosion removed: object_id={}", object_id);
                    delayed_explosions.push(*object_id);
                }

                // 客户端 → 服务器（不需要 apply，已在 handle_outbound_event 中发送）
                NetworkEvent::MagicKeySet => {}
                NetworkEvent::EquipItemRequest { .. } => {}
                NetworkEvent::RemoveItemRequest { .. } => {}
                NetworkEvent::RemoveSlotItemRequest { .. } => {}
                NetworkEvent::SplitItemRequest { .. } => {}
                NetworkEvent::MergeItemRequest { .. } => {}
                NetworkEvent::StoreItemRequest { .. } => {}
                NetworkEvent::TakeBackItemRequest { .. } => {}
                NetworkEvent::DropGoldRequest { .. } => {}
                NetworkEvent::EquipSlotItemRequest { .. } => {}
                NetworkEvent::CombineItemRequest { .. } => {}
                NetworkEvent::DropItemStackRequest { .. } => {}
                NetworkEvent::AddFriendRequest { .. } => {}
                NetworkEvent::RemoveFriendRequest { .. }=> {}
                NetworkEvent::RefreshFriendsRequest=> {}
                NetworkEvent::AddMemoRequest { .. }=> {}
                NetworkEvent::EditGuildMember { .. }=> {}
                NetworkEvent::EditGuildNotice { .. }=> {}
                NetworkEvent::GuildNameReturn { name } => {
                    tracing::trace!("🏛️ Guild name return: {}", name);
                }
                NetworkEvent::RequestGuildInfo=> {}
                NetworkEvent::GuildStorageGoldChange { .. }=> {}
                NetworkEvent::GuildStorageItemChangeRequest=> {}
                NetworkEvent::GuildWarReturn=> {}
                NetworkEvent::GuildBuffUpdate { .. }=> {}
                NetworkEvent::LogOutRequest=> {}
                NetworkEvent::HarvestRequest=> {}
                NetworkEvent::BuyItemBackRequest=> {}
                NetworkEvent::SRepairItemRequest { .. }=> {}
                NetworkEvent::CheckRefineRequest=> {}
                NetworkEvent::ReplaceWedRingRequest=> {}
                NetworkEvent::NPCConfirmInput { .. }=> {}
                NetworkEvent::CreateHeroRequest { .. }=> {}
                NetworkEvent::SetHeroAutoPotValue { .. }=> {}
                NetworkEvent::SetHeroAutoPotItem { .. }=> {}
                NetworkEvent::SetHeroBehaviourRequest { .. }=> {}
                NetworkEvent::ChangeHeroRequest { .. }=> {}
                NetworkEvent::SendMailRequest { .. }=> {}
                NetworkEvent::ReadMailRequest { .. }=> {}
                NetworkEvent::CollectParcelRequest { .. }=> {}
                NetworkEvent::DeleteMailRequest { .. }=> {}
                NetworkEvent::LockMailRequest { .. }=> {}
                NetworkEvent::ConsignItemRequest { .. }=> {}
                NetworkEvent::MarketSearchRequest { .. }=> {}
                NetworkEvent::MarketRefreshRequest=> {}
                NetworkEvent::MarketPageRequest { .. }=> {}
                NetworkEvent::MarketBuyRequest { .. }=> {}
                NetworkEvent::MarketGetBackRequest { .. }=> {}
                NetworkEvent::MarketSellNowRequest { .. }=> {}
                NetworkEvent::UpdateIntelligentCreatureRequest=> {}
                NetworkEvent::IntelligentCreaturePickupRequest=> {}
                NetworkEvent::RequestIntelligentCreatureUpdates=> {}
                NetworkEvent::MarriageRequestSend { .. }=> {}
                NetworkEvent::MarriageReply { .. }=> {}
                NetworkEvent::ChangeMarriageRequest=> {}
                NetworkEvent::DivorceRequestSend=> {}
                NetworkEvent::DivorceReply { .. }=> {}
                NetworkEvent::AddMentorRequest { .. }=> {}
                NetworkEvent::MentorReply { .. }=> {}
                NetworkEvent::AllowMentorRequest { .. }=> {}
                NetworkEvent::CancelMentorRequest=> {}
                NetworkEvent::GetRentedItemsRequest=> {}
                NetworkEvent::RentalItemDepositRequest { .. }=> {}
                NetworkEvent::RentalItemRetrieveRequest { .. }=> {}
                NetworkEvent::ItemRentalConfirm=> {}
                NetworkEvent::ItemRentalCancel=> {}
                NetworkEvent::CraftItemRequest { recipe_unique_id, count, ref slots } => {
                    tracing::trace!("🔨 Craft item request: unique_id={}, count={}, slots={:?}", recipe_unique_id, count, slots);
                }
                NetworkEvent::FishingCastRequest=> {}
                NetworkEvent::FishingAutocastToggle { .. }=> {}
                NetworkEvent::AcceptReincarnationRequest=> {}
                NetworkEvent::CancelReincarnationRequest=> {}
                NetworkEvent::GameShopBuyRequest { .. }=> {}
                NetworkEvent::ReportIssueRequest { .. }=> {}
                NetworkEvent::GetRankingRequest { .. }=> {}
                // Group events (deferred to ECS)
                NetworkEvent::GroupInvite { inviter } => {
                    tracing::trace!("👥 Group invite from {}", inviter);
                }
                NetworkEvent::GroupMemberAdded { name } => {
                    tracing::trace!("👥 Group member added: {}", name);
                    group_members_added.push(name.clone());
                }
                NetworkEvent::GroupMemberRemoved { name } => {
                    tracing::trace!("👥 Group member removed: {}", name);
                    group_members_removed.push(name.clone());
                }
                NetworkEvent::GroupDisbanded => {
                    tracing::trace!("👥 Group disbanded");
                    group_disbanded = true;
                }
                NetworkEvent::GroupMembersMapUpdated { .. } => {
                    tracing::trace!("👥 Group members map updated");
                }
                NetworkEvent::GroupMemberLocationUpdated { .. } => {
                    tracing::trace!("👥 Group member location updated");
                }
                NetworkEvent::GroupModeChanged { allow_group } => {
                    tracing::trace!("👥 Group mode changed: allow_group={}", allow_group);
                    group_allow_join = Some(*allow_group == 0);
                }
                // Chat / Social
                NetworkEvent::ChatMessage { sender, message, chat_type } => {
                    tracing::trace!("💬 Chat from {} (type={:?}): {}", sender, chat_type, message);
                }
                NetworkEvent::SystemMessage { message } => {
                    tracing::trace!("📢 System message: {}", message);
                }
                // Trade
                NetworkEvent::TradeRequested { requester } => {
                    tracing::trace!("🤝 Trade requested by {}", requester);
                }
                NetworkEvent::TradeStarted { partner } => {
                    tracing::trace!("🤝 Trade started with {}", partner);
                    trade_started = Some(partner.clone());
                }
                NetworkEvent::TradeCompleted => {
                    tracing::trace!("🤝 Trade completed");
                    trade_completed = true;
                }
                // Quest lifecycle
                NetworkEvent::QuestAccepted { quest_id } => {
                    tracing::trace!("📋 Quest accepted: {}", quest_id);
                }
                NetworkEvent::QuestCompleted { quest_id } => {
                    tracing::trace!("📋 Quest completed: {}", quest_id);
                }
                // NPC / UI
                NetworkEvent::NpcDialog { npc_id, dialog } => {
                    tracing::trace!("🗣️ NPC dialog: npc={} dialog_len={}", npc_id, dialog.len());
                }
                NetworkEvent::NPCGoods { items, .. } => {
                    tracing::trace!("🏪 NPC goods: {} items", items.len());
                }
                NetworkEvent::NPCCallRequest { npc_object_id, key } => {
                    tracing::trace!("📞 NPC call request: obj={} key={}", npc_object_id, key);
                }
                NetworkEvent::BuyItemRequest { item_index, count, .. } => {
                    tracing::trace!("🛒 Buy item request: idx={} count={}", item_index, count);
                }
                // Password / Login
                NetworkEvent::ChangePasswordSuccess => {
                    tracing::trace!("🔑 Change password success");
                }
                NetworkEvent::ChangePasswordFailed { reason } => {
                    tracing::trace!("🔑 Change password failed: {}", reason);
                }
                NetworkEvent::LogOutSuccess { .. } => {
                    tracing::trace!("👋 Logout success");
                }
                NetworkEvent::LogOutFailed => {
                    tracing::trace!("👋 Logout failed");
                }
                NetworkEvent::ReturnToLogin => {
                    tracing::trace!("🔙 Return to login");
                }
                // Rankings
                NetworkEvent::RankingsReceivedWithEntries { tab, entries } => {
                    tracing::trace!("🏆 Rankings tab={} entries={}", tab, entries.len());
                }
                NetworkEvent::OpenDoorRequest { .. }=> {}
                NetworkEvent::RequestMapInfoRequest=> {}
                NetworkEvent::TeleportToNPCRequest { .. }=> {}
                NetworkEvent::SearchMapRequest { .. }=> {}
                NetworkEvent::ObserveRequest { .. }=> {}

                // 未处理的数据包（调试用）
                NetworkEvent::UnhandledPacket { opcode } => {
                    tracing::warn!("⚠️ Unhandled packet opcode: {:04X}", opcode);
                }

                // 其他未显式匹配的事件（包括 Connected, Disconnected 等原始事件）
                _ => {}
            }
        }

        // NPC image updates (deferred to avoid E0502)
        for (npc_id, image) in npc_image_updates {
            if let Some(&e) = entity_index.get(&npc_id) {
                let library = crate::resources::LibraryName::Npcs(image as usize);
                {
                    if let Ok(mut spr) = ctx.world.get::<&mut crate::components::LibrarySprite>(e) {
                        spr.library = library;
                        spr.index = 0;
                        spr.frame = 0;
                    }
                }
                {
                    if let Ok(mut npc) = ctx.world.get::<&mut crate::components::NPC>(e) {
                        npc.npc_type = format!("npc:{}", image);
                    }
                }
            }
        }

        // Projectile effects (deferred to avoid E0502)
        for (spell, source, destination) in projectiles {
            let from_pos = NetworkApplySystem::object_position(&ctx.world, &entity_index, source);
            let to_pos = NetworkApplySystem::object_position(&ctx.world, &entity_index, destination);
            if let (Some(from), Some(to)) = (from_pos, to_pos) {
                if let Some(projectile_type) = NetworkApplySystem::spell_to_projectile_type(spell) {
                    ctx.events_mut().send_presentation(crate::event_bus::PresentationEvent::ProjectileEffect {
                        projectile_type,
                        from,
                        to,
                        speed: PROJECTILE_SPEED,
                    });
                }
            }
        }

        // 延迟爆炸移除：触发烟雾粒子
        for object_id in delayed_explosions {
            if let Some((x, y)) = NetworkApplySystem::object_position(&ctx.world, &entity_index, object_id) {
                ctx.events_mut().send_presentation(crate::event_bus::PresentationEvent::SpawnParticle {
                    particle_type: crate::event_bus::ParticleType::Smoke,
                    position: (x, y - PARTICLE_Y_OFFSET),
                    velocity: None,
                    duration: DEATH_SMOKE_DURATION,
                });
            }
        }

        // 地图特效：地雷爆炸等
        if !map_effects.is_empty() {
        for (effect, location_x, location_y, value) in map_effects {
            use mir2_shared::enums::SpellEffect;
            let px = location_x as f32 * 48.0;
            let py = location_y as f32 * 32.0;
            match effect {
                x if x == SpellEffect::Mine as u8 => {
                    ctx.events_mut().send_presentation(crate::event_bus::PresentationEvent::SpawnParticle {
                        particle_type: crate::event_bus::ParticleType::Smoke,
                        position: (px, py - PARTICLE_Y_OFFSET),
                        velocity: None,
                        duration: 1.5,
                    });
                    if value > 0 {
                        ctx.events_mut().send_presentation(crate::event_bus::PresentationEvent::FloatingText {
                            text: format!("-{}", value),
                            position: (px, py - 60.0),
                            color: macroquad::prelude::Color::from_rgba(255, 80, 80, 255),
                            font_size: 18.0,
                            duration: 1.0,
                        });
                    }
                }
                _ => {
                    tracing::trace!("🌈 Map effect: type={} pos=({},{}) value={}", effect, location_x, location_y, value);
                }
            }
        }
        }

        // 公会/社交/组队/交易状态：落地到 ECS 组件
        if let Some(e) = local_player_entity {
            if let Some(ref info) = guild_joined {
                NetworkApplySystem::upsert_component(ctx, e, info.clone());
            }
            if guild_left {
                let _ = ctx.world.remove_one::<crate::components::GuildInfo>(e);
            }
            if let Some(ref name) = guild_name_received {
                NetworkApplySystem::upsert_component(ctx, e, crate::components::GuildInfo {
                    name: name.clone(),
                    rank: String::new(),
                    ..Default::default()
                });
            }
            if let Some((ref name, date)) = lover_updated {
                NetworkApplySystem::upsert_component(ctx, e, crate::components::LoverState {
                    name: name.clone(),
                    date,
                });
            }
            if let Some((ref name, level, online)) = mentor_updated {
                NetworkApplySystem::upsert_component(ctx, e, crate::components::MentorState {
                    name: name.clone(),
                    level,
                    online,
                });
            }
            if let Some(allowed) = observe_allowed {
                NetworkApplySystem::upsert_component(ctx, e, crate::components::ObserveState {
                    allow_observe: allowed,
                    observer: false,
                });
            }
            // Group state
            if group_disbanded || !group_members_added.is_empty() || !group_members_removed.is_empty() || group_allow_join.is_some() {
                let mut has_group = false;
                {
                    if let Ok(mut gs) = ctx.world.get::<&mut crate::components::GroupState>(e) {
                        if group_disbanded {
                            gs.members.clear();
                        }
                        for name in &group_members_removed {
                            gs.members.retain(|m| m != name);
                        }
                        for name in &group_members_added {
                            gs.members.push(name.clone());
                        }
                        gs.members.sort();
                        gs.members.dedup();
                        if let Some(allow) = group_allow_join {
                            gs.allow_join = allow;
                        }
                        has_group = true;
                    }
                }
                if !has_group {
                    let _ = ctx.world.insert_one(
                        e,
                        crate::components::GroupState {
                            allow_join: group_allow_join.unwrap_or(true),
                            members: group_members_added,
                        },
                    );
                }
            }
            // Trade state
            if trade_completed {
                NetworkApplySystem::upsert_component(ctx, e, crate::components::TradeState::default());
            } else if let Some(ref partner) = trade_started {
                NetworkApplySystem::upsert_component(ctx, e, crate::components::TradeState {
                    active: true,
                    partner: partner.clone(),
                });
            }
        }

        // StartGame*：先"消费并落地到会话状态"，避免事件丢失（帧末 clear_frame）
        if let Some(packet) = start_game_delay {
            ctx.session.start_game_delay_ms = Some(packet.milliseconds);
        }
        if let Some(packet) = start_game_banned {
            ctx.session.start_game_banned = Some((packet.reason.clone(), packet.expiry_date));
        }
        if let Some(packet) = start_game {
            ctx.session.start_game_result = Some((packet.result, packet.resolution));
        }

        if let Some(packet) = user_info {
            NetworkApplySystem::apply_user_information(ctx, packet);
        }

        if let Some(packet) = map_changed {
            NetworkApplySystem::apply_map_changed(ctx, packet);
        }

        // Trap rock state applied to local player (skip if unchanged to avoid archetype moves)
        if let Some(trapped) = trap_rock_state {
            if let Some(e) = local_player_entity {
                let existing = ctx.world.get::<&crate::components::InTrapRock>(e)
                    .map(|t| t.trapped)
                    .ok();
                match existing {
                    Some(current) if current == trapped => {} // unchanged, skip
                    Some(_) => {
                        if let Ok(mut trap) = ctx.world.get::<&mut crate::components::InTrapRock>(e) {
                            trap.trapped = trapped;
                        }
                    }
                    None => {
                        let _ = ctx.world.insert_one(e, crate::components::InTrapRock { trapped });
                    }
                }
            }
        }

        // BaseStatsReceived
        if let (Some(e), Some(stats)) = (local_player_entity, base_stats_received) {
            if stats.len() >= 10 {
                let mut updated = false;
                {
                    if let Ok(mut cs) = ctx.world.get::<&mut crate::components::CombatStats>(e) {
                        cs.ac_min = stats[0];
                        cs.ac_max = stats[1];
                        cs.mac_min = stats[2];
                        cs.mac_max = stats[3];
                        cs.dc_min = stats[4];
                        cs.dc_max = stats[5];
                        cs.mc_min = stats[6];
                        cs.mc_max = stats[7];
                        cs.sc_min = stats[8];
                        cs.sc_max = stats[9];
                        cs.defense = cs.ac_max;
                        cs.magic_defense = cs.mac_max;
                        updated = true;
                    }
                }
                if !updated {
                    let _ = ctx.world.insert_one(e, crate::components::CombatStats {
                        ac_min: stats[0],
                        ac_max: stats[1],
                        mac_min: stats[2],
                        mac_max: stats[3],
                        dc_min: stats[4],
                        dc_max: stats[5],
                        mc_min: stats[6],
                        mc_max: stats[7],
                        sc_min: stats[8],
                        sc_max: stats[9],
                        defense: stats[1],
                        magic_defense: stats[3],
                        ..crate::components::CombatStats::default()
                    });
                }
            }
        }

        for packet in player_inspects {
            NetworkApplySystem::apply_player_inspect(ctx, packet);
        }

        // ===== server-driven: 播放声音（无位置，按全局/系统音效处理） =====
        // 说明：
        // - 服务器包只携带 sound_id；文件名由 SoundList.lst 映射。
        // - 同一帧可能收到多个 PlaySound；因此这里为每个声音创建一个临时实体。
        // - SoundSystem 播放完成后会自动 despawn 带 OneShotSoundEmitter 的实体，避免堆积。
        if !play_sounds.is_empty() {
            use crate::components::{OneShotSoundEmitter, SoundTrigger, SoundType};
            for id in play_sounds {
                let e = ctx.world.spawn((
                    OneShotSoundEmitter,
                    SoundTrigger::once(id.to_string(), SoundType::System),
                ));
                let _ = e;
            }
        }

        // 地面物品/金币落地
        if !ground_items.is_empty() {
            use crate::components::{GroundItem as GroundItemComp, NetworkSync, NetworkObjectType, Position};
            for packet in &ground_items {
                let existing = entity_index.get(&packet.object_id).copied();
                if existing.is_none() {
                    let entity = ctx.world.spawn((
                        NetworkSync::new(packet.object_id, NetworkObjectType::Item),
                        GroundItemComp {
                            object_id: packet.object_id,
                            item: packet.item.clone(),
                            gold_amount: 0,
                        },
                        Position { x: packet.location_x as f32, y: packet.location_y as f32 },
                    ));
                    tracing::trace!("📍 Ground item spawned: {} at ({}, {})", entity.id(), packet.location_x, packet.location_y);
                }
            }
        }
        if !ground_golds.is_empty() {
            use crate::components::{GroundItem as GroundItemComp, NetworkSync, NetworkObjectType, Position};
            for packet in &ground_golds {
                let existing = entity_index.get(&packet.object_id).copied();
                if existing.is_none() {
                    ctx.world.spawn((
                        NetworkSync::new(packet.object_id, NetworkObjectType::Item),
                        GroundItemComp {
                            object_id: packet.object_id,
                            item: mir2_shared::data::item::UserItem::default(),
                            gold_amount: packet.gold,
                        },
                        Position { x: packet.location_x as f32, y: packet.location_y as f32 },
                    ));
                }
            }
        }

        // server-driven objects
        for packet in object_players {
            NetworkApplySystem::apply_object_player(ctx, &entity_index, packet);
        }
        for packet in object_monsters {
            NetworkApplySystem::apply_object_monster(ctx, &entity_index, packet);
        }
        for packet in object_npcs {
            NetworkApplySystem::apply_object_npc(ctx, &entity_index, packet);
        }
        for hero_id in object_heroes {
            if let Some(&e) = entity_index.get(&hero_id) {
                if ctx.world.get::<&crate::components::Hero>(e).is_err() {
                    let _ = ctx.world.insert_one(e, crate::components::Hero);
                }
            }
        }
        for object_id in object_removes {
            NetworkApplySystem::apply_object_remove(ctx, &entity_index, object_id);
        }
        for p in object_turns {
            NetworkApplySystem::apply_object_turn(ctx, &entity_index, p);
        }
        for (_, p) in object_moves {
            match p {
                RemoteMovePacket::Walk(p) => NetworkApplySystem::apply_object_walk(ctx, &entity_index, p),
                RemoteMovePacket::Run(p) => NetworkApplySystem::apply_object_run(ctx, &entity_index, p),
            }
        }
        for (object_id, data) in object_attacks {
            NetworkApplySystem::apply_object_attack(ctx, &entity_index, object_id, data);
        }

        // ===== server-driven: local player state落地 =====
        // local_player_entity 已在 match 循环前计算

        // 坐骑更新：按 object_id 落地（本地玩家没有 NetworkSync，因此需要用 PlayerData.object_id 匹配）
        if !mount_updates.is_empty() {
            use crate::components::{MountState, MountStatus, PlayerData, SoundTrigger, SoundType};

            for (object_id, mount_type, riding_mount) in mount_updates {
                let target_entity =
                    if let Some(&e) = entity_index.get(&object_id) {
                        Some(e)
                    } else {
                        // local player path: match by PlayerData.object_id
                        ctx.world.iter()
                            .find_map(|e| {
                                if e.get::<&crate::components::LocalPlayer>().is_some() {
                                    if let Some(pd) = e.get::<&PlayerData>() {
                                        if pd.object_id == object_id {
                                            return Some(e.entity());
                                        }
                                    }
                                }
                                None
                            })
                    };

                let Some(e) = target_entity else { continue; };

                // capture previous mount status to decide whether to play mount sound
                let prev = ctx.world.get::<&MountStatus>(e).ok().map(|r| *r);

                // upsert MountStatus
                let mut inserted = false;
                {
                    if let Ok(mut ms) = ctx.world.get::<&mut MountStatus>(e) {
                        ms.mount_type = mount_type;
                        ms.riding_mount = riding_mount;
                        inserted = true;
                    }
                }
                if !inserted {
                    let _ = ctx.world.insert_one(
                        e,
                        MountStatus {
                            mount_type,
                            riding_mount,
                        },
                    );
                }

                // best-effort MountState sync for rendering/animation switching
                if let Ok(mut m) = ctx.world.get::<&mut MountState>(e) {
                    m.mount_index = if riding_mount && mount_type >= 0 {
                        Some(mount_type as usize)
                    } else {
                        None
                    };
                }

                // PlayMountSound() 对齐：MountUpdate 时播放一次
                let changed = prev.map(|p| (p.mount_type, p.riding_mount))
                    != Some((mount_type, riding_mount));
                if changed && mount_type >= 0 {
                    let sound_id: Option<i32> = if riding_mount {
                        if mount_type < 7 { Some(10218) } else if mount_type < 12 { Some(10188) } else { None }
                    } else if mount_type < 7 { Some(10219) } else if mount_type < 12 { Some(10189) } else { None };
                    if let Some(id) = sound_id {
                        let _ = ctx.world.insert_one(
                            e,
                            SoundTrigger::once(id.to_string(), SoundType::CharacterAction),
                        );
                    }
                }
            }
        }

        if let Some(e) = local_player_entity {
            // 魔法同步（来自服务器）
            if let Some((cur, max)) = player_mana_changed {
                let new_current = (cur as i32).max(0);
                let new_max = (max as i32).max(0);
                let mut inserted = false;
                {
                    if let Ok(mut mp) = ctx.world.get::<&mut crate::components::Mana>(e) {
                        mp.current = if new_max > 0 { new_current.clamp(0, new_max) } else { new_current };
                        if new_max != 0 {
                            mp.max = new_max;
                        } else if mp.max < mp.current {
                            mp.max = mp.current;
                        }
                        inserted = true;
                    }
                }
                if !inserted {
                    let effective_max = if new_max != 0 { new_max } else { new_current };
                    let _ = ctx.world.insert_one(
                        e,
                        crate::components::Mana {
                            current: if effective_max > 0 { new_current.clamp(0, effective_max) } else { new_current },
                            max: effective_max,
                        },
                    );
                }
            }

            // 位置校正（格子坐标 -> 世界像素）
            // 仅在"服务器权威移动"开启时落地；否则会与本地 MovementSystem 双驱动，导致抖动/乱跳。
            if let Some((gx, gy)) = player_location_changed {
                let should_apply = if ctx.session.server_authoritative_movement {
                    true
                } else {
                    // 非 server-authoritative movement 时，只允许"死亡/复活"类修正：
                    // - Mock/真服在同步移动意图时，回包可能滞后于本地连续像素移动。
                    // - 若按"偏差足够大"触发纠偏，会出现自动寻路时被拉回起点（rubber-banding）。
                    // 因此：活着时一律不应用 PlayerLocationChanged 的位置校正。
                    //
                    // 注意：此前这里允许 "large_jump"（大跨度）时纠偏，期望覆盖"传送/回城"。
                    // 但当前 world<->grid 的换算在连续像素移动场景下可能产生较大误差，
                    // 进而把正常走两步误判为大跳变，导致被拉回出生点。
                    // 传送/切图等应优先通过 MapChanged / UserInformation 落地。
                    let dead = ctx
                        .world
                        .get::<&crate::components::Health>(e)
                        .ok()
                        .map(|hp| hp.current <= 0)
                        .unwrap_or(false);

                    dead
                };

                if should_apply {
                    // 强制对齐（传送/复活回城）
                    let (wx, wy) = crate::coord::Coord::grid_to_world_center(gx, gy);
                    if let Ok(mut pos) = ctx.world.get::<&mut crate::components::Position>(e) {
                        pos.x = wx;
                        pos.y = wy;
                    }

                    // 重置移动/攻击状态，避免"复活还在追砍/寻路"
                    NetworkApplySystem::stop_player_actions(&mut ctx.world, e);
                } else if ctx.session.server_authoritative_movement {
                    // 兼容旧逻辑：小偏差不纠正
                    if let Ok(mut pos) = ctx.world.get::<&mut crate::components::Position>(e) {
                        let (wx, wy) = crate::coord::Coord::grid_to_world_center(gx, gy);
                        let dx = pos.x - wx;
                        let dy = pos.y - wy;

                        // 半格阈值：超过就强制对齐
                        let threshold_px = 24.0_f32;
                        if dx * dx + dy * dy > threshold_px * threshold_px {
                            pos.x = wx;
                            pos.y = wy;
                        }
                    }
                }
            }

            // 金币变化（正/负）
            if gold_delta_sum != 0 {
                let apply_delta = |gold: &mut u32, delta: i32| {
                    if delta >= 0 {
                        *gold = gold.saturating_add(delta as u32);
                    } else {
                        *gold = gold.saturating_sub((-delta) as u32);
                    }
                };

                // Currency
                let has_currency = ctx.world.get::<&crate::components::Currency>(e).is_ok();
                if has_currency {
                    if let Ok(mut cur) = ctx.world.get::<&mut crate::components::Currency>(e) {
                        apply_delta(&mut cur.gold, gold_delta_sum);
                    }
                } else {
                    let mut gold: u32 = 0;
                    apply_delta(&mut gold, gold_delta_sum);
                    let _ = ctx.world.insert_one(e, crate::components::Currency { gold, credit: 0 });
                }
            }

            // 元宝/点券变化
            if credit_delta_sum != 0 {
                let mut cur = match ctx.world.get::<&crate::components::Currency>(e) {
                    Ok(c) => *c,
                    Err(_) => crate::components::Currency::new(),
                };
                cur.apply_credit_delta(credit_delta_sum);
                NetworkApplySystem::upsert_component(ctx, e, cur);
            }

            // 物品移动（格子内移动）
            if !items_moved.is_empty() {
                if let Ok(mut inv) = ctx.world.get::<&mut crate::components::Inventory>(e) {
                    for (from, to) in items_moved {
                        let from = from as usize;
                        let to = to as usize;
                        if from >= inv.items.len() || to >= inv.items.len() {
                            continue;
                        }
                        inv.items.swap(from, to);
                    }
                }
            }

            // 删除物品（按 unique_id 查找，支持堆叠数量）
            if !items_lost.is_empty() || !items_dropped.is_empty() {
                if let Ok(mut inv) = ctx.world.get::<&mut crate::components::Inventory>(e) {
                    for (uid, count) in items_lost.iter().chain(items_dropped.iter()) {
                        inv.remove_by_unique_id(*uid, *count);
                    }
                }
                if let Ok(mut eq) = ctx.world.get::<&mut crate::components::Equipment>(e) {
                    for (uid, _) in items_lost.iter().chain(items_dropped.iter()) {
                        eq.remove_by_id(*uid);
                    }
                }
            }

            // 获得物品（找空格塞入）
            if !items_gained.is_empty() {
                // 若背包还不存在：创建一个默认容量背包，避免 Mock 购买时无处可放。
                if ctx.world.get::<&crate::components::Inventory>(e).is_err() {
                    let _ = ctx.world.insert_one(e, crate::components::Inventory::default());
                }
                if let Ok(mut inv) = ctx.world.get::<&mut crate::components::Inventory>(e) {
                    for item in items_gained {
                        let _ = inv.add_item(item);
                    }
                }
            }

            // 装备物品（从背包移动到装备栏，支持替换旧装备）
            if !items_equipped.is_empty() {
                if let Ok(mut inv) = ctx.world.get::<&mut crate::components::Inventory>(e) {
                    if let Ok(mut eq) = ctx.world.get::<&mut crate::components::Equipment>(e) {
                        for (unique_id, slot) in items_equipped {
                            if let Some(item) = inv.take_by_unique_id(unique_id) {
                                if let Some(old_item) = eq.equip(slot, item) {
                                    let _ = inv.add_item(old_item);
                                }
                            }
                        }
                    }
                }
            }

            // 物品存入仓库（背包 -> 仓库）
            if !items_stored.is_empty() {
                if let Ok(mut inv) = ctx.world.get::<&mut crate::components::Inventory>(e) {
                    if let Ok(mut st) = ctx.world.get::<&mut crate::components::Storage>(e) {
                        for (from, to) in items_stored {
                            NetworkApplySystem::transfer_slot(
                                &mut inv.items,
                                &mut st.items,
                                from.max(0) as usize,
                                to.max(0) as usize,
                            );
                        }
                    }
                }
            }

            // 物品从仓库取回（仓库 -> 背包）
            if !items_taken_back.is_empty() {
                if let Ok(mut st) = ctx.world.get::<&mut crate::components::Storage>(e) {
                    if let Ok(mut inv) = ctx.world.get::<&mut crate::components::Inventory>(e) {
                        for (from, to) in items_taken_back {
                            NetworkApplySystem::transfer_slot(
                                &mut st.items,
                                &mut inv.items,
                                from.max(0) as usize,
                                to.max(0) as usize,
                            );
                        }
                    }
                }
            }
        }

        // 服务器下发的完整背包刷新（覆盖当前背包）
        if let (Some(e), Some(items)) = (local_player_entity, user_inventory_received) {
            let capacity = items.len().max(1);
            let mut inserted = false;
            {
                if let Ok(mut inv) = ctx.world.get::<&mut crate::components::Inventory>(e) {
                    inv.items = items.iter().cloned().map(Some).collect();
                    inv.capacity = capacity;
                    inserted = true;
                }
            }
            if !inserted {
                let mut inv = crate::components::Inventory::new(capacity);
                inv.items = items.into_iter().map(Some).collect();
                let _ = ctx.world.insert_one(e, inv);
            }
        }

        // 服务器下发的完整装备刷新（覆盖当前装备）
        if let (Some(e), Some(items)) = (local_player_entity, user_equipment_received) {
            let mut eq = crate::components::Equipment::new();
            for item in items {
                if let Some(info) = item.info.as_ref() {
                    if let Some(slot) = eq.get_slot_for_type(info.item_type) {
                        eq.equip(slot, item);
                    }
                }
            }
            let _ = ctx.world.insert_one(e, eq);
        }

        // 服务器下发的完整仓库刷新（覆盖当前仓库）
        if let (Some(e), Some(items)) = (local_player_entity, user_storage_received) {
            let capacity = items.len().max(1);
            let mut inserted = false;
            {
                if let Ok(mut st) = ctx.world.get::<&mut crate::components::Storage>(e) {
                    st.items = items.iter().cloned().map(Some).collect();
                    st.capacity = capacity;
                    inserted = true;
                }
            }
            if !inserted {
                let mut st = crate::components::Storage::new(capacity);
                st.items = items.into_iter().map(Some).collect();
                let _ = ctx.world.insert_one(e, st);
            }
        }

        // ===== server-driven: combat落地到对象（怪物/其他对象） =====
        // DamageIndicator：很多服务端把实际伤害放在这个包里。
        // 这里复用 ObjectStruck 的可见闭环（扣血 + 飘字），attacker_id 用 0 占位。
        if !damage_indicators.is_empty() {
            for (object_id, damage, _damage_type) in damage_indicators {
                if damage != 0 {
                    object_struck.push((object_id, 0, damage));
                }
            }
        }

        // ObjectHealthPercent：驱动远程对象血条显示。
        // 由于协议只给 percent，这里使用 0..100 的虚拟血池（max=100）。
        for (object_id, percent, _expire) in object_health_percents {
            let Some(&target) = entity_index.get(&object_id) else {
                continue;
            };
            let p = (percent as i32).clamp(0, 100);
            let mut inserted = false;
            {
                if let Ok(mut hp) = ctx.world.get::<&mut crate::components::Health>(target) {
                    // 若之前 max 不是虚拟血池，则尽量按既有 max 计算 current
                    if hp.max > 0 && hp.max != 100 {
                        hp.current = ((hp.max as i64) * (p as i64) / 100) as i32;
                    } else {
                        hp.max = 100;
                        hp.current = p;
                    }
                    inserted = true;
                }
            }
            if !inserted {
                let _ = ctx.world.insert_one(
                    target,
                    crate::components::Health {
                        current: p,
                        max: 100,
                    },
                );
            }
        }

        // ObjectStruck: 最小可见闭环：扣血 + 飘字（用于 mock / 真实服都可用）。
        for (object_id, attacker_id, damage) in object_struck {
            let Some(&target) = entity_index.get(&object_id) else {
                continue;
            };

            // ===== 音效：受击（对齐原版） =====
            // - 怪物：BaseSound + 2 (Flinch)
            // - 玩家：PlayStruckSound（骑乘/护甲 add/按攻击者武器）
            {
                use crate::components::{Monster, MountStatus, PlayerAppearance, SoundTrigger, SoundType};

                // helper: attacker weapon shape (unknown => -1)
                let struck_weapon: i16 = entity_index.get(&attacker_id)
                    .and_then(|&att| ctx.world.get::<&PlayerAppearance>(att).ok().map(|a| a.weapon))
                    .unwrap_or(-1);

                let monster_type = ctx.world.get::<&Monster>(target).ok().map(|m| m.monster_type);
                if let Some(monster_type) = monster_type {
                    let base = monster_type * 10;
                    let _ = ctx.world.insert_one(
                        target,
                        SoundTrigger::once((base + 2).to_string(), SoundType::CharacterAction),
                    );

                    // 动画：怪物受击动作
                    NetworkApplySystem::set_monster_anim(&ctx.world, &entity_index, object_id, Some(crate::components::MirAction::Struck), None);
                } else if let Some((victim_class, victim_armour)) = ctx
                    .world
                    .get::<&PlayerAppearance>(target)
                    .ok()
                    .map(|a| (a.class, a.armour))
                {
                    // riding mount struck sounds
                    let mount_status = ctx.world.get::<&MountStatus>(target).ok().map(|r| *r);
                    let is_riding = mount_status.map(|ms| ms.riding_mount).unwrap_or(false);
                    let mount_type = mount_status.map(|ms| ms.mount_type).unwrap_or(-1);

                    let mut sound_id: Option<i32> = None;
                    if is_riding {
                        let mut rng = rand::rng();
                        if mount_type < 7 {
                            sound_id = Some(rng.random_range(10179..=10180));
                        } else if mount_type < 12 {
                            sound_id = Some(10193);
                        }
                    } else {
                        let mut add: i32 = 0;
                        if victim_class != crate::components::MirClass::Assassin {
                            match victim_armour {
                                3 | 6 | 9 => add = 10,
                                _ => {}
                            }
                        }

                        let base = match struck_weapon {
                            // sword-ish groups
                            0 | 23 | 1 | 12 | 28 | 40
                            | 2 | 8 | 11 | 15 | 18 | 20 | 25 | 31 | 33 | 34 | 37 | 41
                            | 3 | 5 | 7 | 9 | 13 | 19 | 24 | 26 | 29 | 32 | 35 => 10070,
                            // axe
                            4 | 14 | 16 | 38 => 10071,
                            // long stick / club
                            6 | 10 | 17 | 22 | 27 | 30 | 36 | 39 | 21 => 10072,
                            // fist / unknown
                            _ => 10073,
                        };
                        sound_id = Some(base + add);
                    }

                    if let Some(id) = sound_id {
                        let _ = ctx.world.insert_one(
                            target,
                            SoundTrigger::once(id.to_string(), SoundType::CharacterAction),
                        );
                    }

                    // 受击动画：对玩家（本地/远程）切换到 Struck 动作。
                    // 复用 AttackState 作为 one-shot 计时器：AnimationSystem 会按
                    // Struck 帧表时长播完后自动回到 Stand。
                    // - 死亡中的实体不切（避免覆盖 Die→Dead 衔接）
                    // - 骑乘态下 animation_system 会自动把 Struck 映射到 MountStruck 帧表
                    if ctx.world.get::<&crate::components::DeathState>(target).is_err() {
                        if let Ok(mut p) = ctx.world.get::<&mut crate::components::Player>(target) {
                            p.action = crate::components::PlayerAction::Struck;
                        }
                        let _ = ctx.world.insert_one(
                            target,
                            crate::components::AttackState {
                                start_time: std::time::Instant::now(),
                                attack_type: crate::components::PlayerAction::Struck,
                                server_attack_type: 0,
                            },
                        );
                    }
                }
            }

            // 更新 Health（若无则创建一个默认血池，便于测试可见）
            let (spawn_x, spawn_y) = {
                if let Ok(pos) = ctx.world.get::<&crate::components::Position>(target) {
                    (pos.x, pos.y)
                } else {
                    (0.0, 0.0)
                }
            };

            let mut had_hp = false;
            let mut killed_player_to_zero = false;
            let mut hp_after_damage: i32 = 0;
            {
                if let Ok(mut hp) = ctx.world.get::<&mut crate::components::Health>(target) {
                    hp.take_damage(damage);
                    hp_after_damage = hp.current;
                    // 先记录是否打到 0，避免在持有 RefMut 时再改 world（会触发借用冲突）
                    killed_player_to_zero = hp.current <= 0;
                    had_hp = true;
                }
            }

            // 血条平滑过渡：更新 HealthBarAnim
            if had_hp {
                NetworkApplySystem::upsert_component(ctx, target, crate::components::HealthBarAnim { displayed: hp_after_damage as f32 });
            }

            // 玩家被打到 0：触发死亡动画（不依赖 ObjectDied 是否及时到达）
            if killed_player_to_zero
                && ctx.world.get::<&crate::components::Player>(target).is_ok()
                && ctx.world.get::<&crate::components::DeathState>(target).is_err()
            {
                let _ = ctx.world.insert_one(target, crate::components::DeathState::new());
                let _ = ctx.world.remove_one::<crate::components::AttackState>(target);
            }

            if !had_hp {
                let mut hp = crate::components::Health { current: 100, max: 100 };
                hp.take_damage(damage);
                let _ = ctx.world.insert_one(target, hp);

                // 同上：若默认血池也被打到 0，补齐死亡动画状态
                if ctx.world.get::<&crate::components::Player>(target).is_ok()
                    && ctx.world.get::<&crate::components::DeathState>(target).is_err()
                {
                    // 这里用本地 hp 变量即可，避免 get() 借用与 insert_one 冲突
                    if hp.current <= 0 {
                    let _ = ctx.world.insert_one(target, crate::components::DeathState::new());
                    let _ = ctx.world.remove_one::<crate::components::AttackState>(target);
                    }
                }
            }

            // 飘字实体（独立 entity，避免污染目标组件）
            let now = current_time_secs();
            let text = format!("-{}", damage);
            ctx.world.spawn((
                crate::components::Position::new(spawn_x, spawn_y - 72.0),
                crate::components::FloatingText {
                    text,
                    start_time: now,
                    duration: 1.0,
                    rise_speed: 40.0,
                    color: Some(macroquad::prelude::RED),
                },
            ));
        }

        // ObjectDied: 标记血量为 0（ObjectRemove 可能会在后续把 entity 删掉）
        for object_id in object_died {
            if let Some(&target) = entity_index.get(&object_id) {
                if let Ok(mut hp) = ctx.world.get::<&mut crate::components::Health>(target) {
                    hp.current = 0;
                }

                NetworkApplySystem::set_visibility_dead(ctx, target, true);

                // 玩家死亡动画：挂上/重置 DeathState（Die → Dead），并停止攻击动画
                if ctx.world.get::<&crate::components::Player>(target).is_ok() {
                    NetworkApplySystem::upsert_component(ctx, target, crate::components::DeathState::new());
                    let _ = ctx.world.remove_one::<crate::components::AttackState>(target);
                }

                // 本地玩家死亡：立刻停止移动/攻击输入，避免"死了还在走/追砍"。
                if ctx.world.get::<&crate::components::LocalPlayer>(target).is_ok() {
                    NetworkApplySystem::stop_player_actions(&mut ctx.world, target);
                }

                // ===== 音效：死亡 =====
                {
                    use crate::components::{MirGender, Monster, PlayerAppearance, SoundTrigger, SoundType};
                    let monster_type = ctx.world.get::<&Monster>(target).ok().map(|m| m.monster_type);
                    if let Some(monster_type) = monster_type {
                        let base = monster_type * 10;
                        let _ = ctx.world.insert_one(
                            target,
                            SoundTrigger::once((base + 3).to_string(), SoundType::CharacterAction),
                        );

                        // 动画：怪物死亡动作
                        NetworkApplySystem::set_monster_anim(&ctx.world, &entity_index, object_id, Some(crate::components::MirAction::Die), None);
                    } else if let Some(gender) = ctx
                        .world
                        .get::<&PlayerAppearance>(target)
                        .ok()
                        .map(|a| a.gender)
                    {
                        let id = match gender {
                            MirGender::Male => 10144,
                            _ => 10145,
                        };
                        let _ = ctx.world.insert_one(
                            target,
                            SoundTrigger::once(id.to_string(), SoundType::CharacterAction),
                        );
                    }
                }

                // 死亡粒子效果
                if let Some((x, y)) = NetworkApplySystem::entity_position(ctx, target) {
                    ctx.events_mut().send_presentation(crate::event_bus::PresentationEvent::SpawnParticle {
                        particle_type: crate::event_bus::ParticleType::Smoke,
                        position: (x, y - PARTICLE_Y_OFFSET),
                        velocity: None,
                        duration: DEATH_SMOKE_DURATION,
                    });
                }
            }
        }

        // PlayerDied：本地玩家死亡落地
        if let Some((x, y, direction)) = player_died {
            if let Some(e) = local_player_entity {
                // 血量归零
                let mut inserted = false;
                {
                    if let Ok(mut hp) = ctx.world.get::<&mut crate::components::Health>(e) {
                        hp.current = 0;
                        inserted = true;
                    }
                }
                if !inserted {
                    let _ = ctx.world.insert_one(
                        e,
                        crate::components::Health {
                            current: 0,
                            max: 100,
                        },
                    );
                }

                // 添加死亡状态
                let _ = ctx.world.insert_one(e, crate::components::DeathState::new());

                // 位置落地到死亡点
                let (wx, wy) = crate::coord::Coord::grid_to_world_center(x as i32, y as i32);
                if let Ok(mut pos) = ctx.world.get::<&mut crate::components::Position>(e) {
                    pos.x = wx;
                    pos.y = wy;
                }

                // 动画：死亡（本地玩家用 PlayerAction，MonsterAnimState 仅对怪物/NPC/远程玩家有效）
                let dir = mir2_shared::MirDirection::try_from(direction).ok();
                if let Some(oid) = local_player_object_id {
                    NetworkApplySystem::set_monster_anim(&ctx.world, &entity_index, oid, Some(crate::components::MirAction::Dead), dir);
                }

                // 停止移动/攻击输入
                NetworkApplySystem::stop_player_actions(&mut ctx.world, e);

                // 本地玩家死亡粒子
                ctx.events_mut().send_presentation(crate::event_bus::PresentationEvent::SpawnParticle {
                    particle_type: crate::event_bus::ParticleType::Smoke,
                    position: (wx, wy - PARTICLE_Y_OFFSET),
                    velocity: None,
                    duration: DEATH_SMOKE_DURATION,
                });
            }
        }

        // ===== final reconcile: local player HP is server-authoritative =====
        // 说明：本帧内可能同时到达 ObjectStruck/ObjectDied 与 HealthChanged。
        // 为避免后续处理把本地玩家血量又改回 0（导致死亡 UI 卡住），这里在帧末再用 HealthChanged 覆盖一次。
        if let (Some(e), Some((cur, max))) = (local_player_entity, player_health_changed) {
            let mut inserted = false;
            {
                if let Ok(mut hp) = ctx.world.get::<&mut crate::components::Health>(e) {
                    let new_current = (cur as i32).max(0);
                    let new_max = (max as i32).max(0);
                    hp.current = new_current.clamp(0, new_max);
                    hp.max = new_max;
                    inserted = true;
                }
            }
            if !inserted {
                let new_current = (cur as i32).max(0);
                let new_max = (max as i32).max(0);
                let _ = ctx.world.insert_one(
                    e,
                    crate::components::Health {
                        current: new_current.clamp(0, new_max),
                        max: new_max,
                    },
                );
            }

            // 复活/回血：清掉死亡动画状态
            if (cur as i32) > 0 {
                let _ = ctx.world.remove_one::<crate::components::DeathState>(e);
            }
        }

        // ===== 新协议落地逻辑 =====

        // Buff 添加
        let now_ms = (current_time_secs() * 1000.0) as i64;
        for (object_id, buff_id, expire_time, infinite, paused) in buff_adds {
            tracing::trace!("🔮 Buff added: object_id={}, buff_id={}", object_id, buff_id);
            if let Some(&e) = entity_index.get(&object_id) {
                let server_buff = mir2_shared::enums::BuffType::try_from(buff_id as u8).ok();
                let combat_buff = server_buff.and_then(NetworkApplySystem::map_server_buff);
                if let Some(cb) = combat_buff {
                    let remaining_ms = if infinite {
                        u64::MAX
                    } else {
                        (crate::utils::dotnet_ticks_to_unix_ms(expire_time).max(0) as u64)
                            .saturating_sub(now_ms as u64)
                    };
                    let mut buff = crate::components::Buff::new(cb, buff_id);
                    buff.remaining_duration = remaining_ms;
                    buff.paused = paused;
                    NetworkApplySystem::with_component::<crate::components::combat::BuffList>(ctx, e, |bl| {
                        bl.add_buff(buff.clone());
                    });
                }
            }
        }

        // Buff 移除
        for (object_id, buff_id) in buff_removes {
            tracing::trace!("🔮 Buff removed: object_id={}, buff_id={}", object_id, buff_id);
            if let Some(&e) = entity_index.get(&object_id) {
                if let Ok(mut buff_list) = ctx.world.get::<&mut crate::components::BuffList>(e) {
                    buff_list.remove_buff(buff_id);
                }
            }
        }

        // Buff 暂停/恢复
        for (object_id, buff_id, paused) in buff_pauses {
            tracing::trace!("🔮 Buff {} object_id={}, buff_id={}", if paused { "paused" } else { "resumed" }, object_id, buff_id);
            if let Some(&e) = entity_index.get(&object_id) {
                if let Ok(mut buff_list) = ctx.world.get::<&mut crate::components::BuffList>(e) {
                    buff_list.set_buff_paused(buff_id, paused);
                }
            }
        }

        // 攻击模式/宠物模式变化
        for (entity, mode) in attack_mode_changes {
            tracing::debug!("⚔️ Attack mode changed: {}", mode);
            NetworkApplySystem::upsert_component(ctx, entity, crate::components::AttackMode::new(mode));
        }
        for (entity, mode) in pet_mode_changes {
            tracing::debug!("🐾 Pet mode changed: {}", mode);
            NetworkApplySystem::upsert_component(ctx, entity, crate::components::PetMode::new(mode));
        }

        // 隐身/显形
        for object_id in hidden_objects {
            tracing::trace!("👻 Object hidden: {}", object_id);
            if let Some(&e) = entity_index.get(&object_id) {
                if let Ok(mut vis) = ctx.world.get::<&mut crate::components::Visibility>(e) {
                    vis.hidden = true;
                }
            }
        }
        for object_id in shown_objects {
            tracing::trace!("👁 Object shown: {}", object_id);
            if let Some(&e) = entity_index.get(&object_id) {
                if let Ok(mut vis) = ctx.world.get::<&mut crate::components::Visibility>(e) {
                    vis.hidden = false;
                }
            }
        }

        // Backstep / Dash / Pushed / DashAttacked 位置落地（与 ObjectWalk/ObjectRun 保持一致）
        for (object_id, x, y) in backsteps {
            NetworkApplySystem::apply_object_move(ctx, &entity_index, object_id, x, y);
        }
        for (object_id, x, y) in dashes {
            NetworkApplySystem::apply_object_move(ctx, &entity_index, object_id, x, y);
        }
        for (object_id, x, y) in pushed {
            NetworkApplySystem::apply_object_move(ctx, &entity_index, object_id, x, y);
        }
        for (object_id, x, y) in dash_attacked {
            NetworkApplySystem::apply_object_move(ctx, &entity_index, object_id, x, y);
        }
        for (object_id, x, y) in attack_moved {
            NetworkApplySystem::apply_object_move(ctx, &entity_index, object_id, x, y);
        }

        // Dash 失败：怪物/NPC 播放 DashFail 动画
        for object_id in dash_failed {
            NetworkApplySystem::set_monster_anim(&ctx.world, &entity_index, object_id, Some(crate::components::MirAction::DashFail), None);
        }

        // 坐下：怪物/NPC 更新动画状态（玩家暂不支持 SitDown PlayerAction）
        for object_id in sat_down {
            NetworkApplySystem::set_monster_anim(&ctx.world, &entity_index, object_id, Some(crate::components::MirAction::SitDown), None);
        }

        // 采集：位置落地 + 更新怪物/NPC 动画
        for (object_id, x, y, dir) in harvested {
            NetworkApplySystem::apply_object_move(ctx, &entity_index, object_id, x, y);
            let direction = mir2_shared::MirDirection::try_from(dir).ok();
            NetworkApplySystem::set_monster_anim(&ctx.world, &entity_index, object_id, Some(crate::components::MirAction::Harvest), direction);
        }

        // 中毒/流血（ObjectPoisoned 推送的 poison_type 是 PoisonType bits）
        for (object_id, poison_type) in poisoned_objects {
            tracing::trace!("☠ Object poisoned: object_id={}, type={}", object_id, poison_type);
            if let Some(&e) = entity_index.get(&object_id) {
                use mir2_shared::enums::PoisonType;
                let pt = PoisonType::from_bits_truncate(poison_type as u16);
                NetworkApplySystem::apply_poison_to_entity(ctx, e, pt);
            }
        }

        // 复活
        for object_id in revived {
            tracing::trace!("💚 Object revived: {}", object_id);
            if let Some(&e) = entity_index.get(&object_id) {
                // 移除死亡状态
                let _ = ctx.world.remove_one::<crate::components::DeathState>(e);

                NetworkApplySystem::set_visibility_dead(ctx, e, false);

                // 恢复默认血量（若当前为 0）
                if let Ok(mut hp) = ctx.world.get::<&mut crate::components::Health>(e) {
                    if hp.current <= 0 {
                        hp.current = hp.max.max(1);
                    }
                }

                // 恢复站立动画（怪物/NPC）
                if let Ok(mut s) = ctx.world.get::<&mut crate::components::MonsterAnimState>(e) {
                    s.action = crate::components::MirAction::Standing;
                    s.start_time = std::time::Instant::now();
                }

                // 复活粒子效果
                if let Some((x, y)) = NetworkApplySystem::entity_position(ctx, e) {
                    ctx.events_mut().send_presentation(crate::event_bus::PresentationEvent::SpawnParticle {
                        particle_type: crate::event_bus::ParticleType::Heal,
                        position: (x, y - PARTICLE_Y_OFFSET),
                        velocity: None,
                        duration: REVIVE_HEAL_DURATION,
                    });
                }
            }
        }

        // 远程攻击：位置落地 + 朝向更新
        for (object_id, direction, x, y) in range_attacks {
            NetworkApplySystem::apply_object_move(ctx, &entity_index, object_id, x, y);
            let dir = mir2_shared::MirDirection::try_from(direction).ok();
            NetworkApplySystem::set_monster_anim(&ctx.world, &entity_index, object_id, None, dir);
        }

        // 远程攻击投射物
        for (from_id, target_id, target_x, target_y, spell) in range_projectiles {
            let from_pos = NetworkApplySystem::object_position(&ctx.world, &entity_index, from_id);
            let to = if target_id != 0 {
                NetworkApplySystem::object_position(&ctx.world, &entity_index, target_id)
            } else {
                None
            };
            let to = to.unwrap_or_else(|| {
                crate::coord::Coord::grid_to_world_center(target_x as i32, target_y as i32)
            });
            if let (Some(from), Some(projectile_type)) = (from_pos, NetworkApplySystem::spell_to_projectile_type(spell as u8)) {
                ctx.events_mut().send_presentation(crate::event_bus::PresentationEvent::ProjectileEffect {
                    projectile_type,
                    from,
                    to,
                    speed: PROJECTILE_SPEED,
                });
            }
        }

        // 玩家经验
        for amount in &player_exp_gains {
            if let Some(e) = local_player_entity {
                if let Ok(mut exp) = ctx.world.get::<&mut crate::components::Experience>(e) {
                    exp.current += amount;
                    tracing::debug!("⭐ Experience gained: {} (total={})", amount, exp.current);
                }
                if let Some((x, y)) = NetworkApplySystem::entity_position(ctx, e) {
                    ctx.events_mut().send_presentation(crate::event_bus::PresentationEvent::FloatingExperience {
                        amount: *amount,
                        position: (x, y - FLOAT_Y_EXP_OFFSET),
                    });
                }
            }
        }

        // 玩家升级
        for new_level in &player_level_ups {
            if let Some(e) = local_player_entity {
                if let Ok(mut data) = ctx.world.get::<&mut crate::components::PlayerData>(e) {
                    data.level = *new_level;
                    tracing::info!("🎉 Player leveled up to {}!", new_level);
                }
                if let Ok(mut exp) = ctx.world.get::<&mut crate::components::Experience>(e) {
                    exp.required = crate::components::Experience::calculate_required(*new_level);
                }
                if let Ok(mut stats) = ctx.world.get::<&mut crate::components::CombatStats>(e) {
                    stats.level = *new_level;
                }
                if let Some((x, y)) = NetworkApplySystem::entity_position(ctx, e) {
                    ctx.events_mut().send_presentation(crate::event_bus::PresentationEvent::FloatingText {
                        text: format!("Level Up! Lv.{}", new_level),
                        position: (x, y - FLOAT_Y_LEVELUP_OFFSET),
                        color: macroquad::prelude::Color::from_rgba(255, 220, 50, 255),
                        font_size: LEVELUP_FONT_SIZE_PLAYER,
                        duration: LEVELUP_TEXT_DURATION,
                    });
                }
            }
        }

        // Player colour change
        if let Some((colour, e)) = player_colour.zip(local_player_entity) {
            NetworkApplySystem::upsert_component(ctx, e, crate::components::NameColor(colour as i32));
        }

        // Object colour changes
        for (object_id, colour) in object_colours {
            if let Some(&e) = entity_index.get(&object_id) {
                NetworkApplySystem::upsert_component(ctx, e, crate::components::NameColor(colour as i32));
            }
        }

        // Object guild name changes
        for (object_id, guild_name) in object_guild_names {
            if let Some(&e) = entity_index.get(&object_id) {
                if let Ok(mut other) = ctx.world.get::<&mut crate::components::OtherPlayer>(e) {
                    other.guild_name = if guild_name.is_empty() { None } else { Some(guild_name) };
                }
            }
        }

        // Object name updates
        for (object_id, name) in object_names {
            if let Some(&e) = entity_index.get(&object_id) {
                if let Ok(mut other) = ctx.world.get::<&mut crate::components::OtherPlayer>(e) {
                    other.name = name;
                }
            }
        }

        // Object level ups
        for (object_id, level) in object_level_ups {
            if let Some(&e) = entity_index.get(&object_id) {
                if let Ok(mut other) = ctx.world.get::<&mut crate::components::OtherPlayer>(e) {
                    other.level = level;
                }
                if let Ok(mut stats) = ctx.world.get::<&mut crate::components::CombatStats>(e) {
                    stats.level = level;
                }
                if let Some((x, y)) = NetworkApplySystem::entity_position(ctx, e) {
                    ctx.events_mut().send_presentation(crate::event_bus::PresentationEvent::FloatingText {
                        text: format!("Lv.{}", level),
                        position: (x, y - FLOAT_Y_LEVELUP_OFFSET),
                        color: macroquad::prelude::Color::from_rgba(255, 220, 50, 255),
                        font_size: LEVELUP_FONT_SIZE_OBJECT,
                        duration: OBJECT_LEVELUP_DURATION,
                    });
                }
            }
        }

        // 元素状态更新
        for (object_id, enabled, value, element, expire_time) in elemental_updates {
            if let Some(&e) = entity_index.get(&object_id) {
                NetworkApplySystem::upsert_component(
                    ctx,
                    e,
                    crate::components::combat::ElementalState {
                        enabled,
                        element,
                        value,
                        expire_time,
                    },
                );
            }
        }

        // 对象装饰更新
        for (object_id, deco, remove) in deco_updates {
            if let Some(&e) = entity_index.get(&object_id) {
                if remove {
                    let _ = ctx.world.remove_one::<crate::components::render::ObjectDeco>(e);
                } else {
                    NetworkApplySystem::upsert_component(ctx, e, crate::components::render::ObjectDeco { deco_id: deco });
                }
            }
        }

        // 对象 mana 百分比
        for (object_id, percent) in object_mana_percents {
            tracing::trace!("💎 Object mana: object_id={}, {}%", object_id, percent);
            if let Some(&e) = entity_index.get(&object_id) {
                if let Ok(mut mana) = ctx.world.get::<&mut crate::components::combat::Mana>(e) {
                    mana.current = (mana.max as f32 * (percent as f32 / 100.0)).round() as i32;
                }
            }
        }

        // 对象隐身状态
        for (object_id, sneaking) in object_sneaking {
            if let Some(&e) = entity_index.get(&object_id) {
                NetworkApplySystem::upsert_component(ctx, e, crate::components::Visibility { hidden: sneaking, dead: false });
            }
        }

        // 对象等级特效
        for (object_id, level_effects) in object_level_effects {
            let flags = crate::components::LevelEffectsFlags(
                mir2_shared::enums::LevelEffects::from_bits_truncate(level_effects as u16)
            );
            if let Some(&e) = entity_index.get(&object_id) {
                NetworkApplySystem::upsert_component(ctx, e, flags);
            }
        }

        // 对象施法动画
        for (object_id, _spell) in object_spells {
            NetworkApplySystem::set_monster_anim(&ctx.world, &entity_index, object_id, Some(crate::components::MirAction::Spell), None);
        }

        // Player appearance updates
        for (object_id, weapon, weapon_effect, armor, wings_effect) in player_appearance_updates {
            if let Some(&e) = entity_index.get(&object_id) {
                if let Ok(mut appearance) = ctx.world.get::<&mut crate::components::PlayerAppearance>(e) {
                    appearance.weapon = weapon;
                    appearance.armour = armor;
                    appearance.weapon_effect = weapon_effect;
                    appearance.wing_effect = wings_effect;
                }
            }
        }

        // 英雄经验
        for amount in hero_exp_gains {
            if let Some(e) = local_player_entity {
                if let Ok(mut hero) = ctx.world.get::<&mut crate::components::HeroState>(e) {
                    hero.experience += amount;
                    tracing::debug!("⭐ Hero experience gained: {} (total={})", amount, hero.experience);
                }
                let hero_id = ctx.world.get::<&crate::components::HeroState>(e).ok().map(|h| h.hero_object_id).unwrap_or(0);
                if hero_id != 0 {
                    if let Some((x, y)) = NetworkApplySystem::object_position(&ctx.world, &entity_index, hero_id) {
                        ctx.events_mut().send_presentation(crate::event_bus::PresentationEvent::FloatingExperience {
                            amount,
                            position: (x, y - FLOAT_Y_EXP_OFFSET),
                        });
                    }
                }
            }
        }

        // 英雄升级
        for new_level in hero_level_ups {
            if let Some(e) = local_player_entity {
                if let Ok(mut hero) = ctx.world.get::<&mut crate::components::HeroState>(e) {
                    hero.level = new_level;
                    tracing::info!("🌟 Hero leveled up to {}!", new_level);
                }
                let hero_id = ctx.world.get::<&crate::components::HeroState>(e).ok().map(|h| h.hero_object_id).unwrap_or(0);
                if hero_id != 0 {
                    if let Some((x, y)) = NetworkApplySystem::object_position(&ctx.world, &entity_index, hero_id) {
                        ctx.events_mut().send_presentation(crate::event_bus::PresentationEvent::FloatingText {
                            text: format!("Hero Level Up! Lv.{}", new_level),
                            position: (x, y - FLOAT_Y_LEVELUP_OFFSET),
                            color: macroquad::prelude::Color::from_rgba(100, 200, 255, 255),
                            font_size: 14.0,
                            duration: LEVELUP_TEXT_DURATION,
                        });
                    }
                }
            }
        }

        // 英雄血量/魔法更新
        for (hp, mp) in hero_health_changes {
            if let Some(e) = local_player_entity {
                let hero_id = ctx.world.get::<&crate::components::HeroState>(e).ok().map(|h| h.hero_object_id).unwrap_or(0);
                if hero_id != 0 {
                    if let Some(&hero_entity) = entity_index.get(&hero_id) {
                        let mut updated_hp = false;
                        {
                            if let Ok(mut health) = ctx.world.get::<&mut crate::components::Health>(hero_entity) {
                                health.current = hp.max(0);
                                updated_hp = true;
                            }
                        }
                        if !updated_hp {
                            let _ = ctx.world.insert_one(hero_entity, crate::components::Health { current: hp.max(0), max: hp.max(1) });
                        }
                        let mut updated_mp = false;
                        {
                            if let Ok(mut mana) = ctx.world.get::<&mut crate::components::Mana>(hero_entity) {
                                mana.current = mp.max(0);
                                updated_mp = true;
                            }
                        }
                        if !updated_mp {
                            let _ = ctx.world.insert_one(hero_entity, crate::components::Mana { current: mp.max(0), max: mp.max(1) });
                        }
                        tracing::debug!("🦸 Hero health updated: hp={} mp={}", hp, mp);
                    }
                }
            }
        }

        // Hero magic updates (deferred from event loop)
        for (spell, level, experience, key) in hero_magic_learned {
            NetworkApplySystem::with_hero_magic_list(ctx, &entity_index, local_player_entity, |magic_list| {
                NetworkApplySystem::update_learned_magic(magic_list, spell, level, experience, key);
            });
        }
        for spell in hero_magic_removed {
            NetworkApplySystem::with_hero_magic_list(ctx, &entity_index, local_player_entity, |magic_list| {
                NetworkApplySystem::remove_magic(magic_list, spell);
            });
        }
        for (spell, level) in hero_magic_leveled_up {
            NetworkApplySystem::with_hero_magic_list(ctx, &entity_index, local_player_entity, |magic_list| {
                NetworkApplySystem::update_magic_level(magic_list, spell, level);
            });
        }
        for (spell, can_use) in hero_spell_toggled {
            NetworkApplySystem::with_hero_magic_list(ctx, &entity_index, local_player_entity, |magic_list| {
                NetworkApplySystem::update_spell_toggle(magic_list, spell, can_use);
            });
        }

        // 耐久度变化
        for (unique_id, durability) in dura_changes {
            tracing::trace!("🔧 Item durability changed: unique_id={}, durability={}", unique_id, durability);
            let Some(e) = local_player_entity else { continue };
            let new_dura = durability.max(0) as u16;
            if let Ok(mut inv) = ctx.world.get::<&mut crate::components::Inventory>(e) {
                if let Some(item) = inv.find_mut_by_id(unique_id) {
                    item.current_dura = new_dura;
                    item.dura_changed = true;
                }
            }
            if let Ok(mut eq) = ctx.world.get::<&mut crate::components::Equipment>(e) {
                if let Some(item) = eq.find_mut_by_id(unique_id) {
                    item.current_dura = new_dura;
                    item.dura_changed = true;
                }
            }
        }

        // 物品修理：更新 current_dura 和 max_dura
        for (unique_id, current_dura, max_dura) in item_repairs {
            let Some(e) = local_player_entity else { continue };
            if let Ok(mut inv) = ctx.world.get::<&mut crate::components::Inventory>(e) {
                if let Some(item) = inv.find_mut_by_id(unique_id) {
                    item.current_dura = current_dura;
                    item.max_dura = max_dura;
                    item.dura_changed = true;
                }
            }
            if let Ok(mut eq) = ctx.world.get::<&mut crate::components::Equipment>(e) {
                if let Some(item) = eq.find_mut_by_id(unique_id) {
                    item.current_dura = current_dura;
                    item.max_dura = max_dura;
                    item.dura_changed = true;
                }
            }
        }

        // Mock world objects
        for object_id in mock_despawns {
            NetworkApplySystem::apply_mock_library_sprite_despawn(ctx, object_id);
        }
        for (object_id, object_type, library, index, x, y) in mock_spawns {
            NetworkApplySystem::apply_mock_library_sprite_spawn(ctx, object_id, object_type, library, index, x, y);
        }

        // ===== 技能特效可视化 =====
        // ObjectMagicCast: 为施法对象生成技能特效实体
        for (object_id, spell, target_id) in spell_casts {
            let caster_pos = entity_index.get(&object_id)
                .and_then(|&e| ctx.world.get::<&crate::components::Position>(e).ok().map(|p| *p));
            let target_pos = entity_index.get(&target_id)
                .and_then(|&e| ctx.world.get::<&crate::components::Position>(e).ok().map(|p| *p));

            let spawn_pos = target_pos.or(caster_pos)
                .unwrap_or_else(|| crate::components::Position::new(0.0, 0.0));
            let spawn_x = spawn_pos.x;
            let spawn_y = spawn_pos.y;

            // 根据技能类型选择特效颜色
            use mir2_shared::enums::Spell;
            let spell_enum = spell;
            let (effect_text, effect_color) = match spell_enum {
                x if x == Spell::FireBall as u8 => ("火球术", macroquad::prelude::ORANGE),
                x if x == Spell::GreatFireBall as u8 => ("大火球", macroquad::prelude::RED),
                x if x == Spell::HellFire as u8 => ("地狱火", macroquad::prelude::YELLOW),
                x if x == Spell::ThunderBolt as u8 => ("雷电", macroquad::prelude::Color::from_rgba(150, 150, 255, 255)),
                x if x == Spell::Lightning as u8 => ("闪电", macroquad::prelude::Color::from_rgba(100, 100, 255, 255)),
                x if x == Spell::Healing as u8 => ("治疗术", macroquad::prelude::GREEN),
                x if x == Spell::Poisoning as u8 => ("施毒", macroquad::prelude::Color::from_rgba(128, 0, 128, 255)),
                x if x == Spell::Teleport as u8 => ("瞬移", macroquad::prelude::WHITE),
                x if x == Spell::MagicShield as u8 => ("魔法盾", macroquad::prelude::Color::from_rgba(100, 200, 255, 255)),
                x if x == Spell::HalfMoon as u8 => ("半月", macroquad::prelude::Color::from_rgba(255, 255, 200, 255)),
                x if x == Spell::ShoulderDash as u8 => ("野蛮冲撞", macroquad::prelude::Color::from_rgba(200, 200, 200, 255)),
                _ => ("施法", macroquad::prelude::Color::from_rgba(255, 255, 100, 255)),
            };

            // 飘字特效
            let now = current_time_secs();
            ctx.world.spawn((
                crate::components::Position::new(spawn_x, spawn_y - 90.0),
                crate::components::FloatingText {
                    text: effect_text.to_string(),
                    start_time: now,
                    duration: 0.6,
                    rise_speed: 50.0,
                    color: Some(effect_color),
                },
            ));

            // 施法者：设置施法动画
            NetworkApplySystem::set_monster_anim(&ctx.world, &entity_index, object_id, Some(crate::components::MirAction::Spell), None);

            // 玩家施法者（本地/远程）：切到 SpellCast 前摇动作。
            // 复用 AttackState 作为 one-shot 计时器，按 Spell 帧表时长播完自动回到 Stand。
            if let Some(&caster) = entity_index.get(&object_id) {
                if ctx.world.get::<&crate::components::DeathState>(caster).is_err()
                    && ctx.world.get::<&crate::components::Player>(caster).is_ok()
                {
                    if let Ok(mut p) = ctx.world.get::<&mut crate::components::Player>(caster) {
                        p.action = crate::components::PlayerAction::SpellCast;
                    }
                    let _ = ctx.world.insert_one(
                        caster,
                        crate::components::AttackState {
                            start_time: std::time::Instant::now(),
                            attack_type: crate::components::PlayerAction::SpellCast,
                            server_attack_type: 0,
                        },
                    );
                }
            }

            tracing::trace!("🔮 Spell cast: {:?} from {} to {}", spell_enum, object_id, target_id);
        }

        // ObjectEffectReceived: 命中/暴击等特效
        for (object_id, effect, effect_type) in effect_received {
            use mir2_shared::enums::SpellEffect;

            let (px, py) = NetworkApplySystem::object_position(&ctx.world, &entity_index, object_id).unwrap_or((0.0, 0.0));

            let now = current_time_secs();

            match effect {
                x if x == SpellEffect::Critical as u8 => {
                    // 暴击特效：黄色大字
                    ctx.world.spawn((
                        crate::components::Position::new(px, py - FLOAT_Y_LEVELUP_OFFSET),
                        crate::components::FloatingText {
                            text: "暴击!".to_string(),
                            start_time: now,
                            duration: 1.2,
                            rise_speed: 45.0,
                            color: Some(macroquad::prelude::YELLOW),
                        },
                    ));
                }
                x if x == SpellEffect::FatalSword as u8 => {
                    ctx.world.spawn((
                        crate::components::Position::new(px, py - FLOAT_Y_LEVELUP_OFFSET),
                        crate::components::FloatingText {
                            text: "致命!".to_string(),
                            start_time: now,
                            duration: 1.2,
                            rise_speed: 45.0,
                            color: Some(macroquad::prelude::Color::from_rgba(255, 100, 100, 255)),
                        },
                    ));
                }
                x if x == SpellEffect::MagicShieldUp as u8 => {
                    ctx.world.spawn((
                        crate::components::Position::new(px, py - FLOAT_Y_LEVELUP_OFFSET),
                        crate::components::FloatingText {
                            text: "护盾".to_string(),
                            start_time: now,
                            duration: 1.0,
                            rise_speed: 40.0,
                            color: Some(macroquad::prelude::Color::from_rgba(100, 200, 255, 255)),
                        },
                    ));
                }
                x if x == SpellEffect::MagicShieldDown as u8 => {
                    ctx.world.spawn((
                        crate::components::Position::new(px, py - FLOAT_Y_LEVELUP_OFFSET),
                        crate::components::FloatingText {
                            text: "护盾破碎".to_string(),
                            start_time: now,
                            duration: 1.0,
                            rise_speed: 40.0,
                            color: Some(macroquad::prelude::RED),
                        },
                    ));
                }
                x if x == SpellEffect::Healing as u8 => {
                    ctx.world.spawn((
                        crate::components::Position::new(px, py - FLOAT_Y_LEVELUP_OFFSET),
                        crate::components::FloatingText {
                            text: "治疗".to_string(),
                            start_time: now,
                            duration: 1.0,
                            rise_speed: 40.0,
                            color: Some(macroquad::prelude::GREEN),
                        },
                    ));
                }
                x if x == SpellEffect::Stunned as u8 => {
                    ctx.world.spawn((
                        crate::components::Position::new(px, py - FLOAT_Y_LEVELUP_OFFSET),
                        crate::components::FloatingText {
                            text: "眩晕!".to_string(),
                            start_time: now,
                            duration: 1.5,
                            rise_speed: 30.0,
                            color: Some(macroquad::prelude::Color::from_rgba(255, 200, 0, 255)),
                        },
                    ));
                }
                _ => {
                    tracing::trace!("✨ Effect received: effect={}, type={}", effect, effect_type);
                }
            }
        }

        // ===== 技能冷却应用 =====
        // MagicDelayReceived: 设置本地玩家技能冷却
        if !spell_delays.is_empty() {
            if let Some(e) = local_player_entity {
                if ctx.world.get::<&crate::components::spell::SpellCooldowns>(e).is_err() {
                    let _ = ctx.world.insert_one(e, crate::components::spell::SpellCooldowns::new());
                }
                if let Ok(mut cooldowns) = ctx.world.get::<&mut crate::components::spell::SpellCooldowns>(e) {
                    for (spell_id, delay_ms) in spell_delays {
                        cooldowns.set(spell_id, delay_ms);
                    }
                }
            }
        }

        // ===== 钓鱼动画 =====
        // FishingUpdate（仅本地玩家）：根据服务器下发的 progress/success 切换钓鱼动作。
        // - success=true         → FishingReel（收竿，one-shot，播完回 Stand）
        // - success=false 且 progress>0 → Fishing（抛竿/等待，持续状态，由服务器驱动退出）
        // - success=false 且 progress=0 → 回到 Stand（收竿结束/停止钓鱼）
        if let Some((state, success)) = fishing_status {
            if let Some(e) = local_player_entity {
                let dead = ctx
                    .world
                    .get::<&crate::components::DeathState>(e)
                    .is_ok();
                if !dead && ctx.world.get::<&crate::components::Player>(e).is_ok() {
                    let new_action = if success {
                        Some(crate::components::PlayerAction::FishingReel)
                    } else if state > 0 {
                        Some(crate::components::PlayerAction::Fishing)
                    } else {
                        None
                    };

                    if let Some(action) = new_action {
                        if let Ok(mut p) = ctx.world.get::<&mut crate::components::Player>(e) {
                            p.action = action;
                        }
                        // one-shot 动作（收竿）挂 AttackState 作为计时器，播完自动回 Stand；
                        // Fishing（等待）是持续状态，不挂计时器。
                        if action.is_one_shot() {
                            let _ = ctx.world.insert_one(
                                e,
                                crate::components::AttackState {
                                    start_time: std::time::Instant::now(),
                                    attack_type: action,
                                    server_attack_type: 0,
                                },
                            );
                        } else {
                            // 进入持续钓鱼态时清掉残留的一次性计时器，避免被动画系统提前打断
                            let _ = ctx.world.remove_one::<crate::components::AttackState>(e);
                        }
                    } else {
                        // progress=0 且未成功：退出钓鱼态
                        if let Ok(mut p) = ctx.world.get::<&mut crate::components::Player>(e) {
                            p.action = crate::components::PlayerAction::Stand;
                        }
                        let _ = ctx.world.remove_one::<crate::components::AttackState>(e);
                    }
                }
            }
        }

        Ok(())
}
