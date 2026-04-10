// NPC Handler - NPC相关数据包处理

use mir2_shared::packets::{PacketHeader, Packet, server};
use mir2_shared::enums::ServerPacketIds;
use crate::network::handlers::{NetworkEvent, PacketHandler};
use std::io::Cursor;

pub struct NpcHandler;

impl PacketHandler for NpcHandler {
    fn handle(&self, header: &PacketHeader, payload: &[u8]) -> Vec<NetworkEvent> {
        let mut events = Vec::new();
        let mut cursor = Cursor::new(payload);

        match header.opcode as u16 {
            // NPCResponse (dialog)
            x if x == ServerPacketIds::NPCResponse as u16 => {
                if let Ok(packet) = server::NPCResponse::read_body(&mut cursor) {
                    let dialog = packet.page.join("\n");
                    events.push(NetworkEvent::NpcDialog {
                        npc_id: 0,  // NPCResponse只有page字段，没有object_id
                        dialog: dialog.clone(),
                    });
                    tracing::debug!("🗨️ NPC dialog: {}", dialog);
                }
            }

            // NPCGoods (shop list)
            x if x == ServerPacketIds::NPCGoods as u16 => {
                if let Ok(packet) = server::NPCGoods::read_body(&mut cursor) {
                    let count = packet.list.len();
                    events.push(NetworkEvent::NPCGoods {
                        items: packet.list,
                        rate: packet.rate,
                        panel_type: packet.panel_type,
                        hide_added_stats: packet.hide_added_stats,
                    });
                    tracing::debug!("🛒 NPC goods: {} items (rate={}, type={:?}, hide_added={})", count, packet.rate, packet.panel_type, packet.hide_added_stats);
                }
            }

            // NPCSell
            x if x == ServerPacketIds::NPCSell as u16 => {
                if let Ok(_packet) = server::NPCSell::read_body(&mut cursor) {
                    events.push(NetworkEvent::NPCSellReceived);
                    tracing::debug!("🏪 NPC sell dialog opened");
                }
            }

            // NPCRepair
            x if x == ServerPacketIds::NPCRepair as u16 => {
                if let Ok(packet) = server::NPCRepair::read_body(&mut cursor) {
                    events.push(NetworkEvent::NPCRepairReceived);
                    tracing::debug!("🔧 NPC repair dialog opened (rate={})", packet.rate);
                }
            }

            // NPCSRepair
            x if x == ServerPacketIds::NPCSRepair as u16 => {
                if let Ok(packet) = server::NPCSRepair::read_body(&mut cursor) {
                    events.push(NetworkEvent::NPCSRepairReceived);
                    tracing::debug!("🔧 NPC special repair dialog opened (rate={})", packet.rate);
                }
            }

            // NPCRefine
            x if x == ServerPacketIds::NPCRefine as u16 => {
                if let Ok(packet) = server::NPCRefine::read_body(&mut cursor) {
                    events.push(NetworkEvent::NPCRefineReceived);
                    tracing::debug!("⚒️ NPC refine dialog opened (rate={}, refining={})", packet.rate, packet.refining);
                }
            }

            // NPCCheckRefine
            x if x == ServerPacketIds::NPCCheckRefine as u16 => {
                if let Ok(_packet) = server::NPCCheckRefine::read_body(&mut cursor) {
                    events.push(NetworkEvent::NPCCheckRefineReceived);
                    tracing::debug!("⚒️ NPC check refine received");
                }
            }

            // NPCCollectRefine
            x if x == ServerPacketIds::NPCCollectRefine as u16 => {
                if let Ok(packet) = server::NPCCollectRefine::read_body(&mut cursor) {
                    events.push(NetworkEvent::NPCCollectRefineReceived);
                    tracing::debug!("⚒️ NPC collect refine: success={}", packet.success);
                }
            }

            // NPCReplaceWedRing
            x if x == ServerPacketIds::NPCReplaceWedRing as u16 => {
                if let Ok(packet) = server::NPCReplaceWedRing::read_body(&mut cursor) {
                    events.push(NetworkEvent::NPCReplaceWedRingReceived);
                    tracing::debug!("💍 NPC replace wedding ring (rate={})", packet.rate);
                }
            }

            // NPCStorage
            x if x == ServerPacketIds::NPCStorage as u16 => {
                if let Ok(_packet) = server::NPCStorage::read_body(&mut cursor) {
                    events.push(NetworkEvent::NPCStorageReceived);
                    tracing::debug!("📦 NPC storage dialog opened");
                }
            }

            // NPCConsign
            x if x == ServerPacketIds::NPCConsign as u16 => {
                if let Ok(_packet) = server::NPCConsign::read_body(&mut cursor) {
                    events.push(NetworkEvent::NPCConsignReceived);
                    tracing::debug!("📋 NPC consign dialog opened");
                }
            }

            // NPCMarket
            x if x == ServerPacketIds::NPCMarket as u16 => {
                if let Ok(packet) = server::NPCMarket::read_body(&mut cursor) {
                    events.push(NetworkEvent::NPCMarketEvent);
                    tracing::debug!("🏪 NPC market: {} pages", packet.pages.len());
                }
            }

            // NPCMarketPage
            x if x == ServerPacketIds::NPCMarketPage as u16 => {
                if let Ok(packet) = server::NPCMarketPage::read_body(&mut cursor) {
                    events.push(NetworkEvent::NPCMarketPageEvent);
                    tracing::debug!("🏪 NPC market page: {} listings", packet.listings.len());
                }
            }

            // ConsignItem
            x if x == ServerPacketIds::ConsignItem as u16 => {
                if let Ok(packet) = server::ConsignItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::ConsignItemReceived);
                    tracing::debug!("📋 Consign item: id={}, success={}", packet.unique_id, packet.success);
                }
            }

