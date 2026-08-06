// ============================================================================
// Layer 3: Combat & Skills - SkillSystem
// Priority: 300
// ============================================================================
//
// **职责**：
// - ✅ 技能施放逻辑（检查MP、冷却、目标）
// - ✅ 技能效果应用（伤害计算、状态附加）
// - ✅ 冷却管理（冷却时间计算、冷却重置）
// - ✅ 技能目标选择和范围检测
//
// **依赖输入**：
// - Layer 1: PlayerInput::UseSpell 事件
// - Layer 2: NpcDialogueSystem 提供的技能学习状态
//
// **输出影响**：
// - 修改 Mana 组件（消耗MP）
// - 修改 SpellCooldown 组件（设置冷却）
// - 发布 NetworkCommand::Magic（通知服务器）
// - Layer 4: 影响移动（施法时可能打断移动）
//
// ============================================================================

use super::super::super::LogicSystem;
use crate::components::{
    LocalPlayer, MagicList, Mana, Monster, NetworkSync, Player, Position, SpellType,
    TargetSelection, TargetType, NPC,
};
use crate::game::GameContext;
use crate::game::GameResult;
use crate::game::KeyCode;
use crate::network::handlers::NetworkEvent as NetworkCommand;
use crossbeam_channel::Sender;
use hecs::World;
use mir2_shared::enums::MirDirection;

/// Layer 3: 技能施放系统
///
/// 处理技能施放的完整流程：
/// 1. 验证技能学习状态
/// 2. 检查MP消耗
/// 3. 检查冷却时间
/// 4. 获取目标信息
/// 5. 发送网络命令
/// 6. 消耗资源（MP、冷却）
#[derive(ecs_macros::LogicSystem)]
pub struct SkillSystem;

impl Default for SkillSystem {
    fn default() -> Self {
        Self
    }
}

impl LogicSystem for SkillSystem {
    fn update(&mut self, ctx: &mut GameContext, _delay_time: f32) -> GameResult {
        // 检查玩家是否有施法意图
        let spell_request = {
            let mut request = None;

            for (_local, input) in ctx
                .world
                .query::<(&LocalPlayer, &crate::components::PlayerInput)>()
                .iter()
            {
                if let Some(spell) = input.cast_spell {
                    request = Some((spell, input.spell_target_pos, input.spell_target_entity));
                    break;
                }
            }

            request
        };

        // 如果有施法请求，执行施法逻辑
        if let Some((spell, _target_pos, _target_entity)) = spell_request {
            // 1. 检查是否已学会该技能
            let learned = ctx
                .world
                .query::<(&LocalPlayer, &MagicList)>()
                .iter()
                .next()
                .map(|(_, ml)| ml.has_learned(spell))
                .unwrap_or(false);

            if !learned {
                tracing::warn!("⚠️ 尚未学会技能: {}", spell.name());

                // 清除施法输入
                #[allow(clippy::never_loop)]
                for (_local, input) in ctx
                    .world
                    .query_mut::<(&LocalPlayer, &mut crate::components::PlayerInput)>()
                {
                    input.cast_spell = None;
                    break;
                }

                return Ok(());
            }

            // 1.5 检查冷却时间（同时清理已过期条目）
            let on_cooldown = {
                let mut cd = false;
                #[allow(clippy::never_loop)]
                for (_local, cooldowns) in ctx
                    .world
                    .query_mut::<(&LocalPlayer, &mut crate::components::spell::SpellCooldowns)>()
                {
                    cooldowns.cleanup();
                    cd = cooldowns.is_on_cooldown(spell as u8);
                    break;
                }
                cd
            };

            if on_cooldown {
                tracing::warn!("⚠️ 技能冷却中: {}", spell.name());

                #[allow(clippy::never_loop)]
                for (_local, input) in ctx
                    .world
                    .query_mut::<(&LocalPlayer, &mut crate::components::PlayerInput)>()
                {
                    input.cast_spell = None;
                    break;
                }

                return Ok(());
            }

            // 2. 检查魔法值
            let mp_cost = Self::get_spell_mp_cost(spell);
            let has_enough_mp = ctx
                .world
                .query::<(&LocalPlayer, &Mana)>()
                .iter()
                .next()
                .map(|(_, mana)| mana.has_enough(mp_cost))
                .unwrap_or(false);

            if !has_enough_mp {
                tracing::warn!("⚠️ 魔法值不足,需要 {} MP", mp_cost);

                // 清除施法输入
                #[allow(clippy::never_loop)]
                for (_local, input) in ctx
                    .world
                    .query_mut::<(&LocalPlayer, &mut crate::components::PlayerInput)>()
                {
                    input.cast_spell = None;
                    break;
                }

                return Ok(());
            }

            // 3. 获取目标信息
            let (direction, target_id, location) = Self::get_target_info(&ctx.world);

            // 4. 发送施法命令到网络
            if let Some(net) = ctx.net.as_ref() {
                let _ = net.send(NetworkCommand::MagicRequest {
                    spell: spell as u8,
                    direction,
                    target_id,
                    location,
                });
            }

            // 5. 设置客户端侧冷却回退（服务器通过 MagicDelayReceived 是权威来源）
            Self::set_client_cooldown(&mut ctx.world, spell);

            // 6. 消耗魔法值
            #[allow(clippy::never_loop)]
            for (_local, mana) in ctx.world.query_mut::<(&LocalPlayer, &mut Mana)>() {
                mana.consume(mp_cost);
                break;
            }

            // 6. 清除施法输入
            #[allow(clippy::never_loop)]
            for (_local, input) in ctx
                .world
                .query_mut::<(&LocalPlayer, &mut crate::components::PlayerInput)>()
            {
                input.cast_spell = None;
                input.spell_target_pos = None;
                input.spell_target_entity = None;
                break;
            }

            tracing::info!("✨ 施放技能: {} (MP: -{})", spell.name(), mp_cost);
        }

        Ok(())
    }
}

