use crate::game::{GameContext, GameResult};
use crate::network::handlers::NetworkEvent;
use crate::systems::LogicSystem;

/// NetworkApplySystem - 网络事件落地系统
///
/// 职责：
/// - 消费 `EventBus.network_events` 中的 P0 关键包
/// - 把“协议层 packet”落地到 ECS 组件/资源
///
/// 设计目标：
/// - 未连接/无事件时完全 no-op
/// - 只做最小落地：本地玩家位置/朝向/基础外观/血蓝/等级
#[derive(ecs_macros::LogicSystem)]
pub struct NetworkApplySystem;

impl Default for NetworkApplySystem {
    fn default() -> Self {
        Self
    }
}

impl NetworkApplySystem {
    fn apply_user_information(ctx: &mut GameContext, packet: mir2_shared::packets::server::UserInformation) {
        use crate::components::{
            AnimationFrame, CombatStats, Health, LocalPlayer, Mana, MovementVelocity, Path, Player,
            PlayerAction, PlayerAppearance, PlayerInput, Position,
        };
        use crate::components::{Currency, Equipment, Experience, Inventory, MagicList, PlayerData, QuestInventory};
        use crate::components::{GuildInfo, HeroState, LevelEffectsFlags, NameColor, ObserveState, SummonedCreatureState};
        use mir2_shared::enums::ItemType;

        // 先找本地玩家实体；如果还没创建，则最小创建一个
        let existing = {
            let mut q = ctx.world.query::<&LocalPlayer>();
            q.iter().next().map(|(e, _)| e)
        };
        let local_entity = match existing {
            Some(e) => e,
            None => ctx.world.spawn((
                LocalPlayer,
                Player {
                    direction: mir2_shared::MirDirection::Up,
                    action: PlayerAction::Stand,
                },
                Position::new(0.0, 0.0),
                Health::new(packet.hp.max(1)),
                Mana::new(packet.mp.max(1)),
                PlayerAppearance::default(),
                AnimationFrame::default(),
                PlayerInput::default(),
                Path::new(),
                MovementVelocity::new(crate::components::movement::DEFAULT_MAX_SPEED),
            )),
        };

        // 位置：协议是格子坐标（i32），ECS 用世界像素（f32）
        let (wx, wy) = crate::coord::Coord::grid_to_world_center(packet.location_x, packet.location_y);
        if let Ok(mut pos) = ctx.world.get::<&mut Position>(local_entity) {
            pos.x = wx;
            pos.y = wy;
        }

        if let Ok(mut player) = ctx.world.get::<&mut Player>(local_entity) {
            player.direction = packet.direction;
        }

        // 外观/职业/性别（只落到渲染用组件；更完整的 PlayerData/背包/装备后续再做）
        if let Ok(mut appearance) = ctx.world.get::<&mut PlayerAppearance>(local_entity) {
            appearance.class = packet.class;
            appearance.gender = packet.gender;
            appearance.hair = packet.hair;
        }

        // 身份卡：PlayerData（UI/逻辑可直接读取）
        let has_player_data = ctx.world.get::<&PlayerData>(local_entity).is_ok();
        if has_player_data {
            if let Ok(mut pd) = ctx.world.get::<&mut PlayerData>(local_entity) {
                pd.id = packet.real_id;
                pd.name = packet.name.clone();
                pd.class = packet.class;
                pd.gender = packet.gender;
                pd.level = packet.level;
            }
        } else {
            let _ = ctx.world.insert_one(
                local_entity,
                PlayerData {
                    id: packet.real_id,
                    name: packet.name.clone(),
                    class: packet.class,
                    gender: packet.gender,
                    level: packet.level,
                },
            );
        }

        // 公会/称号/名字颜色/等级特效/观战状态/英雄/召唤物：先落地到组件，后续 UI/表现再消费
        let mut updated = false;
        if let Ok(mut color) = ctx.world.get::<&mut NameColor>(local_entity) {
            color.0 = packet.name_colour;
            updated = true;
        }
        if !updated {
            let _ = ctx.world.insert_one(local_entity, NameColor(packet.name_colour));
        }

        let mut updated = false;
        if let Ok(mut ge) = ctx.world.get::<&mut GuildInfo>(local_entity) {
            ge.name = packet.guild_name.clone();
            ge.rank = packet.guild_rank.clone();
            updated = true;
        }
        if !updated {
            let _ = ctx.world.insert_one(
                local_entity,
                GuildInfo {
                    name: packet.guild_name.clone(),
                    rank: packet.guild_rank.clone(),
                },
            );
        }

        let mut updated = false;
        if let Ok(mut le) = ctx.world.get::<&mut LevelEffectsFlags>(local_entity) {
            le.0 = packet.level_effects;
            updated = true;
        }
        if !updated {
            let _ = ctx.world.insert_one(local_entity, LevelEffectsFlags(packet.level_effects));
        }

        let mut updated = false;
        if let Ok(mut obs) = ctx.world.get::<&mut ObserveState>(local_entity) {
            obs.allow_observe = packet.allow_observe;
            obs.observer = packet.observer;
            updated = true;
        }
        if !updated {
            let _ = ctx.world.insert_one(
                local_entity,
                ObserveState {
                    allow_observe: packet.allow_observe,
                    observer: packet.observer,
                },
            );
        }

        let mut updated = false;
        if let Ok(mut hero) = ctx.world.get::<&mut HeroState>(local_entity) {
            hero.has_hero = packet.has_hero;
            hero.behaviour = packet.hero_behaviour;
            updated = true;
        }
        if !updated {
            let _ = ctx.world.insert_one(
                local_entity,
                HeroState {
                    has_hero: packet.has_hero,
                    behaviour: packet.hero_behaviour,
                },
            );
        }

        let mut updated = false;
        if let Ok(mut summon) = ctx.world.get::<&mut SummonedCreatureState>(local_entity) {
            summon.creature_type = packet.summoned_creature_type;
            summon.summoned = packet.creature_summoned;
            updated = true;
        }
        if !updated {
            let _ = ctx.world.insert_one(
                local_entity,
                SummonedCreatureState {
                    creature_type: packet.summoned_creature_type,
                    summoned: packet.creature_summoned,
                },
            );
        }

        // 血蓝：只更新 current，max 目前用“至少不小于 current”的策略避免 UI/逻辑出错
        if let Ok(mut hp) = ctx.world.get::<&mut Health>(local_entity) {
            hp.current = packet.hp.max(0);
            if hp.max < hp.current {
                hp.max = hp.current;
            }
        }

        if let Ok(mut mp) = ctx.world.get::<&mut Mana>(local_entity) {
            mp.current = packet.mp.max(0);
            if mp.max < mp.current {
                mp.max = mp.current;
            }
        }

        // 经验（以服务器下发为准；required=0 时回退到本地公式）
        let has_exp = ctx.world.get::<&Experience>(local_entity).is_ok();
        if has_exp {
            if let Ok(mut exp) = ctx.world.get::<&mut Experience>(local_entity) {
                exp.current = packet.experience;
                exp.required = if packet.max_experience > 0 {
                    packet.max_experience
                } else {
                    Experience::new(packet.level).required
                };
            }
        } else {
            let mut exp = Experience::new(packet.level);
            exp.current = packet.experience;
            if packet.max_experience > 0 {
                exp.required = packet.max_experience;
            }
            let _ = ctx.world.insert_one(local_entity, exp);
        }

        // 货币
        let has_currency = ctx.world.get::<&Currency>(local_entity).is_ok();
        if has_currency {
            if let Ok(mut cur) = ctx.world.get::<&mut Currency>(local_entity) {
                cur.gold = packet.gold;
                cur.credit = packet.credit;
            }
        } else {
            let _ = ctx.world.insert_one(local_entity, Currency { gold: packet.gold, credit: packet.credit });
        }

        // 背包/任务背包/装备
        if let Some(items) = packet.inventory.clone() {
            let has_inv = ctx.world.get::<&Inventory>(local_entity).is_ok();
            if has_inv {
                if let Ok(mut inv) = ctx.world.get::<&mut Inventory>(local_entity) {
                    inv.capacity = items.len();
                    inv.items = items;
                    inv.gold = packet.gold;
                }
            } else {
                let mut inv = Inventory::new(items.len().max(1));
                inv.items = items;
                inv.gold = packet.gold;
                let _ = ctx.world.insert_one(local_entity, inv);
            }
        }

        if let Some(items) = packet.quest_inventory.clone() {
            let has_q = ctx.world.get::<&QuestInventory>(local_entity).is_ok();
            if has_q {
                if let Ok(mut q) = ctx.world.get::<&mut QuestInventory>(local_entity) {
                    q.capacity = items.len();
                    q.items = items;
                }
            } else {
                let mut q = QuestInventory::new(items.len().max(1));
                q.items = items;
                let _ = ctx.world.insert_one(local_entity, q);
            }
        }

        if let Some(items) = packet.equipment.clone() {
            let has_equip = ctx.world.get::<&Equipment>(local_entity).is_ok();
            if has_equip {
                if let Ok(mut eq) = ctx.world.get::<&mut Equipment>(local_entity) {
                    Self::apply_equipment_vec(&mut eq, &items);
                }
            } else {
                let mut eq = Equipment::default();
                Self::apply_equipment_vec(&mut eq, &items);
                let _ = ctx.world.insert_one(local_entity, eq);
            }

            // 参考 C# UserObject.RefreshEquipmentStats():
            // - Armour/Weapon/特效由装备的 realItem.Shape / realItem.Effect 导出
            // - info 尚未到达时（网络异步）不要覆写，避免外观闪烁
            if let Ok(mut appearance) = ctx.world.get::<&mut PlayerAppearance>(local_entity) {
                // weapon slot = 0
                match items.get(0).and_then(|x| x.as_ref()) {
                    None => {
                        appearance.weapon = -1;
                        appearance.weapon_effect = 0;
                    }
                    Some(item) => {
                        if let Some(info) = item.info.as_ref() {
                            let broken = item.current_dura == 0 && info.durability > 0;
                            if !broken && info.item_type == ItemType::Weapon {
                                appearance.weapon = info.shape;
                                appearance.weapon_effect = info.effect as i16;
                            } else {
                                appearance.weapon = -1;
                                appearance.weapon_effect = 0;
                            }
                        }
                    }
                }

                // armour slot = 1
                match items.get(1).and_then(|x| x.as_ref()) {
                    None => {
                        appearance.armour = 0;
                        appearance.wing_effect = 0;
                    }
                    Some(item) => {
                        if let Some(info) = item.info.as_ref() {
                            let broken = item.current_dura == 0 && info.durability > 0;
                            if !broken && info.item_type == ItemType::Armour {
                                appearance.armour = info.shape;
                                appearance.wing_effect = info.effect;
                            } else {
                                appearance.armour = 0;
                                appearance.wing_effect = 0;
                            }
                        }
                    }
                }
            }
        }

        // 技能列表（服务器下发 ClientMagic）
        let has_magic = ctx.world.get::<&MagicList>(local_entity).is_ok();
        if has_magic {
            if let Ok(mut ml) = ctx.world.get::<&mut MagicList>(local_entity) {
                ml.magics = Self::map_magics(&packet.magics);
            }
        } else {
            let _ = ctx.world.insert_one(
                local_entity,
                MagicList {
                    magics: Self::map_magics(&packet.magics),
                },
            );
        }

        // 等级：落到 CombatStats（现有战斗/技能系统依赖）
        let has_stats = ctx.world.get::<&CombatStats>(local_entity).is_ok();
        if has_stats {
            if let Ok(mut stats) = ctx.world.get::<&mut CombatStats>(local_entity) {
                stats.level = packet.level;
            }
        } else {
            let _ = ctx
                .world
                .insert_one(local_entity, CombatStats { level: packet.level, ..CombatStats::default() });
        }
    }

