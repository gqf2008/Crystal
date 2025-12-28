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
    fn net_recv_diag_enabled() -> bool {
        static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *ENABLED.get_or_init(|| {
            std::env::var_os("CRYSTAL_NETRECV_DIAG").is_some()
                || std::env::var_os("CRYSTAL_NETMOVE_DIAG").is_some()
        })
    }

    fn apply_object_player(ctx: &mut GameContext, packet: mir2_shared::packets::server::ObjectPlayer) {
        use crate::components::{
            AnimationFrame, MountState, MountStatus, OtherPlayer, Player, PlayerAction, PlayerAppearance, Position,
            RemotePlayer,
        };
        use crate::components::network::{NetworkObjectType, NetworkSync};

        let (wx, wy) = crate::coord::Coord::grid_to_world_center(packet.location_x, packet.location_y);

        let appearance = PlayerAppearance {
            class: packet.class,
            gender: packet.gender,
            hair: packet.hair,
            weapon: packet.weapon,
            armour: packet.armour,
            weapon_effect: packet.weapon_effect,
            wing_effect: packet.wing_effect,
        };

        // 诊断：确认远程玩家（ObjectPlayer）确实收到并落地（只打印一次）。
        static OBJECT_PLAYER_DIAG_ONCE: std::sync::OnceLock<()> = std::sync::OnceLock::new();
        let _ = OBJECT_PLAYER_DIAG_ONCE.set(()).map(|_| {
            println!(
                "[DIAG][NetworkApplySystem] ObjectPlayer applied: id={} name={} class={:?} gender={:?} hair={} loc=({}, {}) dir={:?} weapon={} weapon_eff={} armour={} wing_eff={} riding_mount={}",
                packet.object_id,
                packet.name,
                packet.class,
                packet.gender,
                packet.hair,
                packet.location_x,
                packet.location_y,
                packet.direction,
                packet.weapon,
                packet.weapon_effect,
                packet.armour,
                packet.wing_effect,
                packet.riding_mount
            );
        });

        let player = Player {
            direction: packet.direction,
            action: PlayerAction::Stand,
        };

        let mount_index_from_packet: Option<usize> = if packet.riding_mount && packet.mount_type >= 0 {
            Some(packet.mount_type as usize)
        } else {
            None
        };

        if let Some(e) = Self::find_entity_by_object_id(ctx, packet.object_id) {
            // NetworkSync 只要存在即可；类型不匹配时更新。
            let mut has_sync = false;
            {
                if let Ok(mut sync) = ctx.world.get::<&mut NetworkSync>(e) {
                    sync.object_id = packet.object_id;
                    sync.object_type = NetworkObjectType::Player;
                    has_sync = true;
                }
            }
            if !has_sync {
                let _ = ctx
                    .world
                    .insert_one(e, NetworkSync::new(packet.object_id, NetworkObjectType::Player));
            }

            // 远程玩家标记
            if ctx.world.get::<&RemotePlayer>(e).is_err() {
                let _ = ctx.world.insert_one(e, RemotePlayer { id: packet.object_id });
            }

            // 位置
            let mut has_pos = false;
            {
                if let Ok(mut pos) = ctx.world.get::<&mut Position>(e) {
                    pos.x = wx;
                    pos.y = wy;
                    has_pos = true;
                }
            }
            if !has_pos {
                let _ = ctx.world.insert_one(e, Position::new(wx, wy));
            }

            // 核心 Player 状态
            let mut has_player = false;
            {
                if let Ok(mut p) = ctx.world.get::<&mut Player>(e) {
                    p.direction = player.direction;
                    // 只做最小落地：不覆盖 action（避免将来接入 ObjectWalk/Attack 时抖动）
                    has_player = true;
                }
            }
            if !has_player {
                let _ = ctx.world.insert_one(e, player);
            }

            // 外观
            let mut has_appearance = false;
            {
                if let Ok(mut a) = ctx.world.get::<&mut PlayerAppearance>(e) {
                    *a = appearance.clone();
                    has_appearance = true;
                }
            }
            if !has_appearance {
                let _ = ctx.world.insert_one(e, appearance);
            }

            // 动画帧（若没有就补一个默认，AnimationSystem 会更新）
            if ctx.world.get::<&AnimationFrame>(e).is_err() {
                let _ = ctx.world.insert_one(e, AnimationFrame::default());
            }

            // 坐骑：按服务器 ObjectPlayer 落地（用于行为对齐，如攻击音效选择）
            let mut has_mount_state = false;
            {
                if let Ok(mut m) = ctx.world.get::<&mut MountState>(e) {
                    m.mount_index = mount_index_from_packet;
                    has_mount_state = true;
                }
            }
            if !has_mount_state {
                let _ = ctx.world.insert_one(e, MountState { mount_index: mount_index_from_packet });
            }

            let mut has_mount_status = false;
            {
                if let Ok(mut ms) = ctx.world.get::<&mut MountStatus>(e) {
                    ms.mount_type = packet.mount_type;
                    ms.riding_mount = packet.riding_mount;
                    has_mount_status = true;
                }
            }
            if !has_mount_status {
                let _ = ctx.world.insert_one(
                    e,
                    MountStatus {
                        mount_type: packet.mount_type,
                        riding_mount: packet.riding_mount,
                    },
                );
            }

            // 基本身份信息（未来做名字/血条会用到）
            let mut has_other = false;
            {
                if let Ok(mut op) = ctx.world.get::<&mut OtherPlayer>(e) {
                    op.name = packet.name.clone();
                    op.class = packet.class;
                    op.gender = packet.gender;
                    op.level = packet.level;
                    op.guild_name = if packet.guild_name.is_empty() {
                        None
                    } else {
                        Some(packet.guild_name.clone())
                    };
                    has_other = true;
                }
            }
            if !has_other {
                let mut op = OtherPlayer::new(packet.name.clone(), packet.class, packet.gender, packet.level);
                op.guild_name = if packet.guild_name.is_empty() {
                    None
                } else {
                    Some(packet.guild_name.clone())
                };
                let _ = ctx.world.insert_one(e, op);
            }
        } else {
            ctx.world.spawn((
                NetworkSync::new(packet.object_id, NetworkObjectType::Player),
                RemotePlayer { id: packet.object_id },
                player,
                Position::new(wx, wy),
                appearance,
                AnimationFrame::default(),
                MountState { mount_index: mount_index_from_packet },
                MountStatus {
                    mount_type: packet.mount_type,
                    riding_mount: packet.riding_mount,
                },
                OtherPlayer::new(packet.name, packet.class, packet.gender, packet.level),
            ));
        }
    }
    fn apply_user_information(ctx: &mut GameContext, packet: mir2_shared::packets::server::UserInformation) {
        use crate::components::{
            AnimationFrame, CombatStats, Health, LocalPlayer, Mana, MovementVelocity, Path, Player,
            PlayerAction, PlayerAppearance, PlayerInput, Position, RegenTimer,
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
                RegenTimer::default(),
                PlayerAppearance::default(),
                AnimationFrame::default(),
                PlayerInput::default(),
                Path::new(),
                MovementVelocity::new(crate::components::movement::DEFAULT_MAX_SPEED),
                // 渲染/派生状态依赖：
                // - Weapon/Armour/翅膀/坐骑等外观通常由装备推导
                // - MountStateSyncSystem 需要 Equipment + MountState
                Equipment::default(),
                crate::components::MountState::default(),
                crate::components::MountStatus::default(),
            )),
        };

        // 兜底：旧存档/旧实体可能缺少 RegenTimer，导致 HealthRegenSystem 永远不生效。
        if ctx.world.get::<&RegenTimer>(local_entity).is_err() {
            let _ = ctx.world.insert_one(local_entity, RegenTimer::default());
        }

        // 诊断：确认是否真的收到了 UserInformation 并创建/更新了本地玩家。
        // 只打印一次，避免刷屏。
        static USERINFO_DIAG_ONCE: std::sync::OnceLock<()> = std::sync::OnceLock::new();
        let _ = USERINFO_DIAG_ONCE.set(()).map(|_| {
            println!(
                "[DIAG][NetworkApplySystem] UserInformation applied: entity={:?} name={} class={:?} gender={:?} hair={} loc=({}, {}) dir={:?}",
                local_entity,
                packet.name,
                packet.class,
                packet.gender,
                packet.hair,
                packet.location_x,
                packet.location_y,
                packet.direction
            );
        });

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
                pd.object_id = packet.object_id;
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
                    object_id: packet.object_id,
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
            // 诊断：确认 mock/服务器下发的装备槽位是否完整，以及是否能驱动外观派生。
            // 只打印一次，避免刷屏。
            static EQUIP_DIAG_ONCE: std::sync::OnceLock<()> = std::sync::OnceLock::new();

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

                let _ = EQUIP_DIAG_ONCE.set(()).map(|_| {
                    let weapon_shape = items
                        .get(0)
                        .and_then(|x| x.as_ref())
                        .and_then(|it| it.info.as_ref())
                        .map(|info| (info.item_type, info.shape, info.effect));
                    let mount_shape = items
                        .get(13)
                        .and_then(|x| x.as_ref())
                        .and_then(|it| it.info.as_ref())
                        .map(|info| (info.item_type, info.shape, info.effect));

                    println!(
                        "[DIAG][NetworkApplySystem] equipment received: len={} weapon_slot(info)={:?} mount_slot(info)={:?} => appearance.weapon={} armour={} weapon_effect={} wing_effect={}",
                        items.len(),
                        weapon_shape,
                        mount_shape,
                        appearance.weapon,
                        appearance.armour,
                        appearance.weapon_effect,
                        appearance.wing_effect
                    );
                });
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

    fn apply_player_inspect(ctx: &mut GameContext, packet: mir2_shared::packets::server::PlayerInspect) {
        use crate::components::{Equipment, OtherPlayer, PlayerAppearance};
        use mir2_shared::enums::ItemType;

        let target_entity = {
            let mut q = ctx.world.query::<&OtherPlayer>();
            q.iter()
                .find_map(|(e, op)| if op.name == packet.name { Some(e) } else { None })
        };

        let Some(e) = target_entity else {
            tracing::warn!("[NetworkApplySystem] PlayerInspect for unknown player name={}", packet.name);
            return;
        };

        // 身份信息
        if let Ok(mut op) = ctx.world.get::<&mut OtherPlayer>(e) {
            op.class = packet.class;
            op.gender = packet.gender;
            op.level = packet.level;
            op.guild_name = if packet.guild_name.is_empty() {
                None
            } else {
                Some(packet.guild_name.clone())
            };
        }

        // 装备栏
        let has_equip = ctx.world.get::<&Equipment>(e).is_ok();
        if has_equip {
            if let Ok(mut eq) = ctx.world.get::<&mut Equipment>(e) {
                Self::apply_equipment_vec(&mut eq, &packet.equipment);
            }
        } else {
            let mut eq = Equipment::default();
            Self::apply_equipment_vec(&mut eq, &packet.equipment);
            let _ = ctx.world.insert_one(e, eq);
        }

        // 外观派生（与 UserInformation 的逻辑保持一致：weapon/armour/特效由装备导出）
        if let Ok(mut appearance) = ctx.world.get::<&mut PlayerAppearance>(e) {
            appearance.class = packet.class;
            appearance.gender = packet.gender;
            appearance.hair = packet.hair;

            // weapon slot = 0
            match packet.equipment.get(0).and_then(|x| x.as_ref()) {
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
            match packet.equipment.get(1).and_then(|x| x.as_ref()) {
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
        if let Some((e, _)) = q.iter().find(|(_, ns)| ns.object_id == object_id) {
            return Some(e);
        }

        // LocalPlayer 默认不挂 NetworkSync，但仍可能需要通过 object_id 落地（例如 mock 侧下发的
        // ObjectStruck/ObjectDied/ObjectRemove 等）。
        {
            use crate::components::{LocalPlayer, PlayerData};
            let mut q = ctx.world.query::<(&LocalPlayer, &PlayerData)>();
            q.iter()
                .find(|(_, (_lp, pd))| pd.object_id == object_id)
                .map(|(e, _)| e)
        }
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
        use crate::components::{MirAction, MonsterAnimState, SoundTrigger, SoundType};
        use std::time::Instant;

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

        // 同步怪物名称（用于悬停/调试 overlay，避免“只有贴图没有名字”）
        if let Some(e) = Self::find_entity_by_object_id(ctx, packet.object_id) {
            // 先插入；若已存在则更新
            let inserted_monster_component = ctx
                .world
                .insert_one(e, crate::components::Monster::new(packet.name.clone(), packet.image))
                .is_ok();
            if !inserted_monster_component {
                if let Ok(mut m) = ctx.world.get::<&mut crate::components::Monster>(e) {
                    m.name = packet.name.clone();
                    m.monster_type = packet.image;
                    m.stage = packet.extra_byte;
                }
            } else {
                if let Ok(mut m) = ctx.world.get::<&mut crate::components::Monster>(e) {
                    m.stage = packet.extra_byte;
                }
            }

            // 原版：怪物出现音效 BaseSound + 0（规则 0=Appear）
            // 这里用“首次插入 Monster 组件”近似判断首次出现。
            if inserted_monster_component {
                let base_sound = packet.image * 10;
                let _ = ctx.world.insert_one(
                    e,
                    SoundTrigger::once(base_sound.to_string(), SoundType::CharacterAction),
                );
            }

            // 动画状态：方向来自包；初始动作为 Standing/Dead（最小集）
            let initial_action = if packet.dead { MirAction::Dead } else { MirAction::Standing };
            if ctx
                .world
                .insert_one(
                    e,
                    MonsterAnimState {
                        direction: packet.direction,
                        action: initial_action,
                        start_time: Instant::now(),
                    },
                )
                .is_err()
            {
                if let Ok(mut s) = ctx.world.get::<&mut MonsterAnimState>(e) {
                    s.direction = packet.direction;
                    s.action = initial_action;
                    s.start_time = Instant::now();
                }
            }

            // 最小血条支撑：若无服务器 HP 信息，则给一个默认血池，保证可见
            if ctx.world.get::<&crate::components::Health>(e).is_err() {
                let _ = ctx.world.insert_one(e, crate::components::Health { current: 100, max: 100 });
            }
        }
    }

    fn apply_object_npc(ctx: &mut GameContext, packet: mir2_shared::packets::server::ObjectNpc) {
        use crate::components::network::NetworkObjectType;
        use crate::components::NameColor;

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

        // 同步 NPC 名称（用于悬停显示/交互提示）
        if let Some(e) = Self::find_entity_by_object_id(ctx, packet.object_id) {
            if ctx
                .world
                .insert_one(
                    e,
                    crate::components::NPC::new(packet.name.clone(), format!("npc:{}", packet.image)),
                )
                .is_err()
            {
                if let Ok(mut npc) = ctx.world.get::<&mut crate::components::NPC>(e) {
                    npc.name = packet.name.clone();
                }
            }

            // 对齐 C#：NPCObject.NameColour
            if ctx.world.insert_one(e, NameColor(packet.name_colour)).is_err() {
                if let Ok(mut c) = ctx.world.get::<&mut NameColor>(e) {
                    c.0 = packet.name_colour;
                }
            }
        }
    }

    fn apply_object_remove(ctx: &mut GameContext, object_id: u32) {
        if let Some(e) = Self::find_entity_by_object_id(ctx, object_id) {
            // 对齐原版：不要因为 ObjectRemove 把本地玩家实体删掉。
            // 服务器可能在切图/传送等边界广播 ObjectRemove；本地玩家应由 UserInformation/MapChanged 重建位置。
            if ctx.world.get::<&crate::components::LocalPlayer>(e).is_ok() {
                tracing::warn!("[NETRECV] Ignored ObjectRemove for LocalPlayer: object_id={}", object_id);
                return;
            }
            let _ = ctx.world.despawn(e);
        }
    }

    fn apply_object_move(ctx: &mut GameContext, object_id: u32, x: i32, y: i32) {
        use crate::components::{LocalPlayer, Position};

        let Some(e) = Self::find_entity_by_object_id(ctx, object_id) else {
            return;
        };

        // 本地玩家位置：
        // - 默认由客户端 MovementSystem 驱动（连续像素移动）
        // 若直接落地，会在“AI->手动/同步开关/回包滞后”场景出现 rubber-banding（瞬间回拽到旧坐标）。
        // 因此：非 server-authoritative movement 时，忽略本地玩家的“日常移动包”的位置落地。
        // 例外：
        // - 初次没有 Position（初始化需要落地）
        // - 死亡/复活/回城等强制对齐（通过 Health<=0 放行；或走 PlayerLocationChanged）
        let has_pos = ctx.world.get::<&Position>(e).is_ok();
        let is_local = ctx.world.get::<&LocalPlayer>(e).is_ok();
        if is_local && !ctx.session.server_authoritative_movement && has_pos {
            let dead = ctx
                .world
                .get::<&crate::components::Health>(e)
                .ok()
                .map(|hp| hp.current <= 0)
                .unwrap_or(false);
            if !dead {
                return;
            }
        }

        let (wx, wy) = crate::coord::Coord::grid_to_world_center(x, y);
        if has_pos {
            if let Ok(mut pos) = ctx.world.get::<&mut Position>(e) {
                pos.x = wx;
                pos.y = wy;
            }
        } else {
            let _ = ctx.world.insert_one(e, Position::new(wx, wy));
        }
    }

    fn apply_object_turn(ctx: &mut GameContext, packet: mir2_shared::packets::server::ObjectTurn) {
        use crate::components::{MonsterAnimState, Player};
        use std::time::Instant;

        // 诊断：用于对照客户端发步进与服务器回包。
        // 只对本地玩家打印，避免刷屏。
        if Self::net_recv_diag_enabled() {
            if let Some(e) = Self::find_entity_by_object_id(ctx, packet.object_id) {
                let is_local = ctx.world.get::<&crate::components::LocalPlayer>(e).is_ok();
                if is_local {
                    let before_grid = ctx
                        .world
                        .get::<&crate::components::Position>(e)
                        .ok()
                        .map(|p| crate::coord::Coord::world_to_grid(p.x, p.y));
                    tracing::info!(
                        "[NETRECV] ObjectTurn: id={} loc=({},{}) dir={:?} local_before={:?}",
                        packet.object_id,
                        packet.location_x,
                        packet.location_y,
                        packet.direction,
                        before_grid
                    );
                }
            }
        }

        // 本地玩家默认不做位置落地（除非 server-authoritative 或强制对齐）。
        // 这里保持 apply_object_move 的既有规则。
        Self::apply_object_move(ctx, packet.object_id, packet.location_x, packet.location_y);
        let Some(e) = Self::find_entity_by_object_id(ctx, packet.object_id) else {
            return;
        };

        if let Ok(mut p) = ctx.world.get::<&mut Player>(e) {
            p.direction = packet.direction;
        }

        // Monster：仅更新方向
        let is_monster = ctx.world.get::<&crate::components::Monster>(e).is_ok();
        if is_monster {
            if ctx
                .world
                .insert_one(
                    e,
                    MonsterAnimState {
                        direction: packet.direction,
                        action: crate::components::MirAction::Standing,
                        start_time: Instant::now(),
                    },
                )
                .is_err()
            {
                if let Ok(mut s) = ctx.world.get::<&mut MonsterAnimState>(e) {
                    s.direction = packet.direction;
                    // 仅转向不重置 start_time，避免站立动画跳帧
                }
            }
        }
    }

    fn apply_object_walk(ctx: &mut GameContext, packet: mir2_shared::packets::server::ObjectWalk) {
        use crate::components::{LocalPlayer, MonsterAnimState, Player, PlayerAction, Position, PositionInterpolation, RemoteMoveAnim};
        use std::time::Instant;

        let Some(e) = Self::find_entity_by_object_id(ctx, packet.object_id) else {
            return;
        };

        let is_local = ctx.world.get::<&LocalPlayer>(e).is_ok();
        let (wx, wy) = crate::coord::Coord::grid_to_world_center(packet.location_x, packet.location_y);

        let now_secs = macroquad::prelude::get_time();

        if is_local {
            // 本地玩家：保持原语义（若未来启用 server_authoritative_movement 才会纠偏）
            let has_pos = ctx.world.get::<&Position>(e).is_ok();
            let dead = ctx
                .world
                .get::<&crate::components::Health>(e)
                .ok()
                .map(|hp| hp.current <= 0)
                .unwrap_or(false);
            let will_apply = ctx.session.server_authoritative_movement || !has_pos || dead;

            if Self::net_recv_diag_enabled() {
                let before_grid = ctx
                    .world
                    .get::<&Position>(e)
                    .ok()
                    .map(|p| crate::coord::Coord::world_to_grid(p.x, p.y));
                tracing::info!(
                    "[NETRECV] ObjectWalk(local): id={} loc=({},{}) dir={:?} will_apply={} local_before={:?}",
                    packet.object_id,
                    packet.location_x,
                    packet.location_y,
                    packet.direction,
                    will_apply,
                    before_grid
                );
            }

            if will_apply {
                Self::apply_object_move(ctx, packet.object_id, packet.location_x, packet.location_y);
            }
        } else {
            // 远程玩家：插值到目标点，消除“瞬移感”
            let existing_pos = ctx.world.get::<&Position>(e).ok().map(|pos| (pos.x, pos.y));
            let (sx, sy) = match existing_pos {
                Some(v) => v,
                None => {
                    // 无 Position 时直接落地
                    let _ = ctx.world.insert_one(e, Position::new(wx, wy));
                    (wx, wy)
                }
            };

            // 按实际跨越的格子数缩放插值时长：
            // - Walk 通常是 1 格；Run 往往是 2 格
            // - 若跨越过大（比如卡顿/纠偏/瞬移），直接落地，避免“慢慢滑过去”
            let start_grid = crate::coord::Coord::world_to_grid(sx, sy);
            let target_grid = (packet.location_x, packet.location_y);
            let step_dx = (target_grid.0 - start_grid.0).abs();
            let step_dy = (target_grid.1 - start_grid.1).abs();
            let steps = step_dx.max(step_dy);

            let base_duration = ctx.session.remote_player_walk_interp_secs;
            if base_duration > 0.0 && steps == 1 && ((sx - wx).abs() > 0.01 || (sy - wy).abs() > 0.01) {
                let interp = PositionInterpolation::new(sx, sy, wx, wy, now_secs, base_duration);
                if ctx.world.insert_one(e, interp).is_err() {
                    if let Ok(mut i) = ctx.world.get::<&mut PositionInterpolation>(e) {
                        *i = interp;
                    }
                }
            } else if base_duration <= 0.0 || steps > 1 {
                // 配置禁用插值：直接落地
                Self::apply_object_move(ctx, packet.object_id, packet.location_x, packet.location_y);
            }

            // 记录一个“预计动作结束时间”，用于自动回 Stand。
            // 即使未启用插值，也需要回站立（否则远程会永久 Walk）。
            let anim_duration = if base_duration > 0.0 { base_duration } else { 0.16 };
            let timer = RemoteMoveAnim {
                end_time: now_secs + anim_duration as f64,
            };
            if ctx.world.insert_one(e, timer).is_err() {
                if let Ok(mut t) = ctx.world.get::<&mut RemoteMoveAnim>(e) {
                    *t = timer;
                }
            }
        }

        if let Ok(mut p) = ctx.world.get::<&mut Player>(e) {
            p.direction = packet.direction;
            p.action = PlayerAction::Walk;
        }

        // Monster：走路动作
        if ctx.world.get::<&crate::components::Monster>(e).is_ok() {
            if ctx
                .world
                .insert_one(
                    e,
                    MonsterAnimState {
                        direction: packet.direction,
                        action: crate::components::MirAction::Walking,
                        start_time: Instant::now(),
                    },
                )
                .is_err()
            {
                if let Ok(mut s) = ctx.world.get::<&mut MonsterAnimState>(e) {
                    s.direction = packet.direction;
                    s.action = crate::components::MirAction::Walking;
                    s.start_time = Instant::now();
                }
            }
        }
    }

    fn apply_object_run(ctx: &mut GameContext, packet: mir2_shared::packets::server::ObjectRun) {
        use crate::components::{LocalPlayer, Player, PlayerAction, Position, PositionInterpolation, RemoteMoveAnim};
        use crate::components::MonsterAnimState;
        use std::time::Instant;

        let Some(e) = Self::find_entity_by_object_id(ctx, packet.object_id) else {
            return;
        };

        let is_local = ctx.world.get::<&LocalPlayer>(e).is_ok();
        let (wx, wy) = crate::coord::Coord::grid_to_world_center(packet.location_x, packet.location_y);

        let now_secs = macroquad::prelude::get_time();

        if is_local {
            let has_pos = ctx.world.get::<&Position>(e).is_ok();
            let dead = ctx
                .world
                .get::<&crate::components::Health>(e)
                .ok()
                .map(|hp| hp.current <= 0)
                .unwrap_or(false);
            let will_apply = ctx.session.server_authoritative_movement || !has_pos || dead;

            if Self::net_recv_diag_enabled() {
                let before_grid = ctx
                    .world
                    .get::<&Position>(e)
                    .ok()
                    .map(|p| crate::coord::Coord::world_to_grid(p.x, p.y));
                tracing::info!(
                    "[NETRECV] ObjectRun(local): id={} loc=({},{}) dir={:?} will_apply={} local_before={:?}",
                    packet.object_id,
                    packet.location_x,
                    packet.location_y,
                    packet.direction,
                    will_apply,
                    before_grid
                );
            }

            if will_apply {
                Self::apply_object_move(ctx, packet.object_id, packet.location_x, packet.location_y);
            }
        } else {
            let existing_pos = ctx.world.get::<&Position>(e).ok().map(|pos| (pos.x, pos.y));
            let (sx, sy) = match existing_pos {
                Some(v) => v,
                None => {
                    let _ = ctx.world.insert_one(e, Position::new(wx, wy));
                    (wx, wy)
                }
            };

            // Run 包可能一次跨越 2 格；按跨越格数缩放时长，避免看起来“跑太快”。
            // 若跨越过大（>2 格），认为是纠偏/瞬移，直接落地。
            let start_grid = crate::coord::Coord::world_to_grid(sx, sy);
            let target_grid = (packet.location_x, packet.location_y);
            let step_dx = (target_grid.0 - start_grid.0).abs();
            let step_dy = (target_grid.1 - start_grid.1).abs();
            let steps = step_dx.max(step_dy);

            let base_duration = ctx.session.remote_player_run_interp_secs;
            if base_duration > 0.0 && (steps == 1 || steps == 2) && ((sx - wx).abs() > 0.01 || (sy - wy).abs() > 0.01) {
                let duration = base_duration * steps as f32;
                let interp = PositionInterpolation::new(sx, sy, wx, wy, now_secs, duration);
                if ctx.world.insert_one(e, interp).is_err() {
                    if let Ok(mut i) = ctx.world.get::<&mut PositionInterpolation>(e) {
                        *i = interp;
                    }
                }
            } else if base_duration <= 0.0 || steps > 2 {
                Self::apply_object_move(ctx, packet.object_id, packet.location_x, packet.location_y);
            }

            // 同 Walk：记录一个到期时间，保证远程不会永久 Run。
            let anim_duration = if base_duration > 0.0 {
                base_duration * steps.max(1) as f32
            } else {
                0.11 * steps.max(1) as f32
            };
            let timer = RemoteMoveAnim {
                end_time: now_secs + anim_duration as f64,
            };
            if ctx.world.insert_one(e, timer).is_err() {
                if let Ok(mut t) = ctx.world.get::<&mut RemoteMoveAnim>(e) {
                    *t = timer;
                }
            }
        }

        if let Ok(mut p) = ctx.world.get::<&mut Player>(e) {
            p.direction = packet.direction;
            p.action = PlayerAction::Run;
        }

        // Monster：默认复用 Walking（DefaultMonster 没有 Running）
        if ctx.world.get::<&crate::components::Monster>(e).is_ok() {
            if ctx
                .world
                .insert_one(
                    e,
                    MonsterAnimState {
                        direction: packet.direction,
                        action: crate::components::MirAction::Walking,
                        start_time: Instant::now(),
                    },
                )
                .is_err()
            {
                if let Ok(mut s) = ctx.world.get::<&mut MonsterAnimState>(e) {
                    s.direction = packet.direction;
                    s.action = crate::components::MirAction::Walking;
                    s.start_time = Instant::now();
                }
            }
        }
    }

    fn apply_object_attack(ctx: &mut GameContext, packet: mir2_shared::packets::server::ObjectAttack) {
        use crate::components::{AttackState, LocalPlayer, MirAction, Monster, MonsterAnimState, Player, PlayerAction, Position};
        use std::time::Instant;
        let Some(e) = Self::find_entity_by_object_id(ctx, packet.object_id) else {
            return;
        };

        // 远程对象：不要在每个攻击包上都硬矫正位置，否则会把 walk/run 的插值打断，导致“瞬移/抽风”。
        // 只有差距较大（例如>2格）才强制矫正。
        let is_local = ctx.world.get::<&LocalPlayer>(e).is_ok();
        if is_local {
            Self::apply_object_move(ctx, packet.object_id, packet.location_x as i32, packet.location_y as i32);
        } else {
            let should_apply_pos = match ctx.world.get::<&Position>(e) {
                Ok(pos) => {
                    let (gx, gy) = crate::coord::Coord::world_to_grid(pos.x, pos.y);
                    let dx = (gx - packet.location_x as i32).abs();
                    let dy = (gy - packet.location_y as i32).abs();
                    dx.max(dy) > 2
                }
                Err(_) => true,
            };
            if should_apply_pos {
                Self::apply_object_move(ctx, packet.object_id, packet.location_x as i32, packet.location_y as i32);
            }
        }

        let dir = match mir2_shared::enums::MirDirection::try_from(packet.direction) {
            Ok(d) => d,
            Err(_) => mir2_shared::enums::MirDirection::Down,
        };

        let attack_action = match packet.attack_type {
            1 => PlayerAction::Attack2,
            2 => PlayerAction::Attack3,
            _ => PlayerAction::Attack1,
        };

        if let Ok(mut p) = ctx.world.get::<&mut Player>(e) {
            p.direction = dir;
            p.action = attack_action;
        }

        let mut need_insert_attack_state = true;
        {
            if let Ok(mut s) = ctx.world.get::<&mut AttackState>(e) {
                need_insert_attack_state = false;
                // 远程对象：每次攻击包都应重置攻击起点（否则动画/音效只能触发一次）
                if !is_local {
                    s.start_time = Instant::now();
                    s.attack_type = attack_action;
                    s.server_attack_type = packet.attack_type;
                }
            }
        }

        if need_insert_attack_state {
            let _ = ctx.world.insert_one(
                e,
                AttackState {
                    start_time: Instant::now(),
                    attack_type: attack_action,
                    server_attack_type: packet.attack_type,
                },
            );
        }

        // Monster：进入攻击动作（用于动画帧与帧事件/音效规则）
        if ctx.world.get::<&Monster>(e).is_ok() {
            // 与 AttackState 对齐：若远程更新了 start_time，这里用当前值；否则用 now
            let start_time = ctx
                .world
                .get::<&AttackState>(e)
                .ok()
                .map(|s| s.start_time)
                .unwrap_or_else(Instant::now);

            // 对齐原版：spell!=0 通常代表远程/技能攻击（AttackRange*）
            let is_ranged = packet.spell != 0;

            let action = if is_ranged {
                match packet.attack_type {
                    1 => MirAction::AttackRange2,
                    2 => MirAction::AttackRange3,
                    _ => MirAction::AttackRange1,
                }
            } else {
                match packet.attack_type {
                    1 => MirAction::Attack2,
                    2 => MirAction::Attack3,
                    3 => MirAction::Attack4,
                    4 => MirAction::Attack5,
                    _ => MirAction::Attack1,
                }
            };

            if ctx
                .world
                .insert_one(
                    e,
                    MonsterAnimState {
                        direction: dir,
                        action,
                        start_time,
                    },
                )
                .is_err()
            {
                if let Ok(mut s) = ctx.world.get::<&mut MonsterAnimState>(e) {
                    s.direction = dir;
                    s.action = action;
                    s.start_time = start_time;
                }
            }
        }

        // ===== 音效：攻击音效改为由 AnimationSystem 按“动作起始帧”触发（更贴近 C#：SetAction 时播放）
    }
}

impl LogicSystem for NetworkApplySystem {
    fn update(&mut self, ctx: &mut GameContext, _delay_time: f32) -> GameResult {
        if !ctx.events().has_network_events() {
            return Ok(());
        }

        // 同一帧内 mock/网络层可能会对同一对象推送多条 walk/run/attack。
        // 这里做一次“按 object_id 合并取最后一条”，避免插值/动画被频繁打断导致瞬移/抽风。
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

        let mut object_monsters: Vec<mir2_shared::packets::server::ObjectMonster> = Vec::new();
        let mut object_npcs: Vec<mir2_shared::packets::server::ObjectNpc> = Vec::new();
        let mut object_players: Vec<mir2_shared::packets::server::ObjectPlayer> = Vec::new();
        let mut object_removes: Vec<u32> = Vec::new();
        let mut object_moves: HashMap<u32, RemoteMovePacket> = HashMap::new();
        let mut object_turns: Vec<mir2_shared::packets::server::ObjectTurn> = Vec::new();
        let mut object_attacks: HashMap<u32, mir2_shared::packets::server::ObjectAttack> = HashMap::new();

        // 本地玩家：server-driven 状态（对齐真服）
        let mut player_location_changed: Option<(i32, i32)> = None;
        let mut gold_delta_sum: i32 = 0;
        let mut items_gained: Vec<mir2_shared::data::item::UserItem> = Vec::new();
        let mut items_lost: Vec<u64> = Vec::new();
        let mut items_moved: Vec<(u32, u32)> = Vec::new();

        // combat feedback
        let mut player_health_changed: Option<(u32, u32)> = None;
        let mut player_mana_changed: Option<(u32, u32)> = None;
        let mut object_struck: Vec<(u32, u32, i32)> = Vec::new();
        let mut object_died: Vec<u32> = Vec::new();

        // combat aux packets
        let mut damage_indicators: Vec<(u32, i32, u8)> = Vec::new();
        let mut object_health_percents: Vec<(u32, u8, u16)> = Vec::new();

        // UI / presentation feedback
        let mut play_sounds: Vec<i32> = Vec::new();
        let mut mount_updates: Vec<(u32, i16, bool)> = Vec::new();
        let mut player_inspects: Vec<mir2_shared::packets::server::PlayerInspect> = Vec::new();

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
                NetworkEvent::ObjectRemove { packet } => {
                    object_removes.push(packet.object_id);
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
                NetworkEvent::ObjectAttack { packet } => {
                    object_attacks.insert(packet.object_id, packet.clone());
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

                // ===== combat feedback =====
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
                NetworkEvent::ObjectStruck {
                    object_id,
                    attacker_id,
                    damage,
                    ..
                } => {
                    object_struck.push((*object_id, *attacker_id, *damage));
                }
                NetworkEvent::ObjectDied { object_id } => {
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

        for packet in player_inspects {
            Self::apply_player_inspect(ctx, packet);
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

        // server-driven objects
        for packet in object_players {
            Self::apply_object_player(ctx, packet);
        }
        for packet in object_monsters {
            Self::apply_object_monster(ctx, packet);
        }
        for packet in object_npcs {
            Self::apply_object_npc(ctx, packet);
        }
        for object_id in object_removes {
            Self::apply_object_remove(ctx, object_id);
        }
        for p in object_turns {
            Self::apply_object_turn(ctx, p);
        }
        for (_, p) in object_moves {
            match p {
                RemoteMovePacket::Walk(p) => Self::apply_object_walk(ctx, p),
                RemoteMovePacket::Run(p) => Self::apply_object_run(ctx, p),
            }
        }
        for (_, p) in object_attacks {
            Self::apply_object_attack(ctx, p);
        }

        // ===== server-driven: local player state落地 =====
        // 备注：LocalPlayer 一般由 UserInformation 创建；若不存在则跳过。
        let local_player_entity = {
            use crate::components::LocalPlayer;
            let mut q = ctx.world.query::<&LocalPlayer>();
            q.iter().next().map(|(e, _)| e)
        };

        // 坐骑更新：按 object_id 落地（本地玩家没有 NetworkSync，因此需要用 PlayerData.object_id 匹配）
        if !mount_updates.is_empty() {
            use crate::components::{MountState, MountStatus, PlayerData, SoundTrigger, SoundType};

            for (object_id, mount_type, riding_mount) in mount_updates {
                let target_entity =
                    if let Some(e) = Self::find_entity_by_object_id(ctx, object_id) {
                        Some(e)
                    } else {
                        // local player path: match by PlayerData.object_id
                        let mut q = ctx.world.query::<(&crate::components::LocalPlayer, &PlayerData)>();
                        q.iter()
                            .find(|(_, (_lp, pd))| pd.object_id == object_id)
                            .map(|(e, _)| e)
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
                    } else {
                        if mount_type < 7 { Some(10219) } else if mount_type < 12 { Some(10189) } else { None }
                    };
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
            // 血量同步（来自服务器；优先于本地推断）
            if let Some((cur, max)) = player_health_changed {
                let mut inserted = false;
                {
                    if let Ok(mut hp) = ctx.world.get::<&mut crate::components::Health>(e) {
                        hp.current = cur as i32;
                        if max != 0 {
                            hp.max = max as i32;
                        } else if hp.max < hp.current {
                            // max 未知：至少保证 max >= current
                            hp.max = hp.current;
                        }
                        inserted = true;
                    }
                }
                if !inserted {
                    let _ = ctx.world.insert_one(
                        e,
                        crate::components::Health {
                            current: cur as i32,
                            max: if max != 0 { max as i32 } else { cur as i32 },
                        },
                    );
                }

                // 复活/回血：清掉死亡动画状态
                if (cur as i32) > 0 {
                    let _ = ctx.world.remove_one::<crate::components::DeathState>(e);
                }
            }

            // 魔法同步（来自服务器）
            if let Some((cur, max)) = player_mana_changed {
                let mut inserted = false;
                {
                    if let Ok(mut mp) = ctx.world.get::<&mut crate::components::Mana>(e) {
                        mp.current = cur as i32;
                        if max != 0 {
                            mp.max = max as i32;
                        } else if mp.max < mp.current {
                            mp.max = mp.current;
                        }
                        inserted = true;
                    }
                }
                if !inserted {
                    let _ = ctx.world.insert_one(
                        e,
                        crate::components::Mana {
                            current: cur as i32,
                            max: if max != 0 { max as i32 } else { cur as i32 },
                        },
                    );
                }
            }

            // 位置校正（格子坐标 -> 世界像素）
            // 仅在“服务器权威移动”开启时落地；否则会与本地 MovementSystem 双驱动，导致抖动/乱跳。
            if let Some((gx, gy)) = player_location_changed {
                let should_apply = if ctx.session.server_authoritative_movement {
                    true
                } else {
                    // 非 server-authoritative movement 时，只允许“死亡/复活”类修正：
                    // - Mock/真服在同步移动意图时，回包可能滞后于本地连续像素移动。
                    // - 若按“偏差足够大”触发纠偏，会出现自动寻路时被拉回起点（rubber-banding）。
                    // 因此：活着时一律不应用 PlayerLocationChanged 的位置校正。
                    //
                    // 注意：此前这里允许 "large_jump"（大跨度）时纠偏，期望覆盖“传送/回城”。
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

                    // 重置移动/攻击状态，避免“复活还在追砍/寻路”
                    if let Ok(mut input) = ctx.world.get::<&mut crate::components::PlayerInput>(e) {
                        input.move_to = None;
                        input.movement_mode = crate::components::MovementMode::None;
                        input.attack_target = None;
                        input.cast_spell = None;
                        input.spell_target_pos = None;
                        input.spell_target_entity = None;
                        input.pickup_at = None;
                        input.turn_to = None;
                    }

                    if let Ok(mut path) = ctx.world.get::<&mut crate::components::Path>(e) {
                        path.clear();
                    }
                    if let Ok(mut mv) = ctx.world.get::<&mut crate::components::MovementVelocity>(e) {
                        mv.stop();
                    }
                    if let Ok(mut m) = ctx.world.get::<&mut crate::components::Movement>(e) {
                        m.set_state(crate::components::MovementState::Idle);
                    }
                    if let Ok(mut p) = ctx.world.get::<&mut crate::components::Player>(e) {
                        p.action = crate::components::PlayerAction::Stand;
                    }
                    let _ = ctx.world.remove_one::<crate::components::AttackState>(e);
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
            let Some(target) = Self::find_entity_by_object_id(ctx, object_id) else {
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
            let Some(target) = Self::find_entity_by_object_id(ctx, object_id) else {
                continue;
            };

            // ===== 音效：受击（对齐原版） =====
            // - 怪物：BaseSound + 2 (Flinch)
            // - 玩家：PlayStruckSound（骑乘/护甲 add/按攻击者武器）
            {
                use crate::components::{Monster, MountStatus, PlayerAppearance, SoundTrigger, SoundType};

                // helper: attacker weapon shape (unknown => -1)
                let struck_weapon: i16 = Self::find_entity_by_object_id(ctx, attacker_id)
                    .and_then(|att| ctx.world.get::<&PlayerAppearance>(att).ok().map(|a| a.weapon))
                    .unwrap_or(-1);

                let monster_type = ctx.world.get::<&Monster>(target).ok().map(|m| m.monster_type);
                if let Some(monster_type) = monster_type {
                    let base = monster_type * 10;
                    let _ = ctx.world.insert_one(
                        target,
                        SoundTrigger::once((base + 2).to_string(), SoundType::CharacterAction),
                    );

                    // 动画：怪物受击动作
                    if let Ok(mut s) = ctx.world.get::<&mut crate::components::MonsterAnimState>(target) {
                        s.action = crate::components::MirAction::Struck;
                        s.start_time = std::time::Instant::now();
                    }
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
                        use rand::Rng;
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
            {
                if let Ok(mut hp) = ctx.world.get::<&mut crate::components::Health>(target) {
                    hp.take_damage(damage);

                    // 先记录是否打到 0，避免在持有 RefMut 时再改 world（会触发借用冲突）
                    killed_player_to_zero = hp.current <= 0;
                    had_hp = true;
                }
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
            let now = macroquad::prelude::get_time();
            let text = format!("{}", damage);
            ctx.world.spawn((
                crate::components::Position::new(spawn_x, spawn_y - 72.0),
                crate::components::FloatingText {
                    text,
                    start_time: now,
                    duration: 0.8,
                    rise_speed: 36.0,
                },
            ));
        }

        // ObjectDied: 标记血量为 0（ObjectRemove 可能会在后续把 entity 删掉）
        for object_id in object_died {
            if let Some(target) = Self::find_entity_by_object_id(ctx, object_id) {
                if let Ok(mut hp) = ctx.world.get::<&mut crate::components::Health>(target) {
                    hp.current = 0;
                }

                // 玩家死亡动画：挂上/重置 DeathState（Die → Dead），并停止攻击动画
                if ctx.world.get::<&crate::components::Player>(target).is_ok() {
                    let mut updated = false;
                    {
                        if let Ok(mut ds) = ctx.world.get::<&mut crate::components::DeathState>(target) {
                            ds.start_time = std::time::Instant::now();
                            ds.phase = crate::components::DeathPhase::Dying;
                            updated = true;
                        }
                    }
                    if !updated {
                        let _ = ctx.world.insert_one(target, crate::components::DeathState::new());
                    }
                    let _ = ctx.world.remove_one::<crate::components::AttackState>(target);
                }

                // 本地玩家死亡：立刻停止移动/攻击输入，避免“死了还在走/追砍”。
                if ctx.world.get::<&crate::components::LocalPlayer>(target).is_ok() {
                    if let Ok(mut input) = ctx.world.get::<&mut crate::components::PlayerInput>(target) {
                        input.move_to = None;
                        input.movement_mode = crate::components::MovementMode::None;
                        input.attack_target = None;
                        input.cast_spell = None;
                        input.spell_target_pos = None;
                        input.spell_target_entity = None;
                        input.pickup_at = None;
                        input.turn_to = None;
                    }
                    if let Ok(mut path) = ctx.world.get::<&mut crate::components::Path>(target) {
                        path.clear();
                    }
                    if let Ok(mut mv) = ctx.world.get::<&mut crate::components::MovementVelocity>(target) {
                        mv.stop();
                    }
                    if let Ok(mut m) = ctx.world.get::<&mut crate::components::Movement>(target) {
                        m.set_state(crate::components::MovementState::Idle);
                    }
                    if let Ok(mut p) = ctx.world.get::<&mut crate::components::Player>(target) {
                        p.action = crate::components::PlayerAction::Stand;
                    }
                    let _ = ctx.world.remove_one::<crate::components::AttackState>(target);
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
                        if let Ok(mut s) = ctx.world.get::<&mut crate::components::MonsterAnimState>(target) {
                            s.action = crate::components::MirAction::Die;
                            s.start_time = std::time::Instant::now();
                        }
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
            }
        }

        // ===== final reconcile: local player HP is server-authoritative =====
        // 说明：本帧内可能同时到达 ObjectStruck/ObjectDied 与 HealthChanged。
        // 为避免后续处理把本地玩家血量又改回 0（导致死亡 UI 卡住），这里在帧末再用 HealthChanged 覆盖一次。
        if let (Some(e), Some((cur, max))) = (local_player_entity, player_health_changed) {
            let mut inserted = false;
            {
                if let Ok(mut hp) = ctx.world.get::<&mut crate::components::Health>(e) {
                    hp.current = cur as i32;
                    hp.max = max as i32;
                    inserted = true;
                }
            }
            if !inserted {
                let _ = ctx.world.insert_one(
                    e,
                    crate::components::Health {
                        current: cur as i32,
                        max: max as i32,
                    },
                );
            }

            // 复活/回血：清掉死亡动画状态
            if (cur as i32) > 0 {
                let _ = ctx.world.remove_one::<crate::components::DeathState>(e);
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
