use super::*;

// ============================================================
// 市场/寄售系统
// ============================================================

pub struct MarketSearchRequest {
    pub session_id: u64,
    /// 搜索关键字（C# MarketSearch.Match：名称子串；纯数字兼容按编号）
    pub keyword: String,
    /// C# MarketSearch.Type（ItemType C# 原始值；0=不过滤）
    pub item_type: u8,
    /// C# MarketSearch.Usermode（只看自己寄售）
    pub user_mode: bool,
    /// C# MarketSearch.MinShape（形状下限；0=不过滤）
    pub min_shape: i16,
    /// C# MarketSearch.MaxShape（形状上限；0=不过滤）
    pub max_shape: i16,
    /// C# MarketSearch.MarketType（MarketPanelType C# 原始值：0=Market 1=Consign 2=Auction；0=不过滤）
    pub market_type: u8,
}

/// C# MarketSearch 过滤（ClientPackets.MarketSearch）：类型/形状范围/市场面板/用户模式
/// item_type/shape 为 C# ItemType 原始值（DB）；auction_market_type 为内部 0=Consign 1=Auction
fn market_search_matches(
    filter_item_type: u8,
    user_mode: bool,
    min_shape: i16,
    max_shape: i16,
    filter_market_type: u8,
    seller_name: &str,
    self_name: &str,
    auction_item_type: u8,
    auction_shape: i16,
    auction_market_type: u8,
) -> bool {
    if user_mode && seller_name != self_name {
        return false;
    }
    if filter_item_type != 0 && auction_item_type != filter_item_type {
        return false;
    }
    if min_shape > 0 && auction_shape < min_shape {
        return false;
    }
    if max_shape > 0 && auction_shape > max_shape {
        return false;
    }
    match filter_market_type {
        1 if auction_market_type != 0 => {
            return false;
        } // Consign
        2 if auction_market_type != 1 => {
            return false;
        } // Auction
        _ => {} // Market/其他：不过滤
    }
    true
}

impl Message<MarketSearchRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: MarketSearchRequest, _ctx: &mut Context<Self, Self::Reply>) {
        debug!(
            "MarketSearch: session={} kw={}",
            msg.session_id, msg.keyword
        );

        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // Collect indices of unsold auctions matching criteria（C# MarketSearch：名称 Contains + 编号兼容）
        let kw = msg.keyword.trim().to_lowercase();
        let kw_index = kw.parse::<u32>().ok();
        let mut results: Vec<usize> = Vec::new();
        for (idx, auction) in self.auctions.iter().enumerate() {
            // C#：卖家自己的已售记录保留（供 MarketGetBack 领取金币），他人已售排除
            if auction.sold && auction.seller_name != state.name {
                continue;
            }
            if !kw.is_empty() {
                let name = self
                    .item_infos
                    .get(&auction.item.item_index)
                    .map(|i| i.name.to_lowercase())
                    .unwrap_or_default();
                let by_index = kw_index
                    .map(|k| auction.item.item_index == k as i32)
                    .unwrap_or(false);
                if !name.contains(&kw) && !by_index {
                    continue;
                }
            }
            // C# MarketSearch 过滤字段（类型/形状/市场面板/用户模式）
            let item_info = self.item_infos.get(&auction.item.item_index);
            let auction_item_type = item_info.map(|i| i.item_type as u8).unwrap_or(0);
            let auction_shape = item_info.map(|i| i.shape as i16).unwrap_or(0);
            if !market_search_matches(
                msg.item_type,
                msg.user_mode,
                msg.min_shape,
                msg.max_shape,
                msg.market_type,
                &auction.seller_name,
                &state.name,
                auction_item_type,
                auction_shape,
                auction.item_type,
            ) {
                continue;
            }
            results.push(idx);
        }

        let total = results.len();
        let pages = (total / 10 + if !total.is_multiple_of(10) { 1 } else { 0 }).max(1);

        // Store search results for pagination
        self.market_search_cache.insert(
            msg.session_id,
            MarketSearchCache {
                results: results.clone(),
            },
        );

        // Send page count (NPCMarket)
        let page_packet = mir2_shared::packets::server::market_system::NPCMarket {
            pages: vec!["市场".to_string(); pages],
        };
        let mut body = Vec::new();
        if let Err(e) = page_packet.write_body(&mut body) {
            warn!("Failed to serialize NPCMarket: {}", e);
            return;
        }
        let _ = self
            .gate_ref
            .tell(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(
                    mir2_shared::enums::ServerPacketIds::NPCMarket as i16,
                    &body,
                ),
            })
            .await;

        // Send first page（空结果也发空列表，客户端据此清空旧数据）
        let end = 10.min(results.len());
        {
            let listings: Vec<mir2_shared::packets::server::market_system::MarketListing> = results
                [..end]
                .iter()
                .filter_map(|&idx| self.auctions.get(idx))
                .map(
                    |a| mir2_shared::packets::server::market_system::MarketListing {
                        auction_id: a.auction_id,
                        item: a.item.clone(),
                        seller_name: a.seller_name.clone(),
                        price: a.price,
                        item_type: a.item_type,
                        current_bid: a.current_bid as u32,
                        consignment_date: a.consignment_date,
                    },
                )
                .collect();
            let page_packet =
                mir2_shared::packets::server::market_system::NPCMarketPage { listings };
            let mut body = Vec::new();
            if let Err(e) = page_packet.write_body(&mut body) {
                warn!("Failed to serialize NPCMarketPage: {}", e);
                return;
            }
            let _ = self
                .gate_ref
                .tell(SendToClient {
                    session_id: msg.session_id,
                    data: build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::NPCMarketPage as i16,
                        &body,
                    ),
                })
                .await;
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
        let pages = (total / 10 + if !total.is_multiple_of(10) { 1 } else { 0 }).max(1);

        // Update search cache
        self.market_search_cache.insert(
            msg.session_id,
            MarketSearchCache {
                results: results.clone(),
            },
        );

        // Send page count (NPCMarket)
        let page_packet = mir2_shared::packets::server::market_system::NPCMarket {
            pages: vec!["市场".to_string(); pages],
        };
        let mut body = Vec::new();
        if let Err(e) = page_packet.write_body(&mut body) {
            warn!("Failed to serialize NPCMarket: {}", e);
            return;
        }
        let _ = self
            .gate_ref
            .tell(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(
                    mir2_shared::enums::ServerPacketIds::NPCMarket as i16,
                    &body,
                ),
            })
            .await;

        // Send first page（空结果也发空列表，客户端据此清空旧数据）
        let end = 10.min(results.len());
        {
            let listings: Vec<mir2_shared::packets::server::market_system::MarketListing> = results
                [..end]
                .iter()
                .filter_map(|&idx| self.auctions.get(idx))
                .map(
                    |a| mir2_shared::packets::server::market_system::MarketListing {
                        auction_id: a.auction_id,
                        item: a.item.clone(),
                        seller_name: a.seller_name.clone(),
                        price: a.price,
                        item_type: a.item_type,
                        current_bid: a.current_bid as u32,
                        consignment_date: a.consignment_date,
                    },
                )
                .collect();
            let page_packet =
                mir2_shared::packets::server::market_system::NPCMarketPage { listings };
            let mut body = Vec::new();
            if let Err(e) = page_packet.write_body(&mut body) {
                warn!("Failed to serialize NPCMarketPage: {}", e);
                return;
            }
            let _ = self
                .gate_ref
                .tell(SendToClient {
                    session_id: msg.session_id,
                    data: build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::NPCMarketPage as i16,
                        &body,
                    ),
                })
                .await;
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
                let _ = self
                    .gate_ref
                    .tell(SendToClient {
                        session_id: msg.session_id,
                        data: build_packet_bytes(
                            mir2_shared::enums::ServerPacketIds::NPCMarketPage as i16,
                            &body,
                        ),
                    })
                    .await;
                return;
            }
        };

        let page = msg.page as usize;
        let start = page * 10;
        let end = (start + 10).min(cache.results.len());

        let listings: Vec<mir2_shared::packets::server::market_system::MarketListing> = cache
            .results[start..end]
            .iter()
            .filter_map(|&idx| self.auctions.get(idx))
            .map(
                |a| mir2_shared::packets::server::market_system::MarketListing {
                    auction_id: a.auction_id,
                    item: a.item.clone(),
                    seller_name: a.seller_name.clone(),
                    price: a.price,
                    item_type: a.item_type,
                    current_bid: a.current_bid as u32,
                    consignment_date: a.consignment_date,
                },
            )
            .collect();

        let packet = mir2_shared::packets::server::market_system::NPCMarketPage { listings };
        let mut body = Vec::new();
        if let Err(e) = packet.write_body(&mut body) {
            warn!("Failed to serialize NPCMarketPage: {}", e);
            return;
        }
        let _ = self
            .gate_ref
            .tell(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(
                    mir2_shared::enums::ServerPacketIds::NPCMarketPage as i16,
                    &body,
                ),
            })
            .await;
    }
}

