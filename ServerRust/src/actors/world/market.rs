use super::*;
use tracing::error;

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

        // #2573：页 key 门槛（C# PlayerObject.cs:8436 MarketSearch 要求 MarketKey）
        if !self.npc_page_allows(msg.session_id, &["[@MARKET]"]) {
            send_system_message(&self.gate_ref, msg.session_id, "请先打开市场页");
            return;
        }
        // #2573：搜索节流（C# PlayerObject.cs:8329 SearchTime；Globals.SearchDelay=500ms）
        let now_ms = crate::db::now_unix_ms();
        if let Some(next) = self.market_search_next_ms.get(&msg.session_id) {
            if now_ms < *next {
                return;
            }
        }
        self.market_search_next_ms
            .insert(msg.session_id, now_ms + 500);

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
                user_mode: msg.user_mode,
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
        if let Err(e) = self
            .gate_ref
            .tell(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(
                    mir2_shared::enums::ServerPacketIds::NPCMarket as i16,
                    &body,
                ),
            })
            .try_send()
        {
            warn!(
                "gate mailbox full: SendToClient dropped (session={} opcode={:?} err={})",
                msg.session_id,
                super::dropped_send_opcode(&e),
                e
            );
        }

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
                        seller_name: market_seller_label(
                            a.item_type,
                            a.sold,
                            a.expired,
                            a.current_bid,
                            a.price,
                            &a.seller_name,
                            msg.user_mode,
                        ),
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
            if let Err(e) = self
                .gate_ref
                .tell(SendToClient {
                    session_id: msg.session_id,
                    data: build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::NPCMarketPage as i16,
                        &body,
                    ),
                })
                .try_send()
            {
                warn!(
                    "gate mailbox full: SendToClient dropped (session={} opcode={:?} err={})",
                    msg.session_id,
                    super::dropped_send_opcode(&e),
                    e
                );
            }
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
        // #2573：页 key 门槛（C# 市场操作要求 MarketKey）
        if !self.npc_page_allows(msg.session_id, &["[@MARKET]"]) {
            return;
        }

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
                // C# `MarketRefresh` 是全局刷新（非用户模式）
                user_mode: false,
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
        if let Err(e) = self
            .gate_ref
            .tell(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(
                    mir2_shared::enums::ServerPacketIds::NPCMarket as i16,
                    &body,
                ),
            })
            .try_send()
        {
            warn!(
                "gate mailbox full: SendToClient dropped (session={} opcode={:?} err={})",
                msg.session_id,
                super::dropped_send_opcode(&e),
                e
            );
        }

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
                        // `MarketRefresh` 是全局刷新（非用户模式）→ 显示卖家名
                        seller_name: market_seller_label(
                            a.item_type,
                            a.sold,
                            a.expired,
                            a.current_bid,
                            a.price,
                            &a.seller_name,
                            false,
                        ),
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
            if let Err(e) = self
                .gate_ref
                .tell(SendToClient {
                    session_id: msg.session_id,
                    data: build_packet_bytes(
                        mir2_shared::enums::ServerPacketIds::NPCMarketPage as i16,
                        &body,
                    ),
                })
                .try_send()
            {
                warn!(
                    "gate mailbox full: SendToClient dropped (session={} opcode={:?} err={})",
                    msg.session_id,
                    super::dropped_send_opcode(&e),
                    e
                );
            }
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
        // #2573：页 key 门槛（C# PlayerObject.cs:8335 翻页要求 MarketKey）
        if !self.npc_page_allows(msg.session_id, &["[@MARKET]"]) {
            return;
        }
        // #2573：翻页同受搜索节流（C# :8426-8428 MarketPageNext 复用 SearchTime）
        let now_ms = crate::db::now_unix_ms();
        if let Some(next) = self.market_search_next_ms.get(&msg.session_id) {
            if now_ms < *next {
                return;
            }
        }
        self.market_search_next_ms
            .insert(msg.session_id, now_ms + 500);

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
                if let Err(e) = self
                    .gate_ref
                    .tell(SendToClient {
                        session_id: msg.session_id,
                        data: build_packet_bytes(
                            mir2_shared::enums::ServerPacketIds::NPCMarketPage as i16,
                            &body,
                        ),
                    })
                    .try_send()
                {
                    warn!(
                        "gate mailbox full: SendToClient dropped (session={} opcode={:?} err={})",
                        msg.session_id,
                        super::dropped_send_opcode(&e),
                        e
                    );
                }
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
                    // 翻页沿用本次搜索的 `Usermode`（C# `AuctionInfo.GetSellerLabel`）
                    seller_name: market_seller_label(
                        a.item_type,
                        a.sold,
                        a.expired,
                        a.current_bid,
                        a.price,
                        &a.seller_name,
                        cache.user_mode,
                    ),
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
        if let Err(e) = self
            .gate_ref
            .tell(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(
                    mir2_shared::enums::ServerPacketIds::NPCMarketPage as i16,
                    &body,
                ),
            })
            .try_send()
        {
            warn!(
                "gate mailbox full: SendToClient dropped (session={} opcode={:?} err={})",
                msg.session_id,
                super::dropped_send_opcode(&e),
                e
            );
        }
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
        // #2573：页 key 门槛（C# 市场购买要求 MarketKey）
        if !self.npc_page_allows(msg.session_id, &["[@MARKET]"]) {
            send_system_message(&self.gate_ref, msg.session_id, "请先打开市场页");
            return;
        }

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
            let deducted = record
                .actor_ref
                .ask(DeductGold { amount: bid })
                .await
                .unwrap_or(false);
            if !deducted {
                send_system_message(&self.gate_ref, msg.session_id, "金币扣除失败");
                return;
            }
            // #2566：出价托管态先落库再改内存/退款——写库失败时全额退回新出价，
            // 内存态与被超价者均不动（否则重启后托管金蒸发、或被超价者退款后记录又回退 → 刷金）
            let bid_persist = db::update_auction_bid(
                &self.db_pool,
                msg.listing_id as i64,
                bid as i64,
                &buyer_state.name,
            )
            .await;
            if !db_write_ok(bid_persist) {
                warn!(
                    "Failed to persist auction bid (auction={}), rolling back",
                    msg.listing_id
                );
                // 退款必须全额：TryAddGold 原子语义（截顶=托管金蒸发），
                // 近封顶失败经系统邮件全额兜底 + error! 审计
                self.refund_gold_atomic(
                    &record.actor_ref,
                    &buyer_state.name,
                    bid,
                    "竞拍出价退回",
                    format!("出价失败（数据库错误），出价 {} 金币已退回", bid),
                )
                .await;
                send_system_message(
                    &self.gate_ref,
                    msg.session_id,
                    "出价失败：数据库错误，金币已退回",
                );
                return;
            }
            if let Some(a) = self.auctions.get_mut(auction_idx) {
                a.current_bid = bid;
                a.current_buyer = Some(buyer_state.name.clone());
            }
            if let Some(prev_buyer) = current_buyer {
                // 退还被超价者之前的出价（C# OutbidRefundGold 邮件 → Envir.MailCharacter 在线感知；
                // 在线必须进内存邮箱，直接 insert_mail 会被收件人下次存档 DELETE 重写抹掉）
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
                // 托管金退款：新出价已落库，此邮件丢失=旧出价托管金蒸发——
                // critical 变体失败 error! + 重试一次（不只 warn）
                let _ = self.deliver_system_mail_critical(mail).await;
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
                if let Err(e) = self
                    .gate_ref
                    .tell(SendToClient {
                        session_id: msg.session_id,
                        data: build_packet_bytes(
                            mir2_shared::enums::ServerPacketIds::MarketSuccess as i16,
                            &body,
                        ),
                    })
                    .try_send()
                {
                    warn!(
                        "gate mailbox full: SendToClient dropped (session={} opcode={:?} err={})",
                        msg.session_id,
                        super::dropped_send_opcode(&e),
                        e
                    );
                }
            }
            if let Ok(Some(new_state)) = record.actor_ref.ask(GetPlayerState).await {
                let packet = super::build_user_information_packet(&new_state, &self.item_infos);
                if let Err(e) = self
                    .gate_ref
                    .tell(SendToClient {
                        session_id: msg.session_id,
                        data: packet,
                    })
                    .try_send()
                {
                    warn!(
                        "gate mailbox full: SendToClient dropped (session={} opcode={:?} err={})",
                        msg.session_id,
                        super::dropped_send_opcode(&e),
                        e
                    );
                }
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
        // 交付入包会重发 unique_id（inventory.add_item）：记录真实 uid，回滚按它收回
        let delivered_uid = record
            .actor_ref
            .ask(AddItemToInventory { item: item.clone() })
            .await
            .ok()
            .delivered_item_uid(item.unique_id);
        let Some(delivered_uid) = delivered_uid else {
            // 退款必须全额：买家刚被 DeductGold 扣款，退款窗口内任何并发入账都会让
            // 截顶语义 AddGold 静默吞掉差额——TryAddGold 原子语义，近封顶失败经系统邮件
            // 全额兜底 + error! 审计
            self.refund_gold_atomic(
                &record.actor_ref,
                &buyer_state.name,
                price,
                "市场购买退款",
                format!("背包已满购买失败，{} 金币已退回", price),
            )
            .await;
            send_system_message(
                &self.gate_ref,
                msg.session_id,
                "背包已满，购买失败，金币已退回",
            );
            return;
        };

        // Item delivered successfully — now persist the sale
        let sold_persist =
            db::mark_auction_sold(&self.db_pool, msg.listing_id as i64, &buyer_state.name).await;
        if !db_write_ok(sold_persist) {
            // 写库失败优先回滚内存态：按交付时的真实 uid 收回已交付物品并退款，寄售记录保持未售
            // （否则重启后该单在 DB 仍未售，可被重复购买 → 物品复制）
            warn!(
                "Failed to mark auction {} sold in DB, rolling back",
                msg.listing_id
            );
            // 回收按【真实 uid + 交付数量】：交付可能堆叠合并进买家自有栈，
            // 整堆收回会连买家自有同类物品一起没收；部分收回不得当作全额回收（超退=复制）
            let (clawed, clawback) =
                clawback_delivered_item(&record.actor_ref, delivered_uid, item.count).await;
            if clawback == ClawbackOutcome::Full {
                // 交付物已全额收回，退款必须全额：截顶语义 AddGold 会让买家物财两失且
                // 无日志——TryAddGold 原子语义，近封顶失败经系统邮件全额兜底 + error! 审计
                self.refund_gold_atomic(
                    &record.actor_ref,
                    &buyer_state.name,
                    price,
                    "市场购买退款",
                    format!("购买失败（数据库错误），物品已收回，{} 金币已退回", price),
                )
                .await;
                send_system_message(
                    &self.gate_ref,
                    msg.session_id,
                    "购买失败：数据库错误，物品与金币已退回",
                );
                // 背包/金币回刷
                if let Ok(Some(new_state)) = record.actor_ref.ask(GetPlayerState).await {
                    let packet =
                        super::build_user_information_packet(&new_state, &self.item_infos);
                    if let Err(e) = self
                        .gate_ref
                        .tell(SendToClient {
                            session_id: msg.session_id,
                            data: packet,
                        })
                        .try_send()
                    {
                        warn!(
                            "gate mailbox full: SendToClient dropped (session={} opcode={:?} err={})",
                            msg.session_id,
                            super::dropped_send_opcode(&e),
                            e
                        );
                    }
                }
                return;
            }
            // 未全额收回：【不得退款】（买家仍持有全部/部分物品，退款=白送金币）、
            // 【不得保持未售】（重启后可二次售卖 → 物品复制）。
            // 部分收回时已收回部分必须归还买家（回滚全有或全无，吞掉=无故没收），
            // 未收回数量计入未回收告警；随后重试落库 sold 标记，仍失败 error! 待人工核查，
            // 落成功路径：内存标 sold、不退款
            if let ClawbackOutcome::Shortfall { removed, missing } = clawback {
                if let Some(back) = clawed {
                    self.return_clawed_item(&record.actor_ref, &buyer_state.name, back)
                        .await;
                }
                error!(
                    "Consign buy rollback shortfall: auction={} buyer={} delivered_uid={} removed={} missing={} — partial recall returned to buyer, unrecovered count logged; no refund issued",
                    msg.listing_id, buyer_state.name, delivered_uid, removed, missing
                );
            } else {
                error!(
                    "Consign buy rollback failed: auction={} buyer={} delivered_uid={} — buyer keeps item, no refund issued",
                    msg.listing_id, buyer_state.name, delivered_uid
                );
            }
            let retry =
                db::mark_auction_sold(&self.db_pool, msg.listing_id as i64, &buyer_state.name)
                    .await;
            if !db_write_ok(retry) {
                error!(
                    "Consign buy sold persist retry failed: auction={} buyer={} — DB still unsold, restart may duplicate the item; manual reconciliation required",
                    msg.listing_id, buyer_state.name
                );
            }
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
            if let Err(e) = self
                .gate_ref
                .tell(SendToClient {
                    session_id: msg.session_id,
                    data: packet,
                })
                .try_send()
            {
                warn!(
                    "gate mailbox full: SendToClient dropped (session={} opcode={:?} err={})",
                    msg.session_id,
                    super::dropped_send_opcode(&e),
                    e
                );
            }
        }

        let packet = mir2_shared::packets::server::market_system::MarketSuccess {
            message: "购买成功".to_string(),
        };
        let mut body = Vec::new();
        if let Err(e) = packet.write_body(&mut body) {
            warn!("Failed to serialize MarketSuccess: {}", e);
            return;
        }
        if let Err(e) = self
            .gate_ref
            .tell(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(
                    mir2_shared::enums::ServerPacketIds::MarketSuccess as i16,
                    &body,
                ),
            })
            .try_send()
        {
            warn!(
                "gate mailbox full: SendToClient dropped (session={} opcode={:?} err={})",
                msg.session_id,
                super::dropped_send_opcode(&e),
                e
            );
        }
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
        // #2573：页 key 门槛（C# PlayerObject.cs:8489 MarketGetBack 要求 MarketKey）
        if !self.npc_page_allows(msg.session_id, &["[@MARKET]"]) {
            self.send_market_fail(msg.session_id, 0);
            return;
        }
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
            // 取回物品（C# CanGainItem 失败 → Fail 5）
            // 交付入包会重发 unique_id：写库失败回滚必须按返回的真实 uid 收回
            let delivered_uid = record
                .actor_ref
                .ask(AddItemToInventory {
                    item: auction.item.clone(),
                })
                .await
                .ok()
                .delivered_item_uid(auction.item.unique_id);
            let Some(delivered_uid) = delivered_uid else {
                self.send_market_fail(msg.session_id, 5);
                return;
            };
            // 先落库删除再退款：写库失败时收回物品、寄售记录原样保留
            // （否则重启后记录复现 → 物品复制；且出价人若先被退款会双重退款 → 刷金）
            let deleted = db::delete_auction(&self.db_pool, msg.auction_id as i64).await;
            if !db_write_ok(deleted) {
                warn!(
                    "Failed to delete auction {} on take-back, rolling back",
                    msg.auction_id
                );
                // 回收按【真实 uid + 交付数量】：交付可能堆叠合并进卖家自有栈，
                // 整堆收回会连卖家自有同类物品一起没收；部分收回不得当作全额回收
                let (clawed, clawback) =
                    clawback_delivered_item(&record.actor_ref, delivered_uid, auction.item.count)
                        .await;
                match clawback {
                    ClawbackOutcome::Full => {}
                    ClawbackOutcome::Shortfall { removed, missing } => {
                        // 部分收回：已收回部分归还卖家（回滚全有或全无，吞掉=无故没收），
                        // 未收回数量计入未回收告警；物品部分留存且 DB 记录仍在 → 待人工核查
                        if let Some(back) = clawed {
                            self.return_clawed_item(&record.actor_ref, &state.name, back)
                                .await;
                        }
                        error!(
                            "MarketGetBack rollback shortfall: auction={} seller={} delivered_uid={} removed={} missing={} — partial recall returned to seller while record persists; manual reconciliation required",
                            msg.auction_id, state.name, delivered_uid, removed, missing
                        );
                    }
                    ClawbackOutcome::Nothing => {
                        // 收不回（物品已流转）：卖家已持物品且 DB 记录仍在——
                        // 重复取回/重启复现都会复制，error! 告警待人工核查（不得静默放过）
                        error!(
                            "MarketGetBack rollback failed: auction={} seller={} delivered_uid={} — item kept while record persists; manual reconciliation required",
                            msg.auction_id, state.name, delivered_uid
                        );
                    }
                }
                send_system_message(
                    &self.gate_ref,
                    msg.session_id,
                    "取回失败：数据库错误，请重试",
                );
                self.send_market_fail(msg.session_id, 0);
                return;
            }
            self.auctions.remove(auction_idx);
            // C# TakeAuction：过期拍卖若有当前出价 → 退款给出价人（:8680-8684；
            // 在线必须进内存邮箱，直接 insert_mail 会被其下次存档 DELETE 重写抹掉 → 托管金蒸发）
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
                    // 托管金退款邮件丢失=出价人托管金蒸发——critical 变体 error! + 重试一次
                    let _ = self.deliver_system_mail_critical(mail).await;
                }
            }
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
            // 先落库删除再发金币：写库失败时不付钱、记录保留
            // （否则重启后记录复现可重复领取 → 无中生有刷金）
            let deleted = db::delete_auction(&self.db_pool, msg.auction_id as i64).await;
            if !db_write_ok(deleted) {
                warn!(
                    "Failed to delete auction {} on gold collection, aborting",
                    msg.auction_id
                );
                send_system_message(
                    &self.gate_ref,
                    msg.session_id,
                    "领取失败：数据库错误，请重试",
                );
                self.send_market_fail(msg.session_id, 0);
                return;
            }
            // DB 记录已删：付款必须全额——TryAddGold 原子语义（CanGainGold 预检之后、
            // 付款之前的并发入账仍可能截顶，差额=蒸发），近封顶失败全额经系统邮件
            // 兜底 + error! 审计；无论直付/邮件兜底，售卖均已终结，内存记录同步移除
            let paid = self
                .refund_gold_atomic(
                    &record.actor_ref,
                    &state.name,
                    gold,
                    "市场售出金币",
                    format!("你的商品已售出，成交款 {} 金币（已扣 5% 佣金）", gold),
                )
                .await;
            self.auctions.remove(auction_idx);
            let commission = cost - gold;
            let text = if paid {
                format!("售出金币 {}（含佣金 {}）已领取", gold, commission)
            } else {
                format!("售出金币 {} 发放失败，已记录待人工核查", gold)
            };
            send_system_message(&self.gate_ref, msg.session_id, &text);
            self.send_market_success(msg.session_id, text);
            return;
        }

        send_system_message(&self.gate_ref, msg.session_id, "当前状态无法领取");
    }
}