    fn apply_equipment_vec(eq: &mut crate::components::Equipment, items: &[Option<mir2_shared::data::item::UserItem>]) {
        // C# 侧 Equipment 数组与这里的槽位约定：0..13
        // 0 weapon, 1 armour, 2 helmet, 3 necklace, 4 bracelet_l, 5 bracelet_r, 6 ring_l,
        // 7 ring_r, 8 amulet, 9 belt, 10 boots, 11 stone, 12 torch, 13 mount
        for (idx, item) in items.iter().enumerate() {
            let Some(item) = item.clone() else {
                // 空槽位就清空
                match idx {
                    0 => eq.weapon = None,
                    1 => eq.armour = None,
                    2 => eq.helmet = None,
                    3 => eq.necklace = None,
                    4 => eq.bracelet_l = None,
                    5 => eq.bracelet_r = None,
                    6 => eq.ring_l = None,
                    7 => eq.ring_r = None,
                    8 => eq.amulet = None,
                    9 => eq.belt = None,
                    10 => eq.boots = None,
                    11 => eq.stone = None,
                    12 => eq.torch = None,
                    13 => eq.mount = None,
                    _ => {}
                }
                continue;
            };

            match idx {
                0 => eq.weapon = Some(item),
                1 => eq.armour = Some(item),
                2 => eq.helmet = Some(item),
                3 => eq.necklace = Some(item),
                4 => eq.bracelet_l = Some(item),
                5 => eq.bracelet_r = Some(item),
                6 => eq.ring_l = Some(item),
                7 => eq.ring_r = Some(item),
                8 => eq.amulet = Some(item),
                9 => eq.belt = Some(item),
                10 => eq.boots = Some(item),
                11 => eq.stone = Some(item),
                12 => eq.torch = Some(item),
                13 => eq.mount = Some(item),
                _ => {}
            }
        }
    }

