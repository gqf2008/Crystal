use super::*;

// ============================================================
// 市场/寄售系统
// ============================================================

pub struct MarketSearchRequest {
    pub session_id: u64,
    pub item_index: u32,
}

impl Message<MarketSearchRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: MarketSearchRequest, _ctx: &mut Context<Self, Self::Reply>) {
        debug!("MarketSearch: session={} item={}", msg.session_id, msg.item_index);

        // Collect indices of unsold auctions matching criteria
        let mut results: Vec<usize> = Vec::new();
        for (idx, auction) in self.auctions.iter().enumerate() {
            if auction.sold {
                continue;
            }
            if msg.item_index > 0 && auction.item.item_index != msg.item_index as i32 {
                continue;
            }
            results.push(idx);
        }

        let total = results.len();
        let pages = (total / 10 + if total % 10 > 0 { 1 } else { 0 }).max(1);

        // Store search results for pagination
        self.market_search_cache.insert(msg.session_id, MarketSearchCache {
            results: results.clone(),
        });

        // Send page count (NPCMarket)
        let page_packet = mir2_shared::packets::server::market_system::NPCMarket {
            pages: vec!["市场".to_string(); pages],
        };
        let mut body = Vec::new();
        if let Err(e) = page_packet.write_body(&mut body) {
            warn!("Failed to serialize NPCMarket: {}", e);
            return;
        }
        let _ = self.gate_ref.tell(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::NPCMarket as i16, &body),
        }).await;

        // Send first page（空结果也发空列表，客户端据此清空旧数据）
        let end = 10.min(results.len());
        {
            let listings: Vec<mir2_shared::packets::server::market_system::MarketListing> = results[..end]
                .iter()
                .filter_map(|&idx| self.auctions.get(idx))
                .map(|a| mir2_shared::packets::server::market_system::MarketListing {
                    auction_id: a.auction_id,
                    item: a.item.clone(),
                    seller_name: a.seller_name.clone(),
                    price: a.price,
                    consignment_date: a.consignment_date,
                })
                .collect();
            let page_packet = mir2_shared::packets::server::market_system::NPCMarketPage { listings };
            let mut body = Vec::new();
            if let Err(e) = page_packet.write_body(&mut body) {
                warn!("Failed to serialize NPCMarketPage: {}", e);
                return;
            }
            let _ = self.gate_ref.tell(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::NPCMarketPage as i16, &body),
            }).await;
        }
    }
}

pub struct MarketRefreshRequest {
    pub session_id: u64,
}

impl Message<MarketRefreshRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: MarketRefreshRequest, _ctx: &mut Context<Self, Self::Reply>) {
        debug!("MarketRefresh: session={}", msg.session_id);

        // Collect all unsold auctions
        let mut results: Vec<usize> = Vec::new();
        for (idx, auction) in self.auctions.iter().enumerate() {
            if !auction.sold {
                results.push(idx);
            }
        }

        let total = results.len();
        let pages = (total / 10 + if total % 10 > 0 { 1 } else { 0 }).max(1);

        // Update search cache
        self.market_search_cache.insert(msg.session_id, MarketSearchCache {
            results: results.clone(),
        });

        // Send page count (NPCMarket)
        let page_packet = mir2_shared::packets::server::market_system::NPCMarket {
            pages: vec!["市场".to_string(); pages],
        };
        let mut body = Vec::new();
        if let Err(e) = page_packet.write_body(&mut body) {
            warn!("Failed to serialize NPCMarket: {}", e);
            return;
        }
        let _ = self.gate_ref.tell(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::NPCMarket as i16, &body),
        }).await;

        // Send first page（空结果也发空列表，客户端据此清空旧数据）
        let end = 10.min(results.len());
        {
            let listings: Vec<mir2_shared::packets::server::market_system::MarketListing> = results[..end]
                .iter()
                .filter_map(|&idx| self.auctions.get(idx))
                .map(|a| mir2_shared::packets::server::market_system::MarketListing {
                    auction_id: a.auction_id,
                    item: a.item.clone(),
                    seller_name: a.seller_name.clone(),
                    price: a.price,
                    consignment_date: a.consignment_date,
                })
                .collect();
            let page_packet = mir2_shared::packets::server::market_system::NPCMarketPage { listings };
            let mut body = Vec::new();
            if let Err(e) = page_packet.write_body(&mut body) {
                warn!("Failed to serialize NPCMarketPage: {}", e);
                return;
            }
            let _ = self.gate_ref.tell(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::NPCMarketPage as i16, &body),
            }).await;
        }
    }
}