pub struct MarketBuyRequest {
    pub session_id: u64,
    pub listing_id: u64,
    pub count: u32,
    /// 拍卖出价（C# MarketBuy.BidPrice；寄售忽略）
    pub bid_price: u32,
}

impl Message<MarketBuyRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: MarketBuyRequest, _ctx: &mut Context<Self, Self::Reply>) {
        debug!(
            "MarketBuy: session={} listing={} count={}",
            msg.session_id, msg.listing_id, msg.count
        );

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

        let auction_idx = match self
            .auctions
            .iter()
            .position(|a| a.auction_id == msg.listing_id && !a.sold && !a.expired)
        {
            Some(idx) => idx,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "该商品已下架");
                return;
            }
        };

        // Prevent buying own listing
        if self.auctions[auction_idx].seller_name == buyer_state.name {
            send_system_message(&self.gate_ref, msg.session_id, "不能购买/竞价自己的商品");
            return;
        }

        // #1325：拍卖竞价（C# MarketBuy 对 Auction 类型：出价 > 当前价，退还被超价者）
        if self.auctions[auction_idx].item_type == 1 {
            let (current_bid, current_buyer) = {
                let a = &self.auctions[auction_idx];
                (a.current_bid, a.current_buyer.clone())
            };
            let bid = msg.bid_price as u64;
            if let Err(e) =
                auction_bid_validate(self.auctions[auction_idx].price as u64, current_bid, bid)
            {
                send_system_message(&self.gate_ref, msg.session_id, e);
                return;
            }
            let has_gold = record
                .actor_ref
                .ask(crate::actors::player::HasGold { amount: bid })
                .await
                .unwrap_or(false);
            if !has_gold {
                send_system_message(&self.gate_ref, msg.session_id, "金币不足");
                return;
            }
            if let Some(prev_buyer) = current_buyer {
                // 退还被超价者之前的出价（C# OutbidRefundGold 邮件）
                let mail = MailMessage {
                    mail_id: generate_mail_id(),
                    sender_name: "市场交易".to_string(),
                    receiver_name: prev_buyer.clone(),
                    subject: "竞拍被超价".to_string(),
                    body: format!("你的出价 {} 金币已被超过，金币已退回", current_bid),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0),
                    read: false,
                    collected: false,
                    locked: false,
                    gold: current_bid,
                    items: Vec::new(),
                };
                let _ = db::insert_mail(&self.db_pool, &prev_buyer, &mail).await;
            }
            let deducted = record
                .actor_ref
                .ask(DeductGold { amount: bid })
                .await
                .unwrap_or(false);
            if !deducted {
                send_system_message(&self.gate_ref, msg.session_id, "金币扣除失败");
                return;
            }
            if let Some(a) = self.auctions.get_mut(auction_idx) {
                a.current_bid = bid;
                a.current_buyer = Some(buyer_state.name.clone());
            }
            // #2566：出价托管态落库（重启后 current_bid/current_buyer 不回退，托管金不蒸发）
            if let Err(e) = db::update_auction_bid(
                &self.db_pool,
                msg.listing_id as i64,
                bid as i64,
                &buyer_state.name,
            )
            .await
            {
                warn!(
                    "Failed to persist auction bid (auction={}): {}",
                    msg.listing_id, e
                );
            }
            send_system_message(
                &self.gate_ref,
                msg.session_id,
                &format!("已出价 {} 金币", bid),
            );
            let packet = mir2_shared::packets::server::market_system::MarketSuccess {
                message: "出价成功".to_string(),
            };
            let mut body = Vec::new();
            if packet.write_body(&mut body).is_ok() {
                let _ = self
                    .gate_ref
                    .tell(SendToClient {
                        session_id: msg.session_id,
                        data: build_packet_bytes(
                            mir2_shared::enums::ServerPacketIds::MarketSuccess as i16,
                            &body,
                        ),
                    })
                    .await;
            }
            if let Ok(Some(new_state)) = record.actor_ref.ask(GetPlayerState).await {
                let packet = super::build_user_information_packet(&new_state, &self.item_infos);
                let _ = self
                    .gate_ref
                    .tell(SendToClient {
                        session_id: msg.session_id,
                        data: packet,
                    })
                    .await;
            }
            return;
        }

        let auction = &self.auctions[auction_idx];
        let price = auction.price as u64;
        let seller_name = auction.seller_name.clone();
        let item = auction.item.clone();

        let has_gold = record
            .actor_ref
            .ask(crate::actors::player::HasGold { amount: price })
            .await
            .unwrap_or(false);
        if !has_gold {
            send_system_message(&self.gate_ref, msg.session_id, "金币不足");
            return;
        }

        let deducted = record
            .actor_ref
            .ask(DeductGold { amount: price })
            .await
            .unwrap_or(false);
        if !deducted {
            send_system_message(&self.gate_ref, msg.session_id, "金币扣除失败");
            return;
        }

        // Try to add item to inventory first — if full, refund gold
        let added = record
            .actor_ref
            .ask(AddItemToInventory { item: item.clone() })
            .await
            .unwrap_or(false);
        if !added {
            let _ = record.actor_ref.ask(AddGold { amount: price }).await;
            send_system_message(
                &self.gate_ref,
                msg.session_id,
                "背包已满，购买失败，金币已退回",
            );
            return;
        }

        // Item delivered successfully — now persist the sale
        if let Err(e) =
            db::mark_auction_sold(&self.db_pool, msg.listing_id as i64, &buyer_state.name).await
        {
            warn!(
                "Failed to mark auction {} sold in DB: {}",
                msg.listing_id, e
            );
            // In-memory state is still updated; the sale is valid
        }

        if let Some(a) = self.auctions.get_mut(auction_idx) {
            a.sold = true;
            a.buyer_name = Some(buyer_state.name.clone());
        }

        // C#：售出金币托管在寄售记录上，卖家经 MarketGetBack(Sold/Any) 领取（含 5% 佣金；不直接支付）
        if let Some(r) = self.players.values().find(|r| r.name == seller_name) {
            send_system_message(
                &self.gate_ref,
                r.session_id,
                &format!("{} 购买了你的商品，可在市场领取金币", buyer_state.name),
            );
        }

        send_system_message(&self.gate_ref, msg.session_id, "购买成功：获得物品");

        // 完整 UserInformation 刷新（背包 + 金币）
        if let Ok(Some(new_state)) = record.actor_ref.ask(GetPlayerState).await {
            let packet = super::build_user_information_packet(&new_state, &self.item_infos);
            let _ = self
                .gate_ref
                .tell(SendToClient {
                    session_id: msg.session_id,
                    data: packet,
                })
                .await;
        }

        let packet = mir2_shared::packets::server::market_system::MarketSuccess {
            message: "购买成功".to_string(),
        };
        let mut body = Vec::new();
        if let Err(e) = packet.write_body(&mut body) {
            warn!("Failed to serialize MarketSuccess: {}", e);
            return;
        }
        let _ = self
            .gate_ref
            .tell(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(
                    mir2_shared::enums::ServerPacketIds::MarketSuccess as i16,
                    &body,
                ),
            })
            .await;
    }
}

