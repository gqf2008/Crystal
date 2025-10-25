// ============================================================================
// 战斗系统 - 处理攻击、伤害计算、战斗效果
// ============================================================================

use hecs::World;
use crate::ecs::components::{
    LocalPlayer, Position, Health, Monster, Player, NetworkSync,
    Equipment, MirClass, PlayerData
};
use crate::network::NetworkCommand;
use tokio::sync::mpsc;
use mir2_shared::enums::MirDirection;

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

/// 战斗系统
pub struct CombatSystem;

impl CombatSystem {
    pub fn new() -> Self {
        Self
    }
    
    /// 计算物理攻击伤害
    pub fn calculate_physical_damage(
        attacker_attack: (i32, i32),  // (最小攻击, 最大攻击)
        target_defense: i32,
        attacker_level: u16,
        target_level: u16,
    ) -> CombatResult {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        // 1. 基础伤害 = 随机(最小攻击, 最大攻击)
        let base_damage = rng.gen_range(attacker_attack.0..=attacker_attack.1);
        
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
        let is_critical = rng.gen_ratio(1, 10);
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
    pub fn calculate_magic_damage(
        attacker_magic: (i32, i32),   // (最小魔攻, 最大魔攻)
        target_magic_defense: i32,
        spell_power: i32,              // 技能威力系数
        attacker_level: u16,
        target_level: u16,
    ) -> CombatResult {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        // 1. 基础伤害 = 随机(最小魔攻, 最大魔攻) + 技能威力
        let base_damage = rng.gen_range(attacker_magic.0..=attacker_magic.1) + spell_power;
        
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
        let is_critical = rng.gen_ratio(1, 20);
        if is_critical {
            damage = (damage as f32 * 2.0) as i32;
        }
        
        CombatResult {
            damage: damage.max(1),
            is_critical,
            damage_type: DamageType::Magic,
        }
    }
    
    /// 玩家攻击怪物
    pub fn player_attack_monster(
        world: &mut World,
        target_id: u32,
        direction: MirDirection,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> bool {
        // 检查目标是否存在
        let target_exists = {
            let mut exists = false;
            for (_, (_, net_sync)) in world.query::<(&Monster, &NetworkSync)>().iter() {
                if net_sync.object_id == target_id {
                    exists = true;
                    break;
                }
            }
            exists
        };
        
        if !target_exists {
            println!("⚠️ 攻击目标不存在");
            return false;
        }
        
        println!("⚔️ 攻击怪物: ID={}", target_id);
        
        // 发送攻击命令到服务器
        let _ = network_tx.send(NetworkCommand::Attack {
            direction,
            spell: mir2_shared::enums::Spell::None,
        });
        
        // 本地预计算伤害 (实际伤害由服务器计算)
        Self::calculate_local_attack_preview(world, target_id);
        
        true
    }
    
    /// 本地预览伤害 (不修改实际数据)
    fn calculate_local_attack_preview(world: &World, target_id: u32) {
        // 获取玩家攻击力 (简化版，实际应该从Equipment计算)
        let (player_attack, player_level) = {
            let mut attack = (10, 20);  // 默认攻击力
            let mut level = 1;
            
            for (_, (_, player_comp)) in world.query::<(&LocalPlayer, &PlayerData)>().iter() {
                level = 10; // TODO: 从实际数据获取
                // TODO: 从装备计算攻击力
                attack = match player_comp.class {
                    MirClass::Warrior => (15, 30),
                    MirClass::Wizard => (5, 15),
                    MirClass::Taoist => (8, 20),
                    MirClass::Assassin => (12, 25),
                    MirClass::Archer => (10, 22),
                };
                break;
            }
            
            (attack, level)
        };
        
        // 获取目标防御力 (简化版)
        let target_defense = 10; // TODO: 从怪物数据获取
        let target_level = 5;    // TODO: 从怪物数据获取
        
        // 计算预期伤害
        let result = Self::calculate_physical_damage(
            player_attack,
            target_defense,
            player_level,
            target_level,
        );
        
        println!("💥 预期伤害: {} {}", 
            result.damage,
            if result.is_critical { "(暴击!)" } else { "" }
        );
    }
    
    /// 处理受到伤害
    pub fn take_damage(
        world: &mut World,
        target_id: u32,
        damage: i32,
        damage_type: DamageType,
    ) {
        // 查找目标并扣血
        for (_, (_, health, net_sync)) in world.query_mut::<(&Monster, &mut Health, &NetworkSync)>() {
            if net_sync.object_id == target_id {
                let old_hp = health.current;
                health.current = (health.current - damage).max(0);
                
                println!("🩸 {} 受到 {} 点{:?}伤害 (HP: {} → {})",
                    target_id,
                    damage,
                    damage_type,
                    old_hp,
                    health.current
                );
                
                // 检查是否死亡
                if health.current == 0 {
                    println!("💀 {} 已死亡", target_id);
                }
                
                break;
            }
        }
        
        // 玩家受伤
        for (_, (_, health)) in world.query_mut::<(&LocalPlayer, &mut Health)>() {
            let old_hp = health.current;
            health.current = (health.current - damage).max(0);
            
            println!("❤️ 玩家受到 {} 点{:?}伤害 (HP: {} → {})",
                damage,
                damage_type,
                old_hp,
                health.current
            );
            
            if health.current == 0 {
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
}

/// 技能效果系统
pub struct SkillEffectSystem;

impl SkillEffectSystem {
    pub fn new() -> Self {
        Self
    }
    
    /// 应用技能效果
    pub fn apply_skill_effect(
        world: &mut World,
        spell_type: crate::ecs::components::SpellType,
        caster_id: u32,
        target_id: Option<u32>,
        location: Option<(i32, i32)>,
    ) {
        use crate::ecs::components::SpellType;
        
        match spell_type {
            // 单体攻击技能
            SpellType::FireBall | SpellType::ThunderBolt | SpellType::Lightning => {
                if let Some(target) = target_id {
                    println!("🔥 施放 {:?} 攻击目标 {}", spell_type, target);
                    // TODO: 应用魔法伤害
                }
            }
            
            // 范围攻击技能
            SpellType::HellFire | SpellType::MeteorStrike => {
                if let Some((x, y)) = location {
                    let center = Position::from_grid(x, y);
                    let targets = CombatSystem::find_targets_in_range(world, center, 96.0); // 2格范围
                    println!("💥 {:?} 范围攻击，命中 {} 个目标", spell_type, targets.len());
                    // TODO: 对所有目标应用伤害
                }
            }
            
            // 治疗技能
            SpellType::Healing => {
                if let Some(target) = target_id {
                    println!("💚 治疗目标 {}", target);
                    // TODO: 恢复HP
                }
            }
            
            // 辅助技能
            SpellType::MagicShield | SpellType::SoulShield => {
                println!("🛡️ 施放护盾: {:?}", spell_type);
                // TODO: 应用护盾效果
            }
            
            _ => {
                println!("⚠️ 技能效果未实现: {:?}", spell_type);
            }
        }
    }
}