pub struct MarketPageRequest {
    pub session_id: u64,
    pub page: u32,
}

impl Message<MarketPageRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: MarketPageRequest, _ctx: &mut Context<Self, Self::Reply>) {
        debug!("MarketPage: session={} page={}", msg.session_id, msg.page);

        let cache = match self.market_search_cache.get(&msg.session_id) {
            Some(c) => c.clone(),
            None => {
                let packet = mir2_shared::packets::server::market_system::NPCMarketPage {
                    listings: Vec::new(),
                };
                let mut body = Vec::new();
                if let Err(e) = packet.write_body(&mut body) {
                    warn!("Failed to serialize NPCMarketPage: {}", e);
                    return;
                }
                let _ = self.gate_ref.tell(SendToClient {
                    session_id: msg.session_id,
                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::NPCMarketPage as i16, &body),
                }).await;
                return;
            }
        };

        let page = msg.page as usize;
        let start = page * 10;
        let end = (start + 10).min(cache.results.len());

        let listings: Vec<mir2_shared::packets::server::market_system::MarketListing> = cache.results[start..end]
            .iter()
            .filter_map(|&idx| self.auctions.get(idx))
            .map(|a| mir2_shared::packets::server::market_system::MarketListing {
                auction_id: a.auction_id,
                item: a.item.clone(),
                seller_name: a.seller_name.clone(),
                price: a.price,
                consignment_date: a.consignment_date,
            })
            .collect();

        let packet = mir2_shared::packets::server::market_system::NPCMarketPage { listings };
        let mut body = Vec::new();
        if let Err(e) = packet.write_body(&mut body) {
            warn!("Failed to serialize NPCMarketPage: {}", e);
            return;
        }
        let _ = self.gate_ref.tell(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::NPCMarketPage as i16, &body),
        }).await;
    }
}

pub struct MarketBuyRequest {
    pub session_id: u64,
    pub listing_id: u64,
    pub count: u32,
}

impl Message<MarketBuyRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: MarketBuyRequest, _ctx: &mut Context<Self, Self::Reply>) {
        debug!("MarketBuy: session={} listing={} count={}", msg.session_id, msg.listing_id, msg.count);

        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let buyer_state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        if buyer_state.is_dead {
            send_system_message(&self.gate_ref, msg.session_id, "死亡状态下无法购买");
            return;
        }