            // MarketFail
            x if x == ServerPacketIds::MarketFail as u16 => {
                if let Ok(packet) = server::MarketFail::read_body(&mut cursor) {
                    events.push(NetworkEvent::MarketFailedEvent {
                        reason: format!("reason_code={}", packet.reason),
                    });
                    tracing::warn!("🏪 Market failed: reason={}", packet.reason);
                }
            }

            // MarketSuccess
            x if x == ServerPacketIds::MarketSuccess as u16 => {
                if let Ok(packet) = server::MarketSuccess::read_body(&mut cursor) {
                    events.push(NetworkEvent::MarketSuccessEvent);
                    tracing::info!("🏪 Market success: {}", packet.message);
                }
            }

            // SellItem
            x if x == ServerPacketIds::SellItem as u16 => {
                if let Ok(packet) = server::SellItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::SellItemReceived);
                    tracing::debug!("💰 Sell item: id={}, count={}, success={}", packet.unique_id, packet.count, packet.success);
                }
            }

            // CraftItem
            x if x == ServerPacketIds::CraftItem as u16 => {
                if let Ok(packet) = server::CraftItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::CraftItemReceived);
                    tracing::debug!("⚒️ Craft item: id={}, count={}, success={}", packet.unique_id, packet.count, packet.success);
                }
            }

            // RepairItem
            x if x == ServerPacketIds::RepairItem as u16 => {
                if let Ok(packet) = server::RepairItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::RepairItemReceived);
                    tracing::debug!("🔧 Repair item: id={}", packet.unique_id);
                }
            }

            // ItemRepaired
            x if x == ServerPacketIds::ItemRepaired as u16 => {
                if let Ok(packet) = server::ItemRepaired::read_body(&mut cursor) {
                    events.push(NetworkEvent::ItemRepairedEvent);
                    tracing::debug!("🔧 Item repaired: id={}, max_dura={}, cur_dura={}", packet.unique_id, packet.max_dura, packet.current_dura);
                }
            }

            // DefaultNPC
            x if x == ServerPacketIds::DefaultNPC as u16 => {
                if let Ok(packet) = server::DefaultNPC::read_body(&mut cursor) {
                    let message = packet.page.join("\n");
                    events.push(NetworkEvent::DefaultNPCReceived {
                        npc_id: packet.object_id,
                        message,
                    });
                    tracing::debug!("🗨️ Default NPC triggered: id={}", packet.object_id);
                }
            }

            // NPCUpdate
            x if x == ServerPacketIds::NPCUpdate as u16 => {
                if let Ok(_packet) = server::NPCUpdate::read_body(&mut cursor) {
                    events.push(NetworkEvent::NPCUpdated);
                    tracing::debug!("🗨️ NPC updated");
                }
            }

            // NPCImageUpdate
            x if x == ServerPacketIds::NPCImageUpdate as u16 => {
                if let Ok(_packet) = server::NPCImageUpdate::read_body(&mut cursor) {
                    events.push(NetworkEvent::NPCImageUpdated);
                    tracing::debug!("🖼️ NPC image updated");
                }
            }

            // NPCAwakening
            x if x == ServerPacketIds::NPCAwakening as u16 => {
                if let Ok(_packet) = server::NPCAwakening::read_body(&mut cursor) {
                    events.push(NetworkEvent::NPCAwakeningReceived);
                    tracing::debug!("✨ NPC awakening dialog opened");
                }
            }

            // NPCDisassemble
            x if x == ServerPacketIds::NPCDisassemble as u16 => {
                if let Ok(_packet) = server::NPCDisassemble::read_body(&mut cursor) {
                    events.push(NetworkEvent::NPCDisassembleReceived);
                    tracing::debug!("🔧 NPC disassemble dialog opened");
                }
            }

            // NPCDowngrade
            x if x == ServerPacketIds::NPCDowngrade as u16 => {
                if let Ok(_packet) = server::NPCDowngrade::read_body(&mut cursor) {
                    events.push(NetworkEvent::NPCDowngradeReceived);
                    tracing::debug!("📉 NPC downgrade dialog opened");
                }
            }

            // NPCReset
            x if x == ServerPacketIds::NPCReset as u16 => {
                if let Ok(_packet) = server::NPCReset::read_body(&mut cursor) {
                    events.push(NetworkEvent::NPCResetReceived);
                    tracing::debug!("🔄 NPC reset dialog opened");
                }
            }

            // AwakeningNeedMaterials
            x if x == ServerPacketIds::AwakeningNeedMaterials as u16 => {
                if let Ok(packet) = server::AwakeningNeedMaterials::read_body(&mut cursor) {
                    events.push(NetworkEvent::AwakeningNeedMaterialsReceived);
                    tracing::debug!("✨ Awakening needs materials: item_id={}, {} materials", packet.item_id, packet.materials.len());
                }
            }

            // AwakeningLockedItem
            x if x == ServerPacketIds::AwakeningLockedItem as u16 => {
                if let Ok(packet) = server::AwakeningLockedItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::AwakeningLockedItemReceived);
                    tracing::debug!("✨ Awakening locked item: id={}, locked={}", packet.unique_id, packet.locked);
                }
            }

            // Awakening
            x if x == ServerPacketIds::Awakening as u16 => {
                if let Ok(packet) = server::Awakening::read_body(&mut cursor) {
                    events.push(NetworkEvent::AwakeningReceived);
                    tracing::debug!("✨ Awakening: id={}, success={}", packet.unique_id, packet.success);
                }
            }

            // NPCPearlGoods
            x if x == ServerPacketIds::NPCPearlGoods as u16 => {
                if let Ok(packet) = server::NPCPearlGoods::read_body(&mut cursor) {
                    events.push(NetworkEvent::NPCPearlGoodsReceived);
                    tracing::debug!("💎 NPC pearl goods: rate={}, {} items", packet.rate, packet.item_list.len());
                }
            }

            // NPCRequestInput
            x if x == ServerPacketIds::NPCRequestInput as u16 => {
                if let Ok(packet) = server::NPCRequestInput::read_body(&mut cursor) {
                    events.push(NetworkEvent::NPCRequestInputReceived {
                        npc_id: 0,
                        prompt: packet.message.clone(),
                    });
                    tracing::debug!("📝 NPC request input: {} (max_length={})", packet.message, packet.max_length);
                }
            }

            _ => {
                events.push(NetworkEvent::UnhandledPacket { opcode: header.opcode });
            }
        }

        events
    }
}