/// 系统邮件（退款/成交交付）投递路由：收件人在线 → 内存邮箱（AddMail），离线 → 落库 insert_mail
/// （阻断8：在线玩家若直接 insert_mail，其下次存档按内存邮箱 DELETE 重写会把这封邮件抹掉 → 托管金蒸发）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SystemMailRoute {
    OnlineMailbox(u64),
    OfflineDb,
}

pub(crate) fn system_mail_route(online_session: Option<u64>) -> SystemMailRoute {
    match online_session {
        Some(sid) => SystemMailRoute::OnlineMailbox(sid),
        None => SystemMailRoute::OfflineDb,
    }
}

/// DB 写结果归一化：Ok(true)=成功；Ok(false)（0 行受影响，记录已被并发改写/删除）与 Err 一律视为失败，
/// 调用方必须回滚内存态（auction 不建立/状态还原），不得 warn 后继续（否则重启后双卖/重复领取）
pub(crate) fn db_write_ok(res: anyhow::Result<bool>) -> bool {
    matches!(res, Ok(true))
}

/// AddItemToInventory 应答归一化：交付入包时背包会【重发 unique_id】（inventory.add_item
/// 空位插入走 next_unique_id；合并堆叠则返回既有堆 uid），回滚/收回必须按交付返回的真实 uid，
/// 用寄售记录上的旧 uid 收回恒落空（收不回又不退款=物品复制、退款=白送）。
/// player 侧 Reply 为 Option<u64>（Some=交付后真实 uid，None=入包失败）。
pub(crate) trait DeliveredItemUid {
    fn delivered_item_uid(self, fallback_uid: u64) -> Option<u64>;
}

