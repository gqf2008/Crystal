// ============================================================================
// Layer 3: Combat & Skills - CombatSystem
// Priority: 310
// ============================================================================
// 
// **职责**：
// - ✅ 战斗伤害计算（物理/魔法伤害公式）
// - ✅ 命中判定（攻击范围、命中率、闪避）
// - ✅ 暴击处理（暴击率、暴击倍率）
// - ✅ 死亡判断（HP归零、死亡动画触发）
// - ✅ 攻击范围检测
// 
// **依赖输入**：
// - Layer 1: PlayerInput::Attack 事件
// - Layer 2: MonsterAISystem 产生的攻击意图
// 
// **输出影响**：
// - 修改 Health 组件（扣血/死亡）
// - 发布 NetworkCommand::Attack（通知服务器）
// - 触发音效/特效（Layer 7 Render）
// 
// ============================================================================

use hecs::{Entity, World};
use crate::game::GameContext;
use crate::components::{
    ClientOnly, LocalPlayer, Position, Health, Monster, NetworkSync, CombatStats, NetworkObjectType,
    MountState, MountStatus, MirClass, PlayerAppearance, SoundTrigger, SoundType,
};
use crate::components::network::Lifetime;
use crate::network::handlers::NetworkEvent as NetworkCommand;
use mir2_shared::enums::MirDirection;
use crate::game::GameResult;
use super::super::super::LogicSystem;
use std::sync::OnceLock;

fn combat_log_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("CRYSTAL_COMBAT_LOG").is_some())
}

/// 伤害类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DamageType {
    Physical,      // 物理伤害
    Magic,         // 魔法伤害
    Poison,        // 毒素伤害
    Holy,          // 神圣伤害
}

/// 战斗结果
#[derive(Debug, Clone)]
pub struct CombatResult {
    pub damage: i32,
    pub is_critical: bool,
    pub damage_type: DamageType,
}

/// Layer 3: 战斗系统
/// 
/// 处理战斗相关的所有逻辑：
/// 1. 伤害计算（物理/魔法/毒素/神圣）
/// 2. 命中判定和闪避
/// 3. 暴击判定和暴击伤害
/// 4. 死亡判断和处理
/// 5. 攻击范围检测
#[derive(ecs_macros::LogicSystem)]
pub struct CombatSystem;

impl Default for CombatSystem {
    fn default() -> Self {
        Self
    }
}

impl LogicSystem for CombatSystem {
    