impl SkillSystem {
    /// 施放技能（供外部调用）
    pub fn cast_spell(
        world: &mut World,
        spell: SpellType,
        network_tx: &Sender<NetworkCommand>,
    ) -> bool {
        // 1. 检查是否已学会该技能
        let learned = world
            .query::<(&LocalPlayer, &MagicList)>()
            .iter()
            .next()
            .map(|(_, ml)| ml.has_learned(spell))
            .unwrap_or(false);

        if !learned {
            println!("⚠️ 尚未学会技能: {}", spell.name());
            return false;
        }

        // 2. 检查魔法值
        let mp_cost = Self::get_spell_mp_cost(spell);
        let has_enough_mp = world
            .query::<(&LocalPlayer, &Mana)>()
            .iter()
            .next()
            .map(|(_, mana)| mana.has_enough(mp_cost))
            .unwrap_or(false);

        if !has_enough_mp {
            println!("⚠️ 魔法值不足,需要 {} MP", mp_cost);
            return false;
        }

        // 3. 检查冷却时间
        let on_cooldown = world
            .query::<(&LocalPlayer, &crate::components::spell::SpellCooldowns)>()
            .iter()
            .next()
            .map(|(_, cooldowns)| cooldowns.is_on_cooldown(spell as u8))
            .unwrap_or(false);

        if on_cooldown {
            println!("⚠️ 技能冷却中: {}", spell.name());
            return false;
        }

        // 4. 获取目标信息
        let (direction, target_id, location) = Self::get_target_info(world);

        // 5. 发送施法命令
        let _ = network_tx.send(NetworkCommand::MagicRequest {
            spell: spell as u8,
            direction,
            target_id,
            location,
        });

        // 6. 消耗魔法值
        #[allow(clippy::never_loop)]
        for (_local, mana) in world.query_mut::<(&LocalPlayer, &mut Mana)>() {
            mana.consume(mp_cost);
            break;
        }

        // 7. 设置客户端侧冷却回退
        Self::set_client_cooldown(world, spell);

        println!("✨ 施放技能: {} (MP: {})", spell.name(), mp_cost);
        true
    }

