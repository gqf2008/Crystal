// Item Handler - 物品相关数据包处理

use mir2_shared::packets::{PacketHeader, Packet, server};
use mir2_shared::enums::ServerPacketIds;
use crate::network::handlers::{NetworkEvent, PacketHandler};
use std::io::Cursor;

pub struct ItemHandler;

impl PacketHandler for ItemHandler {
    fn handle(&self, header: &PacketHeader, payload: &[u8]) -> Vec<NetworkEvent> {
        let mut events = Vec::new();
        let mut cursor = Cursor::new(payload);

        match header.opcode as u16 {
            // ====================================================================
            // Item Info & Chat
            // ====================================================================

            // NewItemInfo
            x if x == ServerPacketIds::NewItemInfo as u16 => {
                if let Ok(packet) = server::NewItemInfo::read_body(&mut cursor) {
                    events.push(NetworkEvent::NewItemInfoReceived { item_index: packet.info.index, item_name: packet.info.name.clone() });
                    tracing::debug!("📦 New item info: idx={} name={}", packet.info.index, packet.info.name);
                }
            }

            // NewHeroInfo
            x if x == ServerPacketIds::NewHeroInfo as u16 => {
                if let Ok(packet) = server::NewHeroInfo::read_body(&mut cursor) {
                    events.push(NetworkEvent::NewHeroInfoReceived { info: packet.info.clone() });
                    tracing::debug!("🦸 New hero item info: {}", packet.info);
                }
            }

            // NewChatItem
            x if x == ServerPacketIds::NewChatItem as u16 => {
                if let Ok(packet) = server::NewChatItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::NewChatItemReceived { item_id: packet.item_id });
                    tracing::debug!("💬 Chat item: id={}", packet.item_id);
                }
            }

            // ====================================================================
            // Inventory Operations
            // ====================================================================