    fn update(&mut self, ctx:&mut GameContext, _delay_time: f32) -> GameResult {
        // 原版体验：右键点怪后持续追砍，直到目标死亡或玩家取消。
        // 这里保持 server-authoritative 结果：客户端只发 AttackRequest。

        // 1) 取本地玩家 attack_target
        let (player_entity, target_entity) = {
            let mut result: Option<(hecs::Entity, hecs::Entity)> = None;
            for (e, (_local, input)) in ctx
                .world
                .query::<(&LocalPlayer, &crate::components::PlayerInput)>()
                .iter()
            {
                if let Some(t) = input.attack_target {
                    result = Some((e, t));
                    break;
                }
            }
            match result {
                None => return Ok(()),
                Some(v) => v,
            }
        };

        // 死亡时不允许继续出手/追砍
        if let Ok(hp) = ctx.world.get::<&Health>(player_entity) {
            if hp.current <= 0 {
                if let Ok(mut input) = ctx.world.get::<&mut crate::components::PlayerInput>(player_entity) {
                    input.attack_target = None;
                    input.move_to = None;
                    input.movement_mode = crate::components::MovementMode::None;
                }
                return Ok(());
            }
        }

        // 2) 目标实体拿 object_id；拿不到说明目标已被移除
        let target_id = match ctx.world.get::<&NetworkSync>(target_entity).ok() {
            None => {
                if let Ok(mut input) = ctx.world.get::<&mut crate::components::PlayerInput>(player_entity) {
                    input.attack_target = None;
                }
                return Ok(());
            }
            Some(sync) => sync.object_id,
        };

        // 3) 目标必须是怪物且仍存活（HP>0）
        let target_alive = ctx
            .world
            .query::<(&Monster, &NetworkSync, &Health)>()
            .iter()
            .any(|(_, (_m, sync, hp))| sync.object_id == target_id && hp.current > 0);

        if !target_alive {
            if let Ok(mut input) = ctx.world.get::<&mut crate::components::PlayerInput>(player_entity) {
                input.attack_target = None;
            }
            return Ok(());
        }

        // 4) 不在近战范围：保持 attack_target（输入系统会负责自动走近）
        if !Self::is_in_melee_range(&ctx.world, target_id) {
            return Ok(());
        }

        // 5) 正在攻击动画中：本帧不重复出手（AttackState 作为冷却）
        if ctx.world.get::<&crate::components::AttackState>(player_entity).is_ok() {
            return Ok(());
        }

        // 6) 进入范围且不在攻击中：面向目标 + 播放攻击动画 + 发 AttackRequest
        let direction = Self::calculate_attack_direction(&ctx.world, target_id);
        if let Ok(mut player) = ctx.world.get::<&mut crate::components::Player>(player_entity) {
            player.direction = direction;
            player.action = crate::components::PlayerAction::Attack1;
        }

        let _ = ctx.world.insert_one(
            player_entity,
            crate::components::AttackState {
                start_time: std::time::Instant::now(),
                attack_type: crate::components::PlayerAction::Attack1,
                server_attack_type: 0,
            },
        );

        // test_game_scene 下本地批量刷新的怪物并不在服务器/MockServer 对象表中。
        // 这种目标不会收到 ObjectStruck，因此血条会永远满。
        // 给这些怪物打上 ClientOnly 后，CombatSystem 直接在客户端扣血，保证可见闭环。
        let is_client_only_target = ctx
            .world
            .query::<(&ClientOnly, &NetworkSync)>()
            .iter()
            .any(|(_, (_co, sync))| sync.object_id == target_id);

        if ctx.session.server_authoritative_combat && !is_client_only_target {
            if let Some(net) = ctx.net() {
                let _ = net.send(NetworkCommand::AttackRequest {
                    direction,
                    spell: 0,
                });
            }
        }

        // 攻击时停止继续移动（避免贴脸后还继续跑）
        if let Ok(mut input) = ctx.world.get::<&mut crate::components::PlayerInput>(player_entity) {
            input.move_to = None;
            input.movement_mode = crate::components::MovementMode::None;
        }

        let preview = Self::calculate_local_attack_preview(&ctx.world, target_id);

        // 本地扣血：
        // - 非服务器权威战斗：直接扣血
        // - 服务器权威战斗：仅对 ClientOnly 目标扣血（否则会与服务器回包重复）
        if (!ctx.session.server_authoritative_combat) || is_client_only_target {
            Self::take_damage(&mut ctx.world, target_id, preview.damage, preview.damage_type);
        }

        // 攻击音效改为由 AnimationSystem 按“攻击动作帧”触发（更接近 C# 原版）。
        
        Ok(())
    }
}