pub struct MarketGetBackRequest {
    pub session_id: u64,
    /// C# C.MarketGetBack.Mode（0=取回物品 / 1=领取售出金币 / 2=过期取回；均已实现）
    pub mode: u8,
    /// C# C.MarketGetBack.AuctionID
    pub auction_id: u64,
}

impl Message<MarketGetBackRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: MarketGetBackRequest, _ctx: &mut Context<Self, Self::Reply>) {
        // C# PlayerObject.MarketGetBack：MarketCollectionMode Any=0 / Sold=1 / Expired=2
        debug!(
            "MarketGetBack: session={} mode={} auction={}",
            msg.session_id, msg.mode, msg.auction_id
        );

        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        if state.is_dead {
            self.send_market_fail(msg.session_id, 0);
            return;
        }

        let auction_idx = match self
            .auctions
            .iter()
            .position(|a| a.auction_id == msg.auction_id && a.seller_name == state.name)
        {
            Some(idx) => idx,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "找不到该寄售记录");
                return;
            }
        };
        let auction = self.auctions[auction_idx].clone();

        // Any(0)/Expired(2)：取回物品（未售出或已到期）
        if (msg.mode == 0 || msg.mode == 2) && (!auction.sold || auction.expired) {
            // C# TakeAuction：过期拍卖若有当前出价 → 退款给出价人（:8680-8684）
            if let Some(buyer) = &auction.current_buyer {
                let bid = auction.current_bid;
                if bid > 0 {
                    let mail = MailMessage {
                        mail_id: generate_mail_id(),
                        sender_name: "市场交易".to_string(),
                        receiver_name: buyer.clone(),
                        subject: "拍卖过期退款".to_string(),
                        body: format!("你出价的物品已过期，出价 {} 金币已退回", bid),
                        timestamp: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs() as i64)
                            .unwrap_or(0),
                        read: false,
                        collected: false,
                        locked: false,
                        gold: bid,
                        items: Vec::new(),
                    };
                    let _ = db::insert_mail(&self.db_pool, buyer, &mail).await;
                }
            }
            // 取回物品（C# CanGainItem 失败 → Fail 5）
            let added = record
                .actor_ref
                .ask(AddItemToInventory {
                    item: auction.item.clone(),
                })
                .await
                .unwrap_or(false);
            if !added {
                self.send_market_fail(msg.session_id, 5);
                return;
            }
            let _ = db::delete_auction(&self.db_pool, msg.auction_id as i64).await;
            self.auctions.remove(auction_idx);
            self.send_market_success(msg.session_id, "取回寄售物品成功".to_string());
            return;
        }

        // Any(0)/Sold(1)：领取售出金币（含佣金）
        if (msg.mode == 0 || msg.mode == 1) && auction.sold {
            let cost = if auction.item_type == 1 {
                auction.current_bid
            } else {
                auction.price as u64
            };
            let gold = market_collect_gold(&auction);
            if !record
                .actor_ref
                .ask(crate::actors::player::CanGainGold {
                    amount: gold as u32,
                })
                .await
                .unwrap_or(false)
            {
                self.send_market_fail(msg.session_id, 8);
                return;
            }
            let _ = record.actor_ref.ask(AddGold { amount: gold }).await;
            let _ = db::delete_auction(&self.db_pool, msg.auction_id as i64).await;
            self.auctions.remove(auction_idx);
            let commission = cost - gold;
            let text = format!("售出金币 {}（含佣金 {}）已领取", gold, commission);
            send_system_message(&self.gate_ref, msg.session_id, &text);
            self.send_market_success(msg.session_id, text);
            return;
        }

        send_system_message(&self.gate_ref, msg.session_id, "当前状态无法领取");
    }
}

impl WorldActor {
    fn send_market_success(&self, session_id: u64, message: String) {
        let packet = mir2_shared::packets::server::market_system::MarketSuccess { message };
        let mut body = Vec::new();
        if packet.write_body(&mut body).is_ok() {
            let _ = self
                .gate_ref
                .tell(SendToClient {
                    session_id,
                    data: build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::MarketSuccess as i16,
                        &body,
                    ),
                })
                .try_send();
        }
    }

    fn send_market_fail(&self, session_id: u64, reason: u8) {
        let packet = mir2_shared::packets::server::market_system::MarketFail { reason };
        let mut body = Vec::new();
        if packet.write_body(&mut body).is_ok() {
            let _ = self
                .gate_ref
                .tell(SendToClient {
                    session_id,
                    data: build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::MarketFail as i16,
                        &body,
                    ),
                })
                .try_send();
        }
    }
}

/// #1325：拍卖出价校验（C#：bidPrice 需 >= 起始价 且 > 当前价）
pub fn auction_bid_validate(
    starting_price: u64,
    current_bid: u64,
    bid_price: u64,
) -> Result<(), &'static str> {
    if bid_price < starting_price {
        return Err("出价低于起始价");
    }
    if bid_price <= current_bid {
        return Err("出价需高于当前价");
    }
    Ok(())
}

/// 寄售/拍卖期限（C# Globals.ConsignmentLength 天；配置近似 7 天）
const CONSIGNMENT_LENGTH_SECS: i64 = 7 * 24 * 3600;

/// 市场佣金（C# Globals.Commission = 0.05F → 5%）
pub(crate) const MARKET_COMMISSION_PERCENT: u64 = 5;

/// C# MarketGetBack Sold 分支：cost=寄售价(Consign)/拍卖当前价(Auction)，gold = cost - cost×5%
pub(crate) fn market_collect_gold(auction: &AuctionListing) -> u64 {
    let cost = if auction.item_type == 1 {
        auction.current_bid
    } else {
        auction.price as u64
    };
    cost - cost * MARKET_COMMISSION_PERCENT / 100
}

/// #2566：C# MarketSellNow 结算校验（PlayerObject.cs:8615-8658）：
/// 仅 Auction 模式且 CurrentBid > Price 且已有出价者才允许"立即售出"（买家已托管扣款，
/// 成交=物品交付买家+卖家得 CurrentBid−5%）；其余情形一律拒绝，杜绝按起始价无中生有付钱
pub(crate) fn market_sell_now_settlement(auction: &AuctionListing) -> Result<u64, &'static str> {
    if auction.item_type != 1 {
        return Err("寄售物品不支持立即售出，请等待买家购买或到期取回");
    }
    if auction.sold || auction.expired {
        return Err("该物品已售出或已过期");
    }
    if auction.current_bid <= auction.price as u64 || auction.current_buyer.is_none() {
        return Err("暂无有效出价，无法立即售出");
    }
    Ok(market_collect_gold(auction))
}

/// #2566：C# Globals 价格区间（Globals.cs:44-48）：
/// Consign 5000..50_000_000（MinConsignment/MaxConsignment）；
/// Auction 起始价 0..50_000（MinStartingBid/MaxStartingBid）
pub(crate) const MIN_CONSIGN_PRICE: u32 = 5000;
pub(crate) const MAX_CONSIGN_PRICE: u32 = 50_000_000;
pub(crate) const MAX_STARTING_BID: u32 = 50_000;

/// #2566：寄售/拍卖价格校验按模式区分（此前两模式统一 5000..50M，拍卖起始价错位）
pub(crate) fn consign_price_validate(market_type: u8, price: u32) -> Result<(), &'static str> {
    match market_type {
        1 => {
            if price > MAX_STARTING_BID {
                return Err("拍卖起始价无效（0 - 50,000）");
            }
        }
        _ => {
            if !(MIN_CONSIGN_PRICE..=MAX_CONSIGN_PRICE).contains(&price) {
                return Err("价格无效（5000 - 50,000,000）");
            }
        }
    }
    Ok(())
}