    fn map_magics(magics: &[mir2_shared::data::client_data::ClientMagic]) -> Vec<crate::components::LearnedMagic> {
        magics
            .iter()
            .filter_map(|m| {
                let spell_id = m.spell as u8;
                let spell = crate::components::SpellType::try_from(spell_id).ok()?;
                Some(crate::components::LearnedMagic {
                    spell,
                    level: m.level,
                    experience: m.experience as u32,
                    key_slot: if m.key == 0 { None } else { Some(m.key) },
                })
            })
            .collect()
    }

    fn apply_map_changed(ctx: &mut GameContext, packet: mir2_shared::packets::server::MapChanged) {
        use crate::components::{LocalPlayer, Player, Position};

        // MapChanged 里携带了落点与朝向（切图/传送时很关键）
        let Some((entity, _)) = ctx.world.query::<&LocalPlayer>().iter().next() else {
            return;
        };

        let (wx, wy) = crate::coord::Coord::grid_to_world_center(packet.location_x, packet.location_y);
        if let Ok(mut pos) = ctx.world.get::<&mut Position>(entity) {
            pos.x = wx;
            pos.y = wy;
        }

        if let Ok(mut player) = ctx.world.get::<&mut Player>(entity) {
            if let Ok(dir) = mir2_shared::enums::MirDirection::try_from(packet.direction) {
                player.direction = dir;
            }
        }
    }