/// `ask(...).await.ok()` → Option<Option<u64>>，内层即交付后的真实 uid（None=入包失败）
impl DeliveredItemUid for Option<Option<u64>> {
    fn delivered_item_uid(self, _fallback_uid: u64) -> Option<u64> {
        self.flatten()
    }
}

/// 寄售/拍卖回收（clawback）结果：收回数量必须 == 交付量才算全额回收成功。
/// 交付可能堆叠合并进买家自有栈，买家又可能已消耗/转移部分——按数量收回会被堆叠
/// min 截断，收回数量可能 < 交付量；部分收回【不得】当作全额回收
/// （全额退款=超退复制、直接没收=吞买家物品），不足部分计入未回收，走告警/邮件兜底。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClawbackOutcome {
    /// 收回数量 == 交付量：可安全全额回滚（退金币/退物品）
    Full,
    /// 只收回一部分：removed=实际收回数量，missing=未收回数量（已消耗/转移）
    Shortfall { removed: u16, missing: u16 },
    /// 一无所获（物品已整体流转/玩家状态异常）
    Nothing,
}

/// 回收数量校验：不得把部分收回当作全额回收成功
pub(crate) fn clawback_outcome(
    removed_count: Option<u16>,
    delivered_count: u16,
) -> ClawbackOutcome {
    match removed_count {
        Some(n) if n >= delivered_count => ClawbackOutcome::Full,
        Some(n) if n > 0 => ClawbackOutcome::Shortfall {
            removed: n,
            missing: delivered_count - n,
        },
        _ => ClawbackOutcome::Nothing,
    }
}

/// 寄售/拍卖回收统一入口：按【交付时 AddItemToInventory 返回的真实 uid + 交付数量】收回。
/// 交付入包可能堆叠合并（入参 uid 被丢弃、并入目标堆），整堆 RemoveItemFromInventory
/// 会把持有人自有同类物品一起没收；RemoveItemFromInventoryCount 只拿走交付量，
/// 不动持有人自有部分。返回被移除部分与回收结果——部分收回时调用方必须把已收回部分
/// 归还持有人（回滚全有或全无），并把未收回数量计入未回收告警。
pub(crate) async fn clawback_delivered_item(
    actor_ref: &ActorRef<crate::actors::player::PlayerActor>,
    delivered_uid: u64,
    delivered_count: u16,
) -> (Option<mir2_shared::data::item::UserItem>, ClawbackOutcome) {
    let removed = actor_ref
        .ask(crate::actors::player::RemoveItemFromInventoryCount {
            unique_id: delivered_uid,
            count: delivered_count,
        })
        .await
        .ok()
        .flatten();
    let outcome = clawback_outcome(removed.as_ref().map(|i| i.count), delivered_count);
    (removed, outcome)
}

/// 到期结算门槛：sold 标记【先落库】成功才允许本轮交付买家。
/// 交付后落库失败 → 内存标 sold 但 DB 未售 → 重启重新结算 → 二次交付复制；
/// 落库失败本轮跳过（内存未标 sold，下个结算 tick 自然重试，重启后按 DB sold 状态去重）。
pub(crate) fn expired_delivery_allowed(sold_persist: &anyhow::Result<bool>) -> bool {
    matches!(sold_persist, Ok(true))
}

/// 市场托管金退款/付款的原子入账：TryAddGold——会截顶则整体失败、不加不减。
/// 绝不用截顶语义的 AddGold：买家/卖家近封顶（u32::MAX）时，退款窗口内任何并发入账
/// 都会让退款差额被静默截顶（退款方物财两失且无日志）。返回 false 时调用方必须
/// 经在线感知系统邮件全额兜底 + error! 审计（见 WorldActor::refund_gold_atomic）。
pub(crate) async fn try_add_gold_atomic(
    actor_ref: &ActorRef<crate::actors::player::PlayerActor>,
    amount: u64,
) -> bool {
    if amount == 0 {
        return true;
    }
    actor_ref.ask(TryAddGold { amount }).await.unwrap_or(false)
}

/// 退款/付款兜底邮件（内含全额托管金）：统一字段，便于测试断言金额不被截顶
pub(crate) fn gold_refund_mail(
    holder_name: &str,
    amount: u64,
    subject: &str,
    body: String,
) -> MailMessage {
    MailMessage {
        mail_id: generate_mail_id(),
        sender_name: "市场交易".to_string(),
        receiver_name: holder_name.to_string(),
        subject: subject.to_string(),
        body,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0),
        read: false,
        collected: false,
        locked: false,
        gold: amount,
        items: Vec::new(),
    }
}

/// 租赁取消物主退物的归还邮件（物主背包满兜底）：寄存物品必须随邮件完整归还物主，
/// 字段对齐 gold_refund_mail / session.rs 租赁归还邮件
pub(crate) fn rental_cancel_return_mail(
    owner_name: &str,
    item: mir2_shared::data::item::UserItem,
) -> MailMessage {
    MailMessage {
        mail_id: generate_mail_id(),
        sender_name: "物品租赁".to_string(),
        receiver_name: owner_name.to_string(),
        subject: "租赁归还".to_string(),
        body: "租赁取消退回的物品无法放入背包（背包已满或角色状态异常），改经邮件返还".to_string(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0),
        read: false,
        collected: false,
        locked: false,
        gold: 0,
        items: vec![item],
    }
}

impl WorldActor {
    /// 在线感知投递系统邮件（对齐 mail.rs 玩家邮件与 C# Envir.MailCharacter）：
    /// 收件人在线 → AddMail 进内存邮箱 + ReceiveMail 通知；仅离线才 db::insert_mail（登录时读回）。
    /// 返回是否投递成功；失败时调用方应 warn/回滚（邮件内含托管金/物品，丢失=蒸发）
    pub(crate) async fn deliver_system_mail(&self, mail: MailMessage) -> bool {
        let online = self
            .find_session_by_name_ignore_case(&mail.receiver_name)
            .await;
        if let SystemMailRoute::OnlineMailbox(sid) = system_mail_route(online) {
            if let Some(record) = self.players.get(&sid) {
                if record
                    .actor_ref
                    .ask(crate::actors::player::AddMail { mail: mail.clone() })
                    .await
                    .is_ok()
                {
                    send_mail_received_packet(&self.gate_ref, sid, &mail);
                    // C# PlayerObject.Process：收到新邮件 → 系统消息提示
                    send_system_message(&self.gate_ref, sid, "你收到了一封新邮件");
                    return true;
                }
                // 会话恰在查找后断开：落到离线落库
            }
        }
        match db::insert_mail(&self.db_pool, &mail.receiver_name, &mail).await {
            Ok(()) => true,
            Err(e) => {
                warn!(
                    "Failed to save offline mail for {}: {}",
                    mail.receiver_name, e
                );
                false
            }
        }
    }