        let auction_idx = match self.auctions.iter().position(|a| a.auction_id == msg.listing_id && !a.sold) {
            Some(idx) => idx,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "该商品已下架");
                return;
            }
        };

        // Prevent buying own listing
        if self.auctions[auction_idx].seller_name == buyer_state.name {
            send_system_message(&self.gate_ref, msg.session_id, "不能购买自己的商品");
            return;
        }

        let auction = &self.auctions[auction_idx];
        let price = auction.price as u64;
        let seller_name = auction.seller_name.clone();
        let item = auction.item.clone();

        let has_gold = record.actor_ref.ask(crate::actors::player::HasGold { amount: price }).await.unwrap_or(false);
        if !has_gold {
            send_system_message(&self.gate_ref, msg.session_id, "金币不足");
            return;
        }

        let deducted = record.actor_ref.ask(DeductGold { amount: price }).await.unwrap_or(false);
        if !deducted {
            send_system_message(&self.gate_ref, msg.session_id, "金币扣除失败");
            return;
        }

        // Try to add item to inventory first — if full, refund gold
        let added = record.actor_ref.ask(AddItemToInventory { item: item.clone() }).await.unwrap_or(false);
        if !added {
            let _ = record.actor_ref.ask(AddGold { amount: price }).await;
            send_system_message(&self.gate_ref, msg.session_id, "背包已满，购买失败，金币已退回");
            return;
        }

        // Item delivered successfully — now persist the sale
        if let Err(e) = db::mark_auction_sold(&self.db_pool, msg.listing_id as i64, &buyer_state.name).await {
            warn!("Failed to mark auction {} sold in DB: {}", msg.listing_id, e);
            // In-memory state is still updated; the sale is valid
        }

        if let Some(a) = self.auctions.get_mut(auction_idx) {
            a.sold = true;
            a.buyer_name = Some(buyer_state.name.clone());
        }

        // Give gold to seller (online) or via mail (offline)
        let mut seller_online = false;
        for (_, seller_record) in &self.players {
            if let Ok(Some(seller_state)) = seller_record.actor_ref.ask(GetPlayerState).await {
                if seller_state.name == seller_name {
                    let _ = seller_record.actor_ref.ask(AddGold { amount: price }).await;
                    send_system_message(&self.gate_ref, seller_record.session_id, &format!("{} 购买了你的商品，获得 {} 金币", buyer_state.name, price));
                    seller_online = true;
                    break;
                }
            }
        }
        if !seller_online {
            // Send gold to offline seller via mail
            let mail = MailMessage {
                mail_id: generate_mail_id(),
                sender_name: "市场交易".to_string(),
                receiver_name: seller_name.clone(),
                subject: "商品售出".to_string(),
                body: format!("你寄售的商品已售出，获得 {} 金币", price),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0),
                read: false,
                collected: false,
                locked: false,
                gold: price,
                items: Vec::new(),
            };
            if let Err(e) = db::insert_mail(&self.db_pool, &seller_name, &mail).await {
                warn!("Failed to save market sale mail for {}: {}", seller_name, e);
            }
            debug!("Seller {} is offline, gold {} sent via mail", seller_name, price);
        }

        send_system_message(&self.gate_ref, msg.session_id, &format!("购买成功：获得物品"));

        // 完整 UserInformation 刷新（背包 + 金币）
        if let Ok(Some(new_state)) = record.actor_ref.ask(GetPlayerState).await {
            let packet = super::build_user_information_packet(&new_state, &self.item_infos);
            let _ = self.gate_ref.tell(SendToClient {
                session_id: msg.session_id,
                data: packet,
            }).await;
        }

        let packet = mir2_shared::packets::server::market_system::MarketSuccess {
            message: "购买成功".to_string(),
        };
        let mut body = Vec::new();
        if let Err(e) = packet.write_body(&mut body) {
            warn!("Failed to serialize MarketSuccess: {}", e);
            return;
        }
        let _ = self.gate_ref.tell(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::MarketSuccess as i16, &body),
        }).await;
    }
}

pub struct MarketGetBackRequest {
    pub session_id: u64,
    pub listing_id: u64,
}

impl Message<MarketGetBackRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: MarketGetBackRequest, _ctx: &mut Context<Self, Self::Reply>) {
        debug!("MarketGetBack: session={} listing={}", msg.session_id, msg.listing_id);

        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        let auction_idx = match self.auctions.iter().position(|a| {
            a.auction_id == msg.listing_id && a.seller_name == state.name && !a.sold
        }) {
            Some(idx) => idx,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "找不到该寄售物品或已售出");
                return;
            }
        };

        let item = self.auctions[auction_idx].item.clone();

        // Try to add item to inventory first — if full, don't delete the auction
        let added = record.actor_ref.ask(AddItemToInventory { item: item.clone() }).await.unwrap_or(false);
        if !added {
            send_system_message(&self.gate_ref, msg.session_id, "背包已满，无法取回物品");
            return;
        }

        let _ = db::delete_auction(&self.db_pool, msg.listing_id as i64).await;
        self.auctions.remove(auction_idx);
        send_system_message(&self.gate_ref, msg.session_id, "取回寄售物品成功");

        // 完整 UserInformation 刷新（背包 + 金币）
        if let Ok(Some(new_state)) = record.actor_ref.ask(GetPlayerState).await {
            let packet = super::build_user_information_packet(&new_state, &self.item_infos);
            let _ = self.gate_ref.tell(SendToClient {
                session_id: msg.session_id,
                data: packet,
            }).await;
        }
    }
}

pub struct MarketSellNowRequest {
    pub session_id: u64,
    pub unique_id: u64,
    pub price: u64,
}

impl Message<MarketSellNowRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: MarketSellNowRequest, _ctx: &mut Context<Self, Self::Reply>) {
        debug!("MarketSellNow: session={} uid={} price={}", msg.session_id, msg.unique_id, msg.price);

        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        if state.is_dead {
            send_system_message(&self.gate_ref, msg.session_id, "死亡状态下无法操作");
            return;
        }