    fn apply_mock_library_sprite_spawn(
        ctx: &mut GameContext,
        object_id: u32,
        object_type: crate::network::handlers::ObjectType,
        library: crate::resources::LibraryName,
        index: i32,
        location_x: i32,
        location_y: i32,
    ) {
        use crate::components::{LibrarySprite, Position};
        use crate::components::network::{NetworkObjectType, NetworkSync};

        let obj_type = match object_type {
            crate::network::handlers::ObjectType::Player => NetworkObjectType::Player,
            crate::network::handlers::ObjectType::Monster => NetworkObjectType::Monster,
            crate::network::handlers::ObjectType::Npc => NetworkObjectType::NPC,
            crate::network::handlers::ObjectType::Item => NetworkObjectType::Item,
            crate::network::handlers::ObjectType::Spell => NetworkObjectType::Spell,
        };

        let existing = {
            let mut q = ctx.world.query::<&NetworkSync>();
            q.iter()
                .find(|(_, ns)| ns.object_id == object_id)
                .map(|(e, _)| e)
        };

        let (wx, wy) = crate::coord::Coord::grid_to_world_center(location_x, location_y);

        match existing {
            Some(e) => {
                let has_pos = ctx.world.get::<&Position>(e).is_ok();
                if has_pos {
                    if let Ok(mut pos) = ctx.world.get::<&mut Position>(e) {
                        pos.x = wx;
                        pos.y = wy;
                    }
                } else {
                    let _ = ctx.world.insert_one(e, Position::new(wx, wy));
                }

                let has_sprite = ctx.world.get::<&LibrarySprite>(e).is_ok();
                if has_sprite {
                    if let Ok(mut spr) = ctx.world.get::<&mut LibrarySprite>(e) {
                        spr.library = library;
                        spr.index = index;
                        spr.frame = 0;
                    }
                } else {
                    let _ = ctx.world.insert_one(e, LibrarySprite::new(library, index));
                }
            }
            None => {
                ctx.world.spawn((
                    NetworkSync::new(object_id, obj_type),
                    Position::new(wx, wy),
                    LibrarySprite::new(library, index),
                ));
            }
        }
    }

    fn apply_mock_library_sprite_despawn(ctx: &mut GameContext, object_id: u32) {
        use crate::components::network::NetworkSync;

        let entity = {
            let mut q = ctx.world.query::<&NetworkSync>();
            q.iter()
                .find(|(_, ns)| ns.object_id == object_id)
                .map(|(e, _)| e)
        };

        if let Some(e) = entity {
            let _ = ctx.world.despawn(e);
        }
    }