/// #1325：到期结算（C# Envir.ProcessAuction）
/// - 拍卖且有人出价 → 成交：物品给买家（离线邮件）、金币给卖家（离线邮件）
/// - 无出价 → 标记过期，卖家可取回（MarketGetBack）
pub(crate) async fn resolve_expired_auctions(world: &mut WorldActor) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let mut resolved: Vec<(u64, bool, Option<String>, u64, String, String)> = Vec::new();
    for a in world.auctions.iter() {
        if a.sold || a.expired {
            continue;
        }
        if now < a.consignment_date + CONSIGNMENT_LENGTH_SECS {
            continue;
        }
        if a.item_type == 1 && a.current_buyer.is_some() {
            let winner = a.current_buyer.clone().unwrap_or_default();
            let bid = a.current_bid;
            resolved.push((
                a.auction_id,
                true,
                Some(winner),
                bid,
                a.seller_name.clone(),
                a.item
                    .info
                    .as_ref()
                    .map(|i| i.name.clone())
                    .unwrap_or_default(),
            ));
        } else {
            resolved.push((
                a.auction_id,
                false,
                None,
                0,
                a.seller_name.clone(),
                a.item
                    .info
                    .as_ref()
                    .map(|i| i.name.clone())
                    .unwrap_or_default(),
            ));
        }
    }
    for (id, sold, winner, bid, seller, item_name) in resolved {
        if sold {
            let Some(winner) = winner else { continue };
            // 物品给买家（在线直接给，离线邮件）
            let item = world
                .auctions
                .iter()
                .find(|a| a.auction_id == id)
                .map(|a| a.item.clone());
            if let Some(item) = item {
                let mut delivered = false;
                for record in world.players.values() {
                    if let Ok(Some(st)) = record.actor_ref.ask(GetPlayerState).await {
                        if st.name == winner {
                            let _ = record
                                .actor_ref
                                .ask(AddItemToInventory { item: item.clone() })
                                .await;
                            send_system_message(
                                &world.gate_ref,
                                record.session_id,
                                &format!("你以 {} 金币拍得 {}", bid, item_name),
                            );
                            delivered = true;
                            break;
                        }
                    }
                }
                if !delivered {
                    let mail = MailMessage {
                        mail_id: generate_mail_id(),
                        sender_name: "市场交易".to_string(),
                        receiver_name: winner.clone(),
                        subject: "拍卖成交".to_string(),
                        body: format!("你以 {} 金币拍得 {}", bid, item_name),
                        timestamp: now,
                        read: false,
                        collected: false,
                        locked: false,
                        gold: 0,
                        items: vec![item],
                    };
                    let _ = db::insert_mail(&world.db_pool, &winner, &mail).await;
                }
            }
            // C#：拍卖成交金币托管在寄售记录上，卖家经 MarketGetBack(Sold/Any) 领取（含 5% 佣金；不直接支付）
            if let Some(r) = world.players.values().find(|r| r.name == seller) {
                send_system_message(
                    &world.gate_ref,
                    r.session_id,
                    &format!("你的 {} 以 {} 金币成交，可在市场领取金币", item_name, bid),
                );
            }
            let _ = db::mark_auction_sold(&world.db_pool, id as i64, &winner).await;
            if let Some(a) = world.auctions.iter_mut().find(|a| a.auction_id == id) {
                a.sold = true;
                a.buyer_name = Some(winner);
            }
        } else {
            if let Some(a) = world.auctions.iter_mut().find(|a| a.auction_id == id) {
                a.expired = true;
            }
        }
    }
}

pub struct MarketSellNowRequest {
    pub session_id: u64,
    /// C# C.MarketSellNow.AuctionID（立即出售的拍卖ID）
    pub auction_id: u64,
}

impl Message<MarketSellNowRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: MarketSellNowRequest, _ctx: &mut Context<Self, Self::Reply>) {
        debug!(
            "MarketSellNow: session={} auction={}",
            msg.session_id, msg.auction_id
        );

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

        let auction_idx =
            match self.auctions.iter().position(|a| {
                a.auction_id == msg.auction_id && a.seller_name == state.name && !a.sold
            }) {
                Some(idx) => idx,
                None => {
                    send_system_message(&self.gate_ref, msg.session_id, "找不到该寄售物品");
                    return;
                }
            };

        let auction = self.auctions[auction_idx].clone();
        // #2566：对齐 C# MarketSellNow——仅拍卖且已有更高出价才可立即售出；
        // 禁止旧实现的"按起始价付钱给卖家并删物品"（无买家扣款=无中生有刷金）
        let seller_gold = match market_sell_now_settlement(&auction) {
            Ok(gold) => gold,
            Err(e) => {
                send_system_message(&self.gate_ref, msg.session_id, e);
                self.send_market_fail(msg.session_id, 9);
                return;
            }
        };

        // C# CanGainGold 失败 → MarketFail 8
        if !record
            .actor_ref
            .ask(crate::actors::player::CanGainGold {
                amount: seller_gold as u32,
            })
            .await
            .unwrap_or(false)
        {
            self.send_market_fail(msg.session_id, 8);
            return;
        }

        // 买家出价时金币已托管扣款：物品交付买家（在线直接进包，离线走邮件，C# MailCharacter）
        let buyer_name = auction.current_buyer.clone().unwrap_or_default();
        let cost = auction.current_bid;
        let item_name = self
            .item_infos
            .get(&auction.item.item_index)
            .map(|i| i.name.clone())
            .unwrap_or_else(|| "物品".to_string());
        let mut delivered = false;
        for r in self.players.values() {
            if let Ok(Some(st)) = r.actor_ref.ask(GetPlayerState).await {
                if st.name == buyer_name {
                    let _ = r
                        .actor_ref
                        .ask(AddItemToInventory {
                            item: auction.item.clone(),
                        })
                        .await;
                    send_system_message(
                        &self.gate_ref,
                        r.session_id,
                        &format!("你以 {} 金币购得 {}", cost, item_name),
                    );
                    delivered = true;
                    break;
                }
            }
        }
        if !delivered {
            let mail = MailMessage {
                mail_id: generate_mail_id(),
                sender_name: "市场交易".to_string(),
                receiver_name: buyer_name.clone(),
                subject: "拍卖成交".to_string(),
                body: format!("卖家已立即售出，你以 {} 金币购得 {}", cost, item_name),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0),
                read: false,
                collected: false,
                locked: false,
                gold: 0,
                items: vec![auction.item.clone()],
            };
            let _ = db::insert_mail(&self.db_pool, &buyer_name, &mail).await;
        }

        // 成交：删除寄售记录，卖家得托管出价 − 5% 佣金（佣金回收）
        let _ = db::delete_auction(&self.db_pool, msg.auction_id as i64).await;
        self.auctions.remove(auction_idx);

        let _ = record
            .actor_ref
            .ask(AddGold {
                amount: seller_gold,
            })
            .await;
        let commission = cost - seller_gold;
        let text = format!(
            "立即售出成功，扣除手续费 {} 金币，获得 {} 金币",
            commission, seller_gold
        );
        send_system_message(&self.gate_ref, msg.session_id, &text);
        self.send_market_success(msg.session_id, text);
    }
}

pub struct ConsignItemRequest {
    pub session_id: u64,
    pub unique_id: u64,
    pub price: u64,
    /// 0=寄售 1=拍卖（C# MarketItemType）
    pub market_type: u8,
}