        let auction_idx = match self.auctions.iter().position(|a| {
            a.auction_id == msg.unique_id && a.seller_name == state.name && !a.sold
        }) {
            Some(idx) => idx,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "找不到该寄售物品");
                return;
            }
        };

        let auction = &self.auctions[auction_idx];
        let price = auction.price as u64;
        let commission = price / 10;
        let seller_gold = price - commission;

        let _ = db::delete_auction(&self.db_pool, msg.unique_id as i64).await;
        self.auctions.remove(auction_idx);

        let _ = record.actor_ref.ask(AddGold { amount: seller_gold }).await;
        send_system_message(&self.gate_ref, msg.session_id, &format!("立即售出成功，扣除手续费 {} 金币，获得 {} 金币", commission, seller_gold));
    }
}

pub struct ConsignItemRequest {
    pub session_id: u64,
    pub unique_id: u64,
    pub price: u64,
}

impl Message<ConsignItemRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: ConsignItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        if state.is_dead {
            send_system_message(&self.gate_ref, msg.session_id, "死亡状态下无法寄售");
            return;
        }

        let item = match record.actor_ref.ask(crate::actors::player::GetItemInfo { unique_id: msg.unique_id }).await {
            Ok(Some(i)) => i,
            _ => {
                send_system_message(&self.gate_ref, msg.session_id, "找不到该物品");
                return;
            }
        };

        let item_info = match self.item_infos.get(&item.item_index) {
            Some(i) => i,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "物品信息不存在");
                return;
            }
        };

        // 检查绑定：不能出售绑定的物品
        if item_info.bind_mode & 0x0004 != 0 {
            send_system_message(&self.gate_ref, msg.session_id, "绑定的物品无法寄售");
            return;
        }

        let price = msg.price as u32;
        if price == 0 || price > 1_000_000_000 {
            send_system_message(&self.gate_ref, msg.session_id, "价格无效");
            return;
        }

        // 寄售费用 = 5000 金币
        const CONSIGN_FEE: u64 = 5000;
        let has_gold = record.actor_ref.ask(crate::actors::player::HasGold { amount: CONSIGN_FEE }).await.unwrap_or(false);
        if !has_gold {
            send_system_message(&self.gate_ref, msg.session_id, &format!("寄售需要 {} 金币", CONSIGN_FEE));
            return;
        }

        // 扣除费用
        let fee_ok = record.actor_ref.ask(crate::actors::player::DeductGold { amount: CONSIGN_FEE }).await.unwrap_or(false);
        if !fee_ok {
            send_system_message(&self.gate_ref, msg.session_id, "金币扣除失败");
            return;
        }

        // 从背包移除物品
        let removed = record.actor_ref.ask(crate::actors::player::RemoveItemFromInventory {
            unique_id: msg.unique_id,
        }).await.ok().flatten();
        if removed.is_none() {
            // 退回收寄费
            let _ = record.actor_ref.ask(crate::actors::player::AddGold { amount: CONSIGN_FEE }).await;
            send_system_message(&self.gate_ref, msg.session_id, "移除物品失败，寄售费已退回");
            return;
        }

        let auction_id = self.next_auction_id;
        self.next_auction_id += 1;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let item_json = match serde_json::to_string(&item) {
            Ok(j) => j,
            Err(e) => {
                warn!("Failed to serialize item for auction: {}", e);
                send_system_message(&self.gate_ref, msg.session_id, "寄售失败：数据错误");
                return;
            }
        };

        // 保存到数据库
        if let Err(e) = db::save_auction(&self.db_pool, auction_id as i64, &state.name, &item_json, price as i64, now, 0,
        ).await {
            warn!("Failed to save auction: {}", e);
            // Rollback: return item and refund fee
            let _ = record.actor_ref.ask(AddItemToInventory { item: item.clone() }).await;
            let _ = record.actor_ref.ask(AddGold { amount: CONSIGN_FEE }).await;
            send_system_message(&self.gate_ref, msg.session_id, "寄售失败：数据库错误，物品和金币已退回");
            return;
        }

        // 添加到内存列表
        self.auctions.push(AuctionListing {
            auction_id,
            seller_name: state.name.clone(),
            item: item.clone(),
            price,
            consignment_date: now,
            sold: false,
            buyer_name: None,
            item_type: 0,
        });

        // 发送成功响应
        // 完整 UserInformation 刷新（背包移除 + 寄售费扣除，客户端本地背包同步）
        if let Ok(Some(new_state)) = record.actor_ref.ask(GetPlayerState).await {
            let packet = super::build_user_information_packet(&new_state, &self.item_infos);
            let _ = self.gate_ref.tell(SendToClient {
                session_id: msg.session_id,
                data: packet,
            }).await;
        }

        let packet = mir2_shared::packets::server::market_system::ConsignItem {
            unique_id: msg.unique_id,
            success: true,
        };
        let mut body = Vec::new();
        if let Err(e) = packet.write_body(&mut body) {
            warn!("Failed to serialize ConsignItem response: {}", e);
            return;
        }
        let _ = self.gate_ref.tell(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::ConsignItem as i16, &body),
        }).await;

        send_system_message(&self.gate_ref, msg.session_id,
            &format!("寄售成功！{} 以 {} 金币上架", item_info.name, price));
        debug!("ConsignItem: {} listed {} for {} gold (aid={})", state.name, item.item_index, price, auction_id);
    }
}