            // MoveItem
            x if x == ServerPacketIds::MoveItem as u16 => {
                if let Ok(packet) = server::MoveItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::ItemMoved {
                        grid: packet.grid,
                        from: packet.from as u32,
                        to: packet.to as u32,
                        success: packet.success,
                    });
                    tracing::debug!("🔄 Item moved: {} -> {} (grid={:?}, success={})",
                        packet.from, packet.to, packet.grid, packet.success);
                }
            }

            // EquipItem
            x if x == ServerPacketIds::EquipItem as u16 => {
                if let Ok(packet) = server::EquipItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::ItemEquipped {
                        grid: packet.grid,
                        unique_id: packet.unique_id,
                        slot: packet.to as u8,
                        success: packet.success,
                    });
                    tracing::debug!("⚔️ Item equipped: uid={} slot={} success={}",
                        packet.unique_id, packet.to, packet.success);
                }
            }

            // MergeItem
            x if x == ServerPacketIds::MergeItem as u16 => {
                if let Ok(packet) = server::MergeItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::ItemMerged {
                        grid_from: packet.grid_from,
                        grid_to: packet.grid_to,
                        id_from: packet.id_from,
                        id_to: packet.id_to,
                        success: packet.success,
                    });
                    tracing::debug!("📦 Items merged: from={} to={} success={}",
                        packet.id_from, packet.id_to, packet.success);
                }
            }

            // RemoveItem
            x if x == ServerPacketIds::RemoveItem as u16 => {
                if let Ok(packet) = server::RemoveItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::ItemRemoved {
                        grid: packet.grid,
                        unique_id: packet.unique_id,
                        to: packet.to,
                        success: packet.success,
                    });
                    tracing::debug!("🗑️ Item removed: uid={} to={} success={}",
                        packet.unique_id, packet.to, packet.success);
                }
            }

            // RemoveSlotItem
            x if x == ServerPacketIds::RemoveSlotItem as u16 => {
                if let Ok(packet) = server::RemoveSlotItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::ItemSlotRemoved {
                        grid: packet.grid,
                        grid_to: packet.grid_to,
                        slot: packet.to as u32,
                        unique_id: packet.unique_id,
                        success: packet.success,
                    });
                    tracing::debug!("🗑️ Slot item removed: uid={} to={} success={}",
                        packet.unique_id, packet.to, packet.success);
                }
            }

            // TakeBackItem
            x if x == ServerPacketIds::TakeBackItem as u16 => {
                if let Ok(packet) = server::TakeBackItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::ItemTakenBack {
                        from: packet.from,
                        to: packet.to,
                        success: packet.success,
                    });
                    tracing::debug!("📤 Item taken back: {} -> {} success={}",
                        packet.from, packet.to, packet.success);
                }
            }

            // StoreItem
            x if x == ServerPacketIds::StoreItem as u16 => {
                if let Ok(packet) = server::StoreItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::ItemStored {
                        from: packet.from,
                        to: packet.to,
                        success: packet.success,
                    });
                    tracing::debug!("🏦 Item stored: {} -> {} success={}",
                        packet.from, packet.to, packet.success);
                }
            }

            // SplitItem
            x if x == ServerPacketIds::SplitItem as u16 => {
                if let Ok(packet) = server::SplitItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::ItemSplit {
                        grid: packet.grid,
                        unique_id: packet.unique_id,
                        count: packet.count as u32,
                    });
                    tracing::debug!("✂️ Item split: uid={}, count={}", packet.unique_id, packet.count);
                }
            }

            // SplitItem1
            x if x == ServerPacketIds::SplitItem1 as u16 => {
                if let Ok(packet) = server::SplitItem1::read_body(&mut cursor) {
                    events.push(NetworkEvent::ItemSplit {
                        grid: packet.grid,
                        unique_id: packet.unique_id,
                        count: packet.count as u32,
                    });
                    tracing::debug!("✂️ Item split (alt): uid={}, count={}", packet.unique_id, packet.count);
                }
            }

            // CombineItem
            x if x == ServerPacketIds::CombineItem as u16 => {
                if let Ok(packet) = server::CombineItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::ItemCombined {
                        grid: packet.grid,
                        id_from: packet.id_from,
                        id_to: packet.id_to,
                        success: packet.success,
                        destroy: packet.destroy,
                    });
                    tracing::debug!("🔗 Items combined: from={} to={} success={} destroy={}",
                        packet.id_from, packet.id_to, packet.success, packet.destroy);
                }
            }

            // ====================================================================
            // Refine Operations
            // ====================================================================

            // DepositRefineItem
            x if x == ServerPacketIds::DepositRefineItem as u16 => {
                if let Ok(packet) = server::DepositRefineItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::RefineItemDeposited { from: packet.from, to: packet.to, success: packet.success });
                    tracing::debug!("🔨 Refine item deposited: from={} to={} success={}", packet.from, packet.to, packet.success);
                }
            }

            // RetrieveRefineItem
            x if x == ServerPacketIds::RetrieveRefineItem as u16 => {
                if let Ok(packet) = server::RetrieveRefineItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::RefineItemRetrieved { from: packet.from, to: packet.to, success: packet.success });
                    tracing::debug!("🔨 Refine item retrieved: from={} to={} success={}", packet.from, packet.to, packet.success);
                }
            }

            // RefineCancel
            x if x == ServerPacketIds::RefineCancel as u16 => {
                if let Ok(packet) = server::RefineCancel::read_body(&mut cursor) {
                    events.push(NetworkEvent::RefineCancelled { unlock: packet.unlock });
                    tracing::debug!("🔨 Refine cancelled: unlock={}", packet.unlock);
                }
            }

            // RefineItem
            x if x == ServerPacketIds::RefineItem as u16 => {
                if let Ok(packet) = server::RefineItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::RefineItemCompleted { unique_id: packet.unique_id });
                    tracing::debug!("🔨 Refine item completed: uid={}", packet.unique_id);
                }
            }

            // ====================================================================
            // Trade Operations
            // ====================================================================

            // DepositTradeItem
            x if x == ServerPacketIds::DepositTradeItem as u16 => {
                if let Ok(packet) = server::DepositTradeItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::TradeItemDeposited {
                        from_slot: packet.from_slot,
                        success: packet.success,
                    });
                    tracing::debug!("🤝 Trade item deposited: from={} success={}", packet.from_slot, packet.success);
                }
            }

            // RetrieveTradeItem
            x if x == ServerPacketIds::RetrieveTradeItem as u16 => {
                if let Ok(packet) = server::RetrieveTradeItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::TradeItemRetrieved {
                        from_slot: packet.from_slot,
                        success: packet.success,
                    });
                    tracing::debug!("🤝 Trade item retrieved: from={} success={}", packet.from_slot, packet.success);
                }
            }

            // ====================================================================
            // Use / Drop
            // ====================================================================

            // UseItem
            x if x == ServerPacketIds::UseItem as u16 => {
                if let Ok(packet) = server::UseItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::ItemUsed {
                        unique_id: packet.unique_id,
                    });
                    tracing::debug!("💊 Item used: uid={}", packet.unique_id);
                }
            }

            // DropItem
            x if x == ServerPacketIds::DropItem as u16 => {
                if let Ok(packet) = server::DropItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::ItemDropped {
                        unique_id: packet.unique_id,
                        count: packet.count,
                        success: packet.success,
                    });
                    tracing::debug!("📉 Item dropped: uid={}, count={}, success={}",
                        packet.unique_id, packet.count, packet.success);
                }
            }

            // ====================================================================
            // Hero Items
            // ====================================================================

            // TakeBackHeroItem
            x if x == ServerPacketIds::TakeBackHeroItem as u16 => {
                if let Ok(packet) = server::TakeBackHeroItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::HeroItemTakenBack { from: packet.from, to: packet.to, success: packet.success });
                    tracing::debug!("🦸 Hero item taken back: from={} to={} success={}", packet.from, packet.to, packet.success);
                }
            }

            // TransferHeroItem
            x if x == ServerPacketIds::TransferHeroItem as u16 => {
                if let Ok(packet) = server::TransferHeroItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::HeroItemTransferred { from: packet.from, to: packet.to, success: packet.success });
                    tracing::debug!("🦸 Hero item transferred: from={} to={} success={}", packet.from, packet.to, packet.success);
                }
            }

            // ====================================================================
            // Ground Items & Gold
            // ====================================================================

            // ObjectItem (ground item)
            x if x == ServerPacketIds::ObjectItem as u16 => {
                if let Ok(packet) = server::ObjectItem::read_body(&mut cursor) {
                    tracing::debug!("📦 Ground item spawned: id={}", packet.object_id);
                    events.push(NetworkEvent::GroundItem { packet });
                }
            }

            // ObjectGold
            x if x == ServerPacketIds::ObjectGold as u16 => {
                if let Ok(packet) = server::ObjectGold::read_body(&mut cursor) {
                    tracing::debug!("💰 Ground gold: {} at id={}", packet.gold, packet.object_id);
                    events.push(NetworkEvent::ObjectGoldReceived { packet });
                }
            }

            // GainedItem
            x if x == ServerPacketIds::GainedItem as u16 => {
                if let Ok(packet) = server::GainedItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::ItemGained {
                        item: packet.item.clone(),
                    });
                    tracing::debug!("📦 Item gained: {:?}", packet.item);
                }
            }

            // GainedGold
            x if x == ServerPacketIds::GainedGold as u16 => {
                if let Ok(packet) = server::GainedGold::read_body(&mut cursor) {
                    events.push(NetworkEvent::GoldChanged {
                        delta: packet.gold as i32,
                    });
                    tracing::debug!("💰 Gold gained: {}", packet.gold);
                }
            }

            // LoseGold
            x if x == ServerPacketIds::LoseGold as u16 => {
                if let Ok(packet) = server::LoseGold::read_body(&mut cursor) {
                    events.push(NetworkEvent::GoldChanged {
                        delta: -(packet.gold as i32),
                    });
                    tracing::debug!("💸 Gold lost: {}", packet.gold);
                }
            }

            // GainedCredit
            x if x == ServerPacketIds::GainedCredit as u16 => {
                if let Ok(packet) = server::GainedCredit::read_body(&mut cursor) {
                    events.push(NetworkEvent::CreditChanged {
                        delta: packet.credit as i32,
                    });
                    tracing::debug!("💎 Credit gained: {}", packet.credit);
                }
            }

            // LoseCredit
            x if x == ServerPacketIds::LoseCredit as u16 => {
                if let Ok(packet) = server::LoseCredit::read_body(&mut cursor) {
                    events.push(NetworkEvent::CreditChanged {
                        delta: -(packet.credit as i32),
                    });
                    tracing::debug!("💎 Credit lost: {}", packet.credit);
                }
            }

            // RefreshItem
            x if x == ServerPacketIds::RefreshItem as u16 => {
                if let Ok(packet) = server::RefreshItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::ItemRefreshed {
                        item: packet.item.clone(),
                    });
                    tracing::debug!("🔄 Item refreshed: uid={}", packet.item.unique_id);
                }
            }

            // DeleteItem
            x if x == ServerPacketIds::DeleteItem as u16 => {
                if let Ok(packet) = server::DeleteItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::ItemLost {
                        unique_id: packet.unique_id,
                        count: packet.count,
                    });
                    tracing::debug!("📦 Item deleted: uid={}, count={}", packet.unique_id, packet.count);
                }
            }

            // ====================================================================
            // Harvest
            // ====================================================================

            // ObjectHarvest
            x if x == ServerPacketIds::ObjectHarvest as u16 => {
                if let Ok(packet) = server::ObjectHarvest::read_body(&mut cursor) {
                    events.push(NetworkEvent::ObjectHarvested {
                        object_id: packet.object_id,
                        location_x: packet.location_x,
                        location_y: packet.location_y,
                        direction: packet.direction,
                    });
                    tracing::debug!("🌾 Object harvest started: id={} loc=({},{})",
                        packet.object_id, packet.location_x, packet.location_y);
                }
            }

            // ObjectHarvested
            x if x == ServerPacketIds::ObjectHarvested as u16 => {
                if let Ok(packet) = server::ObjectHarvested::read_body(&mut cursor) {
                    events.push(NetworkEvent::ObjectHarvested {
                        object_id: packet.object_id,
                        location_x: packet.location_x,
                        location_y: packet.location_y,
                        direction: packet.direction,
                    });
                    tracing::debug!("🌾 Object harvest completed: id={} loc=({},{})",
                        packet.object_id, packet.location_x, packet.location_y);
                }
            }

            // ====================================================================
            // Item Properties
            // ====================================================================

            // ItemSlotSizeChanged
            x if x == ServerPacketIds::ItemSlotSizeChanged as u16 => {
                if let Ok(packet) = server::ItemSlotSizeChanged::read_body(&mut cursor) {
                    events.push(NetworkEvent::ItemSlotSizeChanged {
                        slot: packet.unique_id as u32,
                        size: packet.slot_size as u32,
                    });
                    tracing::debug!("📐 Item slot size changed: uid={}, size={}",
                        packet.unique_id, packet.slot_size);
                }
            }

            // ItemSealChanged
            x if x == ServerPacketIds::ItemSealChanged as u16 => {
                if let Ok(packet) = server::ItemSealChanged::read_body(&mut cursor) {
                    events.push(NetworkEvent::ItemSealed {
                        unique_id: packet.unique_id,
                        expiry_date: packet.expiry_date,
                    });
                    tracing::debug!("🔒 Item seal changed: uid={}, expiry={}",
                        packet.unique_id, packet.expiry_date);
                }
            }

            // EquipSlotItem
            x if x == ServerPacketIds::EquipSlotItem as u16 => {
                if let Ok(packet) = server::EquipSlotItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::ItemSlotEquipped {
                        grid: packet.grid,
                        grid_to: packet.grid_to,
                        slot: packet.to as u32,
                        unique_id: packet.unique_id,
                        success: packet.success,
                    });
                    tracing::debug!("⚔️ Equip slot item: uid={}, slot={}, success={}",
                        packet.unique_id, packet.to, packet.success);
                }
            }

            // ItemUpgraded
            x if x == ServerPacketIds::ItemUpgraded as u16 => {
                if let Ok(packet) = server::ItemUpgraded::read_body(&mut cursor) {
                    events.push(NetworkEvent::ItemUpgraded {
                        item: packet.item.clone(),
                    });
                    tracing::debug!("⬆️ Item upgraded: uid={}", packet.item.unique_id);
                }
            }

            // ====================================================================
            // Inventory / Storage Resize
            // ====================================================================

            // ResizeInventory
            x if x == ServerPacketIds::ResizeInventory as u16 => {
                if let Ok(packet) = server::ResizeInventory::read_body(&mut cursor) {
                    events.push(NetworkEvent::InventoryResized {
                        new_size: packet.size as u32,
                    });
                    tracing::debug!("📦 InventoryResized: size={}", packet.size);
                }
            }

            // ResizeStorage
            x if x == ServerPacketIds::ResizeStorage as u16 => {
                if let Ok(packet) = server::ResizeStorage::read_body(&mut cursor) {
                    events.push(NetworkEvent::StorageResized {
                        new_size: packet.size as u32,
                    });
                    tracing::debug!("🏦 StorageResized: size={}", packet.size);
                }
            }

            // TransformUpdate
            x if x == ServerPacketIds::TransformUpdate as u16 => {
                if let Ok(packet) = server::TransformUpdate::read_body(&mut cursor) {
                    events.push(NetworkEvent::TransformUpdated {
                        form: packet.transform_type,
                    });
                    tracing::debug!("🔄 TransformUpdated: type={}", packet.transform_type);
                }
            }

            // NewRecipeInfo
            x if x == ServerPacketIds::NewRecipeInfo as u16 => {
                if let Ok(packet) = server::NewRecipeInfo::read_body(&mut cursor) {
                    events.push(NetworkEvent::NewRecipeInfoReceived { recipe_id: packet.recipe_id });
                    tracing::debug!("📜 NewRecipeInfo received: recipe_id={}", packet.recipe_id);
                }
            }

            // ====================================================================
            // Door
            // ====================================================================

            // Opendoor
            x if x == ServerPacketIds::Opendoor as u16 => {
                if let Ok(packet) = server::Opendoor::read_body(&mut cursor) {
                    events.push(NetworkEvent::DoorOpened {
                        door_id: packet.door_index as u32,
                        close: packet.close,
                    });
                    tracing::debug!("🚪 Door: index={} close={}", packet.door_index, packet.close);
                }
            }

            // ====================================================================
            // Item Rental
            // ====================================================================

            // GetRentedItems
            x if x == ServerPacketIds::GetRentedItems as u16 => {
                if let Ok(packet) = server::GetRentedItems::read_body(&mut cursor) {
                    let count = packet.items.len();
                    events.push(NetworkEvent::RentalItemsReceived { items: packet.items });
                    tracing::debug!("📋 RentalItemsReceived: {} items", count);
                }
            }

            // ItemRentalRequest
            x if x == ServerPacketIds::ItemRentalRequest as u16 => {
                if let Ok(_packet) = server::ItemRentalRequest::read_body(&mut cursor) {
                    events.push(NetworkEvent::ItemRentalRequested);
                    tracing::debug!("🤝 ItemRentalRequested");
                }
            }

            // ItemRentalFee
            x if x == ServerPacketIds::ItemRentalFee as u16 => {
                if let Ok(packet) = server::ItemRentalFee::read_body(&mut cursor) {
                    events.push(NetworkEvent::ItemRentalFeeReceived {
                        fee: packet.fee,
                    });
                    tracing::debug!("💰 ItemRentalFeeReceived: fee={}", packet.fee);
                }
            }

            // ItemRentalPeriod
            x if x == ServerPacketIds::ItemRentalPeriod as u16 => {
                if let Ok(packet) = server::ItemRentalPeriod::read_body(&mut cursor) {
                    events.push(NetworkEvent::ItemRentalPeriodReceived {
                        period: packet.period as u32,
                    });
                    tracing::debug!("⏰ ItemRentalPeriodReceived: period={}", packet.period);
                }
            }

            // DepositRentalItem
            x if x == ServerPacketIds::DepositRentalItem as u16 => {
                if let Ok(packet) = server::DepositRentalItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::RentalItemDeposited {
                        unique_id: packet.unique_id,
                        success: packet.success,
                    });
                    tracing::debug!("📥 RentalItemDeposited: uid={} success={}", packet.unique_id, packet.success);
                }
            }

            // RetrieveRentalItem
            x if x == ServerPacketIds::RetrieveRentalItem as u16 => {
                if let Ok(packet) = server::RetrieveRentalItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::RentalItemRetrieved {
                        unique_id: packet.unique_id,
                        success: packet.success,
                    });
                    tracing::debug!("📤 RentalItemRetrieved: uid={} success={}", packet.unique_id, packet.success);
                }
            }

            // UpdateRentalItem
            x if x == ServerPacketIds::UpdateRentalItem as u16 => {
                if let Ok(packet) = server::UpdateRentalItem::read_body(&mut cursor) {
                    events.push(NetworkEvent::RentalItemUpdated {
                        fee: packet.rental_fee,
                        period: packet.rental_period,
                    });
                    tracing::debug!("🔄 RentalItemUpdated: fee={} period={}", packet.rental_fee, packet.rental_period);
                }
            }

            // CancelItemRental
            x if x == ServerPacketIds::CancelItemRental as u16 => {
                if let Ok(packet) = server::CancelItemRental::read_body(&mut cursor) {
                    events.push(NetworkEvent::ItemRentalCancelled { success: packet.success });
                    tracing::debug!("❌ ItemRentalCancelled: success={}", packet.success);
                }
            }

            // ItemRentalLock
            x if x == ServerPacketIds::ItemRentalLock as u16 => {
                if let Ok(packet) = server::ItemRentalLock::read_body(&mut cursor) {
                    events.push(NetworkEvent::ItemRentalLocked { locked: packet.locked });
                    tracing::debug!("🔒 ItemRentalLocked: locked={}", packet.locked);
                }
            }

            // ItemRentalPartnerLock
            x if x == ServerPacketIds::ItemRentalPartnerLock as u16 => {
                if let Ok(packet) = server::ItemRentalPartnerLock::read_body(&mut cursor) {
                    events.push(NetworkEvent::ItemRentalPartnerLocked { locked: packet.locked });
                    tracing::debug!("🔒 ItemRentalPartnerLocked: locked={}", packet.locked);
                }
            }

            // CanConfirmItemRental
            x if x == ServerPacketIds::CanConfirmItemRental as u16 => {
                if let Ok(packet) = server::CanConfirmItemRental::read_body(&mut cursor) {
                    events.push(NetworkEvent::ItemRentalConfirmable { can_confirm: packet.can_confirm });
                    tracing::debug!("✅ ItemRentalConfirmable: can_confirm={}", packet.can_confirm);
                }
            }

            // ConfirmItemRental
            x if x == ServerPacketIds::ConfirmItemRental as u16 => {
                if let Ok(packet) = server::ConfirmItemRental::read_body(&mut cursor) {
                    events.push(NetworkEvent::ItemRentalConfirmed { success: packet.success });
                    tracing::debug!("✅ ItemRentalConfirmed: success={}", packet.success);
                }
            }

            // ====================================================================
            // Warehouse password (PR #1169)
            // ====================================================================

            // StorageUnlockResult (server -> client)
            x if x == ServerPacketIds::StorageUnlockResult as u16 => {
                if let Ok(packet) = server::StorageUnlockResult::read_body(&mut cursor) {
                    events.push(NetworkEvent::StorageUnlockResultReceived {
                        result: packet.result,
                        has_password: packet.has_password,
                    });
                    tracing::debug!("🔐 StorageUnlockResult: result={} has_password={}",
                        packet.result, packet.has_password);
                }
            }

            // StoragePasswordResult (server -> client)
            x if x == ServerPacketIds::StoragePasswordResult as u16 => {
                if let Ok(packet) = server::StoragePasswordResult::read_body(&mut cursor) {
                    events.push(NetworkEvent::StoragePasswordResultReceived {
                        result: packet.result,
                        removing: packet.removing,
                        has_password: packet.has_password,
                        last_set_time: packet.last_set_time,
                    });
                    tracing::debug!("🔐 StoragePasswordResult: result={} removing={}",
                        packet.result, packet.removing);
                }
            }

            // ====================================================================
            // Detailed info reply (PR #1126 KR NPC/Quest Linking)
            // ====================================================================

            // NewMonsterInfo
            x if x == ServerPacketIds::NewMonsterInfo as u16 => {
                if let Ok(packet) = server::NewMonsterInfo::read_body(&mut cursor) {
                    let info = packet.info;
                    tracing::debug!("👹 NewMonsterInfo: idx={} name={} level={}",
                        info.index, info.name, info.level);
                    events.push(NetworkEvent::NewMonsterInfoReceived { info });
                }
            }

            // NewNPCInfo
            x if x == ServerPacketIds::NewNPCInfo as u16 => {
                if let Ok(packet) = server::NewNPCInfo::read_body(&mut cursor) {
                    let info = packet.info;
                    tracing::debug!("🧙 NewNPCInfo: oid={} name={}", info.object_id, info.name);
                    events.push(NetworkEvent::NewNPCInfoReceived { info });
                }
            }

            _ => {
                tracing::debug!("⚠️ ItemHandler: Unknown opcode {:04X}", header.opcode);
                events.push(NetworkEvent::UnhandledPacket { opcode: header.opcode });
            }
        }

        events
    }
}