    /// 关键系统邮件（退款/成交交付，内含托管金或物品）投递：失败不只 warn——
    /// error! + 重试一次；仍失败返回 false（托管金/物品蒸发，日志告警待人工核查）。
    /// 被超价退款等场景存在「新出价已落库、旧出价退款邮件丢失」窗口：退款邮件持久化
    /// 成功前崩溃即蒸发，故必须尽一切努力投递并留下 error! 痕迹。
    pub(crate) async fn deliver_system_mail_critical(&self, mail: MailMessage) -> bool {
        if self.deliver_system_mail(mail.clone()).await {
            return true;
        }
        error!(
            "System mail delivery failed, retrying once: receiver={} subject={} gold={} items={}",
            mail.receiver_name,
            mail.subject,
            mail.gold,
            mail.items.len()
        );
        let ok = self.deliver_system_mail(mail.clone()).await;
        if !ok {
            error!(
                "System mail delivery failed after retry: receiver={} subject={} gold={} items={} — escrow lost, manual reconciliation required",
                mail.receiver_name,
                mail.subject,
                mail.gold,
                mail.items.len()
            );
        }
        ok
    }

    /// 市场托管金退款/付款统一入口：try_add_gold_atomic 原子入账（截顶即整体失败），
    /// 失败（收款方近封顶 + 退款窗口内并发入账）时全额经在线感知系统邮件兜底
    /// （deliver_system_mail_critical：失败 error! + 重试一次）并 error! 审计。
    /// 返回是否已全额归还（入账或邮件投递成功）；false=蒸发（已 error! 待人工核查）。
    pub(crate) async fn refund_gold_atomic(
        &self,
        actor_ref: &ActorRef<crate::actors::player::PlayerActor>,
        holder_name: &str,
        amount: u64,
        subject: &str,
        body: String,
    ) -> bool {
        if try_add_gold_atomic(actor_ref, amount).await {
            return true;
        }
        error!(
            "Market gold refund TryAddGold failed (holder near cap + concurrent gain), falling back to system mail: holder={} amount={} subject={}",
            holder_name, amount, subject
        );
        self.deliver_system_mail_critical(gold_refund_mail(holder_name, amount, subject, body))
            .await
    }

    /// 回收（clawback）部分收回的反向归还：回滚必须全有或全无——已收回部分吞掉=无故没收，
    /// 必须原样还给持有人；入包失败走在线感知系统邮件兜底（丢失=物品蒸发 → critical 变体）
    async fn return_clawed_item(
        &self,
        actor_ref: &ActorRef<crate::actors::player::PlayerActor>,
        holder_name: &str,
        item: mir2_shared::data::item::UserItem,
    ) {
        let readded = actor_ref
            .ask(AddItemToInventory { item: item.clone() })
            .await
            .ok()
            .delivered_item_uid(item.unique_id)
            .is_some();
        if readded {
            return;
        }
        let mail = MailMessage {
            mail_id: generate_mail_id(),
            sender_name: "市场交易".to_string(),
            receiver_name: holder_name.to_string(),
            subject: "市场回滚归还".to_string(),
            body: "市场回滚过程中部分物品已被收回，现予归还".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0),
            read: false,
            collected: false,
            locked: false,
            gold: 0,
            items: vec![item],
        };
        // 归还邮件丢失=物品蒸发——critical 变体 error! + 重试一次
        let _ = self.deliver_system_mail_critical(mail).await;
    }

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