// ============================================================
// 物品租赁系统
// ============================================================

pub struct ItemRentalRequestMsg {
    pub session_id: u64,
    pub target_name: String,
}

impl Message<ItemRentalRequestMsg> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: ItemRentalRequestMsg, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        if state.is_dead {
            send_system_message(&self.gate_ref, msg.session_id, "死亡状态下无法租赁");
            return;
        }

        // Find target player by name
        let target_session = match self.find_session_by_name(&msg.target_name).await {
            Some(sid) => sid,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "目标玩家不在线");
                return;
            }
        };

        if target_session == msg.session_id {
            send_system_message(&self.gate_ref, msg.session_id, "不能向自己发起租赁");
            return;
        }

        // Create rental session (initiator = renter, partner = owner)
        self.rental_sessions.insert(msg.session_id, RentalSession {
            partner_session: target_session,
            partner_name: msg.target_name.clone(),
            fee: 0,
            period_hours: 0,
            owner_item: None,
            renter_locked: false,
            owner_locked: false,
        });

        // Send rental request to target (owner)
        self.send_rental_packet(target_session, mir2_shared::packets::server::rental_system::ItemRentalRequest {});
        send_system_message(&self.gate_ref, target_session, &format!("{} 想向你租赁物品", state.name));
        debug!("ItemRentalRequest: {} -> {} (session {})", state.name, msg.target_name, target_session);
    }
}

pub struct DepositRentalItemRequest {
    pub session_id: u64,
    pub unique_id: u64,
}

impl Message<DepositRentalItemRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: DepositRentalItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };

        // Find the rental session where this player is the partner (owner)
        let initiator = self.rental_sessions.iter()
            .find(|(_, s)| s.partner_session == msg.session_id)
            .map(|(k, _)| *k);

        let initiator = match initiator {
            Some(sid) => sid,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "没有活跃的租赁会话");
                return;
            }
        };

        let item = match record.actor_ref.ask(crate::actors::player::RemoveItemFromInventory { unique_id: msg.unique_id }).await {
            Ok(Some(i)) => i,
            _ => {
                self.send_rental_packet(msg.session_id, mir2_shared::packets::server::rental_system::DepositRentalItem {
                    unique_id: msg.unique_id,
                    success: false,
                });
                return;
            }
        };

        if let Some(session) = self.rental_sessions.get_mut(&initiator) {
            session.owner_item = Some(item.clone());
        }

        self.send_rental_packet(msg.session_id, mir2_shared::packets::server::rental_system::DepositRentalItem {
            unique_id: msg.unique_id,
            success: true,
        });
        // Also update the renter's dialog
        self.send_rental_packet(initiator, mir2_shared::packets::server::rental_system::UpdateRentalItem {
            item: item.clone(),
            rental_fee: self.rental_sessions.get(&initiator).map(|s| s.fee).unwrap_or(0),
            rental_period: self.rental_sessions.get(&initiator).map(|s| s.period_hours as i32).unwrap_or(0),
        });
        debug!("DepositRentalItem: session={} uid={}", msg.session_id, msg.unique_id);
    }
}

pub struct RetrieveRentalItemRequest {
    pub session_id: u64,
    pub unique_id: u64,
}

