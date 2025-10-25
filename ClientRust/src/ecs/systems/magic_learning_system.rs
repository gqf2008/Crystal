// ============================================================================
// 技能学习系统 - 处理技能学习逻辑
// ============================================================================

use hecs::{World, Entity};
use crate::ecs::components::{
    MagicList, LearnableMagicList, SpellType, PlayerData, LocalPlayer
};
use crate::ecs::ui::MagicLearningDialogComp;
use crate::network::NetworkCommand;
use tokio::sync::mpsc;

/// 技能学习系统
pub struct MagicLearningSystem;

impl MagicLearningSystem {
    pub fn new() -> Self {
        Self
    }
    
    /// 更新可学习技能列表显示
    pub fn update_available_magics(world: &mut World) {
        // 获取本地玩家信息
        let player_info = {
            let mut player_level = 1u16;
            let mut player_class = mir2_shared::MirClass::Warrior;
            
            for (_, (_, player)) in world.query::<(&LocalPlayer, &PlayerData)>().iter() {
                player_level = (player.exp / 1000) as u16; // 简化的等级计算
                player_class = player.class;
                break;
            }
            
            (player_level, player_class)
        };
        
        // 获取已学技能列表
        let learned_magics = {
            let mut magics = MagicList::new();
            for (_, (_, magic_list)) in world.query::<(&LocalPlayer, &MagicList)>().iter() {
                magics = magic_list.clone();
                break;
            }
            magics
        };
        
        // 更新对话框显示
        for (_, dialog_comp) in world.query_mut::<&mut MagicLearningDialogComp>() {
            // 获取职业的可学技能列表
            let learnable = LearnableMagicList::init_for_class(player_info.1);
            let available = learnable.get_available(player_info.0, &learned_magics);
            
            // 转换为 (技能, 所需等级) 格式
            let available_with_level: Vec<(SpellType, u16)> = learnable.spells.iter()
                .filter(|(spell, _)| available.contains(spell))
                .map(|(spell, level)| (*spell, *level))
                .collect();
            
            dialog_comp.dialog.set_available_magics(available_with_level);
        }
    }
    
    /// 学习技能
    pub fn learn_magic(
        world: &mut World,
        spell: SpellType,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> bool {
        // 检查玩家等级是否满足
        let can_learn = {
            let mut player_level = 1u16;
            let mut player_class = mir2_shared::MirClass::Warrior;
            
            for (_, (_, player)) in world.query::<(&LocalPlayer, &PlayerData)>().iter() {
                player_level = (player.exp / 1000) as u16;
                player_class = player.class;
                break;
            }
            
            // 检查职业匹配
            if spell.required_class() != player_class {
                println!("⚠️ 职业不符,无法学习该技能");
                return false;
            }
            
            // 检查等级要求
            let learnable = LearnableMagicList::init_for_class(player_class);
            if let Some((_, req_level)) = learnable.spells.iter().find(|(s, _)| *s == spell) {
                if player_level < *req_level {
                    println!("⚠️ 等级不足,需要等级{}", req_level);
                    return false;
                }
            }
            
            true
        };
        
        if !can_learn {
            return false;
        }
        
        // 添加到已学技能列表
        for (_, (_, magic_list)) in world.query_mut::<(&LocalPlayer, &mut MagicList)>() {
            if magic_list.learn(spell) {
                println!("✨ 学会了技能: {}", spell.name());
                
                // 发送学习技能命令到服务器
                // TODO: 实现网络命令
                // let _ = network_tx.send(NetworkCommand::LearnMagic(spell as u8));
                
                // 更新可学习列表
                Self::update_available_magics(world);
                
                return true;
            } else {
                println!("⚠️ 已经学会了该技能");
                return false;
            }
        }
        
        false
    }
    
    /// 将技能绑定到技能栏槽位
    pub fn bind_to_slot(
        world: &mut World,
        spell: SpellType,
        slot: u8,
    ) -> bool {
        if slot >= 16 {
            println!("⚠️ 无效的技能槽位: {}", slot);
            return false;
        }
        
        // 清除该槽位的旧绑定
        for (_, (_, magic_list)) in world.query_mut::<(&LocalPlayer, &mut MagicList)>() {
            // 清除旧槽位
            for magic in &mut magic_list.magics {
                if magic.key_slot == Some(slot) {
                    magic.key_slot = None;
                }
            }
            
            // 绑定新技能
            if let Some(magic) = magic_list.get_mut(spell) {
                magic.key_slot = Some(slot);
                println!("⚡ 技能 {} 绑定到槽位 F{}", spell.name(), (slot % 8) + 1);
                return true;
            } else {
                println!("⚠️ 尚未学会该技能");
                return false;
            }
        }
        
        false
    }
    
    /// 从技能学习对话框拖拽技能到技能栏
    pub fn handle_drag_to_skillbar(
        world: &mut World,
        spell: SpellType,
        target_slot: u8,
    ) -> bool {
        println!("📋 拖拽技能 {} 到槽位 {}", spell.name(), target_slot);
        Self::bind_to_slot(world, spell, target_slot)
    }
}