/// C# `AuctionInfo.GetSellerLabel(userMatch)`（Server/MirDatabase/AuctionInfo.cs:89-102）：
/// UserMode（寄售/拍卖页签，只看自己的记录）时「卖家」列显示状态标记串，否则显示卖家名。
/// 客户端 `AuctionRow` 依赖这些标记（`Sold` 金 / `Expired` 红 / `Bid Met` 草绿 + `Bid Met` 才可立即售出）。
pub(crate) fn market_seller_label(
    item_type: u8,
    sold: bool,
    expired: bool,
    current_bid: u64,
    price: u32,
    seller_name: &str,
    user_match: bool,
) -> String {
    // C# `MarketItemType`：0=Consign 1=Auction 2=GameShop
    match item_type {
        0 => {
            if !user_match {
                return seller_name.to_string();
            }
            if sold {
                "Sold".to_string()
            } else if expired {
                "Expired".to_string()
            } else {
                "For Sale".to_string()
            }
        }
        1 => {
            if !user_match {
                return seller_name.to_string();
            }
            if sold {
                "Sold".to_string()
            } else if expired {
                "Expired".to_string()
            } else if current_bid > price as u64 {
                "Bid Met".to_string()
            } else {
                "No Bid".to_string()
            }
        }
        _ => String::new(), // GameShop（C# 返回空串）
    }
}

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
            // 交付前先把 sold 标记落库：交付后落库失败 → 内存标 sold 但 DB 未售 →
            // 重启重新结算 → 二次交付复制。落库失败本轮不交付（内存未标 sold，
            // 下个结算 tick 自然重试；重启后按 DB sold 状态去重，不会重复结算）。
            let sold_persist = db::mark_auction_sold(&world.db_pool, id as i64, &winner).await;
            if !expired_delivery_allowed(&sold_persist) {
                error!(
                    "Failed to persist sold for expired auction {} (winner={}), skipping delivery this round",
                    id, winner
                );
                continue;
            }
            if let Some(a) = world.auctions.iter_mut().find(|a| a.auction_id == id) {
                a.sold = true;
                a.buyer_name = Some(winner.clone());
            }
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
                            // 入包失败（背包满）不得记 delivered——改走邮件交付，否则物品蒸发
                            let added = record
                                .actor_ref
                                .ask(AddItemToInventory { item: item.clone() })
                                .await
                                .ok()
                                .delivered_item_uid(item.unique_id)
                                .is_some();
                            if added {
                                send_system_message(
                                    &world.gate_ref,
                                    record.session_id,
                                    &format!("你以 {} 金币拍得 {}", bid, item_name),
                                );
                                delivered = true;
                            }
                            break;
                        }
                    }
                }
                if !delivered {
                    // 在线感知投递：买家在线（背包满）→ 内存邮箱；离线 → 落库
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
                    // 成交物品邮件丢失=物品蒸发（卖家已可领取金币）——critical 变体 error! + 重试一次
                    let _ = world.deliver_system_mail_critical(mail).await;
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

        // #2573：页 key 门槛（C# PlayerObject.cs:8599 MarketSellNow 要求 MarketKey）
        if !self.npc_page_allows(msg.session_id, &["[@MARKET]"]) {
            send_system_message(&self.gate_ref, msg.session_id, "请先打开市场页");
            return;
        }

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

        // 买家出价时金币已托管扣款。先落库删除寄售记录：失败则整体中止
        // （不交付、不付款，记录与托管态原样保留；否则重启后记录复现 → 物品/金币双份）
        let deleted = db::delete_auction(&self.db_pool, msg.auction_id as i64).await;
        if !db_write_ok(deleted) {
            warn!(
                "Failed to delete auction {} on sell-now, aborting",
                msg.auction_id
            );
            send_system_message(
                &self.gate_ref,
                msg.session_id,
                "立即售出失败：数据库错误，请重试",
            );
            self.send_market_fail(msg.session_id, 0);
            return;
        }

        // 物品交付买家（在线直接进包，入包失败/离线走邮件，C# MailCharacter）
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
                    // 入包失败（背包满）不得记 delivered——改走邮件交付，否则物品蒸发
                    let added = r
                        .actor_ref
                        .ask(AddItemToInventory {
                            item: auction.item.clone(),
                        })
                        .await
                        .ok()
                        .delivered_item_uid(auction.item.unique_id)
                        .is_some();
                    if added {
                        send_system_message(
                            &self.gate_ref,
                            r.session_id,
                            &format!("你以 {} 金币购得 {}", cost, item_name),
                        );
                        delivered = true;
                    }
                    break;
                }
            }
        }
        if !delivered {
            // 在线感知投递：买家在线（背包满）→ 内存邮箱；离线 → 落库
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
            // 成交物品邮件丢失=物品蒸发（记录已删、卖家已付款）——critical 变体 error! + 重试一次
            let _ = self.deliver_system_mail_critical(mail).await;
        }

        // 成交：内存移除记录，卖家得托管出价 − 5% 佣金（佣金回收）
        self.auctions.remove(auction_idx);

        // 付款必须全额（记录已删、买家已收物，截顶差额=蒸发）：TryAddGold 原子语义
        // （CanGainGold 预检之后仍可能并发入账截顶），近封顶失败经系统邮件全额兜底 + error! 审计
        let paid = self
            .refund_gold_atomic(
                &record.actor_ref,
                &state.name,
                seller_gold,
                "拍卖成交款",
                format!("你的拍卖已立即售出，成交款 {} 金币（已扣 5% 佣金）", seller_gold),
            )
            .await;
        let commission = cost - seller_gold;
        let text = if paid {
            format!(
                "立即售出成功，扣除手续费 {} 金币，获得 {} 金币",
                commission, seller_gold
            )
        } else {
            format!(
                "立即售出成功，成交款 {} 金币发放失败，已记录待人工核查",
                seller_gold
            )
        };
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

        // #2573：页 key 门槛（C# ConsignItem 仅要求 NPCPage != null，任意对话页即可）
        if !self.session_npc_page.contains_key(&msg.session_id) {
            send_system_message(&self.gate_ref, msg.session_id, "请先与 NPC 对话");
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

        // 序列化先于扣费/移除物品：此分支失败时无任何状态变更，无需回滚
        let item_json = match serde_json::to_string(&item) {
            Ok(j) => j,
            Err(e) => {
                warn!("Failed to serialize item for auction: {}", e);
                send_system_message(&self.gate_ref, msg.session_id, "寄售失败：数据错误");
                return;
            }
        };

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
            // 退回收寄费：TryAddGold 原子语义，近封顶失败经系统邮件全额兜底 + error! 审计
            self.refund_gold_atomic(
                &record.actor_ref,
                &state.name,
                fee,
                "寄售费退回",
                format!("寄售失败（移除物品失败），寄售费 {} 金币已退回", fee),
            )
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
            // Rollback: return item and refund fee（物品已离包、费用已扣，必须归还；
            // 背包异常时改走在线感知邮件归还，杜绝物品蒸发）
            let returned = record
                .actor_ref
                .ask(AddItemToInventory { item: item.clone() })
                .await
                .ok()
                .delivered_item_uid(item.unique_id)
                .is_some();
            if !returned {
                let mail = MailMessage {
                    mail_id: generate_mail_id(),
                    sender_name: "市场交易".to_string(),
                    receiver_name: state.name.clone(),
                    subject: "寄售失败退回".to_string(),
                    body: "寄售失败，物品已退回".to_string(),
                    timestamp: now,
                    read: false,
                    collected: false,
                    locked: false,
                    gold: 0,
                    items: vec![item.clone()],
                };
                // 物品退回邮件丢失=物品蒸发——critical 变体 error! + 重试一次
                let _ = self.deliver_system_mail_critical(mail).await;
            }
            // 退费：TryAddGold 原子语义（截顶=托管费蒸发），
            // 近封顶失败经系统邮件全额兜底 + error! 审计
            self.refund_gold_atomic(
                &record.actor_ref,
                &state.name,
                fee,
                "寄售费退回",
                format!("寄售失败（数据库错误），寄售费 {} 金币已退回", fee),
            )
            .await;
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

        // 严重14：关闭寄售崩溃窗口——物品已离包且 auction 已落库，立即存档该玩家；
        // 否则崩溃后按延迟旧存档回档，玩家背包与寄售记录各有一份物品（复制）
        if let Ok(Some(save_state)) = record.actor_ref.ask(GetPlayerState).await {
            if let Err(e) =
                db::save_character(&self.db_pool, &save_state, &record.account_username).await
            {
                warn!(
                    "ConsignItem: immediate save failed for {}: {}",
                    save_state.name, e
                );
            }
        }

        // 发送成功响应
        // 完整 UserInformation 刷新（背包移除 + 寄售费扣除，客户端本地背包同步）
        if let Ok(Some(new_state)) = record.actor_ref.ask(GetPlayerState).await {
            let packet = super::build_user_information_packet(&new_state, &self.item_infos);
            if let Err(e) = self
                .gate_ref
                .tell(SendToClient {
                    session_id: msg.session_id,
                    data: packet,
                })
                .try_send()
            {
                warn!(
                    "gate mailbox full: SendToClient dropped (session={} opcode={:?} err={})",
                    msg.session_id,
                    super::dropped_send_opcode(&e),
                    e
                );
            }
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
        if let Err(e) = self
            .gate_ref
            .tell(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(
                    mir2_shared::enums::ServerPacketIds::ConsignItem as i16,
                    &body,
                ),
            })
            .try_send()
        {
            warn!(
                "gate mailbox full: SendToClient dropped (session={} opcode={:?} err={})",
                msg.session_id,
                super::dropped_send_opcode(&e),
                e
            );
        }

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

        // 租赁会话按 C# 角色建键：发起方 = 物主（自有物品窗），partner = 租客（自有费用窗）
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

        // C# `PlayerObject.ItemRentalRequest`（Server/MirObjects/PlayerObject.cs:14072）：
        // 两端各收一份 —— 发起方 `Renting=false`（Name = 对方租客名），
        // 目标方 `Renting=true`（Name = 发起方即物主名）
        self.send_rental_packet(
            msg.session_id,
            mir2_shared::packets::server::rental_system::ItemRentalRequest {
                name: msg.target_name.clone(),
                renting: false,
            },
        );
        self.send_rental_packet(
            target_session,
            mir2_shared::packets::server::rental_system::ItemRentalRequest {
                name: state.name.clone(),
                renting: true,
            },
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

        // C# `DepositRentalItem` 由物主发出（会话键即物主 sid）
        let owner_sid = match self.rental_sessions.contains_key(&msg.session_id) {
            true => msg.session_id,
            false => {
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

        if let Some(session) = self.rental_sessions.get_mut(&owner_sid) {
            session.owner_item = Some(item.clone());
        }

        self.send_rental_packet(
            msg.session_id,
            mir2_shared::packets::server::rental_system::DepositRentalItem {
                unique_id: uid,
                success: true,
            },
        );
        // C# `UpdateRentalItem()`：把存入物品同步给租客（对方物品窗）
        if let Some(session) = self.rental_sessions.get(&owner_sid) {
            let (renter, fee, period) = (
                session.partner_session,
                session.fee,
                session.period_hours as i32,
            );
            self.send_rental_packet(
                renter,
                mir2_shared::packets::server::rental_system::UpdateRentalItem {
                    item: Some(item.clone()),
                    rental_fee: fee,
                    rental_period: period,
                },
            );
        }
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

        // C# `RetrieveRentalItem` 由物主发出（会话键即物主 sid）
        let owner_sid = match self.rental_sessions.contains_key(&msg.session_id) {
            true => msg.session_id,
            false => {
                send_system_message(&self.gate_ref, msg.session_id, "没有活跃的租赁会话");
                return;
            }
        };

        let item = if let Some(session) = self.rental_sessions.get_mut(&owner_sid) {
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
                    .ok()
                    .delivered_item_uid(item.unique_id)
                    .is_some();
            }
            self.send_rental_packet(
                msg.session_id,
                mir2_shared::packets::server::rental_system::RetrieveRentalItem {
                    unique_id: item.unique_id,
                    success: added,
                },
            );
            // 清空租客侧对方物品窗（C# `UpdateRentalItem` HasData=false）
            if let Some(renter) = self
                .rental_sessions
                .get(&owner_sid)
                .map(|s| s.partner_session)
            {
                self.send_rental_packet(
                    renter,
                    mir2_shared::packets::server::rental_system::UpdateRentalItem {
                        item: None,
                        rental_fee: 0,
                        rental_period: 0,
                    },
                );
            }
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
            // 存入物品退回物主：会话键 initiator 恒为物主（存物方），partner_session 是租客——
            // 误退 partner 会把寄存物白送租客；入包失败（None=背包满 / ask Err=actor 异常）
            // 不得静默吞物——error! 审计后走在线感知系统归还邮件兜底（critical：失败重试一次）
            if let Some(item) = s.owner_item {
                match self.players.get(&initiator) {
                    Some(record) => {
                        let record = record.clone();
                        let readded = record
                            .actor_ref
                            .ask(AddItemToInventory { item: item.clone() })
                            .await
                            .ok()
                            .delivered_item_uid(item.unique_id)
                            .is_some();
                        if !readded {
                            error!(
                                "CancelItemRental: return to owner bag failed (owner={} uid={} item_index={}), falling back to system mail",
                                record.name, item.unique_id, item.item_index
                            );
                            let mail = rental_cancel_return_mail(&record.name, item);
                            let _ = self.deliver_system_mail_critical(mail).await;
                        }
                    }
                    None => {
                        error!(
                            "CancelItemRental: owner offline, deposited item has no return path (owner_session={} uid={} item_index={}) — manual reconciliation required",
                            initiator, item.unique_id, item.item_index
                        );
                    }
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
        // C# `SetItemRentalFee`：费用由租客设置，只回给物主（对方费用窗）
        let owner_sid = self
            .rental_sessions
            .iter()
            .find(|(_, s)| s.partner_session == msg.session_id)
            .map(|(k, _)| *k);

        let owner_sid = match owner_sid {
            Some(sid) => sid,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "没有活跃的租赁会话");
                return;
            }
        };

        if let Some(session) = self.rental_sessions.get_mut(&owner_sid) {
            session.fee = msg.amount;
        }

        self.send_rental_packet(
            owner_sid,
            mir2_shared::packets::server::rental_system::ItemRentalFee { fee: msg.amount },
        );
        debug!(
            "ItemRentalFee: owner={} renter={} fee={}",
            owner_sid, msg.session_id, msg.amount
        );
    }
}

pub struct ItemRentalPeriodMsg {
    pub session_id: u64,
    pub duration: u32,
}

impl Message<ItemRentalPeriodMsg> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: ItemRentalPeriodMsg, _ctx: &mut Context<Self, Self::Reply>) {
        // C# `SetItemRentalPeriodLength`：期限由物主设置，只回给租客（对方物品窗）
        let owner_sid = match self.rental_sessions.contains_key(&msg.session_id) {
            true => msg.session_id,
            false => {
                send_system_message(&self.gate_ref, msg.session_id, "没有活跃的租赁会话");
                return;
            }
        };

        if let Some(session) = self.rental_sessions.get_mut(&owner_sid) {
            session.period_hours = msg.duration;
        }

        if let Some(renter) = self
            .rental_sessions
            .get(&owner_sid)
            .map(|s| s.partner_session)
        {
            self.send_rental_packet(
                renter,
                mir2_shared::packets::server::rental_system::ItemRentalPeriod {
                    period: msg.duration as i32,
                },
            );
        }
        debug!(
            "ItemRentalPeriod: owner={} hours={}",
            owner_sid, msg.duration
        );
    }
}