impl Message<ConsignItemRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: ConsignItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        if state.is_dead {
            send_system_message(&self.gate_ref, msg.session_id, "死亡状态下无法寄售");
            return;
        }

        // C# ConsignItem：需先与市场 NPC 对话（NPCPage）+ InRange(NPC, DataRange=16)
        let npc_oid = match self.session_npc.get(&msg.session_id) {
            Some(o) => *o,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "请先与市场 NPC 对话");
                return;
            }
        };
        let npc = match self.npcs.get(&npc_oid) {
            Some(n) => n,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "找不到该 NPC");
                return;
            }
        };
        if state.map_index != npc.map_index
            || crate::actors::world::ai::max_distance(state.x, state.y, npc.x, npc.y) > 16
        {
            send_system_message(&self.gate_ref, msg.session_id, "距离 NPC 太远，无法寄售");
            return;
        }

        let item = match record
            .actor_ref
            .ask(crate::actors::player::GetItemInfo {
                unique_id: msg.unique_id,
            })
            .await
        {
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
        // #1325：寄售/拍卖费用（C# Globals：ConsignmentCost/AuctionCost 均为 5000）
        const CONSIGN_FEE: u64 = 5000;
        // #2566：价格区间按模式区分（C# Globals.cs:44-48：Consign 5000-50M；Auction 起始价 0-50,000）
        if let Err(e) = consign_price_validate(msg.market_type, price) {
            send_system_message(&self.gate_ref, msg.session_id, e);
            return;
        }
        let fee = CONSIGN_FEE;
        let has_gold = record
            .actor_ref
            .ask(crate::actors::player::HasGold { amount: fee })
            .await
            .unwrap_or(false);
        if !has_gold {
            send_system_message(
                &self.gate_ref,
                msg.session_id,
                &format!(
                    "{}需要 {} 金币",
                    if msg.market_type == 1 {
                        "拍卖"
                    } else {
                        "寄售"
                    },
                    fee
                ),
            );
            return;
        }
        let fee_ok = record
            .actor_ref
            .ask(crate::actors::player::DeductGold { amount: fee })
            .await
            .unwrap_or(false);
        if !fee_ok {
            send_system_message(&self.gate_ref, msg.session_id, "金币扣除失败");
            return;
        }

        // 从背包移除物品
        let removed = record
            .actor_ref
            .ask(crate::actors::player::RemoveItemFromInventory {
                unique_id: msg.unique_id,
            })
            .await
            .ok()
            .flatten();
        if removed.is_none() {
            // 退回收寄费
            let _ = record
                .actor_ref
                .ask(crate::actors::player::AddGold {
                    amount: CONSIGN_FEE,
                })
                .await;
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
        if let Err(e) = db::save_auction(
            &self.db_pool,
            auction_id as i64,
            &state.name,
            &item_json,
            price as i64,
            now,
            msg.market_type as i32,
        )
        .await
        {
            warn!("Failed to save auction: {}", e);
            // Rollback: return item and refund fee
            let _ = record
                .actor_ref
                .ask(AddItemToInventory { item: item.clone() })
                .await;
            let _ = record.actor_ref.ask(AddGold { amount: fee }).await;
            send_system_message(
                &self.gate_ref,
                msg.session_id,
                "寄售失败：数据库错误，物品和金币已退回",
            );
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
            item_type: msg.market_type,
            current_bid: if msg.market_type == 1 {
                price as u64
            } else {
                0
            },
            current_buyer: None,
            expired: false,
        });

        // 发送成功响应
        // 完整 UserInformation 刷新（背包移除 + 寄售费扣除，客户端本地背包同步）
        if let Ok(Some(new_state)) = record.actor_ref.ask(GetPlayerState).await {
            let packet = super::build_user_information_packet(&new_state, &self.item_infos);
            let _ = self
                .gate_ref
                .tell(SendToClient {
                    session_id: msg.session_id,
                    data: packet,
                })
                .await;
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
        let _ = self
            .gate_ref
            .tell(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(
                    mir2_shared::enums::ServerPacketIds::ConsignItem as i16,
                    &body,
                ),
            })
            .await;

        send_system_message(
            &self.gate_ref,
            msg.session_id,
            &format!("寄售成功！{} 以 {} 金币上架", item_info.name, price),
        );
        debug!(
            "ConsignItem: {} listed {} for {} gold (aid={})",
            state.name, item.item_index, price, auction_id
        );
    }
}

// ============================================================
// 物品租赁系统
// ============================================================

/// C# ConfirmItemRental（:14416）绑定旗标：DontDrop|DontStore|DontSell|DontTrade|UnableToRent|DontUpgrade|UnableToDisassemble
pub(crate) fn rental_binding_flags() -> mir2_shared::enums::BindMode {
    mir2_shared::enums::BindMode::DONT_DROP
        | mir2_shared::enums::BindMode::DONT_STORE
        | mir2_shared::enums::BindMode::DONT_SELL
        | mir2_shared::enums::BindMode::DONT_TRADE
        | mir2_shared::enums::BindMode::UNABLE_TO_RENT
        | mir2_shared::enums::BindMode::DONT_UPGRADE
        | mir2_shared::enums::BindMode::UNABLE_TO_DISASSEMBLE
}

pub struct ItemRentalRequestMsg {
    pub session_id: u64,
    pub target_name: String,
}

impl Message<ItemRentalRequestMsg> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: ItemRentalRequestMsg, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

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
        self.rental_sessions.insert(
            msg.session_id,
            RentalSession {
                partner_session: target_session,
                partner_name: msg.target_name.clone(),
                fee: 0,
                period_hours: 0,
                owner_item: None,
                renter_locked: false,
                owner_locked: false,
            },
        );

        // Send rental request to target (owner)
        self.send_rental_packet(
            target_session,
            mir2_shared::packets::server::rental_system::ItemRentalRequest {},
        );
        send_system_message(
            &self.gate_ref,
            target_session,
            &format!("{} 想向你租赁物品", state.name),
        );
        debug!(
            "ItemRentalRequest: {} -> {} (session {})",
            state.name, msg.target_name, target_session
        );
    }
}

pub struct DepositRentalItemRequest {
    pub session_id: u64,
    /// C# C.DepositRentalItem.From：背包格索引
    pub from: i32,
    /// C# C.DepositRentalItem.To：租赁栏格索引（Rust 单槽，须为 0）
    pub to: i32,
}

impl Message<DepositRentalItemRequest> for WorldActor {
    type Reply = ();
    async fn handle(
        &mut self,
        msg: DepositRentalItemRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // C# DepositRentalItem（:14112）：From=背包格、To=租赁栏格（Rust 单槽 to=0）
        let from = msg.from;
        if from < 0 || from as usize >= state.inventory.backpack.len() || msg.to != 0 {
            self.send_rental_packet(
                msg.session_id,
                mir2_shared::packets::server::rental_system::DepositRentalItem {
                    unique_id: 0,
                    success: false,
                },
            );
            return;
        }
        let Some(item) = state.inventory.backpack[from as usize]
            .as_ref()
            .map(|s| s.item.clone())
        else {
            self.send_rental_packet(
                msg.session_id,
                mir2_shared::packets::server::rental_system::DepositRentalItem {
                    unique_id: 0,
                    success: false,
                },
            );
            return;
        };
        let uid = item.unique_id;

        // C# DepositRentalItem（:14143/:14157）：租赁锁定中 / 带 UnableToRent 旗标的物品不可再出租
        if item
            .rental_information
            .as_ref()
            .map(|r| r.rental_locked)
            .unwrap_or(false)
        {
            send_system_message(
                &self.gate_ref,
                msg.session_id,
                "该物品租赁锁定中，无法再次出租",
            );
            self.send_rental_packet(
                msg.session_id,
                mir2_shared::packets::server::rental_system::DepositRentalItem {
                    unique_id: item.unique_id,
                    success: false,
                },
            );
            return;
        }
        if super::rental_has_flag(&item, mir2_shared::enums::BindMode::UNABLE_TO_RENT.bits()) {
            let owner = item
                .rental_information
                .as_ref()
                .map(|r| r.owner_name.clone())
                .unwrap_or_default();
            send_system_message(
                &self.gate_ref,
                msg.session_id,
                &format!("该物品属于 {}，无法出租", owner),
            );
            self.send_rental_packet(
                msg.session_id,
                mir2_shared::packets::server::rental_system::DepositRentalItem {
                    unique_id: item.unique_id,
                    success: false,
                },
            );
            return;
        }

        // Find the rental session where this player is the partner (owner)
        let initiator = self
            .rental_sessions
            .iter()
            .find(|(_, s)| s.partner_session == msg.session_id)
            .map(|(k, _)| *k);

        let initiator = match initiator {
            Some(sid) => sid,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "没有活跃的租赁会话");
                return;
            }
        };

        let item = match record
            .actor_ref
            .ask(crate::actors::player::RemoveItemFromInventory { unique_id: uid })
            .await
        {
            Ok(Some(i)) => i,
            _ => {
                self.send_rental_packet(
                    msg.session_id,
                    mir2_shared::packets::server::rental_system::DepositRentalItem {
                        unique_id: uid,
                        success: false,
                    },
                );
                return;
            }
        };

        if let Some(session) = self.rental_sessions.get_mut(&initiator) {
            session.owner_item = Some(item.clone());
        }

        self.send_rental_packet(
            msg.session_id,
            mir2_shared::packets::server::rental_system::DepositRentalItem {
                unique_id: uid,
                success: true,
            },
        );
        // Also update the renter's dialog
        self.send_rental_packet(
            initiator,
            mir2_shared::packets::server::rental_system::UpdateRentalItem {
                item: item.clone(),
                rental_fee: self
                    .rental_sessions
                    .get(&initiator)
                    .map(|s| s.fee)
                    .unwrap_or(0),
                rental_period: self
                    .rental_sessions
                    .get(&initiator)
                    .map(|s| s.period_hours as i32)
                    .unwrap_or(0),
            },
        );
        debug!(
            "DepositRentalItem: session={} from={} uid={}",
            msg.session_id, from, uid
        );
    }
}

