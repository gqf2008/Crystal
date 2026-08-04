// ============================================================================
// 服务端事件（ServerEvent）——网络层与游戏/UI 解耦的事件总线
// ============================================================================
// 背景：network_system 曾直接 ResMut 各 UI State（God System，issue #65）。
// 目标：网络层只负责 解码 → 广播 ServerEvent；各游戏/UI 模块自己消费事件。
// 本模块先覆盖高频/核心包作为样板，其余包逐步迁移。
// 使用：`events.write(ServerEvent::...)`；消费方 `EventReader<ServerEvent>`。

use bevy::prelude::*;
use mir2_shared::packets::server::{chat, combat, drops, experience, item_operations, npc_interaction};

/// 服务端事件（按包类型组织；字段为消费方需要的最终值）
/// Bevy 0.19：Message（替代旧 EventReader/EventWriter）
#[derive(Message, Debug, Clone)]
pub enum ServerEvent {
    /// HealthChanged：HP/MP 当前值
    HealthChanged { hp: i32, mp: i32 },
    /// GainedGold：增量（击杀掉落等），消费方负责累加余额
    GoldGained { gold: u32 },
    /// GainExperience：经验增量
    ExperienceGained { amount: i64 },
    /// LevelChanged：新等级 + 经验（原实现一并更新 exp/max_exp）
    LevelChanged { level: u16, exp: i64, max_exp: i64 },
    /// Chat / ObjectChat：聊天消息（颜色映射由消费端 chat.rs 负责）
    Chat {
        text: String,
        chat_type: mir2_shared::enums::ChatType,
    },
    /// NPCResponse：NPC 对话页（行 + 可见）
    NpcDialog { lines: Vec<String>, visible: bool },
    /// MoveItem：背包内交换（仅 Inventory grid 成功响应）
    InventoryMoved { from: usize, to: usize },
    /// EquipItem：装备成功（背包→装备槽，旧装备放回背包）
    ItemEquipped { unique_id: u64, to: usize },
    /// RemoveItem：卸下装备（装备槽→背包）
    ItemRemoved { unique_id: u64 },
    /// UseItem：使用成功（背包计数减一/移除）
    ItemUsed { unique_id: u64 },
    /// Roll：骰子/尤茨结果
    Roll {
        npc_id: u32,
        r#type: i32,
        page: String,
        result: i32,
        auto_roll: bool,
        visible: bool,
        started_at: f32,
        finished: bool,
    },
    /// AwakeningNeedMaterials：觉醒材料需求（item_id, count）
    AwakeningMaterials { materials: Vec<(i32, i32)> },
    /// Awakening：觉醒结果
    AwakeningResult { result: i32, result_text: String },
    /// UserStorage：仓库物品全量（服务端打开仓库时下发）
    StorageOpened { items: Vec<Option<InvItem>>, visible: bool },
    /// GuildStatus（1 字节格式）：是否在行会；false 时消费端清空行会数据
    GuildInGuild { in_guild: bool },
    /// GuildStatus（完整格式）：行会全量信息
    GuildData {
        name: String,
        leader: String,
        notice: Vec<String>,
        members: Vec<GuildMember>,
        gold: u32,
    },
    /// GuildStorageList：行会仓库物品
    GuildStorage { items: Vec<Option<StorageItem>> },
}

use crate::game::dialogs::guild::{GuildMember, StorageItem};
use crate::game::dialogs::inventory::InvItem;

/// 从已解码的服务端包构造 ServerEvent（便于各分支统一发送）
pub mod from_packet {
    use super::*;

    pub fn health_changed(p: &combat::HealthChanged) -> ServerEvent {
        ServerEvent::HealthChanged { hp: p.hp as i32, mp: p.mp as i32 }
    }
    pub fn gold_gained(p: &drops::GainedGold) -> ServerEvent {
        ServerEvent::GoldGained { gold: p.gold }
    }
    pub fn experience_gained(p: &experience::GainExperience) -> ServerEvent {
        ServerEvent::ExperienceGained { amount: p.amount as i64 }
    }
    pub fn level_changed(p: &experience::LevelChanged) -> ServerEvent {
        ServerEvent::LevelChanged {
            level: p.level,
            exp: p.experience,
            max_exp: p.max_experience,
        }
    }
    pub fn chat(p: &chat::Chat) -> ServerEvent {
        ServerEvent::Chat { text: p.message.clone(), chat_type: p.chat_type }
    }
    pub fn object_chat(p: &chat::ObjectChat) -> ServerEvent {
        ServerEvent::Chat { text: p.text.clone(), chat_type: p.chat_type }
    }
    pub fn npc_dialog(p: &npc_interaction::NPCResponse) -> ServerEvent {
        ServerEvent::NpcDialog { lines: p.page.clone(), visible: true }
    }
    pub fn move_item(p: &item_operations::MoveItem) -> ServerEvent {
        ServerEvent::InventoryMoved { from: p.from as usize, to: p.to as usize }
    }
    pub fn equip_item(p: &item_operations::EquipItem) -> ServerEvent {
        ServerEvent::ItemEquipped { unique_id: p.unique_id as u64, to: p.to as usize }
    }
    pub fn remove_item(p: &item_operations::RemoveItem) -> ServerEvent {
        ServerEvent::ItemRemoved { unique_id: p.unique_id as u64 }
    }
    pub fn use_item(p: &item_operations::UseItem) -> ServerEvent {
        ServerEvent::ItemUsed { unique_id: p.unique_id as u64 }
    }
}
