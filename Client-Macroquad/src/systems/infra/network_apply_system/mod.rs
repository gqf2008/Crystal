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

/// 从装备列表推导 PlayerAppearance 的武器/护甲外观
fn derive_appearance_from_equipment(
    appearance: &mut crate::components::player::PlayerAppearance,
    equipment: &[Option<mir2_shared::data::item::UserItem>],
) {
    use mir2_shared::enums::ItemType;

    // weapon slot = 0
    match equipment.first().and_then(|x| x.as_ref()) {
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
    match equipment.get(1).and_then(|x| x.as_ref()) {
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

// Test builds can't call macroquad::get_time() (asserts it's on main thread).
#[cfg(not(test))]
fn current_time_secs() -> f64 {
    macroquad::prelude::get_time()
}

#[cfg(test)]
fn current_time_secs() -> f64 {
    0.0
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
    use crate::components::{
        LocalPlayer, MonsterAnimState, Player, Position, PositionInterpolation, RemoteMoveAnim,
    };
    use std::time::Instant;

    let Some(e) = entity_index.get(&object_id).copied() else {
        return;
    };

    let is_local = ctx.world.get::<&LocalPlayer>(e).is_ok();
    let (wx, wy) = crate::coord::Coord::grid_to_world_center(location_x, location_y);
    let now_secs = current_time_secs();

    if is_local {
        let has_pos = ctx.world.get::<&Position>(e).is_ok();
        let dead = ctx
            .world
            .get::<&crate::components::Health>(e)
            .ok()
            .map(|hp| hp.current <= 0)
            .unwrap_or(false);
        let will_apply = ctx.session.server_authoritative_movement || !has_pos || dead;

        if NetworkApplySystem::net_recv_diag_enabled() {
            let before_grid = ctx
                .world
                .get::<&Position>(e)
                .ok()
                .map(|p| crate::coord::Coord::world_to_grid(p.x, p.y));
            tracing::info!(
                "[NETRECV] {:?}(local): id={} loc=({},{}) dir={:?} will_apply={} local_before={:?}",
                cfg.player_action,
                object_id,
                location_x,
                location_y,
                direction,
                will_apply,
                before_grid
            );
        }

        if will_apply {
            NetworkApplySystem::apply_object_move(
                ctx,
                entity_index,
                object_id,
                location_x,
                location_y,
            );
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
        let interp_dur = if cfg.scale_interp_by_steps {
            cfg.base_interp_secs * steps as f32
        } else {
            cfg.base_interp_secs
        };

        if cfg.base_interp_secs > 0.0
            && steps <= cfg.interp_max_steps
            && ((sx - wx).abs() > 0.01 || (sy - wy).abs() > 0.01)
        {
            let interp = PositionInterpolation::new(sx, sy, wx, wy, now_secs, interp_dur);
            NetworkApplySystem::upsert_component(ctx, e, interp);
        } else if cfg.base_interp_secs <= 0.0 || steps > cfg.interp_max_steps {
            NetworkApplySystem::apply_object_move(
                ctx,
                entity_index,
                object_id,
                location_x,
                location_y,
            );
        }

        let anim_secs = if cfg.base_interp_secs > 0.0 {
            interp_dur
        } else {
            cfg.fallback_anim_secs
        };
        NetworkApplySystem::upsert_component(
            ctx,
            e,
            RemoteMoveAnim {
                end_time: now_secs + anim_secs as f64,
            },
        );
    }

    if let Ok(mut p) = ctx.world.get::<&mut Player>(e) {
        p.direction = direction;
        p.action = cfg.player_action;
    }

    if ctx.world.get::<&crate::components::Monster>(e).is_ok() {
        NetworkApplySystem::upsert_component(
            ctx,
            e,
            MonsterAnimState {
                direction,
                action: crate::components::MirAction::Walking,
                start_time: Instant::now(),
            },
        );
    }
}

impl NetworkApplySystem {
    /// 获取实体的世界坐标（不修改 ECS）。
    fn entity_position(ctx: &GameContext, entity: hecs::Entity) -> Option<(f32, f32)> {
        ctx.world
            .get::<&crate::components::Position>(entity)
            .ok()
            .map(|p| (p.x, p.y))
    }

    /// 按 object_id 从索引查找实体并获取世界坐标（O(1)）。
    fn object_position(
        world: &hecs::World,
        entity_index: &std::collections::HashMap<u32, hecs::Entity>,
        object_id: u32,
    ) -> Option<(f32, f32)> {
        let e = entity_index.get(&object_id)?;
        world
            .get::<&crate::components::Position>(*e)
            .ok()
            .map(|p| (p.x, p.y))
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

    fn apply_object_player(
        ctx: &mut GameContext,
        entity_index: &std::collections::HashMap<u32, hecs::Entity>,
        packet: mir2_shared::packets::server::ObjectPlayer,
    ) {
        use crate::components::network::{NetworkObjectType, NetworkSync};
        use crate::components::{
            AnimationFrame, MountState, MountStatus, OtherPlayer, Player, PlayerAction,
            PlayerAppearance, Position, RemotePlayer,
        };

        let (wx, wy) =
            crate::coord::Coord::grid_to_world_center(packet.location_x, packet.location_y);

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

        let mount_index_from_packet: Option<usize> =
            if packet.riding_mount && packet.mount_type >= 0 {
                Some(packet.mount_type as usize)
            } else {
                None
            };

        if let Some(e) = entity_index.get(&packet.object_id).copied() {
            // NetworkSync 只要存在即可；类型不匹配时更新。
            Self::upsert_component(
                ctx,
                e,
                NetworkSync::new(packet.object_id, NetworkObjectType::Player),
            );

            // 远程玩家标记
            Self::upsert_component(
                ctx,
                e,
                RemotePlayer {
                    id: packet.object_id,
                },
            );

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
            Self::upsert_component(
                ctx,
                e,
                MountState {
                    mount_index: mount_index_from_packet,
                },
            );
            Self::upsert_component(
                ctx,
                e,
                MountStatus {
                    mount_type: packet.mount_type,
                    riding_mount: packet.riding_mount,
                },
            );

            // 基本身份信息（未来做名字/血条会用到）
            Self::upsert_component(ctx, e, {
                let mut op = OtherPlayer::new(
                    packet.name.clone(),
                    packet.class,
                    packet.gender,
                    packet.level,
                );
                op.guild_name = if packet.guild_name.is_empty() {
                    None
                } else {
                    Some(packet.guild_name.clone())
                };
                op
            });

            if ctx.world.get::<&crate::components::Health>(e).is_err() {
                let current = if packet.dead { 0 } else { 100 };
                let _ = ctx
                    .world
                    .insert_one(e, crate::components::Health { current, max: 100 });
            }

            if packet.dead && ctx.world.get::<&crate::components::DeathState>(e).is_err() {
                let _ = ctx
                    .world
                    .insert_one(e, crate::components::DeathState::new());
            }

            Self::upsert_visibility(ctx, e, packet.hidden, packet.dead);

            Self::upsert_component(ctx, e, crate::components::NameColor(packet.name_colour));
            Self::upsert_component(
                ctx,
                e,
                crate::components::LevelEffectsFlags(packet.level_effects),
            );

            if !packet.poison.is_empty() {
                Self::apply_poison_to_entity(ctx, e, packet.poison);
            }
            if !packet.buffs.is_empty() {
                Self::apply_buffs_to_entity(ctx, e, &packet.buffs);
            }
        } else {
            let new_entity = ctx.world.spawn((
                NetworkSync::new(packet.object_id, NetworkObjectType::Player),
                RemotePlayer {
                    id: packet.object_id,
                },
                player,
                Position::new(wx, wy),
                appearance,
                AnimationFrame::default(),
                MountState {
                    mount_index: mount_index_from_packet,
                },
                MountStatus {
                    mount_type: packet.mount_type,
                    riding_mount: packet.riding_mount,
                },
                OtherPlayer::new(
                    packet.name.clone(),
                    packet.class,
                    packet.gender,
                    packet.level,
                ),
            ));

            let _ = ctx.world.insert_one(
                new_entity,
                crate::components::Visibility {
                    hidden: packet.hidden,
                    dead: packet.dead,
                },
            );
            let _ = ctx
                .world
                .insert_one(new_entity, crate::components::NameColor(packet.name_colour));
            let _ = ctx.world.insert_one(
                new_entity,
                crate::components::LevelEffectsFlags(packet.level_effects),
            );

            let current = if packet.dead { 0 } else { 100 };
            let _ = ctx
                .world
                .insert_one(new_entity, crate::components::Health { current, max: 100 });
            if packet.dead {
                let _ = ctx
                    .world
                    .insert_one(new_entity, crate::components::DeathState::new());
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
            Spell::FireBall
            | Spell::GreatFireBall
            | Spell::HellFire
            | Spell::FireBang
            | Spell::FlameDisruptor
            | Spell::SoulFireBall
            | Spell::FireBurst => Some(ProjectileType::Fireball),
            Spell::ThunderBolt | Spell::Lightning | Spell::ThunderStorm | Spell::ElectricShock => {
                Some(ProjectileType::Lightning)
            }
            Spell::FrostCrunch | Spell::IceStorm | Spell::Blizzard | Spell::IceThrust => {
                Some(ProjectileType::IceBolt)
            }
            Spell::StraightShot | Spell::DoubleShot | Spell::ElementalShot | Spell::BackStep => {
                Some(ProjectileType::Arrow)
            }
            _ => None,
        }
    }

    fn apply_user_information(
        ctx: &mut GameContext,
        packet: mir2_shared::packets::server::UserInformation,
    ) {
        use crate::components::{
            AnimationFrame, CombatStats, Health, LocalPlayer, Mana, MovementVelocity, Path, Player,
            PlayerAction, PlayerAppearance, PlayerInput, Position, RegenTimer,
        };
        use crate::components::{
            Currency, Equipment, Experience, Inventory, MagicList, PlayerData, QuestInventory,
        };
        use crate::components::{
            GuildInfo, HeroState, LevelEffectsFlags, NameColor, ObserveState, SummonedCreatureState,
        };

        // 先找本地玩家实体；如果还没创建，则最小创建一个
        let existing = {
            ctx.world
                .iter()
                .find_map(|e| e.get::<&LocalPlayer>().map(|_| e.entity()))
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
        let (wx, wy) =
            crate::coord::Coord::grid_to_world_center(packet.location_x, packet.location_y);
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
        Self::upsert_component(
            ctx,
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

        // 经验值：若缺失则兜底创建
        if ctx.world.get::<&Experience>(local_entity).is_err() {
            let _ = ctx
                .world
                .insert_one(local_entity, Experience::new(packet.level));
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
        Self::upsert_component(
            ctx,
            local_entity,
            ObserveState {
                allow_observe: packet.allow_observe,
                observer: packet.observer,
            },
        );

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

        Self::upsert_component(
            ctx,
            local_entity,
            SummonedCreatureState {
                creature_type: packet.summoned_creature_type,
                summoned: packet.creature_summoned,
            },
        );

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
        Self::upsert_component(
            ctx,
            local_entity,
            Currency {
                gold: packet.gold,
                credit: packet.credit,
            },
        );

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
                derive_appearance_from_equipment(&mut appearance, &items);

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
            let _ = ctx.world.insert_one(
                local_entity,
                CombatStats {
                    level: packet.level,
                    ..CombatStats::default()
                },
            );
        }
    }

    fn apply_player_inspect(
        ctx: &mut GameContext,
        packet: mir2_shared::packets::server::PlayerInspect,
    ) {
        use crate::components::{Equipment, OtherPlayer, PlayerAppearance};

        let target_entity = {
            ctx.world.iter().find_map(|e| {
                e.get::<&OtherPlayer>()
                    .filter(|op| op.name == packet.name)
                    .map(|_| e.entity())
            })
        };

        let Some(e) = target_entity else {
            tracing::warn!(
                "[NetworkApplySystem] PlayerInspect for unknown player name={}",
                packet.name
            );
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
            derive_appearance_from_equipment(&mut appearance, &packet.equipment);
        }
    }

    fn apply_equipment_vec(
        eq: &mut crate::components::Equipment,
        items: &[Option<mir2_shared::data::item::UserItem>],
    ) {
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

    fn map_magics(
        magics: &[mir2_shared::data::client_data::ClientMagic],
    ) -> Vec<crate::components::LearnedMagic> {
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
        if let Some((_ws_entity, mut ws)) = ctx
            .world
            .iter()
            .find_map(|e| e.get::<&mut WeatherState>().map(|w| (e.entity(), w)))
        {
            ws.weather_code = packet.weather;
            ws.emitter_entity = None; // 重置发射器，由 WeatherSystem 重建
        }

        // MapChanged 里携带了落点与朝向（切图/传送时很关键）
        let Some((entity, _)) = ctx
            .world
            .iter()
            .find_map(|e| e.get::<&LocalPlayer>().map(|lp| (e.entity(), lp)))
        else {
            return;
        };

        let (wx, wy) =
            crate::coord::Coord::grid_to_world_center(packet.location_x, packet.location_y);
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
        use crate::components::network::{NetworkObjectType, NetworkSync};
        use crate::components::{LibrarySprite, Position};

        let obj_type = match object_type {
            crate::network::handlers::ObjectType::Player => NetworkObjectType::Player,
            crate::network::handlers::ObjectType::Monster => NetworkObjectType::Monster,
            crate::network::handlers::ObjectType::Npc => NetworkObjectType::NPC,
            crate::network::handlers::ObjectType::Item => NetworkObjectType::Item,
            crate::network::handlers::ObjectType::Spell => NetworkObjectType::Spell,
        };

        let existing = {
            ctx.world.iter().find_map(|e| {
                e.get::<&NetworkSync>()
                    .filter(|ns| ns.object_id == object_id)
                    .map(|_| e.entity())
            })
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
            ctx.world.iter().find_map(|e| {
                e.get::<&NetworkSync>()
                    .filter(|ns| ns.object_id == object_id)
                    .map(|_| e.entity())
            })
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
    fn with_hero_magic_list<F>(
        ctx: &mut GameContext,
        entity_index: &std::collections::HashMap<u32, hecs::Entity>,
        local_player_entity: Option<hecs::Entity>,
        f: F,
    ) where
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
        if let Ok(mut magic_list) = ctx
            .world
            .get::<&mut crate::components::spell::MagicList>(hero_entity)
        {
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

    fn update_magic_level(
        magic_list: &mut crate::components::spell::MagicList,
        spell: u8,
        level: u8,
    ) {
        if let Some(magic) = Self::find_magic_mut(&mut magic_list.magics, spell) {
            magic.level = level;
        }
    }

    fn update_spell_toggle(
        magic_list: &mut crate::components::spell::MagicList,
        spell: u8,
        can_use: bool,
    ) {
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
        use crate::components::network::NetworkSync;
        use crate::components::{LibrarySprite, Position};

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

    fn apply_object_monster(
        ctx: &mut GameContext,
        entity_index: &std::collections::HashMap<u32, hecs::Entity>,
        packet: mir2_shared::packets::server::ObjectMonster,
    ) {
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
                .insert_one(
                    e,
                    crate::components::Monster::new(packet.name.clone(), packet.image),
                )
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
            let initial_action = if packet.dead {
                MirAction::Dead
            } else {
                MirAction::Standing
            };
            Self::upsert_component(
                ctx,
                e,
                MonsterAnimState {
                    direction: packet.direction,
                    action: initial_action,
                    start_time: Instant::now(),
                },
            );

            // 最小血条支撑：若无服务器 HP 信息，则给一个默认血池，保证可见
            if ctx.world.get::<&crate::components::Health>(e).is_err() {
                let current = if packet.dead { 0 } else { 100 };
                let _ = ctx
                    .world
                    .insert_one(e, crate::components::Health { current, max: 100 });
            } else if packet.dead {
                if let Ok(mut h) = ctx.world.get::<&mut crate::components::Health>(e) {
                    h.current = 0;
                }
            }

            // 死亡状态：dead=true 时插入 DeathState（动画已在 MonsterAnimState 中处理）
            if packet.dead && ctx.world.get::<&crate::components::DeathState>(e).is_err() {
                let _ = ctx
                    .world
                    .insert_one(e, crate::components::DeathState::new());
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

    fn map_server_buff(
        buff: mir2_shared::enums::BuffType,
    ) -> Option<crate::components::combat::BuffType> {
        use crate::components::combat::BuffType as C;
        use mir2_shared::enums::BuffType as S;
        match buff {
            S::MagicShield | S::EnergyShield => Some(C::MagicShield),
            S::SoulShield
            | S::BlessedArmour
            | S::ProtectionField
            | S::UltimateEnhancer
            | S::ImmortalSkin
            | S::ElementalBarrier
            | S::GeneralMeowMeowShield
            | S::HornedWarriorShield
            | S::HornedCommanderShield => Some(C::DefenseBoost),
            S::Fury
            | S::Rage
            | S::CounterAttack
            | S::HornedArcherBuff
            | S::ColdArcherBuff
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

    fn upsert_visibility(ctx: &mut GameContext, entity: hecs::Entity, hidden: bool, dead: bool) {
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

    fn apply_object_npc(
        ctx: &mut GameContext,
        entity_index: &std::collections::HashMap<u32, hecs::Entity>,
        packet: mir2_shared::packets::server::ObjectNpc,
    ) {
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
                    crate::components::NPC::new(
                        packet.name.clone(),
                        format!("npc:{}", packet.image),
                    ),
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

    fn apply_object_remove(
        ctx: &mut GameContext,
        entity_index: &std::collections::HashMap<u32, hecs::Entity>,
        object_id: u32,
    ) {
        if let Some(e) = entity_index.get(&object_id).copied() {
            // 对齐原版：不要因为 ObjectRemove 把本地玩家实体删掉。
            // 服务器可能在切图/传送等边界广播 ObjectRemove；本地玩家应由 UserInformation/MapChanged 重建位置。
            if ctx.world.get::<&crate::components::LocalPlayer>(e).is_ok() {
                tracing::warn!(
                    "[NETRECV] Ignored ObjectRemove for LocalPlayer: object_id={}",
                    object_id
                );
                return;
            }
            let _ = ctx.world.despawn(e);
        }
    }

    fn apply_object_move(
        ctx: &mut GameContext,
        entity_index: &std::collections::HashMap<u32, hecs::Entity>,
        object_id: u32,
        x: i32,
        y: i32,
    ) {
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

    fn apply_object_turn(
        ctx: &mut GameContext,
        entity_index: &std::collections::HashMap<u32, hecs::Entity>,
        packet: mir2_shared::packets::server::ObjectTurn,
    ) {
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
                Self::apply_object_move(
                    ctx,
                    entity_index,
                    packet.object_id,
                    packet.location_x,
                    packet.location_y,
                );
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

    fn apply_object_walk(
        ctx: &mut GameContext,
        entity_index: &std::collections::HashMap<u32, hecs::Entity>,
        packet: mir2_shared::packets::server::ObjectWalk,
    ) {
        apply_remote_movement(
            ctx,
            entity_index,
            packet.object_id,
            packet.location_x,
            packet.location_y,
            packet.direction,
            RemoteMoveConfig {
                player_action: crate::components::PlayerAction::Walk,
                base_interp_secs: ctx.session.remote_player_walk_interp_secs,
                interp_max_steps: 1,
                scale_interp_by_steps: false,
                fallback_anim_secs: 0.16,
            },
        );
    }

    fn apply_object_run(
        ctx: &mut GameContext,
        entity_index: &std::collections::HashMap<u32, hecs::Entity>,
        packet: mir2_shared::packets::server::ObjectRun,
    ) {
        apply_remote_movement(
            ctx,
            entity_index,
            packet.object_id,
            packet.location_x,
            packet.location_y,
            packet.direction,
            RemoteMoveConfig {
                player_action: crate::components::PlayerAction::Run,
                base_interp_secs: ctx.session.remote_player_run_interp_secs,
                interp_max_steps: 2,
                scale_interp_by_steps: true,
                fallback_anim_secs: 0.11,
            },
        );
    }

    fn apply_object_attack(
        ctx: &mut GameContext,
        entity_index: &std::collections::HashMap<u32, hecs::Entity>,
        object_id: u32,
        data: ObjectAttackData,
    ) {
        use crate::components::{
            AttackState, LocalPlayer, MirAction, Monster, MonsterAnimState, Player, PlayerAction,
            Position,
        };
        use std::time::Instant;
        let Some(e) = entity_index.get(&object_id).copied() else {
            return;
        };

        // 远程对象：不要在每个攻击包上都硬矫正位置，否则会把 walk/run 的插值打断，导致"瞬移/抽风"。
        // 只有差距较大（例如>2格）才强制矫正。
        let is_local = ctx.world.get::<&LocalPlayer>(e).is_ok();
        if is_local {
            let has_pos = ctx.world.get::<&Position>(e).is_ok();
            let dead = ctx
                .world
                .get::<&crate::components::Health>(e)
                .ok()
                .map(|hp| hp.current <= 0)
                .unwrap_or(false);
            let will_apply = ctx.session.server_authoritative_movement || !has_pos || dead;
            if will_apply {
                Self::apply_object_move(
                    ctx,
                    entity_index,
                    object_id,
                    data.location_x as i32,
                    data.location_y as i32,
                );
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
                Self::apply_object_move(
                    ctx,
                    entity_index,
                    object_id,
                    data.location_x as i32,
                    data.location_y as i32,
                );
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

mod update;

impl LogicSystem for NetworkApplySystem {
    fn update(&mut self, ctx: &mut GameContext, _delay_time: f32) -> GameResult {
        update::update(ctx, _delay_time)
    }
}

#[cfg(test)]
mod e2e_tests {
    use super::*;
    use crate::components::{
        Currency, Equipment, Experience, Health, Inventory, LocalPlayer, Mana, PlayerData,
        Position, PositionInterpolation,
    };
    use crate::network::handlers::NetworkEvent;
    use mir2_shared::enums::{HeroBehaviour, MirClass, MirDirection, MirGender};

    fn setup() -> (GameContext, NetworkApplySystem) {
        (GameContext::new(), NetworkApplySystem::default())
    }

    fn find_local_entity(ctx: &GameContext) -> hecs::Entity {
        ctx.world
            .iter()
            .find_map(|e| e.get::<&LocalPlayer>().map(|_| e.entity()))
            .unwrap()
    }

    fn make_user_info(
        object_id: u32,
        name: &str,
        hp: i32,
        mp: i32,
    ) -> mir2_shared::packets::server::UserInformation {
        mir2_shared::packets::server::UserInformation {
            object_id,
            real_id: object_id,
            name: name.to_string(),
            guild_name: String::new(),
            guild_rank: String::new(),
            name_colour: 0,
            class: MirClass::Warrior,
            gender: MirGender::Male,
            level: 1,
            location_x: 50,
            location_y: 60,
            direction: MirDirection::Down,
            hair: 0,
            hp,
            mp,
            experience: 0,
            max_experience: 100,
            level_effects: mir2_shared::LevelEffects::empty(),
            has_hero: false,
            hero_behaviour: HeroBehaviour::Attack,
            inventory: None,
            equipment: None,
            quest_inventory: None,
            gold: 0,
            credit: 0,
            has_expanded_storage: false,
            expanded_storage_expiry_time: 0,
            magics: Vec::new(),
            summoned_creature_type: 0,
            creature_summoned: false,
            allow_observe: false,
            observer: false,
        }
    }

    #[test]
    fn test_login_flow_user_information() {
        let (mut ctx, mut sys) = setup();

        let packet = make_user_info(1001, "TestHero", 500, 300);
        ctx.events_mut()
            .send_network(NetworkEvent::UserInformation { packet });
        sys.update(&mut ctx, 0.016).unwrap();

        let local_entity = find_local_entity(&ctx);

        let health = ctx.world.get::<&Health>(local_entity).unwrap();
        assert_eq!(health.current, 500);
        assert_eq!(health.max, 500);

        let mana = ctx.world.get::<&Mana>(local_entity).unwrap();
        assert_eq!(mana.current, 300);
        assert_eq!(mana.max, 300);

        let pos = ctx.world.get::<&Position>(local_entity).unwrap();
        assert!(pos.x > 0.0 && pos.y > 0.0);

        let pd = ctx.world.get::<&PlayerData>(local_entity).unwrap();
        assert_eq!(pd.object_id, 1001);
        assert_eq!(pd.name, "TestHero");
        assert_eq!(pd.class, MirClass::Warrior);
    }

    #[test]
    fn test_health_changed_updates_component() {
        let (mut ctx, mut sys) = setup();

        let packet = make_user_info(1001, "TestHero", 500, 300);
        ctx.events_mut()
            .send_network(NetworkEvent::UserInformation { packet });
        sys.update(&mut ctx, 0.016).unwrap();

        let local_entity = find_local_entity(&ctx);

        ctx.events_mut().send_network(NetworkEvent::HealthChanged {
            current: 250,
            max: 600,
        });
        sys.update(&mut ctx, 0.016).unwrap();

        let health = ctx.world.get::<&Health>(local_entity).unwrap();
        assert_eq!(health.current, 250);
        assert_eq!(health.max, 600);
    }

    #[test]
    fn test_remote_player_walk_creates_interpolation() {
        let (mut ctx, mut sys) = setup();

        let player = mir2_shared::packets::server::ObjectPlayer {
            object_id: 2001,
            name: "RemotePlayer".to_string(),
            guild_name: String::new(),
            guild_rank_name: String::new(),
            name_colour: 0,
            class: MirClass::Wizard,
            gender: MirGender::Female,
            level: 10,
            location_x: 10,
            location_y: 20,
            direction: MirDirection::Right,
            hair: 0,
            light: 0,
            weapon: 0,
            weapon_effect: 0,
            armour: 0,
            poison: mir2_shared::enums::PoisonType::empty(),
            dead: false,
            hidden: false,
            effect: mir2_shared::enums::SpellEffect::None,
            wing_effect: 0,
            extra: false,
            mount_type: 0,
            riding_mount: false,
            fishing: false,
            transform_type: 0,
            element_orb_effect: 0,
            element_orb_lvl: 0,
            element_orb_max: 0,
            buffs: Vec::new(),
            level_effects: mir2_shared::LevelEffects::empty(),
        };
        ctx.events_mut()
            .send_network(NetworkEvent::ObjectPlayer { packet: player });
        sys.update(&mut ctx, 0.016).unwrap();

        let walk = mir2_shared::packets::server::ObjectWalk {
            object_id: 2001,
            location_x: 11,
            location_y: 21,
            direction: MirDirection::Right,
        };
        ctx.events_mut()
            .send_network(NetworkEvent::ObjectWalk { packet: walk });
        sys.update(&mut ctx, 0.016).unwrap();

        let entity = ctx
            .world
            .iter()
            .find_map(|e| {
                if let Some(rp) = e.get::<&crate::components::RemotePlayer>() {
                    if rp.id == 2001 {
                        return Some(e.entity());
                    }
                }
                None
            })
            .expect("远程玩家实体应存在");

        assert!(
            ctx.world.get::<&PositionInterpolation>(entity).is_ok(),
            "远程玩家收到 ObjectWalk 后应产生 PositionInterpolation"
        );
    }

    #[test]
    fn test_item_gained_adds_to_inventory() {
        let (mut ctx, mut sys) = setup();

        let mut packet = make_user_info(1001, "TestHero", 100, 50);
        packet.inventory = Some(vec![None; 40]);
        ctx.events_mut()
            .send_network(NetworkEvent::UserInformation { packet });
        sys.update(&mut ctx, 0.016).unwrap();

        let local_entity = find_local_entity(&ctx);

        let item = mir2_shared::UserItem {
            unique_id: 999,
            item_index: 3001,
            count: 5,
            ..Default::default()
        };
        ctx.events_mut()
            .send_network(NetworkEvent::ItemGained { item });
        sys.update(&mut ctx, 0.016).unwrap();

        let inv = ctx.world.get::<&Inventory>(local_entity).unwrap();
        // Assert
        let found = inv.items.iter().any(|slot| {
            slot.as_ref()
                .map(|it| it.unique_id == 999 && it.count == 5)
                .unwrap_or(false)
        });
        assert!(found, "背包中应存在刚获得的物品");
    }

    #[test]
    fn test_monster_spawn_and_remove() {
        let (mut ctx, mut sys) = setup();

        let packet = make_user_info(1001, "TestHero", 500, 300);
        ctx.events_mut()
            .send_network(NetworkEvent::UserInformation { packet });
        sys.update(&mut ctx, 0.016).unwrap();

        let monster = mir2_shared::packets::server::ObjectMonster {
            object_id: 3001,
            name: "BigRat".to_string(),
            name_colour: 0,
            location_x: 60,
            location_y: 70,
            image: 450,
            direction: MirDirection::Down,
            effect: 0,
            ai: 0,
            light: 0,
            dead: false,
            skeleton: false,
            poison: mir2_shared::enums::PoisonType::empty(),
            hidden: false,
            shock_time: 0,
            binding_shot_center: false,
            extra: false,
            extra_byte: 0,
            buffs: vec![],
        };
        ctx.events_mut()
            .send_network(NetworkEvent::ObjectMonster { packet: monster });
        sys.update(&mut ctx, 0.016).unwrap();

        let monster_entity = ctx
            .world
            .iter()
            .find_map(|e| {
                if let Some(sync) = e.get::<&crate::components::network::NetworkSync>() {
                    if sync.object_id == 3001 {
                        return Some(e.entity());
                    }
                }
                None
            })
            .expect("怪物实体应存在");

        assert!(ctx
            .world
            .get::<&crate::components::Position>(monster_entity)
            .is_ok());

        ctx.events_mut()
            .send_network(NetworkEvent::ObjectHealthPercent {
                object_id: 3001,
                percent: 50,
                expire: 0,
            });
        sys.update(&mut ctx, 0.016).unwrap();

        // ObjectHealthPercent should create a Health component
        if let Ok(health) = ctx.world.get::<&Health>(monster_entity) {
            assert!(health.current > 0, "怪物血量应 > 0");
        }

        ctx.events_mut()
            .send_network(NetworkEvent::ObjectRemove { object_id: 3001 });
        sys.update(&mut ctx, 0.016).unwrap();

        let still_alive = ctx.world.iter().any(|e| {
            if let Some(sync) = e.get::<&crate::components::network::NetworkSync>() {
                return sync.object_id == 3001;
            }
            false
        });
        assert!(!still_alive, "怪物实体应被移除");
    }

    #[test]
    fn test_experience_and_level_up() {
        let (mut ctx, mut sys) = setup();

        let packet = make_user_info(1001, "TestHero", 500, 300);
        ctx.events_mut()
            .send_network(NetworkEvent::UserInformation { packet });
        sys.update(&mut ctx, 0.016).unwrap();

        let local_entity = find_local_entity(&ctx);

        ctx.events_mut()
            .send_network(NetworkEvent::ExperienceGained { amount: 50 });
        sys.update(&mut ctx, 0.016).unwrap();

        {
            let exp = ctx.world.get::<&Experience>(local_entity).unwrap();
            assert_eq!(exp.current, 50, "经验应增长到 50");
        }

        ctx.events_mut()
            .send_network(NetworkEvent::LevelUp { new_level: 2 });
        sys.update(&mut ctx, 0.016).unwrap();

        {
            let pd = ctx.world.get::<&PlayerData>(local_entity).unwrap();
            assert_eq!(pd.level, 2, "等级应升至 2");
        }
    }

    #[test]
    fn test_mana_changed() {
        let (mut ctx, mut sys) = setup();

        let packet = make_user_info(1001, "TestHero", 500, 300);
        ctx.events_mut()
            .send_network(NetworkEvent::UserInformation { packet });
        sys.update(&mut ctx, 0.016).unwrap();

        let local_entity = find_local_entity(&ctx);

        ctx.events_mut().send_network(NetworkEvent::ManaChanged {
            current: 200,
            max: 350,
        });
        sys.update(&mut ctx, 0.016).unwrap();

        {
            let mana = ctx.world.get::<&Mana>(local_entity).unwrap();
            assert_eq!(mana.current, 200, "蓝量应为 200");
            assert_eq!(mana.max, 350, "蓝量上限应为 350");
        }
    }

    #[test]
    fn test_gold_changed() {
        let (mut ctx, mut sys) = setup();

        let mut packet = make_user_info(1001, "TestHero", 500, 300);
        packet.gold = 100;
        ctx.events_mut()
            .send_network(NetworkEvent::UserInformation { packet });
        sys.update(&mut ctx, 0.016).unwrap();

        let local_entity = find_local_entity(&ctx);

        ctx.events_mut()
            .send_network(NetworkEvent::GoldChanged { delta: 50 });
        sys.update(&mut ctx, 0.016).unwrap();

        {
            let currency = ctx.world.get::<&Currency>(local_entity).unwrap();
            assert!(currency.gold > 0, "金币应增加");
        }
    }

    #[test]
    fn test_ground_item_spawn() {
        let (mut ctx, mut sys) = setup();

        let packet = make_user_info(1001, "TestHero", 500, 300);
        ctx.events_mut()
            .send_network(NetworkEvent::UserInformation { packet });
        sys.update(&mut ctx, 0.016).unwrap();

        // 地面掉落物品
        let item = mir2_shared::UserItem {
            unique_id: 777,
            item_index: 4001,
            count: 1,
            ..Default::default()
        };
        let ground = mir2_shared::packets::server::ObjectItem {
            object_id: 9001,
            item,
            location_x: 55,
            location_y: 65,
        };
        ctx.events_mut()
            .send_network(NetworkEvent::GroundItem { packet: ground });
        sys.update(&mut ctx, 0.016).unwrap();

        // 验证地面物品实体存在
        let found = ctx.world.iter().any(|e| {
            if let Some(gi) = e.get::<&crate::components::GroundItem>() {
                return gi.object_id == 9001;
            }
            false
        });
        assert!(found, "地面物品实体应存在");
    }

    #[test]
    fn test_item_equipped_flow() {
        let (mut ctx, mut sys) = setup();

        // 创建玩家（带空背包和装备栏）
        let mut packet = make_user_info(1001, "TestHero", 100, 50);
        packet.inventory = Some(vec![None; 40]);
        packet.equipment = Some(vec![None; 14]);
        ctx.events_mut()
            .send_network(NetworkEvent::UserInformation { packet });
        sys.update(&mut ctx, 0.016).unwrap();

        let local_entity = find_local_entity(&ctx);

        // 获得物品然后装备
        let item = mir2_shared::UserItem {
            unique_id: 100,
            item_index: 2001,
            count: 1,
            ..Default::default()
        };
        ctx.events_mut()
            .send_network(NetworkEvent::ItemGained { item });
        sys.update(&mut ctx, 0.016).unwrap();

        // 验证物品已在背包中
        {
            let inv = ctx.world.get::<&Inventory>(local_entity).unwrap();
            assert!(
                inv.items
                    .iter()
                    .any(|s| s.as_ref().map(|it| it.unique_id == 100).unwrap_or(false)),
                "物品应在背包中"
            );
        }

        // 装备物品到武器槽 (slot=0)
        ctx.events_mut().send_network(NetworkEvent::ItemEquipped {
            grid: mir2_shared::enums::MirGridType::Inventory,
            unique_id: 100,
            slot: 0,
            success: true,
        });
        sys.update(&mut ctx, 0.016).unwrap();

        // 验证装备栏有物品
        {
            let eq = ctx.world.get::<&Equipment>(local_entity).unwrap();
            assert!(eq.weapon.is_some(), "武器槽应有装备");
        }
    }
}