impl CombatSystem {
    pub(crate) fn choose_player_attack_sound_id(world: &World, player_entity: Entity) -> Option<i32> {
        // 对齐 C# PlayerObject.PlayAttackSound()
        // - RidingMount: MountType < 7 => TigerAttack 10181..10183; < 12 => WolfAttack 10190..10192
        // - Assassin with weapon: SwingShort
        // - Archer + HasClassWeapon: return (Rust 暂无 HasClassWeapon 判定)
        // - else: weapon switch => Swing*

        const SWING_SHORT: i32 = 10050;
        const SWING_WOOD: i32 = 10051;
        const SWING_SWORD: i32 = 10052;
        const SWING_SWORD2: i32 = 10053;
        const SWING_AXE: i32 = 10054;
        const SWING_CLUB: i32 = 10055;
        const SWING_LONG: i32 = 10056;
        const SWING_FIST: i32 = 10056;

        let (class, weapon) = world
            .get::<&PlayerAppearance>(player_entity)
            .map(|a| (a.class, a.weapon))
            .unwrap_or((MirClass::Warrior, -1));

        // C# Globals.ClassWeaponCount = 100
        let has_class_weapon = {
            let group: i16 = weapon / 100;
            match group {
                1 => class == MirClass::Assassin,
                2 => class == MirClass::Archer,
                _ => matches!(class, MirClass::Wizard | MirClass::Warrior | MirClass::Taoist),
            }
        };

        let mount_status = world.get::<&MountStatus>(player_entity).ok().map(|r| *r);
        let mount_state = world.get::<&MountState>(player_entity).ok().map(|r| *r);

        let riding_from_status = mount_status.map(|m| m.is_riding()).unwrap_or(false);
        let riding_from_state = mount_state.map(|m| m.mount_index.is_some()).unwrap_or(false);
        let is_riding = riding_from_status || riding_from_state;

        let mount_type = mount_status
            .map(|m| m.mount_type)
            .or_else(|| mount_state.and_then(|m| m.mount_index.map(|idx| idx as i16)))
            .unwrap_or(0);

        if is_riding {
            use rand::Rng;
            let mut rng = rand::rng();
            if mount_type < 7 {
                return Some(rng.random_range(10181..=10183));
            } else if mount_type < 12 {
                return Some(rng.random_range(10190..=10192));
            }
            return None;
        }

        if weapon >= 0 && class == MirClass::Assassin {
            return Some(SWING_SHORT);
        }

        // 原版：弓箭手拿职业武器（弓）时不播放 Swing
        if class == MirClass::Archer && has_class_weapon {
            return None;
        }

        match weapon {
            0 | 23 | 28 | 40 => Some(SWING_WOOD),
            1 | 12 => Some(SWING_SHORT),
            2 | 8 | 11 | 15 | 18 | 20 | 25 | 31 | 33 | 34 | 37 | 41 => Some(SWING_SWORD),
            3 | 5 | 7 | 9 | 13 | 19 | 24 | 26 | 29 | 32 | 35 => Some(SWING_SWORD2),
            4 | 14 | 16 | 38 => Some(SWING_AXE),
            6 | 10 | 17 | 22 | 27 | 30 | 36 | 39 => Some(SWING_LONG),
            21 => Some(SWING_CLUB),
            _ => Some(SWING_FIST),
        }
    }

    fn is_in_melee_range(world: &World, target_id: u32) -> bool {
        let player_grid = world
            .query::<(&LocalPlayer, &Position)>()
            .iter()
            .next()
            .map(|(_, (_local, pos))| crate::coord::Coord::world_to_grid(pos.x, pos.y));
        let Some((pgx, pgy)) = player_grid else {
            return false;
        };

        let target_grid = world
            .query::<(&Monster, &NetworkSync, &Position)>()
            .iter()
            .find_map(|(_, (_m, sync, pos))| {
                if sync.object_id == target_id {
                    Some(crate::coord::Coord::world_to_grid(pos.x, pos.y))
                } else {
                    None
                }
            });
        let Some((tgx, tgy)) = target_grid else {
            return false;
        };

        let dx = (tgx - pgx).abs();
        let dy = (tgy - pgy).abs();
        dx.max(dy) <= 1
    }