pub struct RetrieveRentalItemRequest {
    pub session_id: u64,
    /// C# C.RetrieveRentalItem.From：租赁栏格索引（Rust 单槽，须为 0）
    pub from: i32,
    /// C# C.RetrieveRentalItem.To：背包格索引
    pub to: i32,
}

impl Message<RetrieveRentalItemRequest> for WorldActor {
    type Reply = ();
    async fn handle(
        &mut self,
        msg: RetrieveRentalItemRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // C# RetrieveRentalItem（:14178）：From=租赁栏格（Rust 单槽 0）、To=背包格
        if msg.from != 0 {
            self.send_rental_packet(
                msg.session_id,
                mir2_shared::packets::server::rental_system::RetrieveRentalItem {
                    unique_id: 0,
                    success: false,
                },
            );
            return;
        }
        let to = msg.to;
        if to < 0
            || to as usize >= state.inventory.backpack.len()
            || state.inventory.backpack[to as usize].is_some()
        {
            self.send_rental_packet(
                msg.session_id,
                mir2_shared::packets::server::rental_system::RetrieveRentalItem {
                    unique_id: 0,
                    success: false,
                },
            );
            return;
        }

        let initiator = self
            .rental_sessions
            .iter()
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
            // C# RetrieveRentalItem：返还到指定背包格；失败回退自动空格
            let mut added = record
                .actor_ref
                .ask(crate::actors::player::PlaceItemAtSlot {
                    slot: to,
                    item: item.clone(),
                })
                .await
                .unwrap_or(false);
            if !added {
                added = record
                    .actor_ref
                    .ask(AddItemToInventory { item: item.clone() })
                    .await
                    .unwrap_or(false);
            }
            self.send_rental_packet(
                msg.session_id,
                mir2_shared::packets::server::rental_system::RetrieveRentalItem {
                    unique_id: item.unique_id,
                    success: added,
                },
            );
            // Update renter's dialog (clear item)
            self.send_rental_packet(
                initiator,
                mir2_shared::packets::server::rental_system::UpdateRentalItem {
                    item: mir2_shared::data::item::UserItem::default(),
                    rental_fee: 0,
                    rental_period: 0,
                },
            );
            debug!(
                "RetrieveRentalItem: session={} to={} uid={}",
                msg.session_id, to, item.unique_id
            );
        } else {
            self.send_rental_packet(
                msg.session_id,
                mir2_shared::packets::server::rental_system::RetrieveRentalItem {
                    unique_id: 0,
                    success: false,
                },
            );
        }
    }
}

pub struct CancelItemRentalRequest {
    pub session_id: u64,
}

