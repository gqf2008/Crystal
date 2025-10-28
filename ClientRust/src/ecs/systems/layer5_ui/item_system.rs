// ============================================================================
// 物品系统 - 处理物品使用、装备、丢弃等逻辑
// ============================================================================

use hecs::World;
use crate::ecs::components::{LocalPlayer, Inventory, Equipment};
use crate::network::NetworkCommand;
use tokio::sync::mpsc;
use mir2_shared::data::item::UserItem;

/// 物品系统
pub struct ItemSystem;

impl ItemSystem {
    pub fn new() -> Self {
        Self
    }
    
    /// 使用物品
    pub fn use_item(
        world: &mut World,
        slot_index: usize,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> bool {
        // 获取物品信息
        let item_opt = {
            let mut item = None;
            for (_, (_, inv)) in world.query::<(&LocalPlayer, &Inventory)>().iter() {
                item = inv.get_item(slot_index).cloned();
                break;
            }
            item
        };
        
        if let Some(item) = item_opt {
            // 检查物品类型
            if let Some(info) = &item.info {
                use mir2_shared::enums::ItemType;
                
                match info.item_type {
                    // 消耗品 (药水、卷轴等)
                    ItemType::Potion | ItemType::Scroll | ItemType::Food => {
                        println!("🧪 使用消耗品: {}", info.name);
                        // TODO: 发送使用物品的网络命令
                        // network_tx.send(NetworkCommand::UseItem { slot: slot_index as u8 });
                        
                        // 本地减少数量
                        Self::consume_item(world, slot_index, 1);
                        return true;
                    }
                    
                    // 装备类 - 直接穿戴
                    ItemType::Weapon | ItemType::Armour | ItemType::Helmet | 
                    ItemType::Necklace | ItemType::Bracelet | ItemType::Ring | 
                    ItemType::Amulet | ItemType::Belt | ItemType::Boots | 
                    ItemType::Stone | ItemType::Torch | ItemType::Mount => {
                        return Self::equip_item(world, slot_index, network_tx);
                    }
                    
                    // 其他类型暂不处理
                    _ => {
                        println!("⚠️ 该物品无法使用");
                        return false;
                    }
                }
            }
        }
        
        println!("⚠️ 物品不存在");
        false
    }
    
    /// 装备物品
    pub fn equip_item(
        world: &mut World,
        slot_index: usize,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> bool {
        // 获取物品
        let item_opt = {
            let mut item = None;
            for (_, (_, inv)) in world.query::<(&LocalPlayer, &Inventory)>().iter() {
                item = inv.get_item(slot_index).cloned();
                break;
            }
            item
        };
        
        if let Some(item) = item_opt {
            if let Some(info) = &item.info {
                println!("⚔️ 装备: {}", info.name);
                
                // TODO: 发送装备物品的网络命令
                // network_tx.send(NetworkCommand::EquipItem { 
                //     slot: slot_index as u8,
                //     to_slot: equipment_slot,
                // });
                
                // 暂时本地处理
                // TODO: 等待服务器确认后再真正装备
                return true;
            }
        }
        
        false
    }
    
    /// 卸下装备
    pub fn unequip_item(
        world: &mut World,
        equipment_slot: u8,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> bool {
        println!("🔓 卸下装备槽: {}", equipment_slot);
        
        // TODO: 检查背包是否有空位
        // TODO: 发送卸下装备的网络命令
        // network_tx.send(NetworkCommand::UnequipItem { slot: equipment_slot });
        
        true
    }
    
    /// 丢弃物品
    pub fn drop_item(
        world: &mut World,
        slot_index: usize,
        count: u16,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> bool {
        // 获取物品
        let item_opt = {
            let mut item = None;
            for (_, (_, inv)) in world.query::<(&LocalPlayer, &Inventory)>().iter() {
                item = inv.get_item(slot_index).cloned();
                break;
            }
            item
        };
        
        if let Some(item) = item_opt {
            if item.count < count {
                println!("⚠️ 物品数量不足");
                return false;
            }
            
            if let Some(info) = &item.info {
                println!("🗑️ 丢弃物品: {} x{}", info.name, count);
                
                // TODO: 发送丢弃物品的网络命令
                // network_tx.send(NetworkCommand::DropItem { 
                //     slot: slot_index as u8,
                //     count,
                // });
                
                // 本地减少数量
                Self::consume_item(world, slot_index, count);
                return true;
            }
        }
        
        false
    }
    
    /// 拆分物品
    pub fn split_item(
        world: &mut World,
        from_slot: usize,
        to_slot: usize,
        count: u16,
    ) -> bool {
        // 检查源格子
        let source_item = {
            let mut item = None;
            for (_, (_, inv)) in world.query::<(&LocalPlayer, &Inventory)>().iter() {
                item = inv.get_item(from_slot).cloned();
                break;
            }
            item
        };
        
        if let Some(mut item) = source_item {
            if item.count < count {
                println!("⚠️ 物品数量不足");
                return false;
            }
            
            // 检查目标格子是否为空
            let target_empty = {
                let mut empty = false;
                for (_, (_, inv)) in world.query::<(&LocalPlayer, &Inventory)>().iter() {
                    empty = inv.get_item(to_slot).is_none();
                    break;
                }
                empty
            };
            
            if !target_empty {
                println!("⚠️ 目标格子不为空");
                return false;
            }
            
            // 执行拆分
            for (_, (_, inv)) in world.query_mut::<(&LocalPlayer, &mut Inventory)>() {
                // 减少源格子数量
                if let Some(source) = &mut inv.items[from_slot] {
                    source.count -= count;
                    if source.count == 0 {
                        inv.items[from_slot] = None;
                    }
                }
                
                // 在目标格子创建新物品
                let mut new_item = item.clone();
                new_item.count = count;
                inv.items[to_slot] = Some(new_item);
                
                if let Some(info) = &item.info {
                    println!("✂️ 拆分物品: {} ({} → {}个 + {}个)", 
                        info.name, item.count, count, item.count - count);
                }
                
                return true;
            }
        }
        
        false
    }
    
    /// 移动物品 (背包内或背包↔装备)
    pub fn move_item(
        world: &mut World,
        from_slot: usize,
        to_slot: usize,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> bool {
        // 简单交换两个格子的物品
        for (_, (_, inv)) in world.query_mut::<(&LocalPlayer, &mut Inventory)>() {
            if from_slot >= inv.items.len() || to_slot >= inv.items.len() {
                return false;
            }
            
            // 交换
            inv.items.swap(from_slot, to_slot);
            
            println!("📦 移动物品: {} → {}", from_slot, to_slot);
            
            // TODO: 发送移动物品的网络命令
            // network_tx.send(NetworkCommand::MoveItem { 
            //     from: from_slot as u8,
            //     to: to_slot as u8,
            // });
            
            return true;
        }
        
        false
    }
    
    /// 消耗物品 (减少数量)
    fn consume_item(world: &mut World, slot_index: usize, count: u16) {
        for (_, (_, inv)) in world.query_mut::<(&LocalPlayer, &mut Inventory)>() {
            if let Some(item) = &mut inv.items[slot_index] {
                if item.count <= count {
                    // 数量不足,移除整个物品
                    inv.items[slot_index] = None;
                } else {
                    // 减少数量
                    item.count -= count;
                }
            }
        }
    }
    
    /// 整理背包 (自动堆叠和排序)
    pub fn organize_inventory(world: &mut World) {
        for (_, (_, inv)) in world.query_mut::<(&LocalPlayer, &mut Inventory)>() {
            println!("🧹 整理背包...");
            
            // 1. 堆叠相同物品
            for i in 0..inv.items.len() {
                if let Some(item_i) = inv.items[i].clone() {
                    let item_i_info = item_i.info.as_ref();
                    
                    for j in (i + 1)..inv.items.len() {
                        if let Some(item_j) = &inv.items[j] {
                            // 检查是否相同物品
                            if item_i_info == item_j.info.as_ref() {
                                // 堆叠
                                let stack_size = item_i_info
                                    .map(|info| info.stack_size)
                                    .unwrap_or(1);
                                
                                if item_i.count < stack_size {
                                    let can_add = stack_size - item_i.count;
                                    let to_add = can_add.min(item_j.count);
                                    
                                    // 更新数量
                                    if let Some(item) = &mut inv.items[i] {
                                        item.count += to_add;
                                    }
                                    
                                    if let Some(item) = &mut inv.items[j] {
                                        item.count -= to_add;
                                        if item.count == 0 {
                                            inv.items[j] = None;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            // 2. 压缩空格子 (将所有物品移到前面)
            let mut write_pos = 0;
            for read_pos in 0..inv.items.len() {
                if inv.items[read_pos].is_some() {
                    if write_pos != read_pos {
                        inv.items.swap(write_pos, read_pos);
                    }
                    write_pos += 1;
                }
            }
            
            println!("✅ 背包整理完成");
        }
    }
}