    /// 计算物理攻击伤害
    /// 
    /// 伤害计算公式：
    /// 1. 基础伤害 = 随机(最小攻击, 最大攻击)
    /// 2. 防御减伤 = 防御 * 0.5 (最多减免80%基础伤害)
    /// 3. 等级修正 = 1.0 + (攻击者等级 - 目标等级) * 0.02 (范围 0.5x ~ 1.5x)
    /// 4. 暴击判定 = 10%概率，1.5倍伤害
    pub fn calculate_physical_damage(
        attacker_attack: (i32, i32),  // (最小攻击, 最大攻击)
        target_defense: i32,
        attacker_level: u16,
        target_level: u16,
    ) -> CombatResult {
        use rand::Rng;
        let mut rng = rand::rng();
        
        // 1. 基础伤害 = 随机(最小攻击, 最大攻击)
        let base_damage = rng.random_range(attacker_attack.0..=attacker_attack.1);
        
        // 2. 防御减伤
        let defense_reduction = (target_defense as f32 * 0.5).min(base_damage as f32 * 0.8);
        let mut damage = (base_damage as f32 - defense_reduction).max(1.0) as i32;
        
        // 3. 等级差异修正
        let level_diff = attacker_level as i32 - target_level as i32;
        if level_diff > 0 {
            damage = (damage as f32 * (1.0 + level_diff as f32 * 0.02)).min(damage as f32 * 1.5) as i32;
        } else if level_diff < 0 {
            damage = (damage as f32 * (1.0 + level_diff as f32 * 0.02)).max(damage as f32 * 0.5) as i32;
        }
        
        // 4. 暴击判定 (10%概率)
        let is_critical = rng.random_ratio(1, 10);
        if is_critical {
            damage = (damage as f32 * 1.5) as i32;
        }
        
        CombatResult {
            damage: damage.max(1),
            is_critical,
            damage_type: DamageType::Physical,
        }
    }
    
    /// 计算魔法伤害
    /// 
    /// 伤害计算公式：
    /// 1. 基础伤害 = 随机(最小魔攻, 最大魔攻) + 技能威力
    /// 2. 魔防减伤 = 魔防 * 0.3 (最多减免70%基础伤害)
    /// 3. 等级修正 = 1.0 + (攻击者等级 - 目标等级) * 0.03 (范围 0.3x ~ 2.0x)
    /// 4. 暴击判定 = 5%概率，2.0倍伤害
    pub fn calculate_magic_damage(
        attacker_magic: (i32, i32),   // (最小魔攻, 最大魔攻)
        target_magic_defense: i32,
        spell_power: i32,              // 技能威力系数
        attacker_level: u16,
        target_level: u16,
    ) -> CombatResult {
        use rand::Rng;
        let mut rng = rand::rng();
        
        // 1. 基础伤害 = 随机(最小魔攻, 最大魔攻) + 技能威力
        let base_damage = rng.random_range(attacker_magic.0..=attacker_magic.1) + spell_power;
        
        // 2. 魔法防御减伤
        let defense_reduction = (target_magic_defense as f32 * 0.3).min(base_damage as f32 * 0.7);
        let mut damage = (base_damage as f32 - defense_reduction).max(1.0) as i32;
        
        // 3. 等级差异修正
        let level_diff = attacker_level as i32 - target_level as i32;
        if level_diff > 0 {
            damage = (damage as f32 * (1.0 + level_diff as f32 * 0.03)).min(damage as f32 * 2.0) as i32;
        } else if level_diff < 0 {
            damage = (damage as f32 * (1.0 + level_diff as f32 * 0.03)).max(damage as f32 * 0.3) as i32;
        }
        
        // 4. 暴击判定 (5%概率)
        let is_critical = rng.random_ratio(1, 20);
        if is_critical {
            damage = (damage as f32 * 2.0) as i32;
        }
        
        CombatResult {
            damage: damage.max(1),
            is_critical,
            damage_type: DamageType::Magic,
        }
    }
    
