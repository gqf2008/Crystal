use crate::game::{GameContext, GameResult};
use crate::network::handlers::NetworkEvent;
use crate::systems::LogicSystem;
use rand::RngExt;

// =============================================================================
// 表现层常量（避免魔法数散落）
// =============================================================================
const PROJECTILE_SPEED: f32 = 400.0;
const FLOAT_Y_EXP_OFFSET: f32 = 64.0;
const FLOAT_Y_LEVELUP_OFFSET: f32 = 80.0;
const PARTICLE_Y_OFFSET: f32 = 20.0;
const DEATH_SMOKE_DURATION: f32 = 1.5;
const REVIVE_HEAL_DURATION: f32 = 1.0;
const LEVELUP_TEXT_DURATION: f32 = 2.0;
const LEVELUP_FONT_SIZE_PLAYER: f32 = 16.0;
const LEVELUP_FONT_SIZE_OBJECT: f32 = 12.0;
const OBJECT_LEVELUP_DURATION: f32 = 1.5;

/// Decomposed attack payload for object attacks.
#[derive(Debug, Clone, Copy)]
struct ObjectAttackData {
    location_x: u32,
    location_y: u32,
    direction: u8,
    spell: u8,
    attack_type: u8,
}

/// NetworkApplySystem - 网络事件落地系统
///
/// 职责：
/// - 消费 `EventBus.network_events` 中的 P0 关键包
/// - 把"协议层 packet"落地到 ECS 组件/资源
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

/// Walk/Run 共享的移动配置
struct RemoteMoveConfig {
    player_action: crate::components::PlayerAction,
    base_interp_secs: f32,
    interp_max_steps: i32,
    scale_interp_by_steps: bool,
    fallback_anim_secs: f32,
}

