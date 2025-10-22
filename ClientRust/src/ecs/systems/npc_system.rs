// ============================================================================
// NPC对话系统 - 处理NPC交互和对话
// ============================================================================

use hecs::World;
use crate::ecs::components::{LocalPlayer, Position, NPC, NetworkSync};
use crate::network::NetworkCommand;
use tokio::sync::mpsc;

/// NPC对话选项
#[derive(Debug, Clone)]
pub struct DialogOption {
    pub text: String,
    pub index: u8,
}

/// NPC对话状态
#[derive(Debug, Clone)]
pub struct NPCDialog {
    pub npc_id: u32,
    pub npc_name: String,
    pub text: String,
    pub options: Vec<DialogOption>,
    pub is_open: bool,
}

impl NPCDialog {
    pub fn new(npc_id: u32, npc_name: String, text: String) -> Self {
        Self {
            npc_id,
            npc_name,
            text,
            options: Vec::new(),
            is_open: true,
        }
    }
    
    pub fn add_option(&mut self, text: String, index: u8) {
        self.options.push(DialogOption { text, index });
    }
    
    pub fn close(&mut self) {
        self.is_open = false;
    }
}

/// NPC系统
pub struct NPCSystem;

impl NPCSystem {
    pub fn new() -> Self {
        Self
    }
    
    /// 点击NPC (发起对话)
    pub fn click_npc(
        world: &mut World,
        npc_id: u32,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) -> bool {
        // 查找NPC
        let npc_info = {
            let mut info = None;
            for (_, (npc, net_sync)) in world.query::<(&NPC, &NetworkSync)>().iter() {
                if net_sync.object_id == npc_id {
                    info = Some((npc.clone(), net_sync.clone()));
                    break;
                }
            }
            info
        };
        
        if let Some((npc, _net_sync)) = npc_info {
            println!("💬 与NPC对话: {}", npc.name);
            
            // TODO: 发送NPC对话请求到服务器
            // network_tx.send(NetworkCommand::NPCTalk { npc_id });
            
            // 暂时模拟打开对话框
            // 实际应该等待服务器返回对话内容
            return true;
        }
        
        println!("⚠️ NPC不存在: {}", npc_id);
        false
    }
    
    /// 选择对话选项
    pub fn select_option(
        world: &mut World,
        npc_id: u32,
        option_index: u8,
        network_tx: &mpsc::UnboundedSender<NetworkCommand>,
    ) {
        println!("📝 选择对话选项: {} (NPC: {})", option_index, npc_id);
        
        // TODO: 发送选项选择到服务器
        // network_tx.send(NetworkCommand::NPCOption { 
        //     npc_id,
        //     option: option_index,
        // });
    }
    
    /// 查找最近的NPC
    pub fn find_nearest_npc(world: &World) -> Option<u32> {
        // 获取玩家位置
        let player_pos = {
            let mut pos = Position::new(0.0, 0.0);
            for (_, (_, p)) in world.query::<(&LocalPlayer, &Position)>().iter() {
                pos = *p;
                break;
            }
            pos
        };
        
        // 查找最近的NPC
        let mut nearest: Option<(u32, f32)> = None;
        
        for (_, (_, pos, net_sync)) in world.query::<(&NPC, &Position, &NetworkSync)>().iter() {
            let dx = pos.x - player_pos.x;
            let dy = pos.y - player_pos.y;
            let distance = (dx * dx + dy * dy).sqrt();
            
            // 对话范围: 3格以内 (3 * 48 = 144像素)
            if distance < 144.0 {
                if let Some((_, min_dist)) = nearest {
                    if distance < min_dist {
                        nearest = Some((net_sync.object_id, distance));
                    }
                } else {
                    nearest = Some((net_sync.object_id, distance));
                }
            }
        }
        
        nearest.map(|(id, _)| id)
    }
    
    /// 检查是否在NPC对话范围内
    pub fn is_in_talk_range(world: &World, npc_id: u32) -> bool {
        // 获取玩家位置
        let player_pos = {
            let mut pos = Position::new(0.0, 0.0);
            for (_, (_, p)) in world.query::<(&LocalPlayer, &Position)>().iter() {
                pos = *p;
                break;
            }
            pos
        };
        
        // 获取NPC位置
        for (_, (_, pos, net_sync)) in world.query::<(&NPC, &Position, &NetworkSync)>().iter() {
            if net_sync.object_id == npc_id {
                let dx = pos.x - player_pos.x;
                let dy = pos.y - player_pos.y;
                let distance = (dx * dx + dy * dy).sqrt();
                
                return distance < 144.0; // 3格范围
            }
        }
        
        false
    }
}