    /// 设置客户端侧冷却回退
    ///
    /// 服务器通过 MagicDelayReceived 是冷却的权威来源，
    /// 但客户端在发包后立即设置一个保守的冷却回退，防止玩家连发。
    fn set_client_cooldown(world: &mut World, spell: SpellType) {
        let cooldown_ms = Self::get_client_cooldown_ms(spell);
        #[allow(clippy::never_loop)]
        for (_local, cooldowns) in
            world.query_mut::<(&LocalPlayer, &mut crate::components::spell::SpellCooldowns)>()
        {
            cooldowns.set(spell as u8, cooldown_ms);
            break;
        }
    }

    /// 获取客户端侧冷却回退时长（毫秒）
    ///
    /// 这些值是保守估计，最终会被服务器的 MagicDelayReceived 覆盖。
    fn get_client_cooldown_ms(spell: SpellType) -> u32 {
        match spell {
            // 战士技能：短冷却
            SpellType::Fencing => 0,
            SpellType::Slaying => 1000,
            SpellType::Thrusting => 1500,
            SpellType::HalfMoon => 2000,
            SpellType::ShoulderDash => 5000,
            SpellType::LionRoar => 8000,

            // 法师技能：中等冷却
            SpellType::FireBall => 1000,
            SpellType::Repulsion => 3000,
            SpellType::ElectricShock => 2000,
            SpellType::GreatFireBall => 2000,
            SpellType::HellFire => 2500,
            SpellType::ThunderBolt => 1500,
            SpellType::Teleport => 5000,
            SpellType::Lightning => 2000,
            SpellType::MagicShield => 10000,

            // 道士技能
            SpellType::Healing => 2000,
            SpellType::SpiritSword => 3000,
            SpellType::Poisoning => 3000,
            SpellType::SoulFireBall => 2000,
            SpellType::SummonSkeleton => 10000,
            SpellType::Hiding => 8000,
            SpellType::SoulShield => 10000,

            // 刺客技能
            SpellType::FatalSword => 1000,
            SpellType::DoubleSlash => 1500,
            SpellType::Haste => 8000,
            SpellType::FlashDash => 10000,

            // 弓箭手技能
            SpellType::Focus => 5000,
            SpellType::StraightShot => 1000,
            SpellType::DoubleShot => 2000,
            SpellType::Meditation => 10000,

            _ => 1000, // 默认 1 秒
        }
    }

    /// 获取技能魔法消耗
    fn get_spell_mp_cost(spell: SpellType) -> i32 {
        match spell {
            // Warrior (战士 - 低MP消耗)
            SpellType::Fencing => 0,
            SpellType::Slaying => 2,
            SpellType::Thrusting => 3,
            SpellType::HalfMoon => 4,
            SpellType::ShoulderDash => 5,
            SpellType::LionRoar => 10,

            // Wizard (法师 - 高MP消耗)
            SpellType::FireBall => 5,
            SpellType::Repulsion => 3,
            SpellType::ElectricShock => 4,
            SpellType::GreatFireBall => 9,
            SpellType::HellFire => 12,
            SpellType::ThunderBolt => 8,
            SpellType::Teleport => 20,
            SpellType::Lightning => 15,
            SpellType::MagicShield => 10,

            // Taoist (道士 - 中等MP消耗)
            SpellType::Healing => 8,
            SpellType::SpiritSword => 5,
            SpellType::Poisoning => 3,
            SpellType::SoulFireBall => 6,
            SpellType::SummonSkeleton => 25,
            SpellType::Hiding => 15,
            SpellType::SoulShield => 12,

            // Assassin (刺客 - 低MP消耗)
            SpellType::FatalSword => 2,
            SpellType::DoubleSlash => 4,
            SpellType::Haste => 10,
            SpellType::FlashDash => 8,

            // Archer (弓箭手 - 中等MP消耗)
            SpellType::Focus => 5,
            SpellType::StraightShot => 3,
            SpellType::DoubleShot => 6,
            SpellType::Meditation => 10,

            _ => 5, // 默认消耗
        }
    }