impl Message<RetrieveRentalItemRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: RetrieveRentalItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };

        let initiator = self.rental_sessions.iter()
            .find(|(_, s)| s.partner_session == msg.session_id)
            .map(|(k, _)| *k);

        let initiator = match initiator {
            Some(sid) => sid,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "没有活跃的租赁会话");
                return;
            }
        };

        let item = if let Some(session) = self.rental_sessions.get_mut(&initiator) {
            session.owner_item.take()
        } else {
            None
        };

        if let Some(item) = item {
            let added = record.actor_ref.ask(AddItemToInventory { item: item.clone() }).await.unwrap_or(false);
            self.send_rental_packet(msg.session_id, mir2_shared::packets::server::rental_system::RetrieveRentalItem {
                unique_id: msg.unique_id,
                success: added,
            });
            // Update renter's dialog (clear item)
            self.send_rental_packet(initiator, mir2_shared::packets::server::rental_system::UpdateRentalItem {
                item: mir2_shared::data::item::UserItem::default(),
                rental_fee: 0,
                rental_period: 0,
            });
            debug!("RetrieveRentalItem: session={} uid={}", msg.session_id, msg.unique_id);
        } else {
            self.send_rental_packet(msg.session_id, mir2_shared::packets::server::rental_system::RetrieveRentalItem {
                unique_id: msg.unique_id,
                success: false,
            });
        }
    }
}

pub struct CancelItemRentalRequest {
    pub session_id: u64,
}

impl Message<CancelItemRentalRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: CancelItemRentalRequest, _ctx: &mut Context<Self, Self::Reply>) {
        // Cancel can be sent by either renter or owner
        let (initiator, is_renter) = if let Some(_) = self.rental_sessions.get(&msg.session_id) {
            (msg.session_id, true)
        } else {
            match self.rental_sessions.iter().find(|(_, s)| s.partner_session == msg.session_id).map(|(k, _)| *k) {
                Some(sid) => (sid, false),
                None => {
                    send_system_message(&self.gate_ref, msg.session_id, "没有活跃的租赁会话");
                    return;
                }
            }
        };

        let session = self.rental_sessions.remove(&initiator);
        if let Some(s) = session {
            // Return item to owner if deposited
            if let Some(item) = s.owner_item {
                if let Some(record) = self.players.get(&s.partner_session) {
                    let _ = record.actor_ref.ask(AddItemToInventory { item }).await;
                }
            }
            self.send_rental_packet(msg.session_id, mir2_shared::packets::server::rental_system::CancelItemRental {
                unique_id: 0,
                success: true,
            });
            let other = if is_renter { s.partner_session } else { initiator };
            self.send_rental_packet(other, mir2_shared::packets::server::rental_system::CancelItemRental {
                unique_id: 0,
                success: true,
            });
            debug!("CancelItemRental: session={} (initiator={})", msg.session_id, initiator);
        }
    }
}

pub struct ItemRentalFeeMsg {
    pub session_id: u64,
    pub amount: u32,
}

impl Message<ItemRentalFeeMsg> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: ItemRentalFeeMsg, _ctx: &mut Context<Self, Self::Reply>) {
        let initiator = self.rental_sessions.iter()
            .find(|(_, s)| s.partner_session == msg.session_id)
            .map(|(k, _)| *k);

        let initiator = match initiator {
            Some(sid) => sid,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "没有活跃的租赁会话");
                return;
            }
        };

        if let Some(session) = self.rental_sessions.get_mut(&initiator) {
            session.fee = msg.amount;
        }

        // Broadcast fee to both players
        self.send_rental_packet(initiator, mir2_shared::packets::server::rental_system::ItemRentalFee { fee: msg.amount });
        self.send_rental_packet(msg.session_id, mir2_shared::packets::server::rental_system::ItemRentalFee { fee: msg.amount });
        debug!("ItemRentalFee: initiator={} fee={}", initiator, msg.amount);
    }
}

pub struct ItemRentalPeriodMsg {
    pub session_id: u64,
    pub duration: u32,
}

impl Message<ItemRentalPeriodMsg> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: ItemRentalPeriodMsg, _ctx: &mut Context<Self, Self::Reply>) {
        let initiator = self.rental_sessions.iter()
            .find(|(_, s)| s.partner_session == msg.session_id)
            .map(|(k, _)| *k);

        let initiator = match initiator {
            Some(sid) => sid,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "没有活跃的租赁会话");
                return;
            }
        };

        if let Some(session) = self.rental_sessions.get_mut(&initiator) {
            session.period_hours = msg.duration;
        }

        self.send_rental_packet(initiator, mir2_shared::packets::server::rental_system::ItemRentalPeriod { period: msg.duration as i32 });
        self.send_rental_packet(msg.session_id, mir2_shared::packets::server::rental_system::ItemRentalPeriod { period: msg.duration as i32 });
        debug!("ItemRentalPeriod: initiator={} hours={}", initiator, msg.duration);
    }
}

pub struct ItemRentalLockFeeMsg {
    pub session_id: u64,
}