pub struct ItemRentalLockFeeMsg {
    pub session_id: u64,
}

impl Message<ItemRentalLockFeeMsg> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: ItemRentalLockFeeMsg, _ctx: &mut Context<Self, Self::Reply>) {
        // C# `ItemRentalLockFee`：由租客锁定费用 → 本端回执 `GoldLocked`，
        // 通知物主 `ItemRentalPartnerLock{GoldLocked}`，双方锁定后只通知物主可确认
        let owner_sid = self
            .rental_sessions
            .iter()
            .find(|(_, s)| s.partner_session == msg.session_id)
            .map(|(k, _)| *k);
        let owner_sid = match owner_sid {
            Some(sid) => sid,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "没有活跃的租赁会话");
                return;
            }
        };

        let (owner, both_locked) = {
            let session = match self.rental_sessions.get_mut(&owner_sid) {
                Some(s) => s,
                None => return,
            };
            session.renter_locked = true;
            (owner_sid, session.owner_locked)
        };

        self.send_rental_packet(
            msg.session_id,
            mir2_shared::packets::server::rental_system::ItemRentalLock {
                success: true,
                gold_locked: true,
                item_locked: false,
            },
        );
        self.send_rental_packet(
            owner,
            mir2_shared::packets::server::rental_system::ItemRentalPartnerLock {
                gold_locked: true,
                item_locked: false,
            },
        );

        // C# 只在物主侧开放确认按钮
        if both_locked {
            self.send_rental_packet(
                owner,
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
        // C# `ItemRentalLockItem`：由物主锁定物品（会话键即物主）
        let owner_sid = match self.rental_sessions.contains_key(&msg.session_id) {
            true => msg.session_id,
            false => {
                send_system_message(&self.gate_ref, msg.session_id, "没有活跃的租赁会话");
                return;
            }
        };

        let (renter, both_locked) = {
            let session = match self.rental_sessions.get_mut(&owner_sid) {
                Some(s) => s,
                None => return,
            };
            session.owner_locked = true;
            (session.partner_session, session.renter_locked)
        };

        self.send_rental_packet(
            msg.session_id,
            mir2_shared::packets::server::rental_system::ItemRentalLock {
                success: true,
                gold_locked: false,
                item_locked: true,
            },
        );
        self.send_rental_packet(
            renter,
            mir2_shared::packets::server::rental_system::ItemRentalPartnerLock {
                gold_locked: false,
                item_locked: true,
            },
        );
        // C# 只给物主发可确认（Confirm 按钮在物主自有物品窗）
        if both_locked {
            self.send_rental_packet(
                owner_sid,
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
        // 会话键 = 物主（存物/收租），partner = 租客（付费/收物）
        let owner_record = match self.players.get(&initiator) {
            Some(r) => r.clone(),
            None => return,
        };
        let renter_record = match self.players.get(&session.partner_session) {
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
            send_system_message(
                &self.gate_ref,
                session.partner_session,
                "金币不足，无法支付租金",
            );
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
            send_system_message(
                &self.gate_ref,
                session.partner_session,
                "金币扣除失败，租赁取消",
            );
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

        // Give item to renter（先交付后付款：交付失败只需退租客，无需向物主追回租金——
        // 旧序"先付物主、交付失败再 DeductGold 物主"在物主已花掉租金时追回失败 → 刷金）
        let added = renter_record
            .actor_ref
            .ask(AddItemToInventory {
                item: rented_item.clone(),
            })
            .await
            .ok()
            .delivered_item_uid(rented_item.unique_id)
            .is_some();
        if !added {
            // 退租客租金（TryAddGold 原子语义，近封顶失败经系统邮件全额兜底 + error! 审计），
            // 物品归还物主
            self.refund_gold_atomic(
                &renter_record.actor_ref,
                &renter_record.name,
                fee,
                "租赁退款",
                format!("背包已满租赁失败，租金 {} 金币已退回", fee),
            )
            .await;
            let _ = owner_record
                .actor_ref
                .ask(AddItemToInventory { item })
                .await;
            send_system_message(
                &self.gate_ref,
                session.partner_session,
                "背包已满，租赁失败",
            );
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

        // 物品已交付租客：向物主支付租金——TryAddGold 原子语义（截顶=租金蒸发），
        // 物主近封顶失败全额经系统邮件兜底 + error! 审计
        self.refund_gold_atomic(
            &owner_record.actor_ref,
            &owner_record.name,
            fee,
            "租赁租金",
            format!("你的物品已租出，租金 {} 金币", fee),
        )
        .await;

        send_system_message(
            &self.gate_ref,
            session.partner_session,
            &format!("租赁成功！支付 {} 金币，获得物品 {}", fee, item.item_index),
        );
        send_system_message(
            &self.gate_ref,
            initiator,
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
                    .map(|r| {
                        // C# `ItemRentalInformation`：ItemId/ItemName/RentingPlayerName/ItemReturnDate
                        let item_name = self
                            .item_infos
                            .get(&r.item.item_index)
                            .map(|i| i.name.clone())
                            .unwrap_or_else(|| format!("#{}", r.item.item_index));
                        mir2_shared::packets::server::rental_system::RentalItemInfo {
                            item_id: r.item.unique_id,
                            item_name,
                            renting_player_name: r.renter_name.clone(),
                            return_date: r.expiry_timestamp,
                        }
                    })
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

    /// #2720：C# `AuctionInfo.GetSellerLabel(userMatch)`（AuctionInfo.cs:89-102）逐分支对齐
    #[test]
    fn market_seller_label_matches_csharp() {
        // (item_type, sold, expired, current_bid, price, user_match, want)
        let cases = [
            // 非 UserMode → 卖家名（寄售/拍卖都一样）
            (0u8, false, false, 0u64, 100u32, false, "卖家"),
            (1, false, false, 0, 100, false, "卖家"),
            // 寄售（0）：Sold / Expired / For Sale
            (0, true, false, 0, 100, true, "Sold"),
            (0, false, true, 0, 100, true, "Expired"),
            (0, false, false, 0, 100, true, "For Sale"),
            // 拍卖（1）：Sold / Expired / Bid Met（当前出价 > 起始价）/ No Bid
            (1, true, false, 999, 100, true, "Sold"),
            (1, false, true, 999, 100, true, "Expired"),
            (1, false, false, 101, 100, true, "Bid Met"),
            (1, false, false, 100, 100, true, "No Bid"),
            // GameShop（2）→ 空串（C# 同）
            (2, false, false, 0, 100, true, ""),
        ];
        for (item_type, sold, expired, bid, price, user_match, want) in cases {
            assert_eq!(
                market_seller_label(item_type, sold, expired, bid, price, "卖家", user_match),
                want,
                "type={item_type} sold={sold} expired={expired} bid={bid} um={user_match}"
            );
        }
    }

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

    /// 阻断8 回归：系统邮件（被超价退款/拍卖过期退款/成交交付）路由——
    /// 收件人在线必须进内存邮箱（AddMail），仅离线才允许 insert_mail；
    /// 在线直插库会被收件人下次存档按内存邮箱 DELETE 重写抹掉（托管金蒸发）
    #[test]
    fn system_mail_route_online_goes_to_mailbox_not_db() {
        assert_eq!(
            system_mail_route(Some(42)),
            SystemMailRoute::OnlineMailbox(42),
            "在线收件人必须路由到内存邮箱"
        );
        assert_eq!(
            system_mail_route(None),
            SystemMailRoute::OfflineDb,
            "离线收件人才允许落库"
        );
    }

    /// 严重14 回归：DB 写结果归一化——Ok(false)（0 行受影响）与 Err 一律视为失败，
    /// 调用方必须回滚内存态（不得 warn 后继续，否则重启后双卖/重复领取 → 刷金）
    #[test]
    fn db_write_ok_treats_zero_rows_and_err_as_failure() {
        assert!(db_write_ok(Ok(true)));
        assert!(!db_write_ok(Ok(false)), "0 行受影响必须视为失败并回滚");
        assert!(
            !db_write_ok(Err(anyhow::anyhow!("db down"))),
            "写库错误必须视为失败并回滚"
        );
    }

    /// 严重（回滚 uid）回归：交付入包会重发 unique_id，回滚必须按 AddItemToInventory
    /// 返回的【真实 uid】收回，而非寄售记录上的旧 uid（旧 uid 收回恒落空）。
    /// 新签名 Option<u64>：Some(uid)=真实 uid；None=入包失败。
    /// 旧 bool 签名兼容期：true 退回记录 uid 尽力收回、false=入包失败。
    #[test]
    fn clawback_uses_delivered_uid_not_auction_record_uid() {
        let auction_record_uid = 123u64;
        let reissued_uid = 900u64;
        // 交付重发 uid → 回滚必须拿到重发后的 uid
        assert_eq!(
            Some(Some(reissued_uid)).delivered_item_uid(auction_record_uid),
            Some(reissued_uid),
            "回滚必须用交付时重发的真实 uid，而非记录旧 uid"
        );
        // 入包失败 → None（上层退款中止）
        assert_eq!(Some(None).delivered_item_uid(auction_record_uid), None);
        assert_eq!(None::<Option<u64>>.delivered_item_uid(auction_record_uid), None);
    }

    /// 严重（租赁取消）回归：物主背包满时取消租赁，寄存物品不得蒸发——
    /// 必须走系统归还邮件把物品完整退回物主（receiver=物主名、附件含原 uid）
    #[test]
    fn rental_cancel_full_bag_falls_back_to_return_mail() {
        let item = mir2_shared::data::item::UserItem {
            unique_id: 42,
            item_index: 7,
            ..Default::default()
        };
        let mail = rental_cancel_return_mail("物主甲", item);
        assert_eq!(mail.receiver_name, "物主甲", "归还邮件必须发给物主");
        assert_eq!(mail.sender_name, "物品租赁");
        assert_eq!(mail.subject, "租赁归还");
        assert_eq!(mail.gold, 0);
        assert_eq!(mail.items.len(), 1, "寄存物品必须随邮件附件归还");
        assert_eq!(mail.items[0].unique_id, 42, "归还的必须是原寄存物品");
        assert_eq!(mail.items[0].item_index, 7);
        assert!(!mail.read && !mail.collected && !mail.locked);
    }

    /// 严重（到期结算）回归：sold 标记先落库——落库失败（Err 或 0 行受影响）本轮不得交付
    /// （交付后落库失败 → 内存 sold 但 DB 未售 → 重启重新结算 → 二次交付复制）
    #[test]
    fn expired_auction_no_delivery_when_sold_persist_fails() {
        assert!(expired_delivery_allowed(&Ok(true)), "落库成功才允许交付");
        assert!(
            !expired_delivery_allowed(&Ok(false)),
            "0 行受影响（并发已售/已删）不得交付"
        );
        assert!(
            !expired_delivery_allowed(&Err(anyhow::anyhow!("db down"))),
            "落库错误不得交付，留下轮 tick 重试"
        );
    }

    /// 严重（到期结算）回归：mark_auction_sold 带 `AND sold = 0` 去重——
    /// 重启后按 DB sold 状态去重，重复结算的第二次落库必失败 → 不二次交付
    #[tokio::test]
    async fn mark_auction_sold_dedupes_resettlement_after_restart() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        // 与 init_db_pool 同 schema（ auctions 表最小列集）
        sqlx::query(
            r#"CREATE TABLE auctions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                auction_id INTEGER NOT NULL UNIQUE,
                seller_name TEXT NOT NULL,
                item_json TEXT NOT NULL,
                price INTEGER NOT NULL DEFAULT 0,
                consignment_date INTEGER NOT NULL DEFAULT 0,
                sold INTEGER NOT NULL DEFAULT 0,
                buyer_name TEXT,
                item_type INTEGER NOT NULL DEFAULT 0,
                current_bid INTEGER,
                current_buyer TEXT
            )"#,
        )
        .execute(&pool)
        .await
        .unwrap();
        db::save_auction(&pool, 1, "卖家", "{}", 1000, 0, 1)
            .await
            .unwrap();
        // 首次结算：落库成功 → 允许交付
        let first = db::mark_auction_sold(&pool, 1, "买家").await;
        assert!(
            expired_delivery_allowed(&first),
            "首次 sold 落库成功必须允许交付"
        );
        // 重启后重复结算同一单：落库必须失败（0 行受影响）→ 不得二次交付
        let second = db::mark_auction_sold(&pool, 1, "买家").await;
        assert!(
            !expired_delivery_allowed(&second),
            "重复结算必须被 DB sold 状态去重，禁止二次交付"
        );
    }

    // ============================================================
    // 寄售/拍卖回收（clawback）数量正确性 回归测试
    // ============================================================

    /// 市场测试栈：PlayerActor 的 spawn 依赖 world_ref，需拉起最小
    /// WorldActor（内存库、空目录）；SocialActor::spawn 仅为满足 WorldActorArgs
    async fn spawn_market_test_stack() -> (
        crate::db::DbPool,
        kameo::actor::ActorRef<crate::gate::actor::GateActor>,
        kameo::actor::ActorRef<crate::actors::world::WorldActor>,
    ) {
        use kameo::actor::Spawn;
        let db_pool = crate::db::init_db_pool("sqlite::memory:")
            .await
            .expect("init_db");
        let gate_ref = crate::gate::actor::GateActor::spawn(());
        let social_ref = crate::actors::social::SocialActor::spawn(
            crate::actors::social::SocialActorArgs {
                gate_ref: gate_ref.clone(),
                db_pool: db_pool.clone(),
                config: crate::actors::social::SocialActorConfig::default(),
            },
        );
        let world_ref =
            crate::actors::world::WorldActor::spawn(crate::actors::world::WorldActorArgs {
                tick_interval_ms: 1000,
                gate_ref: gate_ref.clone(),
                map_dir: std::path::PathBuf::from("."),
                spawn_dir: None,
                quest_dir: std::path::PathBuf::from("."),
                npc_script_dir: std::path::PathBuf::from("."),
                db_pool: db_pool.clone(),
                social_ref,
                conquest_cfg: crate::util::config::ConquestConfig::default(),
                rested_cfg: crate::util::config::RestedConfig::default(),
                pvp_cfg: crate::util::config::PvpConfig::default(),
                health_regen_weight: 10,
                mana_regen_weight: 10,
                goods_hide_added_stats: true,
                goods_on: true,
                goods_max_stored: 15,
                goods_buy_back_time_minutes: 60,
                goods_buy_back_max_stored: 20,
                safe_zone_healing: false,
                archive_inactive_after_months: 12,
                monster_recall_enabled: true,
                monster_recall_range: 12,
                monster_recall_cooldown_ms: 5000,
                exp_mob_level_difference: true,
                refine_cfg: crate::util::config::RefineConfig::default(),
                replace_wedring_cost: 125,
                lover_exp_bonus: 5,
                mentor_exp_boost: 10,
                mentor_damage_boost: 10,
                mentor_skill_boost: true,
                mentee_exp_bank: 1,
                orbs_exp_list: Vec::new(),
                orbs_dmg_list: Vec::new(),
                orbs_def_list: Vec::new(),
                awakening_cfg: Default::default(),
                gem_cfg: Default::default(),
                hero_exp_list: Vec::new(),
                setup_cfg: Default::default(),
                drop_rate: 1.0,
                exp_rate: 1.0,
                experience_list: Vec::new(),
                item_timeout_ticks: 300,
                max_drop_gold: 2000,
                drop_gold: true,
                rarity_cfg: crate::util::config::RarityConfig::default(),
                notice_path: "Notice.txt".to_string(),
                death_exp_penalty_percent: 0,
                movement_pacing_ms: 0,
                fishing_cfg: crate::util::ini::FishingConfig::default(),
                random_item_stats: Vec::new(),
                guild_buff_infos: Vec::new(),
            });
        (db_pool, gate_ref, world_ref)
    }

    fn mk_market_stack_item(
        uid: u64,
        item_index: i32,
        count: u16,
    ) -> mir2_shared::data::item::UserItem {
        let mut it = mir2_shared::data::item::UserItem::default();
        it.unique_id = uid;
        it.item_index = item_index;
        it.count = count;
        it.info = Some(mir2_shared::data::item::ItemInfo {
            index: item_index,
            stack_size: 20,
            ..Default::default()
        });
        it
    }

    async fn market_bag_count(
        actor_ref: &ActorRef<crate::actors::player::PlayerActor>,
        uid: u64,
    ) -> u16 {
        actor_ref
            .ask(crate::actors::player::GetPlayerState)
            .await
            .unwrap()
            .unwrap()
            .inventory
            .get_item(uid)
            .map(|i| i.count)
            .unwrap_or(0)
    }

    /// 严重（回收整堆没收）回归：寄售/拍卖回收必须按【真实 uid + 交付数量】收回。
    /// 交付堆叠合并进买家自有栈后，整堆收回会把买家自有同类物品一起没收；
    /// 按数量收回只拿走交付量，买家自有堆原样保留。
    /// 红检：clawback_delivered_item 退回整堆 RemoveItemFromInventory 语义时，
    /// 本测试在「bag==15」断言处必红（实际整堆 18 被没收）。
    #[tokio::test]
    async fn market_clawback_count_based_preserves_buyers_own_stack() {
        use kameo::actor::Spawn;
        let (_db_pool, gate_ref, world_ref) = spawn_market_test_stack().await;
        let buyer = crate::actors::player::PlayerActor::spawn((
            1u32,
            "Buyer".to_string(),
            1u64,
            1u16,
            gate_ref.clone(),
            world_ref.clone(),
            0u8,
            0u8,
            false,
        ));

        // 买家自有栈：item 666 x15（stack_size 20）
        let own_uid = buyer
            .ask(crate::actors::player::AddItemToInventory {
                item: mk_market_stack_item(u64::MAX - 900, 666, 15),
            })
            .await
            .unwrap()
            .unwrap();

        // 交付：寄售物品 666 x3 → 堆叠合并进买家自有栈（入参 uid 被丢弃，返回既有栈 uid）
        let delivered_uid = buyer
            .ask(crate::actors::player::AddItemToInventory {
                item: mk_market_stack_item(u64::MAX - 901, 666, 3),
            })
            .await
            .unwrap()
            .unwrap();
        assert_eq!(delivered_uid, own_uid, "堆叠合并：交付并入买家自有栈");
        assert_eq!(market_bag_count(&buyer, own_uid).await, 18, "合并后 15+3=18");

        // 回收：只拿走交付量 3，买家自有 15 原样保留
        let (removed, outcome) = clawback_delivered_item(&buyer, delivered_uid, 3).await;
        assert_eq!(outcome, ClawbackOutcome::Full);
        assert_eq!(removed.map(|i| i.count), Some(3), "收回数量必须 == 交付量");
        assert_eq!(
            market_bag_count(&buyer, own_uid).await,
            15,
            "回收只拿走交付量，不得动买家自有堆"
        );
    }

    /// 严重（回收数量校验）回归：交付后买家部分消耗/转移，收回数量 < 交付量时
    /// 必须判定为 Shortfall（不足部分计入未回收），不得当作全额回收成功。
    #[tokio::test]
    async fn market_clawback_shortfall_when_buyer_consumed_part() {
        use kameo::actor::Spawn;
        let (_db_pool, gate_ref, world_ref) = spawn_market_test_stack().await;
        let buyer = crate::actors::player::PlayerActor::spawn((
            1u32,
            "Buyer".to_string(),
            1u64,
            1u16,
            gate_ref.clone(),
            world_ref.clone(),
            0u8,
            0u8,
            false,
        ));

        // 买家自有栈 666 x1；交付 x3 合并 → 4
        let own_uid = buyer
            .ask(crate::actors::player::AddItemToInventory {
                item: mk_market_stack_item(u64::MAX - 910, 666, 1),
            })
            .await
            .unwrap()
            .unwrap();
        let delivered_uid = buyer
            .ask(crate::actors::player::AddItemToInventory {
                item: mk_market_stack_item(u64::MAX - 911, 666, 3),
            })
            .await
            .unwrap()
            .unwrap();
        assert_eq!(delivered_uid, own_uid);
        assert_eq!(market_bag_count(&buyer, own_uid).await, 4);

        // 买家消耗/转移 2 件 → 栈剩 2（< 交付量 3）
        let consumed = buyer
            .ask(crate::actors::player::RemoveItemFromInventoryCount {
                unique_id: own_uid,
                count: 2,
            })
            .await
            .unwrap()
            .unwrap();
        assert_eq!(consumed.count, 2);

        // 回收：按堆叠 min 截断只收回 2，差额 1 必须判 Shortfall 计入未回收
        let (removed, outcome) = clawback_delivered_item(&buyer, delivered_uid, 3).await;
        assert_eq!(
            outcome,
            ClawbackOutcome::Shortfall {
                removed: 2,
                missing: 1
            },
            "收回不足交付量必须判 Shortfall，不得当作全额回收成功"
        );
        assert_eq!(removed.map(|i| i.count), Some(2));
        assert_eq!(market_bag_count(&buyer, own_uid).await, 0);

        // 判定函数对照：全额 / 一无所获
        assert_eq!(clawback_outcome(Some(3), 3), ClawbackOutcome::Full);
        assert_eq!(clawback_outcome(None, 3), ClawbackOutcome::Nothing);
    }

    // ============================================================
    // 严重（退款截顶蒸发）回归：市场退款/付款必须 TryAddGold 原子语义，
    // 近封顶失败由 refund_gold_atomic 经系统邮件全额兜底
    // ============================================================

    async fn market_player_gold(actor_ref: &ActorRef<crate::actors::player::PlayerActor>) -> u64 {
        actor_ref
            .ask(crate::actors::player::GetPlayerState)
            .await
            .unwrap()
            .unwrap()
            .inventory
            .gold
    }

    /// 严重（:618 背包满退款 / :642 全收回回滚退款共用 refund_gold_atomic → try_add_gold_atomic）
    /// 回归：买家近封顶 + 退款窗口内并发入账（离顶额度 < 退款额）时，退款不得被静默截顶——
    /// 必须整体失败（false）、不加不减，由 refund_gold_atomic 落系统邮件全额兜底。
    /// 红检：把 try_add_gold_atomic 内 TryAddGold 换回截顶语义 AddGold（恒 true、超顶只加
    /// 剩余额度），本测试在「!ok」「金币不变」断言处必红（截顶照加且谎报成功 → 差额蒸发）。
    #[tokio::test]
    async fn market_refund_try_add_gold_atomic_never_truncates() {
        use kameo::actor::Spawn;
        let (_db_pool, gate_ref, world_ref) = spawn_market_test_stack().await;
        let buyer = crate::actors::player::PlayerActor::spawn((
            1u32,
            "NearCapBuyer".to_string(),
            1u64,
            1u16,
            gate_ref.clone(),
            world_ref.clone(),
            0u8,
            0u8,
            false,
        ));

        // 近封顶：退款窗口内并发入账后离顶只剩 100
        buyer
            .ask(crate::actors::player::AddGold {
                amount: u32::MAX as u64 - 100,
            })
            .await
            .unwrap();

        // 退款 1000 > 剩余额度 100：必须整体失败且金币不变（截顶=买家物财两失）
        let ok = try_add_gold_atomic(&buyer, 1000).await;
        assert!(!ok, "会截顶必须整体失败，由 refund_gold_atomic 走邮件全额兜底");
        assert_eq!(
            market_player_gold(&buyer).await,
            u32::MAX as u64 - 100,
            "截顶失败必须不加不减，不得静默截顶"
        );

        // 恰好放得下：全额到账返回 true
        assert!(try_add_gold_atomic(&buyer, 100).await);
        assert_eq!(market_player_gold(&buyer).await, u32::MAX as u64);

        // 0 退款恒成功（无操作）
        assert!(try_add_gold_atomic(&buyer, 0).await);
    }

    /// 严重（:618/:642 邮件兜底金额）回归：TryAddGold 失败后的兜底邮件必须携带【全额】
    /// 退款金币（在线进内存邮箱 / 离线落库均按 MailMessage.gold 兑现），不得带截顶差额。
    #[test]
    fn market_refund_fallback_mail_carries_full_amount() {
        let mail = gold_refund_mail("买家", 1_234_567, "市场购买退款", "购买失败退款".into());
        assert_eq!(mail.receiver_name, "买家");
        assert_eq!(mail.subject, "市场购买退款");
        assert_eq!(mail.gold, 1_234_567, "兜底邮件必须携带全额退款");
        assert!(mail.items.is_empty());
        assert!(!mail.read && !mail.collected && !mail.locked);
    }
}