    /// 本地预览伤害 (不修改实际数据)
    fn calculate_local_attack_preview(world: &World, target_id: u32) -> CombatResult {
        // 获取本地玩家的攻击力和等级
        let (player_attack, player_level) = {
            let mut attack = (5, 10);
            let mut level = 1;
            
            for (_, (_local, combat_stats)) in world.query::<(&LocalPlayer, &CombatStats)>().iter() {
                attack = (combat_stats.attack_min, combat_stats.attack_max);
                level = combat_stats.level;
                break;
            }
            
            (attack, level)
        };
        
        // 获取目标怪物的防御力和等级
        let (target_defense, target_level) = {
            let mut defense = 0;
            let mut level = 1;
            
            for (_, (sync, combat_stats)) in world.query::<(&NetworkSync, &CombatStats)>().iter() {
                if sync.object_id == target_id && sync.object_type == NetworkObjectType::Monster {
                    defense = combat_stats.defense;
                    level = combat_stats.level;
                    break;
                }
            }
            
            (defense, level)
        };
        
        // 计算预期伤害
        let result = Self::calculate_physical_damage(
            player_attack,
            target_defense,
            player_level,
            target_level,
        );
        
        if combat_log_enabled() {
            println!(
                "💥 预期伤害: {} {}",
                result.damage,
                if result.is_critical { "(暴击!)" } else { "" }
            );
        }

        result
    }
    
    /// 处理受到伤害
    pub fn take_damage(
        world: &mut World,
        target_id: u32,
        damage: i32,
        damage_type: DamageType,
    ) {
        // 查找目标并扣血
        let mut target_entity: Option<hecs::Entity> = None;
        let mut target_monster_type: Option<u16> = None;
        let mut target_died = false;
        let mut target_is_client_only = false;

        for (entity, (monster, health, net_sync)) in world.query_mut::<(&Monster, &mut Health, &NetworkSync)>() {
            if net_sync.object_id == target_id {
                let old_hp = health.current;
                health.current = (health.current - damage).max(0);
                
                if combat_log_enabled() {
                    println!(
                        "🩸 {} 受到 {} 点{:?}伤害 (HP: {} → {})",
                        target_id,
                        damage,
                        damage_type,
                        old_hp,
                        health.current
                    );
                }
                
                // 检查是否死亡
                if combat_log_enabled() && health.current == 0 {
                    println!("💀 {} 已死亡", target_id);
                }

                target_entity = Some(entity);
                target_monster_type = Some(monster.monster_type);
                target_died = health.current == 0;
                target_is_client_only = world.get::<&ClientOnly>(entity).is_ok();
                
                break;
            }
        }

        // ClientOnly 怪物：死亡闭环（播放死亡动作 + 延时移除）
        if let Some(entity) = target_entity {
            if target_died && target_is_client_only {
                if let Ok(mut s) = world.get::<&mut crate::components::MonsterAnimState>(entity) {
                    s.action = crate::components::MirAction::Die;
                    s.start_time = std::time::Instant::now();
                }

                // 避免重复插入：有就刷新时间
                if world.insert_one(entity, Lifetime::new(1200)).is_err() {
                    let _ = world.remove_one::<Lifetime>(entity);
                    let _ = world.insert_one(entity, Lifetime::new(1200));
                }
            }
        }

        // Layer 3: 伤害事件 -> SoundTrigger
        // 真正播放由 Layer 5 的 SoundSystem 统一处理（含音量策略/距离衰减）。
        if let Some(entity) = target_entity {
            // 按 Crystal 原版规则（见根目录 SoundRules.txt / C# MonsterObject）：
            // BaseSound = monster_type * 10
            // 2 = Flinch, 3 = Die
            if let Some(monster_type) = target_monster_type {
                let base_sound = monster_type as i32 * 10;
                let sound_id = if target_died { base_sound + 3 } else { base_sound + 2 };
                let _ = world.insert_one(
                    entity,
                    SoundTrigger::once(sound_id.to_string(), SoundType::CharacterAction),
                );
            }
        }
        
        // 玩家受伤
        for (_, (_, health)) in world.query_mut::<(&LocalPlayer, &mut Health)>() {
            let old_hp = health.current;
            health.current = (health.current - damage).max(0);
            
            if combat_log_enabled() {
                println!(
                    "❤️ 玩家受到 {} 点{:?}伤害 (HP: {} → {})",
                    damage,
                    damage_type,
                    old_hp,
                    health.current
                );
            }
            
            if combat_log_enabled() && health.current == 0 {
                println!("💀 玩家死亡");
            }
            
            break;
        }
    }
    