impl Message<ItemRentalLockFeeMsg> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: ItemRentalLockFeeMsg, _ctx: &mut Context<Self, Self::Reply>) {
        // LockFee is sent by the renter (initiator)
        let (partner, both_locked) = {
            let session = match self.rental_sessions.get_mut(&msg.session_id) {
                Some(s) => s,
                None => {
                    send_system_message(&self.gate_ref, msg.session_id, "没有活跃的租赁会话");
                    return;
                }
            };
            session.renter_locked = true;
            (session.partner_session, session.owner_locked)
        };

        self.send_rental_packet(msg.session_id, mir2_shared::packets::server::rental_system::ItemRentalLock {
            unique_id: 0,
            locked: true,
        });
        self.send_rental_packet(partner, mir2_shared::packets::server::rental_system::ItemRentalPartnerLock {
            unique_id: 0,
            locked: true,
        });

        // Check if both locked and can confirm
        if both_locked {
            self.send_rental_packet(msg.session_id, mir2_shared::packets::server::rental_system::CanConfirmItemRental { can_confirm: true });
            self.send_rental_packet(partner, mir2_shared::packets::server::rental_system::CanConfirmItemRental { can_confirm: true });
        }
        debug!("ItemRentalLockFee: session={}", msg.session_id);
    }
}

pub struct ItemRentalLockItemMsg {
    pub session_id: u64,
}

impl Message<ItemRentalLockItemMsg> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: ItemRentalLockItemMsg, _ctx: &mut Context<Self, Self::Reply>) {
        // LockItem is sent by the owner (partner)
        let initiator = self.rental_sessions.iter()
            .find(|(_, s)| s.partner_session == msg.session_id)
            .map(|(k, _)| *k);

        let initiator = match initiator {
            Some(sid) => sid,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "没有活跃的租赁会话");
                return;
            }
        };

        let (partner, item_uid, both_locked) = {
            let session = match self.rental_sessions.get_mut(&initiator) {
                Some(s) => s,
                None => return,
            };
            session.owner_locked = true;
            (
                session.partner_session,
                session.owner_item.as_ref().map(|i| i.unique_id).unwrap_or(0),
                session.renter_locked,
            )
        };

        self.send_rental_packet(msg.session_id, mir2_shared::packets::server::rental_system::ItemRentalLock {
            unique_id: item_uid,
            locked: true,
        });
        self.send_rental_packet(initiator, mir2_shared::packets::server::rental_system::ItemRentalPartnerLock {
            unique_id: item_uid,
            locked: true,
        });
        if both_locked {
            self.send_rental_packet(initiator, mir2_shared::packets::server::rental_system::CanConfirmItemRental { can_confirm: true });
            self.send_rental_packet(partner, mir2_shared::packets::server::rental_system::CanConfirmItemRental { can_confirm: true });
        }
        debug!("ItemRentalLockItem: session={}", msg.session_id);
    }
}

pub struct ConfirmItemRentalMsg {
    pub session_id: u64,
}

impl Message<ConfirmItemRentalMsg> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: ConfirmItemRentalMsg, _ctx: &mut Context<Self, Self::Reply>) {
        let (initiator, _) = if let Some(_) = self.rental_sessions.get(&msg.session_id) {
            (msg.session_id, true)
        } else {
            match self.rental_sessions.iter().find(|(_, s)| s.partner_session == msg.session_id).map(|(k, _)| *k) {
                Some(sid) => (sid, false),
                None => {
                    send_system_message(&self.gate_ref, msg.session_id, "没有活跃的租赁会话");
                    return;
                }
            }
        };

        let session = match self.rental_sessions.remove(&initiator) {
            Some(s) => s,
            None => return,
        };

        if !session.renter_locked || !session.owner_locked {
            send_system_message(&self.gate_ref, msg.session_id, "双方尚未锁定");
            return;
        }