    /// 获取当前目标信息
    fn get_target_info(world: &World) -> (MirDirection, u32, Option<(i32, i32)>) {
        // 查询目标选择组件
        if let Some((_local, target_sel)) = world
            .query::<(&LocalPlayer, &TargetSelection)>()
            .iter()
            .next()
        {
            match target_sel.current {
                TargetType::Monster(id) => {
                    let direction = Self::calculate_direction_to_target(world, id);
                    return (direction, id, None);
                }
                TargetType::Player(id) => {
                    let direction = Self::calculate_direction_to_target(world, id);
                    return (direction, id, None);
                }
                TargetType::NPC(id) => {
                    let direction = Self::calculate_direction_to_target(world, id);
                    return (direction, id, None);
                }
                TargetType::Location(x, y) => {
                    // 地面技能（如：地狱火、冰咆哮）
                    let direction = Self::calculate_direction_to_location(world, x, y);
                    return (direction, 0, Some((x, y)));
                }
                TargetType::None => {
                    // 自身技能（如：魔法盾、隐身）
                    let direction = Self::get_player_direction(world);
                    return (direction, 0, None);
                }
            }
        }

        // 默认朝向下方，无目标
        (MirDirection::Down, 0, None)
    }

    /// 计算朝向目标的方向
    fn calculate_direction_to_target(world: &World, target_id: u32) -> MirDirection {
        // 获取玩家位置
        let player_pos = {
            let mut pos = None;
            for (_local, player_pos) in world.query::<(&LocalPlayer, &Position)>().iter() {
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

            // 检查怪物
            for (_monster, net_sync, target_pos) in
                world.query::<(&Monster, &NetworkSync, &Position)>().iter()
            {
                if net_sync.object_id == target_id {
                    pos = Some((target_pos.x, target_pos.y));
                    break;
                }
            }

            // 检查玩家
            if pos.is_none() {
                for (_player, net_sync, target_pos) in
                    world.query::<(&Player, &NetworkSync, &Position)>().iter()
                {
                    if net_sync.object_id == target_id {
                        pos = Some((target_pos.x, target_pos.y));
                        break;
                    }
                }
            }

            // 检查NPC
            if pos.is_none() {
                for (_npc, net_sync, target_pos) in
                    world.query::<(&NPC, &NetworkSync, &Position)>().iter()
                {
                    if net_sync.object_id == target_id {
                        pos = Some((target_pos.x, target_pos.y));
                        break;
                    }
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

    /// 计算朝向指定位置的方向
    fn calculate_direction_to_location(
        world: &World,
        target_x: i32,
        target_y: i32,
    ) -> MirDirection {
        let player_pos = {
            let mut pos = None;
            for (_local, player_pos) in world.query::<(&LocalPlayer, &Position)>().iter() {
                pos = Some((player_pos.x, player_pos.y));
                break;
            }
            pos
        };

        if let Some((px, py)) = player_pos {
            // 转换格子坐标到像素坐标
            let target_px = target_x as f32 * 48.0;
            let target_py = target_y as f32 * 32.0;
            Self::calculate_direction(px, py, target_px, target_py)
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
            a if (-22.5..22.5).contains(&a) => MirDirection::Right,
            a if (22.5..67.5).contains(&a) => MirDirection::DownRight,
            a if (67.5..112.5).contains(&a) => MirDirection::Down,
            a if (112.5..157.5).contains(&a) => MirDirection::DownLeft,
            a if !(-157.5..157.5).contains(&a) => MirDirection::Left,
            a if (-157.5..-112.5).contains(&a) => MirDirection::UpLeft,
            a if (-112.5..-67.5).contains(&a) => MirDirection::Up,
            a if (-67.5..-22.5).contains(&a) => MirDirection::UpRight,
            _ => MirDirection::Down,
        }
    }

    /// 获取玩家当前朝向
    fn get_player_direction(world: &World) -> MirDirection {
        if let Some((_local, player)) = world.query::<(&LocalPlayer, &Player)>().iter().next() {
            // Player.direction 现在已经是 MirDirection 类型，直接返回
            return player.direction;
        }
        MirDirection::Down
    }
}

// ============================================================================
// SpellInputSystem - 快捷键技能输入
// Priority: 118 (between NETWORK_APPLY and PLAYER_CONTROL)
// ============================================================================
//
// 职责：
// - 检测 F1-F8 按键
// - 根据 MagicList 中 key_slot 绑定或默认顺序映射到 SpellType
// - 写入 PlayerInput.cast_spell 供 SkillSystem 消费
//

/// 快捷键技能输入系统
#[derive(ecs_macros::LogicSystem, Default)]
pub struct SpellInputSystem {
    prev_f_keys: [bool; 8], // F1-F8 上一帧状态
}

impl LogicSystem for SpellInputSystem {
    fn update(&mut self, ctx: &mut GameContext, _dt: f32) -> GameResult {
        use crate::components::{LocalPlayer, MagicList, PlayerInput};

        let f_keys: [KeyCode; 8] = [
            KeyCode::F1,
            KeyCode::F2,
            KeyCode::F3,
            KeyCode::F4,
            KeyCode::F5,
            KeyCode::F6,
            KeyCode::F7,
            KeyCode::F8,
        ];

        // 找到本地玩家实体
        let player_entity = ctx
            .world
            .iter()
            .find_map(|e| e.get::<&LocalPlayer>().map(|_| e.entity()));

        let Some(player_entity) = player_entity else {
            // 更新上一帧状态
            for (i, &key) in f_keys.iter().enumerate() {
                self.prev_f_keys[i] = ctx.input().key_pressed(key);
            }
            return Ok(());
        };

        // 检查是否有施法输入（已有则跳过快捷键）
        let has_existing_spell = ctx
            .world
            .get::<&PlayerInput>(player_entity)
            .ok()
            .map(|input| input.cast_spell.is_some())
            .unwrap_or(false);

        if has_existing_spell {
            // 仍更新按键状态
            for (i, &key) in f_keys.iter().enumerate() {
                self.prev_f_keys[i] = ctx.input().key_pressed(key);
            }
            return Ok(());
        }

        // 检测 F1-F8 边缘触发
        let mut triggered_slot: Option<usize> = None;
        for (i, &key) in f_keys.iter().enumerate() {
            let pressed = ctx.input().key_pressed(key);
            let just_pressed = pressed && !self.prev_f_keys[i];
            self.prev_f_keys[i] = pressed;

            if just_pressed {
                triggered_slot = Some(i);
                break;
            }
        }

        let Some(slot_idx) = triggered_slot else {
            return Ok(());
        };

        // 根据 MagicList 中 key_slot 绑定查找技能
        let slot_u8 = (slot_idx + 1) as u8; // 1-based slot
        let spell = ctx
            .world
            .get::<&MagicList>(player_entity)
            .ok()
            .and_then(|ml| ml.get_by_slot(slot_u8).map(|m| m.spell))
            .or_else(|| {
                // 退而求其次：按 MagicList 中的顺序取第 N 个
                ctx.world
                    .get::<&MagicList>(player_entity)
                    .ok()
                    .and_then(|ml| ml.magics.get(slot_idx).map(|m| m.spell))
            });

        if let Some(spell) = spell {
            if let Ok(mut input) = ctx.world.get::<&mut PlayerInput>(player_entity) {
                input.cast_spell = Some(spell);
                input.spell_target_pos = None;
                input.spell_target_entity = None;
                tracing::debug!("⌨️ 快捷键触发: {:?} (F{})", spell, slot_idx + 1);
            }
        } else {
            tracing::trace!("⌨️ F{} 无绑定技能", slot_idx + 1);
        }

        Ok(())
    }
}