    fn find_entity_by_object_id(ctx: &mut GameContext, object_id: u32) -> Option<hecs::Entity> {
        use crate::components::network::NetworkSync;

        let mut q = ctx.world.query::<&NetworkSync>();
        q.iter()
            .find(|(_, ns)| ns.object_id == object_id)
            .map(|(e, _)| e)
    }

    fn upsert_library_sprite_object(
        ctx: &mut GameContext,
        object_id: u32,
        object_type: crate::components::network::NetworkObjectType,
        library: crate::resources::LibraryName,
        index: i32,
        location_x: i32,
        location_y: i32,
    ) {
        use crate::components::{LibrarySprite, Position};
        use crate::components::network::NetworkSync;

        let (wx, wy) = crate::coord::Coord::grid_to_world_center(location_x, location_y);

        if let Some(e) = Self::find_entity_by_object_id(ctx, object_id) {
            let has_pos = ctx.world.get::<&Position>(e).is_ok();
            if has_pos {
                if let Ok(mut pos) = ctx.world.get::<&mut Position>(e) {
                    pos.x = wx;
                    pos.y = wy;
                }
            } else {
                let _ = ctx.world.insert_one(e, Position::new(wx, wy));
            }

            let has_sprite = ctx.world.get::<&LibrarySprite>(e).is_ok();
            if has_sprite {
                if let Ok(mut spr) = ctx.world.get::<&mut LibrarySprite>(e) {
                    spr.library = library;
                    spr.index = index;
                    spr.frame = 0;
                }
            } else {
                let _ = ctx.world.insert_one(e, LibrarySprite::new(library, index));
            }
        } else {
            ctx.world.spawn((
                NetworkSync::new(object_id, object_type),
                Position::new(wx, wy),
                LibrarySprite::new(library, index),
            ));
        }
    }

    fn apply_object_monster(ctx: &mut GameContext, packet: mir2_shared::packets::server::ObjectMonster) {
        use crate::components::network::NetworkObjectType;

        // C# 对应：Libraries.Monsters[(ushort)MonsterEnum]
        // 这里 image 直接对应 Monster/XYZ 的库索引（XYZ 三位数）
        let library = crate::resources::LibraryName::Monsters(packet.image as usize);

        // 最小可见：先画第 0 帧
        Self::upsert_library_sprite_object(
            ctx,
            packet.object_id,
            NetworkObjectType::Monster,
            library,
            0,
            packet.location_x,
            packet.location_y,
        );
    }

    fn apply_object_npc(ctx: &mut GameContext, packet: mir2_shared::packets::server::ObjectNpc) {
        use crate::components::network::NetworkObjectType;

        // C# 对应：Libraries.NPCs[Image]
        let library = crate::resources::LibraryName::Npcs(packet.image as usize);

        Self::upsert_library_sprite_object(
            ctx,
            packet.object_id,
            NetworkObjectType::NPC,
            library,
            0,
            packet.location_x,
            packet.location_y,
        );
    }

    fn apply_object_remove(ctx: &mut GameContext, object_id: u32) {
        if let Some(e) = Self::find_entity_by_object_id(ctx, object_id) {
            let _ = ctx.world.despawn(e);
        }
    }

    fn apply_object_move(ctx: &mut GameContext, object_id: u32, x: i32, y: i32) {
        use crate::components::Position;

        let Some(e) = Self::find_entity_by_object_id(ctx, object_id) else {
            return;
        };

        let (wx, wy) = crate::coord::Coord::grid_to_world_center(x, y);
        let has_pos = ctx.world.get::<&Position>(e).is_ok();
        if has_pos {
            if let Ok(mut pos) = ctx.world.get::<&mut Position>(e) {
                pos.x = wx;
                pos.y = wy;
            }
        } else {
            let _ = ctx.world.insert_one(e, Position::new(wx, wy));
        }
    }
}