fn apply_remote_movement(
    ctx: &mut GameContext,
    entity_index: &std::collections::HashMap<u32, hecs::Entity>,
    object_id: u32,
    location_x: i32,
    location_y: i32,
    direction: crate::components::MirDirection,
    cfg: RemoteMoveConfig,
) {
    use crate::components::{LocalPlayer, MonsterAnimState, Player, Position, PositionInterpolation, RemoteMoveAnim};
    use std::time::Instant;

    let Some(e) = entity_index.get(&object_id).copied() else {
        return;
    };

    let is_local = ctx.world.get::<&LocalPlayer>(e).is_ok();
    let (wx, wy) = crate::coord::Coord::grid_to_world_center(location_x, location_y);
    let now_secs = macroquad::prelude::get_time();

    if is_local {
        let has_pos = ctx.world.get::<&Position>(e).is_ok();
        let dead = ctx.world.get::<&crate::components::Health>(e).ok().map(|hp| hp.current <= 0).unwrap_or(false);
        let will_apply = ctx.session.server_authoritative_movement || !has_pos || dead;

        if NetworkApplySystem::net_recv_diag_enabled() {
            let before_grid = ctx.world.get::<&Position>(e).ok().map(|p| crate::coord::Coord::world_to_grid(p.x, p.y));
            tracing::info!(
                "[NETRECV] {:?}(local): id={} loc=({},{}) dir={:?} will_apply={} local_before={:?}",
                cfg.player_action, object_id, location_x, location_y, direction, will_apply, before_grid
            );
        }

        if will_apply {
            NetworkApplySystem::apply_object_move(ctx, entity_index, object_id, location_x, location_y);
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

        let start_grid = crate::coord::Coord::world_to_grid(sx, sy);
        let steps = ((location_x - start_grid.0).abs()).max((location_y - start_grid.1).abs());
        let interp_dur = if cfg.scale_interp_by_steps { cfg.base_interp_secs * steps as f32 } else { cfg.base_interp_secs };

        if cfg.base_interp_secs > 0.0 && steps <= cfg.interp_max_steps && ((sx - wx).abs() > 0.01 || (sy - wy).abs() > 0.01) {
            let interp = PositionInterpolation::new(sx, sy, wx, wy, now_secs, interp_dur);
            NetworkApplySystem::upsert_component(ctx, e, interp);
        } else if cfg.base_interp_secs <= 0.0 || steps > cfg.interp_max_steps {
            NetworkApplySystem::apply_object_move(ctx, entity_index, object_id, location_x, location_y);
        }

        let anim_secs = if cfg.base_interp_secs > 0.0 { interp_dur } else { cfg.fallback_anim_secs };
        NetworkApplySystem::upsert_component(ctx, e, RemoteMoveAnim { end_time: now_secs + anim_secs as f64 });
    }

    if let Ok(mut p) = ctx.world.get::<&mut Player>(e) {
        p.direction = direction;
        p.action = cfg.player_action;
    }

    if ctx.world.get::<&crate::components::Monster>(e).is_ok() {
        NetworkApplySystem::upsert_component(ctx, e, MonsterAnimState {
            direction, action: crate::components::MirAction::Walking, start_time: Instant::now(),
        });
    }
}

impl NetworkApplySystem {
    /// 获取实体的世界坐标（不修改 ECS）。
    fn entity_position(ctx: &GameContext, entity: hecs::Entity) -> Option<(f32, f32)> {
        ctx.world.get::<&crate::components::Position>(entity).ok().map(|p| (p.x, p.y))
    }

    /// 按 object_id 从索引查找实体并获取世界坐标（O(1)）。
    fn object_position(
        world: &hecs::World,
        entity_index: &std::collections::HashMap<u32, hecs::Entity>,
        object_id: u32,
    ) -> Option<(f32, f32)> {
        let e = entity_index.get(&object_id)?;
        world.get::<&crate::components::Position>(*e).ok().map(|p| (p.x, p.y))
    }

    /// 将物品从源槽位转移到目标槽位（要求源非空、目标为空）。
    fn transfer_slot(
        src: &mut [Option<mir2_shared::data::item::UserItem>],
        dst: &mut [Option<mir2_shared::data::item::UserItem>],
        from: usize,
        to: usize,
    ) {
        if from < src.len() && to < dst.len() && src[from].is_some() && dst[to].is_none() {
            std::mem::swap(&mut src[from], &mut dst[to]);
        }
    }

    fn net_recv_diag_enabled() -> bool {
        static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *ENABLED.get_or_init(|| {
            std::env::var_os("CRYSTAL_NETRECV_DIAG").is_some()
                || std::env::var_os("CRYSTAL_NETMOVE_DIAG").is_some()
        })
    }

    fn apply_object_player(ctx: &mut GameContext, entity_index: &std::collections::HashMap<u32, hecs::Entity>, packet: mir2_shared::packets::server::ObjectPlayer) {
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

        if let Some(e) = entity_index.get(&packet.object_id).copied() {
            // NetworkSync 只要存在即可；类型不匹配时更新。
            Self::upsert_component(ctx, e, NetworkSync::new(packet.object_id, NetworkObjectType::Player));

            // 远程玩家标记
            Self::upsert_component(ctx, e, RemotePlayer { id: packet.object_id });

            // 位置
            Self::upsert_component(ctx, e, Position::new(wx, wy));

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
            Self::upsert_component(ctx, e, appearance);

            // 动画帧（若没有就补一个默认，AnimationSystem 会更新）
            if ctx.world.get::<&AnimationFrame>(e).is_err() {
                let _ = ctx.world.insert_one(e, AnimationFrame::default());
            }

            // 坐骑：按服务器 ObjectPlayer 落地（用于行为对齐，如攻击音效选择）
            Self::upsert_component(ctx, e, MountState { mount_index: mount_index_from_packet });
            Self::upsert_component(ctx, e, MountStatus {
                mount_type: packet.mount_type,
                riding_mount: packet.riding_mount,
            });

            // 基本身份信息（未来做名字/血条会用到）
            Self::upsert_component(ctx, e, {
                let mut op = OtherPlayer::new(packet.name.clone(), packet.class, packet.gender, packet.level);
                op.guild_name = if packet.guild_name.is_empty() {
                    None
                } else {
                    Some(packet.guild_name.clone())
                };
                op
            });

            if ctx.world.get::<&crate::components::Health>(e).is_err() {
                let current = if packet.dead { 0 } else { 100 };
                let _ = ctx.world.insert_one(e, crate::components::Health { current, max: 100 });
            }

            if packet.dead && ctx.world.get::<&crate::components::DeathState>(e).is_err() {
                let _ = ctx.world.insert_one(e, crate::components::DeathState::new());
            }

            Self::upsert_visibility(ctx, e, packet.hidden, packet.dead);

            Self::upsert_component(ctx, e, crate::components::NameColor(packet.name_colour));
            Self::upsert_component(ctx, e, crate::components::LevelEffectsFlags(packet.level_effects));

            if !packet.poison.is_empty() {
                Self::apply_poison_to_entity(ctx, e, packet.poison);
            }
            if !packet.buffs.is_empty() {
                Self::apply_buffs_to_entity(ctx, e, &packet.buffs);
            }
        } else {
            let new_entity = ctx.world.spawn((
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
                OtherPlayer::new(packet.name.clone(), packet.class, packet.gender, packet.level),
            ));

            let _ = ctx.world.insert_one(
                new_entity,
                crate::components::Visibility { hidden: packet.hidden, dead: packet.dead },
            );
            let _ = ctx.world.insert_one(new_entity, crate::components::NameColor(packet.name_colour));
            let _ = ctx.world.insert_one(new_entity, crate::components::LevelEffectsFlags(packet.level_effects));

            let current = if packet.dead { 0 } else { 100 };
            let _ = ctx.world.insert_one(new_entity, crate::components::Health { current, max: 100 });
            if packet.dead {
                let _ = ctx.world.insert_one(new_entity, crate::components::DeathState::new());
            }

            if !packet.poison.is_empty() {
                Self::apply_poison_to_entity(ctx, new_entity, packet.poison);
            }
            if !packet.buffs.is_empty() {
                Self::apply_buffs_to_entity(ctx, new_entity, &packet.buffs);
            }
        }
    }
    /// 将 Spell 枚举映射到表现层 ProjectileType
    fn spell_to_projectile_type(spell: u8) -> Option<crate::event_bus::ProjectileType> {
        use crate::event_bus::ProjectileType;
        use mir2_shared::enums::Spell;
        let spell_enum = Spell::try_from(spell).ok()?;
        match spell_enum {
            Spell::FireBall | Spell::GreatFireBall | Spell::HellFire | Spell::FireBang
            | Spell::FlameDisruptor | Spell::SoulFireBall | Spell::FireBurst => Some(ProjectileType::Fireball),
            Spell::ThunderBolt | Spell::Lightning | Spell::ThunderStorm | Spell::ElectricShock => Some(ProjectileType::Lightning),
            Spell::FrostCrunch | Spell::IceStorm | Spell::Blizzard | Spell::IceThrust => Some(ProjectileType::IceBolt),
            Spell::StraightShot | Spell::DoubleShot | Spell::ElementalShot | Spell::BackStep => Some(ProjectileType::Arrow),
            _ => None,
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
            ctx.world.iter().find_map(|e| e.get::<&LocalPlayer>().map(|_| e.entity()))
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
        Self::upsert_component(ctx, local_entity, PlayerData {
            id: packet.real_id,
            object_id: packet.object_id,
            name: packet.name.clone(),
            class: packet.class,
            gender: packet.gender,
            level: packet.level,
        });

        // 经验值：若缺失则兜底创建
        if ctx.world.get::<&Experience>(local_entity).is_err() {
            let _ = ctx.world.insert_one(local_entity, Experience::new(packet.level));
        }
        // 更新经验值（协议携带）
        if let Ok(mut exp) = ctx.world.get::<&mut Experience>(local_entity) {
            exp.current = packet.experience;
            exp.required = packet.max_experience;
        }

        // 公会/称号/名字颜色/等级特效/观战状态/英雄/召唤物：先落地到组件，后续 UI/表现再消费
        Self::upsert_component(ctx, local_entity, NameColor(packet.name_colour));

        // GuildInfo 只更新部分字段（完整数据来自 GuildJoined），需要保留现有字段
        let has_guild = ctx.world.get::<&GuildInfo>(local_entity).is_ok();
        if has_guild {
            if let Ok(mut ge) = ctx.world.get::<&mut GuildInfo>(local_entity) {
                ge.name = packet.guild_name.clone();
                ge.rank = packet.guild_rank.clone();
            }
        } else {
            let _ = ctx.world.insert_one(
                local_entity,
                GuildInfo {
                    name: packet.guild_name.clone(),
                    rank: packet.guild_rank.clone(),
                    ..Default::default()
                },
            );
        }

        Self::upsert_component(ctx, local_entity, LevelEffectsFlags(packet.level_effects));
        Self::upsert_component(ctx, local_entity, ObserveState {
            allow_observe: packet.allow_observe,
            observer: packet.observer,
        });

        // HeroState 只更新部分字段，保留现有 level/experience/hero_object_id
        let has_hero_state = ctx.world.get::<&HeroState>(local_entity).is_ok();
        if has_hero_state {
            if let Ok(mut hero) = ctx.world.get::<&mut HeroState>(local_entity) {
                hero.has_hero = packet.has_hero;
                hero.behaviour = packet.hero_behaviour;
            }
        } else {
            let _ = ctx.world.insert_one(
                local_entity,
                HeroState {
                    has_hero: packet.has_hero,
                    behaviour: packet.hero_behaviour,
                    level: 0,
                    experience: 0,
                    hero_object_id: 0,
                },
            );
        }

        Self::upsert_component(ctx, local_entity, SummonedCreatureState {
            creature_type: packet.summoned_creature_type,
            summoned: packet.creature_summoned,
        });

        // 血蓝：只更新 current，max 目前用"至少不小于 current"的策略避免 UI/逻辑出错
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
        Self::upsert_component(ctx, local_entity, Currency { gold: packet.gold, credit: packet.credit });

        // 背包/任务背包/装备
        if let Some(items) = packet.inventory.clone() {
            let has_inv = ctx.world.get::<&Inventory>(local_entity).is_ok();
            if has_inv {
                if let Ok(mut inv) = ctx.world.get::<&mut Inventory>(local_entity) {
                    inv.capacity = items.len();
                    inv.items = items;
                }
            } else {
                let mut inv = Inventory::new(items.len().max(1));
                inv.items = items;
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
                match items.first().and_then(|x| x.as_ref()) {
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
                    let weapon_shape = items.first()
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
            ctx.world.iter()
                .find_map(|e| e.get::<&OtherPlayer>().filter(|op| op.name == packet.name).map(|_| e.entity()))
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
            match packet.equipment.first().and_then(|x| x.as_ref()) {
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
                    can_use: true,
                })
            })
            .collect()
    }

    fn apply_map_changed(ctx: &mut GameContext, packet: mir2_shared::packets::server::MapChanged) {
        use crate::components::{LocalPlayer, Player, Position, WeatherState};

        // 更新天气状态
        if let Some((_ws_entity, mut ws)) = ctx.world.iter().find_map(|e| e.get::<&mut WeatherState>().map(|w| (e.entity(), w))) {
            ws.weather_code = packet.weather;
            ws.emitter_entity = None; // 重置发射器，由 WeatherSystem 重建
        }

        // MapChanged 里携带了落点与朝向（切图/传送时很关键）
        let Some((entity, _)) = ctx.world.iter().find_map(|e| e.get::<&LocalPlayer>().map(|lp| (e.entity(), lp))) else {
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
            ctx.world.iter()
                .find_map(|e| e.get::<&NetworkSync>().filter(|ns| ns.object_id == object_id).map(|_| e.entity()))
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
            ctx.world.iter()
                .find_map(|e| e.get::<&NetworkSync>().filter(|ns| ns.object_id == object_id).map(|_| e.entity()))
        };

        if let Some(e) = entity {
            let _ = ctx.world.despawn(e);
        }
    }

    /// 构建 object_id → Entity 索引，用于 O(1) 替换 deferred 循环中的线性扫描。
    fn build_object_index(world: &hecs::World) -> std::collections::HashMap<u32, hecs::Entity> {
        use crate::components::network::NetworkSync;
        use crate::components::{LocalPlayer, PlayerData};
        let mut map = std::collections::HashMap::new();
        for entity in world.iter() {
            if let Some(ns) = entity.get::<&NetworkSync>() {
                map.insert(ns.object_id, entity.entity());
            }
        }
        for entity in world.iter() {
            if entity.get::<&LocalPlayer>().is_some() {
                if let Some(pd) = entity.get::<&PlayerData>() {
                    map.entry(pd.object_id).or_insert(entity.entity());
                }
            }
        }
        map
    }

    /// 辅助：对英雄实体的 MagicList 执行操作（无则静默跳过）。
    fn with_hero_magic_list<F>(ctx: &mut GameContext, entity_index: &std::collections::HashMap<u32, hecs::Entity>, local_player_entity: Option<hecs::Entity>, f: F)
    where
        F: FnOnce(&mut crate::components::spell::MagicList),
    {
        let hero_id = local_player_entity
            .and_then(|e| ctx.world.get::<&crate::components::HeroState>(e).ok())
            .map(|h| h.hero_object_id)
            .unwrap_or(0);
        if hero_id == 0 {
            return;
        }
        let Some(hero_entity) = entity_index.get(&hero_id).copied() else {
            return;
        };
        if let Ok(mut magic_list) = ctx.world.get::<&mut crate::components::spell::MagicList>(hero_entity) {
            f(&mut magic_list);
        }
    }

    fn find_magic_mut(
        magics: &mut [crate::components::spell::LearnedMagic],
        spell: u8,
    ) -> Option<&mut crate::components::spell::LearnedMagic> {
        magics.iter_mut().find(|m| m.spell as u8 == spell)
    }

    fn update_learned_magic(
        magic_list: &mut crate::components::spell::MagicList,
        spell: u8,
        level: u8,
        experience: u32,
        key: u8,
    ) {
        if let Some(existing) = Self::find_magic_mut(&mut magic_list.magics, spell) {
            existing.level = level;
            existing.experience = experience;
            existing.key_slot = if key > 0 { Some(key) } else { None };
        } else if let Ok(spell_type) = spell.try_into() {
            let mut learned = crate::components::spell::LearnedMagic::new(spell_type);
            learned.level = level;
            learned.experience = experience;
            learned.key_slot = if key > 0 { Some(key) } else { None };
            magic_list.magics.push(learned);
        }
    }

    fn update_magic_level(magic_list: &mut crate::components::spell::MagicList, spell: u8, level: u8) {
        if let Some(magic) = Self::find_magic_mut(&mut magic_list.magics, spell) {
            magic.level = level;
        }
    }

    fn update_spell_toggle(magic_list: &mut crate::components::spell::MagicList, spell: u8, can_use: bool) {
        if let Some(m) = Self::find_magic_mut(&mut magic_list.magics, spell) {
            m.can_use = can_use;
        }
    }

    fn remove_magic(magic_list: &mut crate::components::spell::MagicList, spell: u8) {
        magic_list.magics.retain(|m| m.spell as u8 != spell);
    }

    /// 辅助：设置怪物/NPC 动画状态（O(1)，无则静默跳过）。
    fn set_monster_anim(
        world: &hecs::World,
        entity_index: &std::collections::HashMap<u32, hecs::Entity>,
        object_id: u32,
        action: Option<crate::components::MirAction>,
        direction: Option<crate::components::MirDirection>,
    ) {
        if let Some(&e) = entity_index.get(&object_id) {
            if let Ok(mut s) = world.get::<&mut crate::components::MonsterAnimState>(e) {
                if let Some(a) = action {
                    s.action = a;
                    s.start_time = std::time::Instant::now();
                }
                if let Some(d) = direction {
                    s.direction = d;
                }
            }
        }
    }

    fn upsert_library_sprite_object(
        ctx: &mut GameContext,
        entity_index: &std::collections::HashMap<u32, hecs::Entity>,
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

        if let Some(e) = entity_index.get(&object_id).copied() {
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

    fn apply_object_monster(ctx: &mut GameContext, entity_index: &std::collections::HashMap<u32, hecs::Entity>, packet: mir2_shared::packets::server::ObjectMonster) {
        use crate::components::network::NetworkObjectType;
        use crate::components::{MirAction, MonsterAnimState, SoundTrigger, SoundType};
        use std::time::Instant;

        // C# 对应：Libraries.Monsters[(ushort)MonsterEnum]
        // 这里 image 直接对应 Monster/XYZ 的库索引（XYZ 三位数）
        let library = crate::resources::LibraryName::Monsters(packet.image as usize);

        // 最小可见：先画第 0 帧
        Self::upsert_library_sprite_object(
            ctx,
            entity_index,
            packet.object_id,
            NetworkObjectType::Monster,
            library,
            0,
            packet.location_x,
            packet.location_y,
        );

        // 同步怪物名称（用于悬停/调试 overlay，避免"只有贴图没有名字"）
        if let Some(e) = entity_index.get(&packet.object_id).copied() {
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
            } else if let Ok(mut m) = ctx.world.get::<&mut crate::components::Monster>(e) {
                m.stage = packet.extra_byte;
            }

            // 原版：怪物出现音效 BaseSound + 0（规则 0=Appear）
            // 这里用"首次插入 Monster 组件"近似判断首次出现。
            if inserted_monster_component {
                let base_sound = packet.image * 10;
                let _ = ctx.world.insert_one(
                    e,
                    SoundTrigger::once(base_sound.to_string(), SoundType::CharacterAction),
                );
            }

            // 动画状态：方向来自包；初始动作为 Standing/Dead（最小集）
            let initial_action = if packet.dead { MirAction::Dead } else { MirAction::Standing };
            Self::upsert_component(ctx, e, MonsterAnimState {
                direction: packet.direction,
                action: initial_action,
                start_time: Instant::now(),
            });

            // 最小血条支撑：若无服务器 HP 信息，则给一个默认血池，保证可见
            if ctx.world.get::<&crate::components::Health>(e).is_err() {
                let current = if packet.dead { 0 } else { 100 };
                let _ = ctx.world.insert_one(e, crate::components::Health { current, max: 100 });
            } else if packet.dead {
                if let Ok(mut h) = ctx.world.get::<&mut crate::components::Health>(e) {
                    h.current = 0;
                }
            }

            // 死亡状态：dead=true 时插入 DeathState（动画已在 MonsterAnimState 中处理）
            if packet.dead && ctx.world.get::<&crate::components::DeathState>(e).is_err() {
                let _ = ctx.world.insert_one(e, crate::components::DeathState::new());
            }

            Self::upsert_visibility(ctx, e, packet.hidden, packet.dead);

            // 名字颜色
            Self::upsert_component(ctx, e, crate::components::NameColor(packet.name_colour));

            if !packet.poison.is_empty() {
                Self::apply_poison_to_entity(ctx, e, packet.poison);
            }
            if !packet.buffs.is_empty() {
                Self::apply_buffs_to_entity(ctx, e, &packet.buffs);
            }
        }
    }

    fn map_server_buff(buff: mir2_shared::enums::BuffType) -> Option<crate::components::combat::BuffType> {
        use mir2_shared::enums::BuffType as S;
        use crate::components::combat::BuffType as C;
        match buff {
            S::MagicShield | S::EnergyShield => Some(C::MagicShield),
            S::SoulShield | S::BlessedArmour | S::ProtectionField | S::UltimateEnhancer
            | S::ImmortalSkin | S::ElementalBarrier | S::GeneralMeowMeowShield
            | S::HornedWarriorShield | S::HornedCommanderShield => Some(C::DefenseBoost),
            S::Fury | S::Rage | S::CounterAttack | S::HornedArcherBuff | S::ColdArcherBuff
            | S::PowerBeadBuff => Some(C::AttackBoost),
            S::Haste | S::SwiftFeet | S::LightBody => Some(C::SpeedBoost),
            S::Curse | S::RhinoPriestDebuff | S::Blindness | S::PoisonShot => Some(C::Poison),
            S::VampireShot => Some(C::Bleeding),
            _ => None,
        }
    }

    fn with_component<T: 'static + Default + Send + Sync>(
        ctx: &mut GameContext,
        entity: hecs::Entity,
        mut f: impl FnMut(&mut T),
    ) {
        if let Ok(mut c) = ctx.world.get::<&mut T>(entity) {
            f(&mut c);
            return;
        }
        let mut c = T::default();
        f(&mut c);
        let _ = ctx.world.insert_one(entity, c);
    }

    fn apply_poison_to_entity(
        ctx: &mut GameContext,
        entity: hecs::Entity,
        poison: mir2_shared::enums::PoisonType,
    ) {
        use crate::components::combat::{Buff, BuffList, BuffType};
        use mir2_shared::enums::PoisonType;
        Self::with_component::<BuffList>(ctx, entity, |bl| {
            if poison.contains(PoisonType::GREEN) || poison.contains(PoisonType::RED) {
                bl.add_buff(Buff::new(BuffType::Poison, BuffType::Poison as u32));
            }
            if poison.contains(PoisonType::BLEEDING) {
                bl.add_buff(Buff::new(BuffType::Bleeding, BuffType::Bleeding as u32));
            }
        });
    }

    fn apply_buffs_to_entity(
        ctx: &mut GameContext,
        entity: hecs::Entity,
        buffs: &[mir2_shared::enums::BuffType],
    ) {
        use crate::components::combat::{Buff, BuffList};
        Self::with_component::<BuffList>(ctx, entity, |bl| {
            for b in buffs {
                if let Some(combat_buff) = Self::map_server_buff(*b) {
                    bl.add_buff(Buff::new(combat_buff, *b as u8 as u32));
                }
            }
        });
    }

    fn with_visibility(
        ctx: &mut GameContext,
        entity: hecs::Entity,
        mut f: impl FnMut(&mut crate::components::Visibility),
    ) {
        Self::with_component::<crate::components::Visibility>(ctx, entity, |vis| f(vis));
    }

    fn upsert_visibility(
        ctx: &mut GameContext,
        entity: hecs::Entity,
        hidden: bool,
        dead: bool,
    ) {
        Self::with_visibility(ctx, entity, |vis| {
            vis.hidden = hidden;
            vis.dead = dead;
        });
    }

    fn set_visibility_dead(ctx: &mut GameContext, entity: hecs::Entity, dead: bool) {
        Self::with_visibility(ctx, entity, |vis| {
            vis.dead = dead;
        });
    }

    /// 玩家死亡/复活/回城时统一停止所有战斗/移动输入。
    fn stop_player_actions(world: &mut hecs::World, entity: hecs::Entity) {
        if let Ok(mut input) = world.get::<&mut crate::components::PlayerInput>(entity) {
            input.move_to = None;
            input.movement_mode = crate::components::MovementMode::None;
            input.attack_target = None;
            input.cast_spell = None;
            input.spell_target_pos = None;
            input.spell_target_entity = None;
            input.pickup_at = None;
            input.turn_to = None;
        }
        if let Ok(mut path) = world.get::<&mut crate::components::Path>(entity) {
            path.clear();
        }
        if let Ok(mut mv) = world.get::<&mut crate::components::MovementVelocity>(entity) {
            mv.stop();
        }
        if let Ok(mut m) = world.get::<&mut crate::components::Movement>(entity) {
            m.set_state(crate::components::MovementState::Idle);
        }
        if let Ok(mut p) = world.get::<&mut crate::components::Player>(entity) {
            p.action = crate::components::PlayerAction::Stand;
        }
        let _ = world.remove_one::<crate::components::AttackState>(entity);
    }

    /// 通用组件 upsert：存在则替换，不存在则插入
    fn upsert_component<T: 'static + Clone + Send + Sync>(
        ctx: &mut GameContext,
        entity: hecs::Entity,
        component: T,
    ) {
        if let Ok(mut c) = ctx.world.get::<&mut T>(entity) {
            *c = component;
            return;
        }
        let _ = ctx.world.insert_one(entity, component);
    }

    fn apply_object_npc(ctx: &mut GameContext, entity_index: &std::collections::HashMap<u32, hecs::Entity>, packet: mir2_shared::packets::server::ObjectNpc) {
        use crate::components::network::NetworkObjectType;
        use crate::components::NameColor;

        // C# 对应：Libraries.NPCs[Image]
        let library = crate::resources::LibraryName::Npcs(packet.image as usize);

        Self::upsert_library_sprite_object(
            ctx,
            entity_index,
            packet.object_id,
            NetworkObjectType::NPC,
            library,
            0,
            packet.location_x,
            packet.location_y,
        );

        // 同步 NPC 名称（用于悬停显示/交互提示）
        if let Some(e) = entity_index.get(&packet.object_id).copied() {
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
            Self::upsert_component(ctx, e, NameColor(packet.name_colour));
        }
    }

    fn apply_object_remove(ctx: &mut GameContext, entity_index: &std::collections::HashMap<u32, hecs::Entity>, object_id: u32) {
        if let Some(e) = entity_index.get(&object_id).copied() {
            // 对齐原版：不要因为 ObjectRemove 把本地玩家实体删掉。
            // 服务器可能在切图/传送等边界广播 ObjectRemove；本地玩家应由 UserInformation/MapChanged 重建位置。
            if ctx.world.get::<&crate::components::LocalPlayer>(e).is_ok() {
                tracing::warn!("[NETRECV] Ignored ObjectRemove for LocalPlayer: object_id={}", object_id);
                return;
            }
            let _ = ctx.world.despawn(e);
        }
    }

    fn apply_object_move(ctx: &mut GameContext, entity_index: &std::collections::HashMap<u32, hecs::Entity>, object_id: u32, x: i32, y: i32) {
        use crate::components::{LocalPlayer, Position};

        let Some(e) = entity_index.get(&object_id).copied() else {
            return;
        };

        // 本地玩家位置：
        // - 默认由客户端 MovementSystem 驱动（连续像素移动）
        // 若直接落地，会在"AI->手动/同步开关/回包滞后"场景出现 rubber-banding（瞬间回拽到旧坐标）。
        // 因此：非 server-authoritative movement 时，忽略本地玩家的"日常移动包"的位置落地。
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

    fn apply_object_turn(ctx: &mut GameContext, entity_index: &std::collections::HashMap<u32, hecs::Entity>, packet: mir2_shared::packets::server::ObjectTurn) {
        use crate::components::{MonsterAnimState, Player};
        use std::time::Instant;

        // 诊断：用于对照客户端发步进与服务器回包。
        // 只对本地玩家打印，避免刷屏。
        if Self::net_recv_diag_enabled() {
            if let Some(e) = entity_index.get(&packet.object_id).copied() {
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
        let Some(e) = entity_index.get(&packet.object_id).copied() else {
            return;
        };

        let is_local = ctx.world.get::<&crate::components::LocalPlayer>(e).is_ok();
        if is_local {
            let has_pos = ctx.world.get::<&crate::components::Position>(e).is_ok();
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
                    .get::<&crate::components::Position>(e)
                    .ok()
                    .map(|p| crate::coord::Coord::world_to_grid(p.x, p.y));
                tracing::info!(
                    "[NETRECV] ObjectTurn(local): id={} loc=({},{}) dir={:?} will_apply={} local_before={:?}",
                    packet.object_id,
                    packet.location_x,
                    packet.location_y,
                    packet.direction,
                    will_apply,
                    before_grid
                );
            }

            if will_apply {
                Self::apply_object_move(ctx, entity_index, packet.object_id, packet.location_x, packet.location_y);
            }
        }

        if let Ok(mut p) = ctx.world.get::<&mut Player>(e) {
            p.direction = packet.direction;
        }

        // Monster：仅更新方向
        let is_monster = ctx.world.get::<&crate::components::Monster>(e).is_ok();
        if is_monster
            && ctx
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

    fn apply_object_walk(ctx: &mut GameContext, entity_index: &std::collections::HashMap<u32, hecs::Entity>, packet: mir2_shared::packets::server::ObjectWalk) {
        apply_remote_movement(ctx, entity_index,
            packet.object_id, packet.location_x, packet.location_y, packet.direction,
            RemoteMoveConfig {
                player_action: crate::components::PlayerAction::Walk,
                base_interp_secs: ctx.session.remote_player_walk_interp_secs,
                interp_max_steps: 1,
                scale_interp_by_steps: false,
                fallback_anim_secs: 0.16,
            },
        );
    }

    fn apply_object_run(ctx: &mut GameContext, entity_index: &std::collections::HashMap<u32, hecs::Entity>, packet: mir2_shared::packets::server::ObjectRun) {
        apply_remote_movement(ctx, entity_index,
            packet.object_id, packet.location_x, packet.location_y, packet.direction,
            RemoteMoveConfig {
                player_action: crate::components::PlayerAction::Run,
                base_interp_secs: ctx.session.remote_player_run_interp_secs,
                interp_max_steps: 2,
                scale_interp_by_steps: true,
                fallback_anim_secs: 0.11,
            },
        );
    }

    fn apply_object_attack(ctx: &mut GameContext, entity_index: &std::collections::HashMap<u32, hecs::Entity>, object_id: u32, data: ObjectAttackData) {
        use crate::components::{AttackState, LocalPlayer, MirAction, Monster, MonsterAnimState, Player, PlayerAction, Position};
        use std::time::Instant;
        let Some(e) = entity_index.get(&object_id).copied() else {
            return;
        };

        // 远程对象：不要在每个攻击包上都硬矫正位置，否则会把 walk/run 的插值打断，导致"瞬移/抽风"。
        // 只有差距较大（例如>2格）才强制矫正。
        let is_local = ctx.world.get::<&LocalPlayer>(e).is_ok();
        if is_local {
            let has_pos = ctx.world.get::<&Position>(e).is_ok();
            let dead = ctx.world.get::<&crate::components::Health>(e).ok().map(|hp| hp.current <= 0).unwrap_or(false);
            let will_apply = ctx.session.server_authoritative_movement || !has_pos || dead;
            if will_apply {
                Self::apply_object_move(ctx, entity_index, object_id, data.location_x as i32, data.location_y as i32);
            }
        } else {
            let should_apply_pos = match ctx.world.get::<&Position>(e) {
                Ok(pos) => {
                    let (gx, gy) = crate::coord::Coord::world_to_grid(pos.x, pos.y);
                    let dx = (gx - data.location_x as i32).abs();
                    let dy = (gy - data.location_y as i32).abs();
                    dx.max(dy) > 2
                }
                Err(_) => true,
            };
            if should_apply_pos {
                Self::apply_object_move(ctx, entity_index, object_id, data.location_x as i32, data.location_y as i32);
            }
        }

        let dir = match mir2_shared::enums::MirDirection::try_from(data.direction) {
            Ok(d) => d,
            Err(_) => mir2_shared::enums::MirDirection::Down,
        };

        let attack_action = match data.attack_type {
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
                    s.server_attack_type = data.attack_type;
                }
            }
        }

        if need_insert_attack_state {
            let _ = ctx.world.insert_one(
                e,
                AttackState {
                    start_time: Instant::now(),
                    attack_type: attack_action,
                    server_attack_type: data.attack_type,
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
            let is_ranged = data.spell != 0;

            let action = if is_ranged {
                match data.attack_type {
                    1 => MirAction::AttackRange2,
                    2 => MirAction::AttackRange3,
                    _ => MirAction::AttackRange1,
                }
            } else {
                match data.attack_type {
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

        // ===== 音效：攻击音效改为由 AnimationSystem 按"动作起始帧"触发（更贴近 C#：SetAction 时播放）
    }
}

impl LogicSystem for NetworkApplySystem {
    fn update(&mut self, ctx: &mut GameContext, _delay_time: f32) -> GameResult {
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
        let entity_index = Self::build_object_index(&ctx.world);
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
                            Self::update_learned_magic(&mut magic_list, magic.spell as u8, magic.level, magic.experience as u32, magic.key);
                        }
                    }
                }
                NetworkEvent::MagicRemoved { spell, hero } => {
                    tracing::debug!("📜 Magic removed: {:?} hero={}", spell, hero);
                    if *hero {
                        hero_magic_removed.push(*spell as u8);
                    } else if let Some(e) = local_player_entity {
                        if let Ok(mut magic_list) = ctx.world.get::<&mut crate::components::spell::MagicList>(e) {
                            Self::remove_magic(&mut magic_list, *spell as u8);
                        }
                    }
                }
                NetworkEvent::MagicLeveledUp { spell, level, hero } => {
                    tracing::debug!("📈 Magic leveled up: {:?} level={} hero={}", spell, level, hero);
                    if *hero {
                        hero_magic_leveled_up.push((*spell as u8, *level));
                    } else if let Some(e) = local_player_entity {
                        if let Ok(mut magic_list) = ctx.world.get::<&mut crate::components::spell::MagicList>(e) {
                            Self::update_magic_level(&mut magic_list, *spell as u8, *level);
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
                            Self::update_spell_toggle(&mut magic_list, (*spell).into(), *can_use);
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
                NetworkEvent::NewHeroCreated { hero_info } => {
                    tracing::trace!("🦸 New hero created: {}", hero_info);
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
                NetworkEvent::HeroBehaviourSet { behaviour, pet_mode } => {
                    tracing::trace!("🦸 Hero behaviour set: behaviour={} pet_mode={}", behaviour, pet_mode);
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
                NetworkEvent::ParcelCollectedEvent { success } => {
                    tracing::trace!("📦 Parcel collected: success={}", success);
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
                NetworkEvent::FishingStatusUpdated { state, success } => { tracing::trace!("🎣 Fishing status updated: {} success={}", state, success); }
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
            let from_pos = Self::object_position(&ctx.world, &entity_index, source);
            let to_pos = Self::object_position(&ctx.world, &entity_index, destination);
            if let (Some(from), Some(to)) = (from_pos, to_pos) {
                if let Some(projectile_type) = Self::spell_to_projectile_type(spell) {
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
            if let Some((x, y)) = Self::object_position(&ctx.world, &entity_index, object_id) {
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
                Self::upsert_component(ctx, e, info.clone());
            }
            if guild_left {
                let _ = ctx.world.remove_one::<crate::components::GuildInfo>(e);
            }
            if let Some(ref name) = guild_name_received {
                Self::upsert_component(ctx, e, crate::components::GuildInfo {
                    name: name.clone(),
                    rank: String::new(),
                    ..Default::default()
                });
            }
            if let Some((ref name, date)) = lover_updated {
                Self::upsert_component(ctx, e, crate::components::LoverState {
                    name: name.clone(),
                    date,
                });
            }
            if let Some((ref name, level, online)) = mentor_updated {
                Self::upsert_component(ctx, e, crate::components::MentorState {
                    name: name.clone(),
                    level,
                    online,
                });
            }
            if let Some(allowed) = observe_allowed {
                Self::upsert_component(ctx, e, crate::components::ObserveState {
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
                Self::upsert_component(ctx, e, crate::components::TradeState::default());
            } else if let Some(ref partner) = trade_started {
                Self::upsert_component(ctx, e, crate::components::TradeState {
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
            Self::apply_user_information(ctx, packet);
        }

        if let Some(packet) = map_changed {
            Self::apply_map_changed(ctx, packet);
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
            Self::apply_object_player(ctx, &entity_index, packet);
        }
        for packet in object_monsters {
            Self::apply_object_monster(ctx, &entity_index, packet);
        }
        for packet in object_npcs {
            Self::apply_object_npc(ctx, &entity_index, packet);
        }
        for hero_id in object_heroes {
            if let Some(&e) = entity_index.get(&hero_id) {
                if ctx.world.get::<&crate::components::Hero>(e).is_err() {
                    let _ = ctx.world.insert_one(e, crate::components::Hero);
                }
            }
        }
        for object_id in object_removes {
            Self::apply_object_remove(ctx, &entity_index, object_id);
        }
        for p in object_turns {
            Self::apply_object_turn(ctx, &entity_index, p);
        }
        for (_, p) in object_moves {
            match p {
                RemoteMovePacket::Walk(p) => Self::apply_object_walk(ctx, &entity_index, p),
                RemoteMovePacket::Run(p) => Self::apply_object_run(ctx, &entity_index, p),
            }
        }
        for (object_id, data) in object_attacks {
            Self::apply_object_attack(ctx, &entity_index, object_id, data);
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
                    Self::stop_player_actions(&mut ctx.world, e);
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
                Self::upsert_component(ctx, e, cur);
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
                            Self::transfer_slot(
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
                            Self::transfer_slot(
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
                    Self::set_monster_anim(&ctx.world, &entity_index, object_id, Some(crate::components::MirAction::Struck), None);
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
                Self::upsert_component(ctx, target, crate::components::HealthBarAnim { displayed: hp_after_damage as f32 });
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

                Self::set_visibility_dead(ctx, target, true);

                // 玩家死亡动画：挂上/重置 DeathState（Die → Dead），并停止攻击动画
                if ctx.world.get::<&crate::components::Player>(target).is_ok() {
                    Self::upsert_component(ctx, target, crate::components::DeathState::new());
                    let _ = ctx.world.remove_one::<crate::components::AttackState>(target);
                }

                // 本地玩家死亡：立刻停止移动/攻击输入，避免"死了还在走/追砍"。
                if ctx.world.get::<&crate::components::LocalPlayer>(target).is_ok() {
                    Self::stop_player_actions(&mut ctx.world, target);
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
                        Self::set_monster_anim(&ctx.world, &entity_index, object_id, Some(crate::components::MirAction::Die), None);
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
                if let Some((x, y)) = Self::entity_position(ctx, target) {
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
                    Self::set_monster_anim(&ctx.world, &entity_index, oid, Some(crate::components::MirAction::Dead), dir);
                }

                // 停止移动/攻击输入
                Self::stop_player_actions(&mut ctx.world, e);

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
        let now_ms = (macroquad::time::get_time() * 1000.0) as i64;
        for (object_id, buff_id, expire_time, infinite, paused) in buff_adds {
            tracing::trace!("🔮 Buff added: object_id={}, buff_id={}", object_id, buff_id);
            if let Some(&e) = entity_index.get(&object_id) {
                let server_buff = mir2_shared::enums::BuffType::try_from(buff_id as u8).ok();
                let combat_buff = server_buff.and_then(Self::map_server_buff);
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
                    Self::with_component::<crate::components::combat::BuffList>(ctx, e, |bl| {
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
            Self::upsert_component(ctx, entity, crate::components::AttackMode::new(mode));
        }
        for (entity, mode) in pet_mode_changes {
            tracing::debug!("🐾 Pet mode changed: {}", mode);
            Self::upsert_component(ctx, entity, crate::components::PetMode::new(mode));
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
            Self::apply_object_move(ctx, &entity_index, object_id, x, y);
        }
        for (object_id, x, y) in dashes {
            Self::apply_object_move(ctx, &entity_index, object_id, x, y);
        }
        for (object_id, x, y) in pushed {
            Self::apply_object_move(ctx, &entity_index, object_id, x, y);
        }
        for (object_id, x, y) in dash_attacked {
            Self::apply_object_move(ctx, &entity_index, object_id, x, y);
        }
        for (object_id, x, y) in attack_moved {
            Self::apply_object_move(ctx, &entity_index, object_id, x, y);
        }

        // Dash 失败：怪物/NPC 播放 DashFail 动画
        for object_id in dash_failed {
            Self::set_monster_anim(&ctx.world, &entity_index, object_id, Some(crate::components::MirAction::DashFail), None);
        }

        // 坐下：怪物/NPC 更新动画状态（玩家暂不支持 SitDown PlayerAction）
        for object_id in sat_down {
            Self::set_monster_anim(&ctx.world, &entity_index, object_id, Some(crate::components::MirAction::SitDown), None);
        }

        // 采集：位置落地 + 更新怪物/NPC 动画
        for (object_id, x, y, dir) in harvested {
            Self::apply_object_move(ctx, &entity_index, object_id, x, y);
            let direction = mir2_shared::MirDirection::try_from(dir).ok();
            Self::set_monster_anim(&ctx.world, &entity_index, object_id, Some(crate::components::MirAction::Harvest), direction);
        }

        // 中毒/流血（ObjectPoisoned 推送的 poison_type 是 PoisonType bits）
        for (object_id, poison_type) in poisoned_objects {
            tracing::trace!("☠ Object poisoned: object_id={}, type={}", object_id, poison_type);
            if let Some(&e) = entity_index.get(&object_id) {
                use mir2_shared::enums::PoisonType;
                let pt = PoisonType::from_bits_truncate(poison_type as u16);
                Self::apply_poison_to_entity(ctx, e, pt);
            }
        }

        // 复活
        for object_id in revived {
            tracing::trace!("💚 Object revived: {}", object_id);
            if let Some(&e) = entity_index.get(&object_id) {
                // 移除死亡状态
                let _ = ctx.world.remove_one::<crate::components::DeathState>(e);

                Self::set_visibility_dead(ctx, e, false);

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
                if let Some((x, y)) = Self::entity_position(ctx, e) {
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
            Self::apply_object_move(ctx, &entity_index, object_id, x, y);
            let dir = mir2_shared::MirDirection::try_from(direction).ok();
            Self::set_monster_anim(&ctx.world, &entity_index, object_id, None, dir);
        }

        // 远程攻击投射物
        for (from_id, target_id, target_x, target_y, spell) in range_projectiles {
            let from_pos = Self::object_position(&ctx.world, &entity_index, from_id);
            let to = if target_id != 0 {
                Self::object_position(&ctx.world, &entity_index, target_id)
            } else {
                None
            };
            let to = to.unwrap_or_else(|| {
                crate::coord::Coord::grid_to_world_center(target_x as i32, target_y as i32)
            });
            if let (Some(from), Some(projectile_type)) = (from_pos, Self::spell_to_projectile_type(spell as u8)) {
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
                if let Some((x, y)) = Self::entity_position(ctx, e) {
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
                if let Some((x, y)) = Self::entity_position(ctx, e) {
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
            Self::upsert_component(ctx, e, crate::components::NameColor(colour as i32));
        }

        // Object colour changes
        for (object_id, colour) in object_colours {
            if let Some(&e) = entity_index.get(&object_id) {
                Self::upsert_component(ctx, e, crate::components::NameColor(colour as i32));
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
                if let Some((x, y)) = Self::entity_position(ctx, e) {
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
                Self::upsert_component(
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
                    Self::upsert_component(ctx, e, crate::components::render::ObjectDeco { deco_id: deco });
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
                Self::upsert_component(ctx, e, crate::components::Visibility { hidden: sneaking, dead: false });
            }
        }

        // 对象等级特效
        for (object_id, level_effects) in object_level_effects {
            let flags = crate::components::LevelEffectsFlags(
                mir2_shared::enums::LevelEffects::from_bits_truncate(level_effects as u16)
            );
            if let Some(&e) = entity_index.get(&object_id) {
                Self::upsert_component(ctx, e, flags);
            }
        }

        // 对象施法动画
        for (object_id, _spell) in object_spells {
            Self::set_monster_anim(&ctx.world, &entity_index, object_id, Some(crate::components::MirAction::Spell), None);
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
                    if let Some((x, y)) = Self::object_position(&ctx.world, &entity_index, hero_id) {
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
                    if let Some((x, y)) = Self::object_position(&ctx.world, &entity_index, hero_id) {
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
            Self::with_hero_magic_list(ctx, &entity_index, local_player_entity, |magic_list| {
                Self::update_learned_magic(magic_list, spell, level, experience, key);
            });
        }
        for spell in hero_magic_removed {
            Self::with_hero_magic_list(ctx, &entity_index, local_player_entity, |magic_list| {
                Self::remove_magic(magic_list, spell);
            });
        }
        for (spell, level) in hero_magic_leveled_up {
            Self::with_hero_magic_list(ctx, &entity_index, local_player_entity, |magic_list| {
                Self::update_magic_level(magic_list, spell, level);
            });
        }
        for (spell, can_use) in hero_spell_toggled {
            Self::with_hero_magic_list(ctx, &entity_index, local_player_entity, |magic_list| {
                Self::update_spell_toggle(magic_list, spell, can_use);
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
            Self::apply_mock_library_sprite_despawn(ctx, object_id);
        }
        for (object_id, object_type, library, index, x, y) in mock_spawns {
            Self::apply_mock_library_sprite_spawn(ctx, object_id, object_type, library, index, x, y);
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
            let now = macroquad::prelude::get_time();
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
            Self::set_monster_anim(&ctx.world, &entity_index, object_id, Some(crate::components::MirAction::Spell), None);

            tracing::trace!("🔮 Spell cast: {:?} from {} to {}", spell_enum, object_id, target_id);
        }

        // ObjectEffectReceived: 命中/暴击等特效
        for (object_id, effect, effect_type) in effect_received {
            use mir2_shared::enums::SpellEffect;

            let (px, py) = Self::object_position(&ctx.world, &entity_index, object_id).unwrap_or((0.0, 0.0));

            let now = macroquad::prelude::get_time();

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

        Ok(())
    }
}