    /// 检查攻击范围
    pub fn is_in_attack_range(
        world: &World,
        target_id: u32,
        range: i32,  // 攻击范围(格子数)
    ) -> bool {
        // 获取玩家位置
        let player_pos = {
            let mut pos = Position::new(0.0, 0.0);
            for (_, (_, p)) in world.query::<(&LocalPlayer, &Position)>().iter() {
                pos = *p;
                break;
            }
            pos
        };
        
        // 获取目标位置
        for (_, (_, pos, net_sync)) in world.query::<(&Monster, &Position, &NetworkSync)>().iter() {
            if net_sync.object_id == target_id {
                let dx = (pos.x - player_pos.x) / 48.0;
                let dy = (pos.y - player_pos.y) / 32.0;
                let distance = ((dx * dx + dy * dy).sqrt()) as i32;
                
                return distance <= range;
            }
        }
        
        false
    }
    
    /// 查找攻击范围内的目标
    pub fn find_targets_in_range(
        world: &World,
        center: Position,
        range: f32,
    ) -> Vec<u32> {
        let mut targets = Vec::new();
        
        for (_, (_, pos, net_sync)) in world.query::<(&Monster, &Position, &NetworkSync)>().iter() {
            let dx = pos.x - center.x;
            let dy = pos.y - center.y;
            let distance = (dx * dx + dy * dy).sqrt();
            
            if distance <= range {
                targets.push(net_sync.object_id);
            }
        }
        
        targets
    }
    
    /// 计算朝向目标的攻击方向
    fn calculate_attack_direction(world: &World, target_id: u32) -> MirDirection {
        // 获取玩家位置
        let player_pos = {
            let mut pos = None;
            for (_, (_, player_pos)) in world.query::<(&LocalPlayer, &Position)>().iter() {
                pos = Some((player_pos.x, player_pos.y));
                break;
            }
            pos
        };
        
        let Some((px, py)) = player_pos else {
            return MirDirection::Down;
        };
        
        // 查找目标位置
        let target_pos = {
            let mut pos = None;
            
            for (_, (_, net_sync, target_pos)) in world.query::<(&Monster, &NetworkSync, &Position)>().iter() {
                if net_sync.object_id == target_id {
                    pos = Some((target_pos.x, target_pos.y));
                    break;
                }
            }
            
            pos
        };
        
        if let Some((tx, ty)) = target_pos {
            Self::calculate_direction(px, py, tx, ty)
        } else {
            MirDirection::Down
        }
    }
    
    /// 计算方向（8方向）
    fn calculate_direction(from_x: f32, from_y: f32, to_x: f32, to_y: f32) -> MirDirection {
        let dx = to_x - from_x;
        let dy = to_y - from_y;
        
        if dx == 0.0 && dy == 0.0 {
            return MirDirection::Down;
        }
        
        let angle = dy.atan2(dx);
        let angle_deg = angle.to_degrees();
        
        // 8方向划分（每方向45度）
        match angle_deg {
            a if a >= -22.5 && a < 22.5 => MirDirection::Right,
            a if a >= 22.5 && a < 67.5 => MirDirection::DownRight,
            a if a >= 67.5 && a < 112.5 => MirDirection::Down,
            a if a >= 112.5 && a < 157.5 => MirDirection::DownLeft,
            a if a >= 157.5 || a < -157.5 => MirDirection::Left,
            a if a >= -157.5 && a < -112.5 => MirDirection::UpLeft,
            a if a >= -112.5 && a < -67.5 => MirDirection::Up,
            a if a >= -67.5 && a < -22.5 => MirDirection::UpRight,
            _ => MirDirection::Down,
        }
    }
}