impl Message<CancelItemRentalRequest> for WorldActor {
    type Reply = ();
    async fn handle(
        &mut self,
        msg: CancelItemRentalRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) {
        // Cancel can be sent by either renter or owner
        let (initiator, is_renter) = if self.rental_sessions.contains_key(&msg.session_id) {
            (msg.session_id, true)
        } else {
            match self
                .rental_sessions
                .iter()
                .find(|(_, s)| s.partner_session == msg.session_id)
                .map(|(k, _)| *k)
            {
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
            self.send_rental_packet(
                msg.session_id,
                mir2_shared::packets::server::rental_system::CancelItemRental {
                    unique_id: 0,
                    success: true,
                },
            );
            let other = if is_renter {
                s.partner_session
            } else {
                initiator
            };
            self.send_rental_packet(
                other,
                mir2_shared::packets::server::rental_system::CancelItemRental {
                    unique_id: 0,
                    success: true,
                },
            );
            debug!(
                "CancelItemRental: session={} (initiator={})",
                msg.session_id, initiator
            );
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
        let initiator = self
            .rental_sessions
            .iter()
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
        self.send_rental_packet(
            initiator,
            mir2_shared::packets::server::rental_system::ItemRentalFee { fee: msg.amount },
        );
        self.send_rental_packet(
            msg.session_id,
            mir2_shared::packets::server::rental_system::ItemRentalFee { fee: msg.amount },
        );
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
        let initiator = self
            .rental_sessions
            .iter()
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

        self.send_rental_packet(
            initiator,
            mir2_shared::packets::server::rental_system::ItemRentalPeriod {
                period: msg.duration as i32,
            },
        );
        self.send_rental_packet(
            msg.session_id,
            mir2_shared::packets::server::rental_system::ItemRentalPeriod {
                period: msg.duration as i32,
            },
        );
        debug!(
            "ItemRentalPeriod: initiator={} hours={}",
            initiator, msg.duration
        );
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

        self.send_rental_packet(
            msg.session_id,
            mir2_shared::packets::server::rental_system::ItemRentalLock {
                unique_id: 0,
                locked: true,
            },
        );
        self.send_rental_packet(
            partner,
            mir2_shared::packets::server::rental_system::ItemRentalPartnerLock {
                unique_id: 0,
                locked: true,
            },
        );

        // Check if both locked and can confirm
        if both_locked {
            self.send_rental_packet(
                msg.session_id,
                mir2_shared::packets::server::rental_system::CanConfirmItemRental {
                    can_confirm: true,
                },
            );
            self.send_rental_packet(
                partner,
                mir2_shared::packets::server::rental_system::CanConfirmItemRental {
                    can_confirm: true,
                },
            );
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
        let initiator = self
            .rental_sessions
            .iter()
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
                session
                    .owner_item
                    .as_ref()
                    .map(|i| i.unique_id)
                    .unwrap_or(0),
                session.renter_locked,
            )
        };

        self.send_rental_packet(
            msg.session_id,
            mir2_shared::packets::server::rental_system::ItemRentalLock {
                unique_id: item_uid,
                locked: true,
            },
        );
        self.send_rental_packet(
            initiator,
            mir2_shared::packets::server::rental_system::ItemRentalPartnerLock {
                unique_id: item_uid,
                locked: true,
            },
        );
        if both_locked {
            self.send_rental_packet(
                initiator,
                mir2_shared::packets::server::rental_system::CanConfirmItemRental {
                    can_confirm: true,
                },
            );
            self.send_rental_packet(
                partner,
                mir2_shared::packets::server::rental_system::CanConfirmItemRental {
                    can_confirm: true,
                },
            );
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
        let (initiator, _) = if self.rental_sessions.contains_key(&msg.session_id) {
            (msg.session_id, true)
        } else {
            match self
                .rental_sessions
                .iter()
                .find(|(_, s)| s.partner_session == msg.session_id)
                .map(|(k, _)| *k)
            {
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
        let renter_record = match self.players.get(&initiator) {
            Some(r) => r.clone(),
            None => return,
        };
        let owner_record = match self.players.get(&session.partner_session) {
            Some(r) => r.clone(),
            None => return,
        };

        // C# ConfirmItemRental（:14378）：物品模板 Bind.UnableToRent 或租赁 UnableToRent 不可成交
        if item
            .info
            .as_ref()
            .map(|i| {
                i.bind
                    .contains(mir2_shared::enums::BindMode::UNABLE_TO_RENT)
            })
            .unwrap_or(false)
            || super::rental_has_flag(&item, mir2_shared::enums::BindMode::UNABLE_TO_RENT.bits())
        {
            send_system_message(&self.gate_ref, msg.session_id, "该物品无法出租");
            let _ = owner_record
                .actor_ref
                .ask(AddItemToInventory { item })
                .await;
            self.send_rental_packet(
                initiator,
                mir2_shared::packets::server::rental_system::ConfirmItemRental { success: false },
            );
            self.send_rental_packet(
                session.partner_session,
                mir2_shared::packets::server::rental_system::ConfirmItemRental { success: false },
            );
            return;
        }

        // Check renter has enough gold
        let has_gold = renter_record
            .actor_ref
            .ask(crate::actors::player::HasGold { amount: fee })
            .await
            .unwrap_or(false);
        if !has_gold {
            send_system_message(&self.gate_ref, initiator, "金币不足，无法支付租金");
            // Return item to owner
            let _ = owner_record
                .actor_ref
                .ask(AddItemToInventory { item })
                .await;
            self.send_rental_packet(
                initiator,
                mir2_shared::packets::server::rental_system::ConfirmItemRental { success: false },
            );
            self.send_rental_packet(
                session.partner_session,
                mir2_shared::packets::server::rental_system::ConfirmItemRental { success: false },
            );
            return;
        }

        // Deduct gold from renter
        let deducted = renter_record
            .actor_ref
            .ask(DeductGold { amount: fee })
            .await
            .unwrap_or(false);
        if !deducted {
            send_system_message(&self.gate_ref, initiator, "金币扣除失败，租赁取消");
            let _ = owner_record
                .actor_ref
                .ask(AddItemToInventory { item })
                .await;
            self.send_rental_packet(
                initiator,
                mir2_shared::packets::server::rental_system::ConfirmItemRental { success: false },
            );
            self.send_rental_packet(
                session.partner_session,
                mir2_shared::packets::server::rental_system::ConfirmItemRental { success: false },
            );
            return;
        }

        // Give gold to owner
        let _ = owner_record.actor_ref.ask(AddGold { amount: fee }).await;

        // C# ConfirmItemRental（:14416）：移交前给物品写 RentalInformation（OwnerName/BindingFlags/ExpiryDate）
        let period_hours = session.period_hours.max(1);
        let mut rented_item = item.clone();
        rented_item.rental_information = Some(mir2_shared::data::item::RentalInformation {
            owner_name: owner_record.name.clone(),
            binding_flags: rental_binding_flags(),
            expiry_date_binary: crate::actors::world::tick::dotnet_now_ticks()
                + (period_hours as i64 * 3600 * 10_000_000),
            rental_locked: false,
        });

        // Give item to renter
        let added = renter_record
            .actor_ref
            .ask(AddItemToInventory {
                item: rented_item.clone(),
            })
            .await
            .unwrap_or(false);
        if !added {
            // Give gold back and return item to owner
            let _ = renter_record.actor_ref.ask(AddGold { amount: fee }).await;
            let _ = owner_record.actor_ref.ask(DeductGold { amount: fee }).await;
            let _ = owner_record
                .actor_ref
                .ask(AddItemToInventory { item })
                .await;
            send_system_message(&self.gate_ref, initiator, "背包已满，租赁失败");
            self.send_rental_packet(
                initiator,
                mir2_shared::packets::server::rental_system::ConfirmItemRental { success: false },
            );
            self.send_rental_packet(
                session.partner_session,
                mir2_shared::packets::server::rental_system::ConfirmItemRental { success: false },
            );
            return;
        }

        send_system_message(
            &self.gate_ref,
            initiator,
            &format!("租赁成功！支付 {} 金币，获得物品 {}", fee, item.item_index),
        );
        send_system_message(
            &self.gate_ref,
            session.partner_session,
            &format!(
                "租赁成功！获得 {} 金币，物品 {} 已出租",
                fee, item.item_index
            ),
        );

        // Persist to DB
        let expiry = chrono::Local::now().timestamp() + (period_hours as i64 * 3600);
        let now = chrono::Local::now().timestamp();
        // item_json 用于重启后重建 UserItem（到期归还物主；含 RentalInformation 绑定旗标）
        let item_json = serde_json::to_string(&rented_item).unwrap_or_default();
        let _ = sqlx::query(
            "INSERT INTO rentals (item_unique_id, item_index, owner_name, renter_name, fee, period_days, started_at, expires_at, item_json) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(item.unique_id as i64)
        .bind(item.item_index)
        .bind(owner_record.name.clone())
        .bind(renter_record.name.clone())
        .bind(fee as i64)
        .bind(period_hours as i64 / 24)
        .bind(now)
        .bind(expiry)
        .bind(item_json)
        .execute(&self.db_pool)
        .await;

        // Record the rental for expiry tracking（C# Info.RentedItems 归属物主）
        self.player_rentals
            .entry(owner_record.name.clone())
            .or_default()
            .push(RentedItem {
                item: rented_item.clone(),
                owner_name: owner_record.name.clone(),
                renter_name: renter_record.name.clone(),
                rental_fee: session.fee,
                expiry_timestamp: expiry,
            });

        self.send_rental_packet(
            initiator,
            mir2_shared::packets::server::rental_system::ConfirmItemRental { success: true },
        );
        self.send_rental_packet(
            session.partner_session,
            mir2_shared::packets::server::rental_system::ConfirmItemRental { success: true },
        );
        debug!(
            "ConfirmItemRental: {} -> {} item={} fee={}",
            initiator, session.partner_session, item.item_index, fee
        );
    }
}

pub struct GetRentedItemsRequest {
    pub session_id: u64,
}

impl Message<GetRentedItemsRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: GetRentedItemsRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        let items: Vec<mir2_shared::packets::server::rental_system::RentalItemInfo> = self
            .player_rentals
            .get(&state.name)
            .map(|rentals| {
                rentals
                    .iter()
                    .map(
                        |r| mir2_shared::packets::server::rental_system::RentalItemInfo {
                            item: r.item.clone(),
                            rental_fee: r.rental_fee,
                            rental_period: 0,
                            expiry_date: r.expiry_timestamp,
                            renting_player_name: r.renter_name.clone(),
                        },
                    )
                    .collect()
            })
            .unwrap_or_default();

        let packet = mir2_shared::packets::server::rental_system::GetRentedItems { items };
        self.send_rental_packet(msg.session_id, packet);
        debug!(
            "GetRentedItems: {} count={}",
            state.name,
            self.player_rentals
                .get(&state.name)
                .map(|v| v.len())
                .unwrap_or(0)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// #2208：C# ConfirmItemRental 绑定旗标 = DontDrop|DontStore|DontSell|DontTrade|UnableToRent|DontUpgrade|UnableToDisassemble = 0x305E
    #[test]
    fn rental_binding_flags_match_csharp() {
        assert_eq!(rental_binding_flags().bits(), 0x305E);
        let flags = rental_binding_flags();
        assert!(flags.contains(mir2_shared::enums::BindMode::DONT_DROP));
        assert!(flags.contains(mir2_shared::enums::BindMode::DONT_STORE));
        assert!(flags.contains(mir2_shared::enums::BindMode::DONT_SELL));
        assert!(flags.contains(mir2_shared::enums::BindMode::DONT_TRADE));
        assert!(flags.contains(mir2_shared::enums::BindMode::UNABLE_TO_RENT));
        assert!(flags.contains(mir2_shared::enums::BindMode::DONT_UPGRADE));
        assert!(flags.contains(mir2_shared::enums::BindMode::UNABLE_TO_DISASSEMBLE));
    }

    /// #2208：rental_has_flag 判定租赁绑定（含无租赁信息时返回 false）
    #[test]
    fn rental_has_flag_checks_binding() {
        let item = mir2_shared::data::item::UserItem {
            rental_information: Some(mir2_shared::data::item::RentalInformation {
                owner_name: "owner".into(),
                binding_flags: rental_binding_flags(),
                expiry_date_binary: 0,
                rental_locked: false,
            }),
            ..Default::default()
        };
        assert!(crate::actors::world::rental_has_flag(
            &item,
            mir2_shared::enums::BindMode::DONT_SELL.bits()
        ));
        assert!(crate::actors::world::rental_has_flag(
            &item,
            mir2_shared::enums::BindMode::DONT_TRADE.bits()
        ));
        assert!(!crate::actors::world::rental_has_flag(
            &item,
            mir2_shared::enums::BindMode::NO_MAIL.bits()
        ));
        let free = mir2_shared::data::item::UserItem::default();
        assert!(!crate::actors::world::rental_has_flag(
            &free,
            mir2_shared::enums::BindMode::DONT_SELL.bits()
        ));
    }

    #[test]
    fn auction_bid_validation() {
        // 起始价 1000，当前价 1000（初始=起始价）
        assert!(
            auction_bid_validate(1000, 1000, 1000).is_err(),
            "等于当前价应拒绝"
        );
        assert!(
            auction_bid_validate(1000, 1000, 1001).is_ok(),
            "高于当前价应通过"
        );
        assert!(
            auction_bid_validate(1000, 2000, 1500).is_err(),
            "低于当前价应拒绝"
        );
        assert!(auction_bid_validate(1000, 2000, 2001).is_ok());
    }

    /// #2216：C# MarketGetBack 售出金币 = cost - cost×5%（Consign=Price / Auction=CurrentBid）
    #[test]
    fn market_collect_gold_matches_csharp_commission() {
        let consign = AuctionListing {
            auction_id: 1,
            seller_name: "A".into(),
            item: mir2_shared::data::item::UserItem::new(1001),
            price: 1000,
            consignment_date: 0,
            sold: true,
            buyer_name: None,
            item_type: 0,
            current_bid: 0,
            current_buyer: None,
            expired: false,
        };
        assert_eq!(market_collect_gold(&consign), 950);
        let auction = AuctionListing {
            auction_id: 2,
            seller_name: "A".into(),
            item: mir2_shared::data::item::UserItem::new(1001),
            price: 1000,
            consignment_date: 0,
            sold: true,
            buyer_name: Some("B".into()),
            item_type: 1,
            current_bid: 2000,
            current_buyer: Some("B".into()),
            expired: false,
        };
        assert_eq!(market_collect_gold(&auction), 1900);
    }

    /// #2198：C# MarketSearch 过滤字段（类型/形状/市场面板/用户模式）
    #[test]
    fn market_search_filters_match_csharp() {
        // 默认：全部通过
        assert!(market_search_matches(0, false, 0, 0, 0, "A", "B", 0, 0, 0));
        // 类型过滤（C# ItemType 原始值：Weapon=1）
        assert!(market_search_matches(1, false, 0, 0, 0, "A", "B", 1, 0, 0));
        assert!(!market_search_matches(1, false, 0, 0, 0, "A", "B", 2, 0, 0));
        // 形状范围
        assert!(market_search_matches(
            0, false, 10, 20, 0, "A", "B", 0, 15, 0
        ));
        assert!(!market_search_matches(
            0, false, 10, 20, 0, "A", "B", 0, 9, 0
        ));
        assert!(!market_search_matches(
            0, false, 10, 20, 0, "A", "B", 0, 21, 0
        ));
        // 市场面板：1=Consign 2=Auction（内部 0=Consign 1=Auction）
        assert!(market_search_matches(0, false, 0, 0, 1, "A", "B", 0, 0, 0));
        assert!(!market_search_matches(0, false, 0, 0, 1, "A", "B", 0, 0, 1));
        assert!(market_search_matches(0, false, 0, 0, 2, "A", "B", 0, 0, 1));
        // 用户模式：只看自己寄售
        assert!(market_search_matches(0, true, 0, 0, 0, "A", "A", 0, 0, 0));
        assert!(!market_search_matches(0, true, 0, 0, 0, "A", "B", 0, 0, 0));
    }

    fn listing(
        item_type: u8,
        price: u32,
        current_bid: u64,
        current_buyer: Option<&str>,
    ) -> AuctionListing {
        AuctionListing {
            auction_id: 1,
            seller_name: "S".into(),
            item: mir2_shared::data::item::UserItem::new(1001),
            price,
            consignment_date: 0,
            sold: false,
            buyer_name: None,
            item_type,
            current_bid,
            current_buyer: current_buyer.map(|s| s.into()),
            expired: false,
        }
    }

    /// #2566：C# MarketSellNow（PlayerObject.cs:8615-8658）——仅 Auction 且 CurrentBid > Price
    /// 且有出价者才成交（卖家得 CurrentBid−5%）；寄售/无出价/已售出一律拒绝（防按起始价刷金）
    #[test]
    fn market_sell_now_settlement_matches_csharp() {
        // 寄售（Consign）→ 拒绝
        assert!(market_sell_now_settlement(&listing(0, 10_000, 0, None)).is_err());
        // 拍卖但无出价（current_bid == 起始价）→ 拒绝
        assert!(market_sell_now_settlement(&listing(1, 10_000, 10_000, None)).is_err());
        // 拍卖出价未超过起始价（current_bid <= price）→ 拒绝
        assert!(market_sell_now_settlement(&listing(1, 10_000, 9_999, Some("B"))).is_err());
        // 拍卖有出价者但 current_buyer 缺失 → 拒绝
        assert!(market_sell_now_settlement(&listing(1, 10_000, 12_000, None)).is_err());
        // 有效成交：卖家得 CurrentBid − 5%（12_000 − 600 = 11_400）
        assert_eq!(
            market_sell_now_settlement(&listing(1, 10_000, 12_000, Some("B"))).unwrap(),
            11_400
        );
        // 已售出 → 拒绝
        let mut sold = listing(1, 10_000, 12_000, Some("B"));
        sold.sold = true;
        assert!(market_sell_now_settlement(&sold).is_err());
        // 已过期 → 拒绝
        let mut expired = listing(1, 10_000, 12_000, Some("B"));
        expired.expired = true;
        assert!(market_sell_now_settlement(&expired).is_err());
    }

    /// #2566：价格区间按模式区分（C# Globals.cs:44-48：Consign 5000-50M；Auction 起始价 0-50,000）
    #[test]
    fn consign_price_range_per_market_type() {
        // Consign：[5000, 50_000_000]
        assert!(consign_price_validate(0, 4_999).is_err());
        assert!(consign_price_validate(0, 5_000).is_ok());
        assert!(consign_price_validate(0, 50_000_000).is_ok());
        assert!(consign_price_validate(0, 50_000_001).is_err());
        // Auction 起始价：[0, 50_000]
        assert!(consign_price_validate(1, 0).is_ok());
        assert!(consign_price_validate(1, 50_000).is_ok());
        assert!(consign_price_validate(1, 50_001).is_err());
        assert!(consign_price_validate(1, 5_000).is_ok());
    }
}