        let item = match session.owner_item {
            Some(i) => i,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "没有租赁物品");
                return;
            }
        };

        let fee = session.fee as u64;
        let renter_record = match self.players.get(&initiator) { Some(r) => r.clone(), None => return };
        let owner_record = match self.players.get(&session.partner_session) { Some(r) => r.clone(), None => return };

        // Check renter has enough gold
        let has_gold = renter_record.actor_ref.ask(crate::actors::player::HasGold { amount: fee }).await.unwrap_or(false);
        if !has_gold {
            send_system_message(&self.gate_ref, initiator, "金币不足，无法支付租金");
            // Return item to owner
            let _ = owner_record.actor_ref.ask(AddItemToInventory { item }).await;
            self.send_rental_packet(initiator, mir2_shared::packets::server::rental_system::ConfirmItemRental { success: false });
            self.send_rental_packet(session.partner_session, mir2_shared::packets::server::rental_system::ConfirmItemRental { success: false });
            return;
        }

        // Deduct gold from renter
        let deducted = renter_record.actor_ref.ask(DeductGold { amount: fee }).await.unwrap_or(false);
        if !deducted {
            send_system_message(&self.gate_ref, initiator, "金币扣除失败，租赁取消");
            let _ = owner_record.actor_ref.ask(AddItemToInventory { item }).await;
            self.send_rental_packet(initiator, mir2_shared::packets::server::rental_system::ConfirmItemRental { success: false });
            self.send_rental_packet(session.partner_session, mir2_shared::packets::server::rental_system::ConfirmItemRental { success: false });
            return;
        }

        // Give gold to owner
        let _ = owner_record.actor_ref.ask(AddGold { amount: fee }).await;

        // Give item to renter
        let added = renter_record.actor_ref.ask(AddItemToInventory { item: item.clone() }).await.unwrap_or(false);
        if !added {
            // Give gold back and return item to owner
            let _ = renter_record.actor_ref.ask(AddGold { amount: fee }).await;
            let _ = owner_record.actor_ref.ask(DeductGold { amount: fee }).await;
            let _ = owner_record.actor_ref.ask(AddItemToInventory { item }).await;
            send_system_message(&self.gate_ref, initiator, "背包已满，租赁失败");
            self.send_rental_packet(initiator, mir2_shared::packets::server::rental_system::ConfirmItemRental { success: false });
            self.send_rental_packet(session.partner_session, mir2_shared::packets::server::rental_system::ConfirmItemRental { success: false });
            return;
        }

        send_system_message(&self.gate_ref, initiator, &format!("租赁成功！支付 {} 金币，获得物品 {}", fee, item.item_index));
        send_system_message(&self.gate_ref, session.partner_session, &format!("租赁成功！获得 {} 金币，物品 {} 已出租", fee, item.item_index));

        // Persist to DB
        let period_hours = session.period_hours.max(1);
        let expiry = chrono::Local::now().timestamp() + (period_hours as i64 * 3600);
        let now = chrono::Local::now().timestamp();
        let _ = sqlx::query(
            "INSERT INTO rentals (item_unique_id, item_index, owner_name, renter_name, fee, period_days, started_at, expires_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(item.unique_id as i64)
        .bind(item.item_index)
        .bind(owner_record.name.clone())
        .bind(renter_record.name.clone())
        .bind(fee as i64)
        .bind(period_hours as i64 / 24)
        .bind(now)
        .bind(expiry)
        .execute(&self.db_pool)
        .await;

        // Record the rental for expiry tracking
        self.player_rentals.entry(renter_record.name.clone())
            .or_default()
            .push(RentedItem {
                item: item.clone(),
                owner_name: owner_record.name.clone(),
                renter_name: renter_record.name.clone(),
                rental_fee: session.fee,
                expiry_timestamp: expiry,
            });

        self.send_rental_packet(initiator, mir2_shared::packets::server::rental_system::ConfirmItemRental { success: true });
        self.send_rental_packet(session.partner_session, mir2_shared::packets::server::rental_system::ConfirmItemRental { success: true });
        debug!("ConfirmItemRental: {} -> {} item={} fee={}", initiator, session.partner_session, item.item_index, fee);
    }
}

pub struct GetRentedItemsRequest {
    pub session_id: u64,
}

impl Message<GetRentedItemsRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: GetRentedItemsRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        let items: Vec<mir2_shared::packets::server::rental_system::RentalItemInfo> =
            self.player_rentals.get(&state.name)
                .map(|rentals| rentals.iter().map(|r| {
                    mir2_shared::packets::server::rental_system::RentalItemInfo {
                        item: r.item.clone(),
                        rental_fee: r.rental_fee,
                        rental_period: 0,
                        expiry_date: r.expiry_timestamp,
                    }
                }).collect())
                .unwrap_or_default();

        let packet = mir2_shared::packets::server::rental_system::GetRentedItems { items };
        self.send_rental_packet(msg.session_id, packet);
        debug!("GetRentedItems: {} count={}", state.name, self.player_rentals.get(&state.name).map(|v| v.len()).unwrap_or(0));
    }
}