impl LogicSystem for NetworkApplySystem {
    fn update(&mut self, ctx: &mut GameContext, _delay_time: f32) -> GameResult {
        if !ctx.events().has_network_events() {
            return Ok(());
        }

        let mut user_info: Option<mir2_shared::packets::server::UserInformation> = None;
        let mut map_changed: Option<mir2_shared::packets::server::MapChanged> = None;
        let mut start_game: Option<mir2_shared::packets::server::StartGame> = None;
        let mut start_game_delay: Option<mir2_shared::packets::server::StartGameDelay> = None;
        let mut start_game_banned: Option<mir2_shared::packets::server::StartGameBanned> = None;

        let mut mock_spawns: Vec<(u32, crate::network::handlers::ObjectType, crate::resources::LibraryName, i32, i32, i32)> = Vec::new();
        let mut mock_despawns: Vec<u32> = Vec::new();

        let mut object_monsters: Vec<mir2_shared::packets::server::ObjectMonster> = Vec::new();
        let mut object_npcs: Vec<mir2_shared::packets::server::ObjectNpc> = Vec::new();
        let mut object_removes: Vec<u32> = Vec::new();
        let mut object_moves: Vec<(u32, i32, i32)> = Vec::new();

        // 本地玩家：server-driven 状态（对齐真服）
        let mut player_location_changed: Option<(i32, i32)> = None;
        let mut gold_delta_sum: i32 = 0;
        let mut items_gained: Vec<mir2_shared::data::item::UserItem> = Vec::new();
        let mut items_lost: Vec<u64> = Vec::new();
        let mut items_moved: Vec<(u32, u32)> = Vec::new();

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
                NetworkEvent::ObjectRemove { packet } => {
                    object_removes.push(packet.object_id);
                }
                NetworkEvent::ObjectWalk { packet } => {
                    object_moves.push((packet.object_id, packet.location_x, packet.location_y));
                }
                NetworkEvent::ObjectRun { packet } => {
                    object_moves.push((packet.object_id, packet.location_x, packet.location_y));
                }
                NetworkEvent::ObjectTurn { packet } => {
                    object_moves.push((packet.object_id, packet.location_x, packet.location_y));
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
                NetworkEvent::ItemLost { unique_id } => {
                    items_lost.push(*unique_id);
                }
                NetworkEvent::ItemMoved { from, to } => {
                    items_moved.push((*from, *to));
                }
                _ => {}
            }
        }

        // StartGame*：先“消费并落地到会话状态”，避免事件丢失（帧末 clear_frame）
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
            Self::apply_user_information(ctx, packet);
        }

        if let Some(packet) = map_changed {
            Self::apply_map_changed(ctx, packet);
        }

        // server-driven objects
        for packet in object_monsters {
            Self::apply_object_monster(ctx, packet);
        }
        for packet in object_npcs {
            Self::apply_object_npc(ctx, packet);
        }
        for object_id in object_removes {
            Self::apply_object_remove(ctx, object_id);
        }
        for (object_id, x, y) in object_moves {
            Self::apply_object_move(ctx, object_id, x, y);
        }

        // ===== server-driven: local player state落地 =====
        // 备注：LocalPlayer 一般由 UserInformation 创建；若不存在则跳过。
        let local_player_entity = {
            use crate::components::LocalPlayer;
            let mut q = ctx.world.query::<&LocalPlayer>();
            q.iter().next().map(|(e, _)| e)
        };

        if let Some(e) = local_player_entity {
            // 位置校正（格子坐标 -> 世界像素）
            // 仅在“服务器权威移动”开启时落地；否则会与本地 MovementSystem 双驱动，导致抖动/乱跳。
            if ctx.session.server_authoritative_movement {
                if let Some((gx, gy)) = player_location_changed {
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

                // Inventory.gold（用于 UI 展示）
                if let Ok(mut inv) = ctx.world.get::<&mut crate::components::Inventory>(e) {
                    apply_delta(&mut inv.gold, gold_delta_sum);
                }
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

            // 删除物品（按 unique_id 查找）
            if !items_lost.is_empty() {
                if let Ok(mut inv) = ctx.world.get::<&mut crate::components::Inventory>(e) {
                    for uid in items_lost {
                        for slot in inv.items.iter_mut() {
                            if let Some(it) = slot.as_ref() {
                                if it.unique_id == uid {
                                    *slot = None;
                                    break;
                                }
                            }
                        }
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
        }

        // Mock world objects
        for object_id in mock_despawns {
            Self::apply_mock_library_sprite_despawn(ctx, object_id);
        }
        for (object_id, object_type, library, index, x, y) in mock_spawns {
            Self::apply_mock_library_sprite_spawn(ctx, object_id, object_type, library, index, x, y);
        }

        Ok(())
    }
}
