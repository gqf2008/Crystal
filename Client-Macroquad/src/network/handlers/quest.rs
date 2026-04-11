// Quest Handler - 任务相关数据包处理

use mir2_shared::packets::{PacketHeader, Packet, server};
use mir2_shared::enums::ServerPacketIds;
use crate::network::handlers::{NetworkEvent, PacketHandler};
use std::io::Cursor;

pub struct QuestHandler;

impl PacketHandler for QuestHandler {
    fn handle(&self, header: &PacketHeader, payload: &[u8]) -> Vec<NetworkEvent> {
        let mut events = Vec::new();
        let mut cursor = Cursor::new(payload);

        match header.opcode as u16 {
            // ChangeQuest - 任务状态变更（接取/进度更新）
            x if x == ServerPacketIds::ChangeQuest as u16 => {
                if let Ok(packet) = server::ChangeQuest::read_body(&mut cursor) {
                    let quest_id = packet.quest.id as u32;
                    if packet.quest.new {
                        events.push(NetworkEvent::QuestAccepted { quest_id });
                        tracing::debug!("📜 Quest accepted: {}", quest_id);
                    } else if packet.quest.completed {
                        events.push(NetworkEvent::QuestCompleted { quest_id });
                        tracing::debug!("📜 Quest completed: {}", quest_id);
                    } else {
                        let progress = packet.quest.task_list.join("; ");
                        events.push(NetworkEvent::QuestProgressUpdated {
                            quest_id,
                            progress,
                        });
                        tracing::debug!("📜 Quest progress updated: {}", quest_id);
                    }
                }
            }

            // CompleteQuest - 任务完成
            x if x == ServerPacketIds::CompleteQuest as u16 => {
                if let Ok(packet) = server::CompleteQuest::read_body(&mut cursor) {
                    events.push(NetworkEvent::QuestCompleted {
                        quest_id: packet.quest_id as u32,
                    });
                    tracing::debug!("📜 Quest completed: {}", packet.quest_id);
                }
            }

            // ShareQuest - 任务共享
            x if x == ServerPacketIds::ShareQuest as u16 => {
                if let Ok(packet) = server::ShareQuest::read_body(&mut cursor) {
                    events.push(NetworkEvent::QuestShared {
                        quest_id: packet.quest_id as u32,
                    });
                    tracing::debug!("📜 Quest shared: {}", packet.quest_id);
                }
            }

            // NewQuestInfo - 新任务信息
            x if x == ServerPacketIds::NewQuestInfo as u16 => {
                if let Ok(packet) = server::NewQuestInfo::read_body(&mut cursor) {
                    let quest_id = packet.quest.index as u32;
                    let name = packet.quest.name.clone();
                    let group = packet.quest.group.clone();
                    let description = packet.quest.description.first().cloned().unwrap_or_default();
                    let level_req = packet.quest.min_level_needed as u32;
                    let reward_exp = packet.quest.reward_exp as u64;
                    let reward_gold = packet.quest.reward_gold;
                    events.push(NetworkEvent::QuestInfoReceived {
                        quest_id,
                        name,
                        group,
                        description,
                        level_req,
                        reward_exp,
                        reward_gold,
                    });
                    events.push(NetworkEvent::QuestListUpdated);
                    tracing::debug!("📜 New quest info: #{} {}", quest_id, packet.quest.name);
                }
            }

            // GainedQuestItem - 获得任务物品
            x if x == ServerPacketIds::GainedQuestItem as u16 => {
                if let Ok(_packet) = server::GainedQuestItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::QuestItemGained);
                    tracing::debug!("📜 Gained quest item: {}", _packet.item_id);
                }
            }

            // DeleteQuestItem - 删除任务物品
            x if x == ServerPacketIds::DeleteQuestItem as u16 => {
                if let Ok(packet) = server::DeleteQuestItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::QuestItemLost {
                        unique_id: packet.item_id as u64,
                    });
                    tracing::debug!("📜 Lost quest item: {}", packet.item_id);
                }
            }

            _ => {
                events.push(NetworkEvent::UnhandledPacket { opcode: header.opcode });
            }
        }

        events
    }
}
