use super::*;
use tracing::error;

/// 发送邮件
/// S.MailSent：Result(sbyte)（C# SendMail：1=成功，-1=失败）
fn send_mail_sent_result(gate_ref: &ActorRef<GateActor>, session_id: u64, result: i8) {
    let body = vec![result as u8];
    let _ = gate_ref
        .tell(SendToClient {
            session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::MailSent as i16, &body),
        })
        .try_send();
}

/// S.ParcelCollected：Result(sbyte)（C# CollectMail：1=成功，-1=无可收取包裹/收取失败）
fn send_parcel_collected_result(gate_ref: &ActorRef<GateActor>, session_id: u64, result: i8) {
    let body = vec![result as u8];
    let _ = gate_ref
        .tell(SendToClient {
            session_id,
            data: build_packet_bytes(
                mir2_shared::enums::ServerPacketIds::ParcelCollected as i16,
                &body,
            ),
        })
        .try_send();
}

/// 测试观测缝：按 session + item_index 查背包物品当前 uid。
/// 回滚/归还给付经 AddItemToInventory 重新生成 uid（防复制语义），
/// e2e 断言「退回后可再次寄出」须按真实 uid 重新发起。
/// 仅测试使用（两个 e2e 测试模块均在 cfg(test) 下），release 构建不携带。
#[cfg(test)]
pub struct GetPlayerItemUid {
    pub session_id: u64,
    pub item_index: i32,
}

#[cfg(test)]
impl Message<GetPlayerItemUid> for WorldActor {
    type Reply = Option<u64>;

    async fn handle(&mut self, msg: GetPlayerItemUid, _ctx: &mut Context<Self, Self::Reply>) -> Self::Reply {
        let record = self.players.get(&msg.session_id)?;
        let state = record.actor_ref.ask(GetPlayerState).await.ok()??;
        state
            .inventory
            .backpack
            .iter()
            .flatten()
            .find(|s| s.item.item_index == msg.item_index)
            .map(|s| s.item.unique_id)
    }
}

pub struct SendMailRequest {
    pub session_id: u64,
    pub receiver_name: String,
    pub subject: String,
    pub body: String,
    pub gold: u32,
    pub item_uids: Vec<u64>,
    /// #2538：贴票（C# C.SendMail.Stamped；消耗一张邮票并解锁 5 附件格）
    pub stamped: bool,
}

/// #2538：查询邮资（C# C.MailCost → S.MailCost；写信面板邮资显示）
pub struct MailCostRequest {
    pub session_id: u64,
    pub gold: u32,
    pub item_uids: Vec<u64>,
    pub stamped: bool,
}

/// #2538：邮票判定（C# ItemType.Nothing && Shape==1；C# 枚举 0-based，db item_type==0）
pub(crate) fn is_stamp_item(info: &db::ItemInfo) -> bool {
    info.item_type == 0 && info.shape == 1
}

/// #2538：C# PlayerObject.GetMailCost（11926-11957）——
/// 免费条件 MailFreeWithStamp && stamped；否则金币费 floor(gold/1000)*Per1K
/// + 附件保险 floor(price/100)*Pct（item_uids 调用方已按 stamped?5:1 截断）
pub(crate) fn compute_mail_cost(
    state: &crate::actors::player::PlayerState,
    item_infos: &std::collections::HashMap<i32, db::ItemInfo>,
    item_uids: &[u64],
    gold: u32,
    stamped: bool,
    per_1k: u32,
    insurance_pct: u32,
    free_with_stamp: bool,
) -> u64 {
    if free_with_stamp && stamped {
        return 0;
    }
    let mut prices = Vec::new();
    for uid in item_uids {
        if let Some(item) = state.inventory.get_item(*uid) {
            if let Some(info) = item_infos.get(&item.item_index) {
                // C# GetMailCost：item.Price()（含耐久比例/附加属性）× Count
                prices.push(
                    super::item::compute_item_price_per_unit(item, info)
                        .saturating_mul(item.count as u64),
                );
            }
        }
    }
    mail_cost_from_prices(&prices, gold, per_1k, insurance_pct)
}

/// #2538：计费核心（金币费 + 每件保险费；纯函数便于测试）
pub(crate) fn mail_cost_from_prices(
    item_prices: &[u64],
    gold: u32,
    per_1k: u32,
    insurance_pct: u32,
) -> u64 {
    let gold_fee = (gold as u64 / 1000) * per_1k as u64;
    let item_fee: u64 = item_prices
        .iter()
        .copied()
        .map(|p| p / 100 * insurance_pct as u64)
        .sum();
    gold_fee + item_fee
}

/// 读取邮件
pub struct ReadMailRequest {
    pub session_id: u64,
    pub mail_id: u64,
}

/// 收取邮件附件
pub struct CollectParcelRequest {
    pub session_id: u64,
    pub mail_id: u64,
}

/// 删除邮件
pub struct DeleteMailRequest {
    pub session_id: u64,
    pub mail_id: u64,
}

pub struct LockMailRequest {
    pub session_id: u64,
    pub mail_id: u64,
    pub lock: bool,
}

impl Message<LockMailRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: LockMailRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let mut state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        if let Some(mail) = state.mailbox.get_mail_mut(msg.mail_id) {
            mail.locked = msg.lock;
            let _ = record
                .actor_ref
                .ask(SetPlayerState {
                    state: state.clone(),
                })
                .await;
            debug!(
                "LockMail: {} mail_id={} lock={}",
                state.name, msg.mail_id, msg.lock
            );
        }
    }
}

pub struct MailLockedItemRequest {
    pub session_id: u64,
    /// C# C.MailLockedItem.UniqueID（邮件附件的物品 uid）
    pub unique_id: u64,
    /// C# C.MailLockedItem.Locked
    pub locked: bool,
}

impl Message<MailLockedItemRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: MailLockedItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        // C# MirConnection.cs:677-678：MailLockedItem 仅回显给客户端（无服务端状态）
        let mut body = Vec::new();
        if mir2_shared::packets::base::serialize_packet(
            &mut std::io::Cursor::new(&mut body),
            &mir2_shared::packets::server::mail_system::MailLockedItem {
                unique_id: msg.unique_id,
                locked: msg.locked,
            },
        )
        .is_ok()
        {
            let _ = self
                .gate_ref
                .tell(SendToClient {
                    session_id: msg.session_id,
                    data: body,
                })
                .await;
        }
        debug!(
            "MailLockedItem: session={} uid={} locked={}",
            msg.session_id, msg.unique_id, msg.locked
        );
    }
}

impl Message<MailCostRequest> for WorldActor {
    type Reply = ();

    /// #2538：C# MirConnection.MailCost（2055）→ PlayerObject.GetMailCost → S.MailCost
    async fn handle(&mut self, msg: MailCostRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        let (per_1k, insurance_pct, free_with_stamp, ..) = self
            .social_ref
            .ask(crate::actors::social::NpcGetMailSettings)
            .await
            .unwrap_or((100, 5, true, 100, false, false));
        // C# GetMailCost：物品保险仅计 stamped ? 5 : 1 格
        let uids: Vec<u64> = if msg.stamped {
            msg.item_uids.clone()
        } else {
            msg.item_uids.iter().take(1).copied().collect()
        };
        let cost = compute_mail_cost(
            &state,
            &self.item_infos,
            &uids,
            msg.gold,
            msg.stamped,
            per_1k,
            insurance_pct,
            free_with_stamp,
        ) as u32;
        let mut body = Vec::new();
        if mir2_shared::packets::base::serialize_packet(
            &mut std::io::Cursor::new(&mut body),
            &mir2_shared::packets::server::mail_system::MailCost { cost },
        )
        .is_ok()
        {
            let _ = self
                .gate_ref
                .tell(SendToClient {
                    session_id: msg.session_id,
                    data: body,
                })
                .await;
        }
        debug!(
            "MailCost: session={} gold={} items={} stamped={} cost={}",
            msg.session_id,
            msg.gold,
            uids.len(),
            msg.stamped,
            cost
        );
    }
}

/// 构造 SendMail 回滚失败的系统归还邮件：金币挂首封，物品按收件上限 5 件/封分封
/// （与 social.rs build_trade_return_mails 同款；在线感知投递走 deliver_system_mail_critical）
fn build_send_mail_return_mails(
    receiver_name: &str,
    gold: u64,
    items: Vec<mir2_shared::data::item::UserItem>,
) -> Vec<MailMessage> {
    const MAIL_ITEM_CAP: usize = 5; // MailMessage.items 附件上限
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let mut chunks: Vec<Vec<mir2_shared::data::item::UserItem>> =
        items.chunks(MAIL_ITEM_CAP).map(|c| c.to_vec()).collect();
    if chunks.is_empty() && gold > 0 {
        chunks.push(Vec::new());
    }
    let total = chunks.len();
    chunks
        .into_iter()
        .enumerate()
        .map(|(idx, chunk)| MailMessage {
            mail_id: generate_mail_id(),
            sender_name: "邮件系统".to_string(),
            receiver_name: receiver_name.to_string(),
            subject: "发送失败款项退回".to_string(),
            body: format!(
                "邮件发送失败，回滚时未能直接退回的款项与附件由系统邮件归还（第 {}/{} 封）",
                idx + 1,
                total
            ),
            timestamp: now,
            read: false,
            collected: false,
            locked: false,
            gold: if idx == 0 { gold } else { 0 },
            items: chunk,
        })
        .collect()
}

/// 构造 CollectParcel 附件写回失败的系统归还邮件：金币挂首封，物品按收件上限
/// 5 件/封分封（与 build_send_mail_return_mails / social 交易归还同款）。
fn build_collect_return_mails(
    receiver_name: &str,
    gold: u64,
    items: Vec<mir2_shared::data::item::UserItem>,
) -> Vec<MailMessage> {
    const MAIL_ITEM_CAP: usize = 5; // MailMessage.items 附件上限
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let mut chunks: Vec<Vec<mir2_shared::data::item::UserItem>> =
        items.chunks(MAIL_ITEM_CAP).map(|c| c.to_vec()).collect();
    if chunks.is_empty() && gold > 0 {
        chunks.push(Vec::new());
    }
    let total = chunks.len();
    chunks
        .into_iter()
        .enumerate()
        .map(|(idx, chunk)| MailMessage {
            mail_id: generate_mail_id(),
            sender_name: "邮件系统".to_string(),
            receiver_name: receiver_name.to_string(),
            subject: "收取失败附件归还".to_string(),
            body: format!(
                "收取附件时邮件状态异常，未能写回原邮件的款项与附件由系统邮件归还（第 {}/{} 封）",
                idx + 1,
                total
            ),
            timestamp: now,
            read: false,
            collected: false,
            locked: false,
            gold: if idx == 0 { gold } else { 0 },
            items: chunk,
        })
        .collect()
}

/// CollectParcel 附件写回失败兜底投递（在线感知 + 失败重试，market.rs
/// deliver_system_mail / deliver_system_mail_critical 同款关键投递语义）：
/// 先 AddMail 进玩家内存邮箱（在线玩家直接 db::insert_mail 会被下次存档
/// save_mail 按内存邮箱 DELETE 重写抹掉 → 附件蒸发）；ask 失败落库（登录读回）；
/// 落库再失败 error! + 重试一次；仍失败 error! 待人工核查。返回是否全部投递成功。
/// 独立成自由函数以便确定性回归：restore 失败无法经客户端包路径确定性触发
/// （kameo actor 顺序处理，DeleteMail 不会插入 CollectParcel 处理中途），
/// 测试直接驱动本函数验证「写回失败 → 附件经系统归还邮件兜底，不蒸发」。
async fn deliver_collect_restore_fallback(
    player_ref: &ActorRef<crate::actors::player::PlayerActor>,
    gate_ref: &ActorRef<GateActor>,
    db_pool: &db::DbPool,
    session_id: u64,
    receiver_name: &str,
    gold: u64,
    items: Vec<mir2_shared::data::item::UserItem>,
) -> bool {
    let mut all_ok = true;
    for mail in build_collect_return_mails(receiver_name, gold, items) {
        if player_ref
            .ask(crate::actors::player::AddMail { mail: mail.clone() })
            .await
            .is_ok()
        {
            send_mail_received_packet(gate_ref, session_id, &mail);
            send_system_message(gate_ref, session_id, "你收到了一封新邮件");
            continue;
        }
        // 玩家 actor 异常/刚好离线：落库（登录时读回）
        if db::insert_mail(db_pool, receiver_name, &mail).await.is_ok() {
            continue;
        }
        error!(
            "收取归还邮件落库失败，重试一次: receiver={} gold={} items={}",
            receiver_name,
            mail.gold,
            mail.items.len()
        );
        let retried = db::insert_mail(db_pool, receiver_name, &mail).await.is_ok();
        if !retried {
            error!(
                "收取归还邮件投递最终失败（附件蒸发，人工核查）: receiver={} gold={} items={}",
                receiver_name,
                mail.gold,
                mail.items.len()
            );
        }
        all_ok &= retried;
    }
    all_ok
}

impl WorldActor {
    /// SendMail 回滚统一出口：退回已扣金币（附件 + 邮资）/物品/邮票。
    /// 任何归还失败必须 error! 审计（含 session/物品 uid/数量），未还部分改走
    /// 系统邮件归还（deliver_system_mail_critical：在线 AddMail / 离线 insert_mail，
    /// 与 market 退款、social 交易归还同款在线感知模式，防蒸发）。
    async fn rollback_send_mail_escrow(
        &self,
        session_id: u64,
        record: &PlayerRecord,
        sender_name: &str,
        gold: u64,
        items: Vec<mir2_shared::data::item::UserItem>,
        stamp: Option<mir2_shared::data::item::UserItem>,
    ) {
        let mut lost_gold = 0u64;
        let mut lost_items: Vec<mir2_shared::data::item::UserItem> = Vec::new();

        // 金币用 TryAddGold（原子：会截顶则整体失败、不加不减），失败转系统邮件
        if gold > 0 {
            let refunded = record
                .actor_ref
                .ask(crate::actors::player::TryAddGold { amount: gold })
                .await
                .unwrap_or(false);
            if !refunded {
                error!(
                    "SendMail 回滚退金失败（转系统邮件归还）: session={} player={} gold={}",
                    session_id, sender_name, gold
                );
                lost_gold = gold;
            }
        }
        // 物品与邮票：AddItemToInventory.Reply=Option<u64>（Some=实际入包 uid，None=失败）
        for item in items.into_iter().chain(stamp) {
            let added = record
                .actor_ref
                .ask(AddItemToInventory { item: item.clone() })
                .await
                .ok()
                .flatten()
                .is_some();
            if !added {
                error!(
                    "SendMail 回滚退物失败（转系统邮件归还）: session={} player={} uid={} item_index={} count={}",
                    session_id, sender_name, item.unique_id, item.item_index, item.count
                );
                lost_items.push(item);
            }
        }

        if lost_gold > 0 || !lost_items.is_empty() {
            for mail in build_send_mail_return_mails(sender_name, lost_gold, lost_items) {
                self.deliver_system_mail_critical(mail).await;
            }
        }
    }
}

impl Message<SendMailRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: SendMailRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let sender_state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // #2044：C# SendMail（11674-11683）——10s 发信冷却（NextMailTime，防刷信）
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        if let Some(last) = self.last_mail_time.get(&msg.session_id).copied() {
            if now_ms - last < 10_000 {
                send_mail_sent_result(&self.gate_ref, msg.session_id, -1);
                send_system_message(
                    &self.gate_ref,
                    msg.session_id,
                    "发送邮件过于频繁，请稍后再试",
                );
                return;
            }
        }

        if msg.receiver_name == sender_state.name {
            send_mail_sent_result(&self.gate_ref, msg.session_id, -1);
            send_system_message(&self.gate_ref, msg.session_id, "不能给自己发送邮件");
            return;
        }

        // #2008：C# PlayerObject.SendMail（11737-11749）——消息>500 拒绝；收件人不存在拒绝
        if msg.body.len() > 500 {
            send_mail_sent_result(&self.gate_ref, msg.session_id, -1);
            send_system_message(
                &self.gate_ref,
                msg.session_id,
                "邮件内容过长（最多 500 字）",
            );
            return;
        }
        if !db::character_exists_by_name(&self.db_pool, &msg.receiver_name)
            .await
            .unwrap_or(false)
        {
            send_mail_sent_result(&self.gate_ref, msg.session_id, -1);
            send_system_message(&self.gate_ref, msg.session_id, "找不到该玩家");
            return;
        }

        // #2044：C# RecipientsMailboxFull——收件箱 >50 拒绝（在线用 state，离线查 DB）
        let mut recipient_mail_count = 0usize;
        let mut recipient_online = false;
        for r in self.players.values() {
            if let Ok(Some(st)) = r.actor_ref.ask(GetPlayerState).await {
                if st.name == msg.receiver_name {
                    recipient_mail_count = st.mailbox.inbox.len();
                    recipient_online = true;
                    break;
                }
            }
        }
        if !recipient_online {
            recipient_mail_count = db::load_mail(&self.db_pool, &msg.receiver_name)
                .await
                .map(|m| m.inbox.len())
                .unwrap_or(0);
        }
        if recipient_mail_count > 50 {
            send_mail_sent_result(&self.gate_ref, msg.session_id, -1);
            send_system_message(&self.gate_ref, msg.session_id, "对方邮箱已满");
            return;
        }
        // #2044：C# CannotMailPlayerOnBlacklist——发送者拉黑收件人
        if sender_state.friend_list.is_blocked_name(&msg.receiver_name) {
            send_mail_sent_result(&self.gate_ref, msg.session_id, -1);
            send_system_message(
                &self.gate_ref,
                msg.session_id,
                "你已将该玩家加入黑名单，无法发送",
            );
            return;
        }
        // #2044：C# PlayerNotAcceptingMail——收件人拉黑发送者
        let receiver_blocked = db::load_friends(&self.db_pool, &msg.receiver_name)
            .await
            .map(|fl| fl.is_blocked(sender_state.object_id))
            .unwrap_or(false);
        if receiver_blocked {
            send_mail_sent_result(&self.gate_ref, msg.session_id, -1);
            send_system_message(&self.gate_ref, msg.session_id, "对方不接受你的邮件");
            return;
        }

        // #2008：C# CannotBeMailed——DontTrade(0x10)/NoMail(0x4000) 绑定物品不可寄送（11800-11817）
        for uid in &msg.item_uids {
            let bind = sender_state
                .inventory
                .get_item(*uid)
                .and_then(|it| self.item_infos.get(&it.item_index).map(|i| i.bind_mode))
                .unwrap_or(0);
            let rental_dont_trade = sender_state
                .inventory
                .get_item(*uid)
                .map(|it| {
                    super::rental_has_flag(it, mir2_shared::enums::BindMode::DONT_TRADE.bits())
                })
                .unwrap_or(false);
            if super::has_bind_flag(bind, 16)
                || super::has_bind_flag(bind, 16384)
                || rental_dont_trade
            {
                send_mail_sent_result(&self.gate_ref, msg.session_id, -1);
                send_system_message(&self.gate_ref, msg.session_id, "该物品无法邮寄");
                return;
            }
        }

        // C# PlayerObject.GetMailCost：金币费用 floor(gold/1000)*CostPer1K + 物品保险 floor(price/100*Insurance)
        let (
            mail_cost_per_1k,
            mail_insurance_pct,
            mail_free_with_stamp,
            _mail_capacity,
            auto_send_gold,
            auto_send_items,
        ) = self
            .social_ref
            .ask(crate::actors::social::NpcGetMailSettings)
            .await
            .unwrap_or((100, 5, true, 100, false, false));
        // #2538：C# PlayerObject.SendMail（11756-11786）——先定位邮票（暂不消耗）；
        // C# 顺序：先验金（Account.Gold < totalGold → 拒绝），验金通过后才消耗邮票
        let stamp_uid: Option<u64> = if msg.stamped {
            sender_state
                .inventory
                .backpack
                .iter()
                .flatten()
                .find(|s| {
                    self.item_infos
                        .get(&s.item.item_index)
                        .is_some_and(is_stamp_item)
                })
                .map(|s| s.item.unique_id)
        } else {
            None
        };
        let has_stamp = stamp_uid.is_some();
        // #2538：C# hasStamp ? 5 : 1——未贴票仅寄第 1 格附件
        let item_uids: Vec<u64> = if has_stamp {
            msg.item_uids.clone()
        } else {
            msg.item_uids.iter().take(1).copied().collect()
        };
        // #2538：C# GetMailCost——免费条件 MailFreeWithStamp && stamped（原实现恒免费）
        let mail_cost: u64 = compute_mail_cost(
            &sender_state,
            &self.item_infos,
            &item_uids,
            msg.gold,
            msg.stamped,
            mail_cost_per_1k,
            mail_insurance_pct,
            mail_free_with_stamp,
        );

        // 检查金币是否足够（附件金币 + 寄送费用）——必须先于任何消耗（C# :11762-11768）
        let total_gold = msg.gold as u64;
        if sender_state.inventory.gold < total_gold + mail_cost {
            send_mail_sent_result(&self.gate_ref, msg.session_id, -1);
            send_system_message(&self.gate_ref, msg.session_id, "金币不足（含寄送费用）");
            return;
        }

        // 验金通过：消耗一张邮票（保留被移除部分，投递失败时回滚）。
        // 贴票请求但邮票在快照后并发消失/不可用（移除返回 None）时必须中止发信
        // （与扣物失败同语义：MailSent=-1 并提示）——此刻金币/物品均未扣，无需回滚；
        // 继续发信会让贴票邮件白享免费/5 格附件待遇。
        let removed_stamp = if let Some(uid) = stamp_uid {
            match record
                .actor_ref
                .ask(crate::actors::player::RemoveItemFromInventoryCount {
                    unique_id: uid,
                    count: 1,
                })
                .await
            {
                Ok(Some(stamp)) => {
                    send_system_message(&self.gate_ref, msg.session_id, "消耗一张邮票");
                    Some(stamp)
                }
                other => {
                    if let Err(e) = &other {
                        warn!(
                            "SendMail stamp removal ask failed: session={} uid={}: {}",
                            msg.session_id, uid, e
                        );
                    }
                    send_mail_sent_result(&self.gate_ref, msg.session_id, -1);
                    send_system_message(
                        &self.gate_ref,
                        msg.session_id,
                        "邮票状态已变化，邮件未发送",
                    );
                    return;
                }
            }
        } else {
            None
        };

        // 扣除金币（附件 + 寄送费用）：检查 DeductGold 结果——验金后余额可能被并发扣减，
        // 扣金失败立即中止并退回邮票（原实现 let _ 忽略失败 → 邮资白扣）
        if total_gold + mail_cost > 0 {
            let deducted = record
                .actor_ref
                .ask(DeductGold {
                    amount: total_gold + mail_cost,
                })
                .await
                .unwrap_or(false);
            if !deducted {
                self.rollback_send_mail_escrow(
                    msg.session_id,
                    record,
                    &sender_state.name,
                    0,
                    Vec::new(),
                    removed_stamp,
                )
                .await;
                send_mail_sent_result(&self.gate_ref, msg.session_id, -1);
                send_system_message(&self.gate_ref, msg.session_id, "金币不足（含寄送费用）");
                return;
            }
        }

        // 逐项扣物，邮件附件【只装实际扣除成功的物品】（原实现用旧快照装附件 + let _ 忽略
        // 扣物失败：快照后物品被并发托管/拆分/当邮票消耗时 → 幻影附件复制，投递失败回滚
        // 又按 mail.items 盲目返还 → 双向复制）。任一扣物失败立即中止，
        // 并回滚已扣物品/金币/邮票。
        let mut items: Vec<mir2_shared::data::item::UserItem> = Vec::new();
        for uid in &item_uids {
            match record
                .actor_ref
                .ask(RemoveItemFromInventory { unique_id: *uid })
                .await
            {
                Ok(Some(item)) => items.push(item),
                _ => {
                    self.rollback_send_mail_escrow(
                        msg.session_id,
                        record,
                        &sender_state.name,
                        total_gold + mail_cost,
                        std::mem::take(&mut items),
                        removed_stamp,
                    )
                    .await;
                    send_mail_sent_result(&self.gate_ref, msg.session_id, -1);
                    send_system_message(
                        &self.gate_ref,
                        msg.session_id,
                        "物品状态已变化，邮件未发送，款项与附件已退回",
                    );
                    return;
                }
            }
        }

        // C# MailInfo.Send()：发信初始 Collected 计算（无附件恒 true，包裹按 MailAutoSendGold/Items）
        let collected = crate::actors::mail::initial_collected(
            total_gold > 0,
            !items.is_empty(),
            auto_send_gold,
            auto_send_items,
        );

        // 创建邮件
        let mut mail = MailMessage {
            mail_id: generate_mail_id(),
            sender_name: sender_state.name.clone(),
            receiver_name: msg.receiver_name.clone(),
            subject: msg.subject.clone(),
            body: msg.body.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0),
            read: false,
            collected,
            locked: false,
            gold: total_gold,
            items,
        };

        // 查找收件人
        let mut target_session: Option<u64> = None;
        for (sid, r) in &self.players {
            if let Ok(Some(s)) = r.actor_ref.ask(GetPlayerState).await {
                if s.name == msg.receiver_name {
                    target_session = Some(*sid);
                    break;
                }
            }
        }

        // 投递：在线 AddMail ask 失败回退离线落库；落库也失败则回滚已扣款/物/邮票（防邮件蒸发）
        let mut delivered = false;
        if let Some(target) = target_session {
            if let Some(target_record) = self.players.get(&target) {
                match target_record
                    .actor_ref
                    .ask(crate::actors::player::AddMail { mail: mail.clone() })
                    .await
                {
                    Ok(_) => {
                        send_mail_received_packet(&self.gate_ref, target, &mail);
                        // C# PlayerObject.Process（:499-504）：收到新邮件 → 系统消息提示
                        send_system_message(&self.gate_ref, target, "你收到了一封新邮件");
                        debug!(
                            "Mail delivered online: {} -> {}",
                            sender_state.name, msg.receiver_name
                        );
                        delivered = true;
                    }
                    Err(e) => {
                        warn!(
                            "Online mail delivery failed ({} -> {}): {}; falling back to offline save",
                            sender_state.name, msg.receiver_name, e
                        );
                    }
                }
            }
        }
        if !delivered {
            // 收件人不在线或在线投递失败，保存到数据库
            match db::insert_mail(&self.db_pool, &msg.receiver_name, &mail).await {
                Ok(_) => {
                    debug!(
                        "Mail saved offline: {} -> {}",
                        sender_state.name, msg.receiver_name
                    );
                    delivered = true;
                }
                Err(e) => {
                    warn!(
                        "Failed to save offline mail for {}: {}",
                        msg.receiver_name, e
                    );
                }
            }
        }
        if !delivered {
            // 回滚：退回金币（附件 + 邮资）、附件物品、邮票
            // （统一出口：归还失败 error! 审计 + 系统邮件兜底，防蒸发）
            self.rollback_send_mail_escrow(
                msg.session_id,
                record,
                &sender_state.name,
                total_gold + mail_cost,
                std::mem::take(&mut mail.items),
                removed_stamp,
            )
            .await;
            send_mail_sent_result(&self.gate_ref, msg.session_id, -1);
            send_system_message(
                &self.gate_ref,
                msg.session_id,
                "邮件发送失败，款项与附件已退回",
            );
            return;
        }

        self.last_mail_time.insert(msg.session_id, now_ms);
        // C# SendMail（:11844）：成功发 S.MailSent { Result = 1 }
        send_mail_sent_result(&self.gate_ref, msg.session_id, 1);
        send_system_message(&self.gate_ref, msg.session_id, "邮件已发送");
    }
}

impl Message<ReadMailRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: ReadMailRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        let mail = match record
            .actor_ref
            .ask(crate::actors::player::GetMail {
                mail_id: msg.mail_id,
            })
            .await
        {
            Ok(Some(m)) => m,
            _ => return,
        };

        send_mail_content_packet(&self.gate_ref, msg.session_id, &mail);
        let _ = record
            .actor_ref
            .ask(crate::actors::player::MarkMailRead {
                mail_id: msg.mail_id,
            })
            .await;
    }
}

impl Message<CollectParcelRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: CollectParcelRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        // C# CollectMail（11868-11877）：先 CanGainItems 预检，背包满仅聊天提示、附件保留；
        // 金币同样预检 CanGainGold（C# :11882-11886 截顶前先算差额），防金币封顶差额蒸发。
        // 注意：预检只是提前劝退——预检与实际入包/入金之间存在并发窗口（TOCTOU），
        // 真正的正确性靠下方「失败写回邮件附件 + 写回失败系统归还邮件」兜底。
        // receiver_name 一并取出：写回失败兜底建系统归还邮件时用（邮箱属主即收件人）。
        let (mail_gold, mail_items, receiver_name) = match record
            .actor_ref
            .ask(crate::actors::player::GetMail {
                mail_id: msg.mail_id,
            })
            .await
        {
            Ok(Some(m)) => (m.gold, m.items, m.receiver_name),
            _ => {
                send_parcel_collected_result(&self.gate_ref, msg.session_id, -1);
                return;
            }
        };

        if !mail_items.is_empty() {
            let can_gain = record
                .actor_ref
                .ask(crate::actors::player::CanGainItemsFor {
                    items: mail_items.clone(),
                })
                .await
                .unwrap_or(false);
            if !can_gain {
                send_system_message(&self.gate_ref, msg.session_id, "背包已满，无法收取附件");
                return; // C#：背包满时不发 S.ParcelCollected
            }
        }
        if mail_gold > 0 {
            // C# CanGainGold：amount + 现有金币 > uint.MaxValue 即不可收
            let can_gain_gold = record
                .actor_ref
                .ask(crate::actors::player::CanGainGold {
                    amount: mail_gold.min(u32::MAX as u64) as u32,
                })
                .await
                .unwrap_or(false);
            if !can_gain_gold {
                send_system_message(&self.gate_ref, msg.session_id, "金币已达上限，无法收取附件");
                return; // 与背包满一致：不发 S.ParcelCollected，附件保留
            }
        }

        let result = match record
            .actor_ref
            .ask(crate::actors::player::CollectMailAttachment {
                mail_id: msg.mail_id,
            })
            .await
        {
            Ok(Some(r)) => r,
            _ => {
                // C# S.ParcelCollected.Result=-1：无可收取包裹
                send_parcel_collected_result(&self.gate_ref, msg.session_id, -1);
                return;
            }
        };

        // collect_attachment 已清空邮件附件（mem::take + gold 置 0）：此后任何入包/入金
        // 失败都必须把未收部分【写回邮件附件】，否则附件永久丢失/金币蒸发。
        let (gold, items) = result;

        // 逐项入包并检查结果（AddItemToInventory.Reply = Option<u64>：Some=实际入包 uid，
        // None=入包失败——预检后背包可能已被并发填满）
        let mut failed_items: Vec<mir2_shared::data::item::UserItem> = Vec::new();
        for item in items {
            let added = matches!(
                record
                    .actor_ref
                    .ask(AddItemToInventory { item: item.clone() })
                    .await,
                Ok(Some(_))
            );
            if !added {
                failed_items.push(item);
            }
        }
        // 金币用 TryAddGold（原子：会截顶则整体失败、不加不减），不用截顶语义的 AddGold
        let failed_gold = if gold == 0 {
            0
        } else {
            let ok = record
                .actor_ref
                .ask(crate::actors::player::TryAddGold { amount: gold })
                .await
                .unwrap_or(false);
            if ok { 0 } else { gold }
        };

        if !failed_items.is_empty() || failed_gold > 0 {
            let failed_item_count = failed_items.len();
            // 写回邮件附件：专用消息 RestoreMailAttachment 只改 mailbox 内对应邮件的
            // 附件字段——不再用 GetPlayerState→改副本→SetPlayerState 整体状态 RMW
            // （并发窗口内交易投递/HP 变化/死亡等无关状态改动会被旧副本覆盖丢失）。
            let restored = record
                .actor_ref
                .ask(crate::actors::player::RestoreMailAttachment {
                    mail_id: msg.mail_id,
                    gold: failed_gold,
                    items: failed_items.clone(),
                })
                .await
                .unwrap_or(false);
            if !restored {
                // 写回失败（邮件在收取途中已不存在/玩家 actor 异常）：
                // 附件只 error! = 蒸发——必须兜底新建系统归还邮件
                // （deliver_collect_restore_fallback：在线进内存邮箱、失败落库、
                // 再失败 error! + 重试一次，market.rs deliver_system_mail_critical 同款语义）
                error!(
                    "CollectParcel restore failed (session={} mail_id={} gone or actor error): {} gold / {} items — 转系统归还邮件兜底",
                    msg.session_id, msg.mail_id, failed_gold, failed_item_count
                );
                deliver_collect_restore_fallback(
                    &record.actor_ref,
                    &self.gate_ref,
                    &self.db_pool,
                    msg.session_id,
                    &receiver_name,
                    failed_gold,
                    failed_items,
                )
                .await;
                send_system_message(
                    &self.gate_ref,
                    msg.session_id,
                    "收取失败，附件已通过系统邮件归还",
                );
            } else {
                send_system_message(
                    &self.gate_ref,
                    msg.session_id,
                    "背包空间不足，附件已保留在邮件中",
                );
            }
            send_parcel_collected_result(&self.gate_ref, msg.session_id, -1);
            return;
        }

        // C# S.ParcelCollected.Result=1：收取成功
        send_parcel_collected_result(&self.gate_ref, msg.session_id, 1);
        debug!(
            "CollectParcel: session={} mail_id={} gold={}",
            msg.session_id, msg.mail_id, gold
        );
    }
}

impl Message<DeleteMailRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: DeleteMailRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        // 严重15-2：删除前检查——带未收取附件的邮件拒绝删除（防丢件；
        // C# 客户端 MailDialogs.cs:239-248 删除带附件邮件需玩家确认，Locked 直接拒绝），
        // Mailbox::delete_mail 内同样兜底
        match record
            .actor_ref
            .ask(crate::actors::player::GetMail {
                mail_id: msg.mail_id,
            })
            .await
        {
            Ok(Some(mail)) => {
                if mail.has_uncollected_parcel() {
                    send_system_message(
                        &self.gate_ref,
                        msg.session_id,
                        "该邮件含有未收取的附件，请先收取后再删除",
                    );
                    return;
                }
                if mail.locked {
                    send_system_message(&self.gate_ref, msg.session_id, "邮件已锁定，无法删除");
                    return;
                }
            }
            _ => return,
        }

        let deleted = match record
            .actor_ref
            .ask(crate::actors::player::DeleteMail {
                mail_id: msg.mail_id,
            })
            .await
        {
            Ok(d) => d,
            _ => return,
        };

        if deleted {
            send_system_message(&self.gate_ref, msg.session_id, "邮件已删除");
        }
    }
}

#[cfg(test)]
mod cost_tests {
    use super::*;

    /// #2538：金币费 floor(gold/1000)*Per1K（C# GetMailCost 11932-11935）
    #[test]
    fn mail_cost_gold_fee_floors_per_1k() {
        // 1500 金 × 每 1K 2 → floor(1500/1000)*2 = 2
        assert_eq!(mail_cost_from_prices(&[], 1500, 2, 5), 2);
        // 999 金 → 0；2000 金 × 3 → 6
        assert_eq!(mail_cost_from_prices(&[], 999, 3, 5), 0);
        assert_eq!(mail_cost_from_prices(&[], 2000, 3, 5), 6);
    }

    /// #2538：附件保险 floor(price/100)*Pct（C# GetMailCost 11937-11953）
    #[test]
    fn mail_cost_item_insurance_per_piece() {
        // 价格 10000 保险 5% → 500
        assert_eq!(mail_cost_from_prices(&[10000], 0, 0, 5), 500);
        // 两件 10000 + 250 → 100*5 + 2*5 = 510
        assert_eq!(mail_cost_from_prices(&[10000, 250], 0, 0, 5), 510);
    }

    /// SendMail 回滚兜底邮件：金币挂首封、物品按 5 件/封分封（social 交易归还同款）
    #[test]
    fn send_mail_return_mails_chunk_items_and_gold_first() {
        let items: Vec<mir2_shared::data::item::UserItem> = (0..6u64)
            .map(|u| mir2_shared::data::item::UserItem {
                unique_id: 100 + u,
                ..Default::default()
            })
            .collect();
        let mails = build_send_mail_return_mails("p1", 500, items);
        assert_eq!(mails.len(), 2, "6 件物品必须分 2 封（5+1）");
        assert_eq!(mails[0].gold, 500, "金币挂首封");
        assert_eq!(mails[1].gold, 0);
        assert_eq!(mails[0].items.len(), 5);
        assert_eq!(mails[1].items.len(), 1);
        assert_eq!(mails[0].receiver_name, "p1");
        assert!(mails.iter().all(|m| !m.collected && !m.read && !m.locked));
        // 纯金币：1 封空物品
        let mails = build_send_mail_return_mails("p1", 7, vec![]);
        assert_eq!(mails.len(), 1);
        assert_eq!(mails[0].gold, 7);
        assert!(mails[0].items.is_empty());
        // 空：无邮件
        assert!(build_send_mail_return_mails("p1", 0, vec![]).is_empty());
    }

    /// CollectParcel 写回失败兜底邮件：金币挂首封、物品按 5 件/封分封
    /// （与 send_mail_return_mails 同款分封语义；收取失败场景专用 subject/body）
    #[test]
    fn collect_return_mails_chunk_items_and_gold_first() {
        let items: Vec<mir2_shared::data::item::UserItem> = (0..6u64)
            .map(|u| mir2_shared::data::item::UserItem {
                unique_id: 200 + u,
                ..Default::default()
            })
            .collect();
        let mails = build_collect_return_mails("p2", 900, items);
        assert_eq!(mails.len(), 2, "6 件物品必须分 2 封（5+1）");
        assert_eq!(mails[0].gold, 900, "金币挂首封");
        assert_eq!(mails[1].gold, 0);
        assert_eq!(mails[0].items.len(), 5);
        assert_eq!(mails[1].items.len(), 1);
        assert_eq!(mails[0].receiver_name, "p2");
        assert_eq!(mails[0].subject, "收取失败附件归还");
        assert!(mails.iter().all(|m| !m.collected && !m.read && !m.locked));
        // 纯金币：1 封空物品
        let mails = build_collect_return_mails("p2", 3, vec![]);
        assert_eq!(mails.len(), 1);
        assert_eq!(mails[0].gold, 3);
        assert!(mails[0].items.is_empty());
        // 空：无邮件
        assert!(build_collect_return_mails("p2", 0, vec![]).is_empty());
    }
}

#[cfg(test)]
mod mail_flow_e2e {
    //! 邮件收取/发送并发安全的端到端回归（真 gate/world/account 协议链路，
    //! 线格式与 harness 同 src/actors/world/e2e.rs）。
    use std::time::Duration;
    use tokio::sync::mpsc;

    use kameo::actor::Spawn;

    use crate::actors::account::AccountActor;
    use crate::actors::mail::MailMessage;
    use crate::actors::player::{GetPlayerState, Heal, PlayerActor, SetPlayerState};
    use crate::actors::social::{SocialActor, SocialActorArgs, SocialActorConfig};
    use super::{deliver_collect_restore_fallback, GetPlayerItemUid};
    use crate::actors::world::{WorldActor, WorldActorArgs};
    use crate::db;
    use crate::gate::actor::{ClientData, GateActor, SessionCreated, SetAccountRef, SetWorldRef};
    use crate::util::wire::build_packet_bytes;

    type GateActorRef = kameo::actor::ActorRef<GateActor>;
    type RxChannel = tokio::sync::mpsc::Receiver<Vec<u8>>;

    const S_CHAT: i16 = mir2_shared::enums::ServerPacketIds::Chat as i16;
    const S_MAIL_SENT: i16 = mir2_shared::enums::ServerPacketIds::MailSent as i16;
    const S_PARCEL_COLLECTED: i16 = mir2_shared::enums::ServerPacketIds::ParcelCollected as i16;

    async fn session_created(gate_ref: &GateActorRef, session_id: u64) -> RxChannel {
        let (tx, rx) = mpsc::channel::<Vec<u8>>(1024);
        let _ = gate_ref
            .ask(SessionCreated {
                session_id,
                sender: tx,
                ip: "127.0.0.1".to_string(),
            })
            .await;
        rx
    }

    /// 收集 secs 内到达的所有包（opcode, body）
    async fn collect_packets(rx: &mut RxChannel, secs: u64) -> Vec<(i16, Vec<u8>)> {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(secs);
        let mut out = Vec::new();
        loop {
            let now = tokio::time::Instant::now();
            if now >= deadline {
                break;
            }
            match tokio::time::timeout(deadline - now, rx.recv()).await {
                Ok(Some(data)) if data.len() >= 4 => {
                    out.push((i16::from_le_bytes([data[2], data[3]]), data[4..].to_vec()));
                }
                Ok(Some(_)) => continue,
                _ => break,
            }
        }
        out
    }

    /// 等到目标 opcode 为止；返回（目标 body, 期间全部包）。超时返回 None
    async fn recv_until(
        rx: &mut RxChannel,
        opcode: i16,
        secs: u64,
    ) -> Option<(Vec<u8>, Vec<(i16, Vec<u8>)>)> {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(secs);
        let mut seen = Vec::new();
        while tokio::time::Instant::now() < deadline {
            let remaining = deadline - tokio::time::Instant::now();
            match tokio::time::timeout(remaining, rx.recv()).await {
                Ok(Some(data)) if data.len() >= 4 => {
                    let op = i16::from_le_bytes([data[2], data[3]]);
                    let body = data[4..].to_vec();
                    if op == opcode {
                        seen.push((op, body.clone()));
                        return Some((body, seen));
                    }
                    seen.push((op, body));
                }
                Ok(Some(_)) => continue,
                _ => return None,
            }
        }
        None
    }

    fn chats_contain(pkts: &[(i16, Vec<u8>)], needle: &str) -> bool {
        pkts.iter().any(|(op, body)| {
            *op == S_CHAT && body.windows(needle.len()).any(|w| w == needle.as_bytes())
        })
    }

    async fn login_and_new_char(
        gate_ref: &GateActorRef,
        session_id: u64,
        rx: &mut RxChannel,
        username: &str,
        char_name: &str,
    ) {
        // ClientVersion
        let cv_body = {
            let mut b = Vec::new();
            let hash = b"test";
            b.extend_from_slice(&(hash.len() as i32).to_le_bytes());
            b.extend_from_slice(hash);
            b
        };
        let _ = gate_ref
            .ask(ClientData {
                session_id,
                data: build_packet_bytes(
                    mir2_shared::enums::ClientPacketIds::ClientVersion as i16,
                    &cv_body,
                ),
            })
            .await;
        // Login（账号不存在自动创建，同 e2e.rs 流程）
        let mut login_body = Vec::new();
        let _ = mir2_shared::binary::write_dotnet_string(&mut login_body, username);
        let _ = mir2_shared::binary::write_dotnet_string(&mut login_body, "testpass");
        let _ = gate_ref
            .ask(ClientData {
                session_id,
                data: build_packet_bytes(
                    mir2_shared::enums::ClientPacketIds::Login as i16,
                    &login_body,
                ),
            })
            .await;
        assert!(
            recv_until(
                rx,
                mir2_shared::enums::ServerPacketIds::LoginSuccess as i16,
                3
            )
            .await
            .is_some(),
            "LoginSuccess"
        );
        // NewCharacter
        let mut nc_body = Vec::new();
        let _ = mir2_shared::binary::write_dotnet_string(&mut nc_body, char_name);
        nc_body.push(0u8);
        nc_body.push(0u8);
        let _ = gate_ref
            .ask(ClientData {
                session_id,
                data: build_packet_bytes(
                    mir2_shared::enums::ClientPacketIds::NewCharacter as i16,
                    &nc_body,
                ),
            })
            .await;
        assert!(
            recv_until(
                rx,
                mir2_shared::enums::ServerPacketIds::NewCharacterSuccess as i16,
                3
            )
            .await
            .is_some(),
            "NewCharacterSuccess"
        );
    }

    async fn start_game(gate_ref: &GateActorRef, session_id: u64, rx: &mut RxChannel) {
        let _ = gate_ref
            .ask(ClientData {
                session_id,
                data: build_packet_bytes(
                    mir2_shared::enums::ClientPacketIds::StartGame as i16,
                    &0i32.to_le_bytes().to_vec(),
                ),
            })
            .await;
        assert!(
            recv_until(rx, mir2_shared::enums::ServerPacketIds::StartGame as i16, 5)
                .await
                .is_some(),
            "StartGame"
        );
    }

    async fn spawn_world(
        gate_ref: &GateActorRef,
        db_pool: &db::DbPool,
    ) -> kameo::actor::ActorRef<WorldActor> {
        let social_ref = SocialActor::spawn(SocialActorArgs {
            gate_ref: gate_ref.clone(),
            db_pool: db_pool.clone(),
            config: SocialActorConfig::default(),
        });
        let world_ref = WorldActor::spawn(WorldActorArgs {
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
        let _ = gate_ref.ask(SetWorldRef {
            world_ref: world_ref.clone(),
        })
        .await;
        world_ref
    }

    fn user_item(uid: u64, item_index: i32) -> mir2_shared::data::item::UserItem {
        mir2_shared::data::item::UserItem {
            unique_id: uid,
            item_index,
            count: 1,
            current_dura: 1000,
            max_dura: 1000,
            ..Default::default()
        }
    }

    fn send_mail_packet(
        receiver: &str,
        message: &str,
        gold: u32,
        uids: [u64; 5],
        stamped: bool,
    ) -> Vec<u8> {
        let mut body = Vec::new();
        let _ = mir2_shared::binary::write_dotnet_string(&mut body, receiver);
        let _ = mir2_shared::binary::write_dotnet_string(&mut body, message);
        body.extend_from_slice(&gold.to_le_bytes());
        for u in uids {
            body.extend_from_slice(&u.to_le_bytes());
        }
        body.push(stamped as u8);
        build_packet_bytes(mir2_shared::enums::ClientPacketIds::SendMail as i16, &body)
    }

    /// 收取并发安全②（回归）：扣物失败必须中止并回滚已扣部分。
    /// 攻击面：把【邮票 uid 本身】也放进附件列表——邮票先被消耗，随后扣物循环
    /// 再扣该 uid 必然失败。旧实现 let _ 忽略失败 → 邮件按旧快照携带
    /// [剑, 邮票] 幻影附件照发（复制）；新实现必须 MailSent=-1 且已扣的剑与邮票退回。
    ///
    /// 红检：回退到旧扣物循环（快照装附件 + 忽略失败）→ 第一次 MailSent=1，本用例失败。
    #[test]
    fn e2e_send_mail_item_removal_failure_aborts_and_rolls_back() {
        const STAMP_UID: u64 = 8001;
        const SWORD_UID: u64 = 8002;
        const SENDER: &str = "MailSenderA";
        const RECEIVER: &str = "MailRecvA";

        let rt = tokio::runtime::Builder::new_multi_thread()
            .thread_stack_size(8 * 1024 * 1024)
            .enable_time()
            .build()
            .unwrap();
        rt.block_on(async {
            let gate_ref = GateActor::spawn(());
            let db_pool = db::init_db_pool("sqlite::memory:").await.expect("init_db");
            let account_ref = AccountActor::spawn((gate_ref.clone(), db_pool.clone()));
            let _ = gate_ref.ask(SetAccountRef { account_ref }).await;

            // 物品表：邮票（type=0 && shape=1，C# ItemType.Nothing && Shape==1）+ 武器
            for (idx, name, item_type, shape) in [(900, "TestStamp", 0, 1), (901, "TestSword", 1, 0)]
            {
                sqlx::query("INSERT INTO item_infos (idx, name, type, shape) VALUES (?, ?, ?, ?)")
                    .bind(idx)
                    .bind(name)
                    .bind(item_type)
                    .bind(shape)
                    .execute(&db_pool)
                    .await
                    .expect("insert item_infos");
            }

            let world_ref = spawn_world(&gate_ref, &db_pool).await;

            // 发送者：邮票 + 剑各一
            let s1 = 61u64;
            let mut rx1 = session_created(&gate_ref, s1).await;
            login_and_new_char(&gate_ref, s1, &mut rx1, "mailsendera", SENDER).await;
            for (grid, item) in [
                (0i32, user_item(STAMP_UID, 900)),
                (1i32, user_item(SWORD_UID, 901)),
            ] {
                sqlx::query(
                    "INSERT INTO inventory_backpack (character_name, grid, item_json) VALUES (?, ?, ?)",
                )
                .bind(SENDER)
                .bind(grid)
                .bind(serde_json::to_string(&item).unwrap())
                .execute(&db_pool)
                .await
                .expect("seed inventory");
            }
            start_game(&gate_ref, s1, &mut rx1).await;

            // 接收者
            let s2 = 62u64;
            let mut rx2 = session_created(&gate_ref, s2).await;
            login_and_new_char(&gate_ref, s2, &mut rx2, "mailrecva", RECEIVER).await;
            start_game(&gate_ref, s2, &mut rx2).await;

            // 第一次发信：附件 = [剑, 邮票自身]——邮票先被消耗，扣剑成功、扣邮票失败
            let _ = collect_packets(&mut rx1, 1).await;
            let _ = gate_ref
                .ask(ClientData {
                    session_id: s1,
                    data: send_mail_packet(
                        RECEIVER,
                        "collision",
                        0,
                        [SWORD_UID, STAMP_UID, 0, 0, 0],
                        true,
                    ),
                })
                .await;
            let (body, _) = recv_until(&mut rx1, S_MAIL_SENT, 3)
                .await
                .expect("MailSent #1 missing");
            assert_eq!(
                body,
                vec![(-1i8) as u8],
                "扣物失败必须中止发信（MailSent=-1；旧实现幻影附件照发=1）"
            );

            // 第二次发信：仅附件剑。剑若已退回 → 发送成功；邮票若已退回 → 找得到邮票、
            // 出现「消耗一张邮票」聊天。任一未退回都会暴露（剑丢→MailSent=-1；邮票丢→无聊天）。
            // 注：回滚归还经 AddItemToInventory 会重新生成 uid（防复制语义），
            // 必须按当前真实 uid 重新发起。
            let restored_sword_uid = world_ref
                .ask(GetPlayerItemUid {
                    session_id: s1,
                    item_index: 901,
                })
                .await
                .expect("GetPlayerItemUid ask")
                .expect("回滚后剑必须仍在背包（uid 重新生成）");
            let _ = gate_ref
                .ask(ClientData {
                    session_id: s1,
                    data: send_mail_packet(
                        RECEIVER,
                        "second",
                        0,
                        [restored_sword_uid, 0, 0, 0, 0],
                        true,
                    ),
                })
                .await;
            let (body2, seen2) = recv_until(&mut rx1, S_MAIL_SENT, 3)
                .await
                .expect("MailSent #2 missing");
            assert_eq!(body2, vec![1i8 as u8], "回滚后剑必须可再次寄出（MailSent=1）");
            assert!(
                chats_contain(&seen2, "消耗一张邮票"),
                "回滚后邮票必须仍在背包（第二次贴票寄信应消耗邮票）"
            );
        });
    }

    /// 收取并发安全①（回归）：金币封顶预检——收件人金币 uint.MaxValue 时收取
    /// 带金邮件必须拒绝且附件保留（旧实现 AddGold 截顶 + 邮件 gold 已清零 → 差额蒸发）。
    ///
    /// 红检：去掉 CanGainGold 预检 → ParcelCollected=1 到达，本用例失败。
    #[test]
    fn e2e_collect_parcel_gold_cap_precheck_keeps_attachment() {
        const MAIL_ID: u64 = 5001;
        const RECEIVER: &str = "MailRecvCap";

        let rt = tokio::runtime::Builder::new_multi_thread()
            .thread_stack_size(8 * 1024 * 1024)
            .enable_time()
            .build()
            .unwrap();
        rt.block_on(async {
            let gate_ref = GateActor::spawn(());
            let db_pool = db::init_db_pool("sqlite::memory:").await.expect("init_db");
            let account_ref = AccountActor::spawn((gate_ref.clone(), db_pool.clone()));
            let _ = gate_ref.ask(SetAccountRef { account_ref }).await;
            spawn_world(&gate_ref, &db_pool).await;

            let s1 = 71u64;
            let mut rx1 = session_created(&gate_ref, s1).await;
            login_and_new_char(&gate_ref, s1, &mut rx1, "mailrecvcap", RECEIVER).await;

            // 金币封顶 + 一封已从邮局取回（collected=1）的带金邮件
            sqlx::query("UPDATE characters SET gold = ? WHERE name = ?")
                .bind(u32::MAX as i64)
                .bind(RECEIVER)
                .execute(&db_pool)
                .await
                .expect("cap gold");
            db::insert_mail(
                &db_pool,
                RECEIVER,
                &crate::actors::mail::MailMessage {
                    mail_id: MAIL_ID,
                    sender_name: "sys".into(),
                    receiver_name: RECEIVER.into(),
                    subject: "gold".into(),
                    body: "gold".into(),
                    timestamp: 1,
                    read: false,
                    collected: true,
                    locked: false,
                    gold: 1000,
                    items: vec![],
                },
            )
            .await
            .expect("seed mail");
            start_game(&gate_ref, s1, &mut rx1).await;

            // 收取：必须被金币封顶预检拒绝——只聊天提示，不发 ParcelCollected，附件保留
            let _ = collect_packets(&mut rx1, 1).await;
            let _ = gate_ref
                .ask(ClientData {
                    session_id: s1,
                    data: build_packet_bytes(
                        mir2_shared::enums::ClientPacketIds::CollectParcel as i16,
                        &MAIL_ID.to_le_bytes().to_vec(),
                    ),
                })
                .await;
            let pkts = collect_packets(&mut rx1, 2).await;
            assert!(
                chats_contain(&pkts, "金币已达上限，无法收取附件"),
                "金币封顶必须聊天拒绝（实际包：{:?}）",
                pkts.iter().map(|(op, _)| op).collect::<Vec<_>>()
            );
            assert!(
                !pkts.iter().any(|(op, _)| *op == S_PARCEL_COLLECTED),
                "预检拒绝不得发 ParcelCollected（C# 背包满/金币满不发包）"
            );

            // 附件保留佐证：删邮件必须因「有未收附件」被拒（gold 若已蒸发则可删）
            let _ = gate_ref
                .ask(ClientData {
                    session_id: s1,
                    data: build_packet_bytes(
                        mir2_shared::enums::ClientPacketIds::DeleteMail as i16,
                        &MAIL_ID.to_le_bytes().to_vec(),
                    ),
                })
                .await;
            let pkts2 = collect_packets(&mut rx1, 2).await;
            assert!(
                chats_contain(&pkts2, "该邮件含有未收取的附件，请先收取后再删除"),
                "金币必须保留在邮件里（删除被拒）"
            );
        });
    }

    /// 收取并发安全①b（回归）：背包满预检——收件人背包全满时收取带物品附件的邮件
    /// 必须拒绝且附件保留（旧实现 AddItemToInventory let _ 忽略失败 → 附件永久丢失）。
    ///
    /// 红检：去掉 CanGainItemsFor 预检 → ParcelCollected 到达，本用例失败。
    #[test]
    fn e2e_collect_parcel_backpack_full_precheck_keeps_attachment() {
        const MAIL_ID: u64 = 5002;
        const RECEIVER: &str = "MailRecvFull";

        let rt = tokio::runtime::Builder::new_multi_thread()
            .thread_stack_size(8 * 1024 * 1024)
            .enable_time()
            .build()
            .unwrap();
        rt.block_on(async {
            let gate_ref = GateActor::spawn(());
            let db_pool = db::init_db_pool("sqlite::memory:").await.expect("init_db");
            let account_ref = AccountActor::spawn((gate_ref.clone(), db_pool.clone()));
            let _ = gate_ref.ask(SetAccountRef { account_ref }).await;
            spawn_world(&gate_ref, &db_pool).await;

            let s1 = 81u64;
            let mut rx1 = session_created(&gate_ref, s1).await;
            login_and_new_char(&gate_ref, s1, &mut rx1, "mailrecvfull", RECEIVER).await;

            // 填满全部 46 格背包：max_dura=1 → 堆叠上限 1（不可叠放），预检无空格可放
            for grid in 0..46i32 {
                let mut it = user_item(9000 + grid as u64, 902);
                it.max_dura = 1;
                it.current_dura = 1;
                sqlx::query(
                    "INSERT INTO inventory_backpack (character_name, grid, item_json) VALUES (?, ?, ?)",
                )
                .bind(RECEIVER)
                .bind(grid)
                .bind(serde_json::to_string(&it).unwrap())
                .execute(&db_pool)
                .await
                .expect("fill backpack");
            }
            // 一封已从邮局取回（collected=1）的带物品附件邮件（附件同样不可叠放）
            let mut loot = user_item(9999, 902);
            loot.max_dura = 1;
            loot.current_dura = 1;
            db::insert_mail(
                &db_pool,
                RECEIVER,
                &crate::actors::mail::MailMessage {
                    mail_id: MAIL_ID,
                    sender_name: "sys".into(),
                    receiver_name: RECEIVER.into(),
                    subject: "loot".into(),
                    body: "loot".into(),
                    timestamp: 1,
                    read: false,
                    collected: true,
                    locked: false,
                    gold: 0,
                    items: vec![loot],
                },
            )
            .await
            .expect("seed mail");
            start_game(&gate_ref, s1, &mut rx1).await;

            // 收取：必须被背包满预检拒绝——只聊天提示，不发 ParcelCollected，附件保留
            let _ = collect_packets(&mut rx1, 1).await;
            let _ = gate_ref
                .ask(ClientData {
                    session_id: s1,
                    data: build_packet_bytes(
                        mir2_shared::enums::ClientPacketIds::CollectParcel as i16,
                        &MAIL_ID.to_le_bytes().to_vec(),
                    ),
                })
                .await;
            let pkts = collect_packets(&mut rx1, 2).await;
            assert!(
                chats_contain(&pkts, "背包已满，无法收取附件"),
                "背包满必须聊天拒绝（实际包：{:?}）",
                pkts.iter().map(|(op, _)| op).collect::<Vec<_>>()
            );
            assert!(
                !pkts.iter().any(|(op, _)| *op == S_PARCEL_COLLECTED),
                "预检拒绝不得发 ParcelCollected（C# 背包满不发包）"
            );

            // 附件保留佐证：删邮件必须因「有未收附件」被拒（物品若已丢失则可删）
            let _ = gate_ref
                .ask(ClientData {
                    session_id: s1,
                    data: build_packet_bytes(
                        mir2_shared::enums::ClientPacketIds::DeleteMail as i16,
                        &MAIL_ID.to_le_bytes().to_vec(),
                    ),
                })
                .await;
            let pkts2 = collect_packets(&mut rx1, 2).await;
            assert!(
                chats_contain(&pkts2, "该邮件含有未收取的附件，请先收取后再删除"),
                "物品附件必须保留在邮件里（删除被拒）"
            );
        });
    }

    /// 收取回写并发安全③（回归）：RestoreMailAttachment 专用消息只改 mailbox 对应邮件的
    /// 附件字段，不得覆盖 HP 等无关状态。旧实现（GetPlayerState→改副本→SetPlayerState
    /// 整体状态 RMW）在「快照后、回写前」的并发窗口内把 HP 变化/交易投递/死亡等改动
    /// 整体覆盖丢失。
    ///
    /// 红检：阶段一即旧 RMW 模式的确定性复现（断言 HP 被回滚，钉住 bug 形态）；
    /// 将 world/mail.rs 写回分支回退为该 RMW 序列（或删除 RestoreMailAttachment 消息）
    /// → 阶段二编译失败 / HP 断言失败。
    #[test]
    fn restore_mail_attachment_preserves_concurrent_state() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .thread_stack_size(8 * 1024 * 1024)
            .enable_time()
            .build()
            .unwrap();
        rt.block_on(async {
            let gate_ref = GateActor::spawn(());
            let db_pool = db::init_db_pool("sqlite::memory:").await.expect("init_db");
            let world_ref = spawn_world(&gate_ref, &db_pool).await;
            let player_ref = PlayerActor::spawn((
                9101u32,
                "RestoreT".to_string(),
                91u64,
                0u16,
                gate_ref.clone(),
                world_ref,
                10u8,
                1u8,
                true,
            ));

            // 注入：一封附件已清空的邮件（收取中）+ HP 500/1000
            let mut st = player_ref
                .ask(GetPlayerState)
                .await
                .expect("GetPlayerState")
                .expect("player state");
            st.mailbox.add_mail(MailMessage {
                mail_id: 42,
                sender_name: "sys".into(),
                receiver_name: "RestoreT".into(),
                subject: "s".into(),
                body: "b".into(),
                timestamp: 1,
                read: false,
                collected: true,
                locked: false,
                gold: 0,
                items: vec![],
            });
            st.hp = 500;
            st.max_hp = 1000;
            player_ref
                .ask(SetPlayerState { state: st })
                .await
                .expect("SetPlayerState");

            // 阶段一（旧模式对照，确定性复现 bug）：整体状态 RMW 覆盖并发 HP 变化
            let mut stale = player_ref
                .ask(GetPlayerState)
                .await
                .expect("GetPlayerState")
                .expect("player state"); // 世界侧快照（收取回写前的旧副本）
            let healed = player_ref.ask(Heal { amount: 100 }).await.expect("Heal");
            assert_eq!(healed, 100, "并发窗口内 HP 500→600（战斗回血/治疗）");
            assert!(stale.mailbox.restore_attachment(42, 300, vec![])); // 世界在旧副本上改 mailbox
            player_ref
                .ask(SetPlayerState { state: stale })
                .await
                .expect("SetPlayerState"); // 旧实现整体回写
            let after_rmw = player_ref
                .ask(GetPlayerState)
                .await
                .expect("GetPlayerState")
                .expect("player state");
            assert_eq!(
                after_rmw.hp, 500,
                "旧 RMW 必须把并发 HP 变化覆盖回去（bug 演示：600 被回滚为 500）"
            );
            assert_eq!(
                after_rmw.mailbox.get_mail(42).map(|m| m.gold),
                Some(300),
                "旧副本上的附件写回本身生效"
            );

            // 阶段二（新机制）：并发 HP 变化 + 专用消息写回 → HP 保留、附件追加写回
            let healed2 = player_ref.ask(Heal { amount: 100 }).await.expect("Heal");
            assert_eq!(healed2, 100, "HP 500→600");
            let restored = player_ref
                .ask(crate::actors::player::RestoreMailAttachment {
                    mail_id: 42,
                    gold: 0,
                    items: vec![user_item(7777, 901)],
                })
                .await
                .expect("RestoreMailAttachment ask");
            assert!(restored, "邮件存在，写回必须成功");
            let after = player_ref
                .ask(GetPlayerState)
                .await
                .expect("GetPlayerState")
                .expect("player state");
            assert_eq!(
                after.hp, 600,
                "RestoreMailAttachment 不得覆盖 HP 等无关状态（RMW 修复）"
            );
            let mail = after.mailbox.get_mail(42).expect("mail 42");
            assert_eq!(mail.gold, 300, "阶段一写回的金币必须保留（只追加目标字段）");
            assert_eq!(mail.items.len(), 1, "物品附件必须写回邮件");
            assert_eq!(mail.items[0].unique_id, 7777);
        });
    }

    /// 贴票消耗失败中止（回归）：邮票在快照后不可用（count=0 废票/并发消耗），
    /// RemoveItemFromInventoryCount 返回 None 时必须中止发信（MailSent=-1 并提示），
    /// 与扣物失败同语义；旧实现 removed_stamp=None 仍继续发信（贴票邮件白享免费 +
    /// 5 格附件待遇）。
    ///
    /// 红检：回退为「removed 为 None 也继续发信」→ MailSent=1 到达，本用例失败。
    #[test]
    fn e2e_send_mail_stamp_removal_failure_aborts() {
        const STAMP_UID: u64 = 8101;
        const SWORD_UID: u64 = 8102;
        const SENDER: &str = "MailSenderB";
        const RECEIVER: &str = "MailRecvB";

        let rt = tokio::runtime::Builder::new_multi_thread()
            .thread_stack_size(8 * 1024 * 1024)
            .enable_time()
            .build()
            .unwrap();
        rt.block_on(async {
            let gate_ref = GateActor::spawn(());
            let db_pool = db::init_db_pool("sqlite::memory:").await.expect("init_db");
            let account_ref = AccountActor::spawn((gate_ref.clone(), db_pool.clone()));
            let _ = gate_ref.ask(SetAccountRef { account_ref }).await;

            // 物品表：邮票（type=0 && shape=1）+ 武器
            for (idx, name, item_type, shape) in
                [(900, "TestStampB", 0, 1), (901, "TestSwordB", 1, 0)]
            {
                sqlx::query("INSERT INTO item_infos (idx, name, type, shape) VALUES (?, ?, ?, ?)")
                    .bind(idx)
                    .bind(name)
                    .bind(item_type)
                    .bind(shape)
                    .execute(&db_pool)
                    .await
                    .expect("insert item_infos");
            }

            let world_ref = spawn_world(&gate_ref, &db_pool).await;

            // 发送者：count=0 废票（快照能找到、按数量移除返回 None）+ 剑
            let s1 = 63u64;
            let mut rx1 = session_created(&gate_ref, s1).await;
            login_and_new_char(&gate_ref, s1, &mut rx1, "mailsenderb", SENDER).await;
            let mut stamp = user_item(STAMP_UID, 900);
            stamp.count = 0;
            for (grid, item) in [(0i32, stamp), (1i32, user_item(SWORD_UID, 901))] {
                sqlx::query(
                    "INSERT INTO inventory_backpack (character_name, grid, item_json) VALUES (?, ?, ?)",
                )
                .bind(SENDER)
                .bind(grid)
                .bind(serde_json::to_string(&item).unwrap())
                .execute(&db_pool)
                .await
                .expect("seed inventory");
            }
            start_game(&gate_ref, s1, &mut rx1).await;

            // 接收者
            let s2 = 64u64;
            let mut rx2 = session_created(&gate_ref, s2).await;
            login_and_new_char(&gate_ref, s2, &mut rx2, "mailrecvb", RECEIVER).await;
            start_game(&gate_ref, s2, &mut rx2).await;

            // 贴票寄信：邮票移除失败 → 必须中止（此刻金币/物品均未扣）
            let _ = collect_packets(&mut rx1, 1).await;
            let _ = gate_ref
                .ask(ClientData {
                    session_id: s1,
                    data: send_mail_packet(RECEIVER, "stamper", 0, [SWORD_UID, 0, 0, 0, 0], true),
                })
                .await;
            let (body, _) = recv_until(&mut rx1, S_MAIL_SENT, 3)
                .await
                .expect("MailSent missing");
            assert_eq!(
                body,
                vec![(-1i8) as u8],
                "邮票移除失败必须中止发信（MailSent=-1；旧实现 None 仍照发=1）"
            );
            let pkts = collect_packets(&mut rx1, 1).await;
            assert!(
                chats_contain(&pkts, "邮票状态已变化，邮件未发送"),
                "中止必须提示（实际包：{:?}）",
                pkts.iter().map(|(op, _)| op).collect::<Vec<_>>()
            );

            // 无任何消耗：剑 uid 不变，可再次（不贴票）寄出成功
            let sword_uid = world_ref
                .ask(GetPlayerItemUid {
                    session_id: s1,
                    item_index: 901,
                })
                .await
                .expect("GetPlayerItemUid ask")
                .expect("剑必须仍在背包");
            assert_eq!(sword_uid, SWORD_UID, "中止不得消耗附件（uid 不变）");
            let _ = gate_ref
                .ask(ClientData {
                    session_id: s1,
                    data: send_mail_packet(RECEIVER, "second", 0, [SWORD_UID, 0, 0, 0, 0], false),
                })
                .await;
            let (body2, _) = recv_until(&mut rx1, S_MAIL_SENT, 3)
                .await
                .expect("MailSent #2 missing");
            assert_eq!(body2, vec![1i8 as u8], "未消耗任何款物，第二次发信必须成功");
        });
    }

    /// 收取写回失败兜底（回归，在线路径）：restore 目标邮件已不存在时，
    /// 未收的金币/物品必须经系统归还邮件进入玩家内存邮箱，不得只 error! 蒸发。
    /// （restore 失败无法经客户端包路径确定性触发——kameo actor 顺序处理，
    /// DeleteMail 不会插入 CollectParcel 处理中途——故直接驱动兜底函数。）
    ///
    /// 红检：把 deliver_collect_restore_fallback 的投递体删掉（回退为仅 error!，
    /// 即修复前行为）→ 邮箱断言全红。
    #[test]
    fn collect_restore_fallback_delivers_via_system_mail_online() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .thread_stack_size(8 * 1024 * 1024)
            .enable_time()
            .build()
            .unwrap();
        rt.block_on(async {
            let gate_ref = GateActor::spawn(());
            let db_pool = db::init_db_pool("sqlite::memory:").await.expect("init_db");
            let world_ref = spawn_world(&gate_ref, &db_pool).await;
            let player_ref = PlayerActor::spawn((
                9201u32,
                "CollectFB".to_string(),
                92u64,
                0u16,
                gate_ref.clone(),
                world_ref,
                10u8,
                1u8,
                true,
            ));

            let ok = deliver_collect_restore_fallback(
                &player_ref,
                &gate_ref,
                &db_pool,
                92,
                "CollectFB",
                700,
                vec![user_item(7771, 901)],
            )
            .await;
            assert!(ok, "在线投递必须成功");

            let st = player_ref
                .ask(GetPlayerState)
                .await
                .expect("GetPlayerState")
                .expect("player state");
            assert_eq!(
                st.mailbox.inbox.len(),
                1,
                "兜底必须新建一封系统归还邮件（修复前仅 error!：0 封）"
            );
            let mail = &st.mailbox.inbox[0];
            assert_eq!(mail.receiver_name, "CollectFB");
            assert_eq!(mail.gold, 700, "未收金币必须随归还邮件送达");
            assert_eq!(mail.items.len(), 1, "未收物品必须随归还邮件送达");
            assert_eq!(mail.items[0].unique_id, 7771);
            assert!(!mail.collected && !mail.read && !mail.locked);
        });
    }

    /// 收取写回失败兜底（回归，离线/actor 异常路径）：玩家 actor 已停止时，
    /// 归还邮件必须落库（登录读回），不得蒸发。
    ///
    /// 红检：把 deliver_collect_restore_fallback 的落库分支删掉（回退为仅 error!）
    /// → mail 表断言全红。
    #[test]
    fn collect_restore_fallback_falls_back_to_db_when_player_actor_dead() {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .thread_stack_size(8 * 1024 * 1024)
            .enable_time()
            .build()
            .unwrap();
        rt.block_on(async {
            let gate_ref = GateActor::spawn(());
            let db_pool = db::init_db_pool("sqlite::memory:").await.expect("init_db");
            let world_ref = spawn_world(&gate_ref, &db_pool).await;
            let player_ref = PlayerActor::spawn((
                9301u32,
                "CollectFC".to_string(),
                93u64,
                0u16,
                gate_ref.clone(),
                world_ref,
                10u8,
                1u8,
                true,
            ));
            // 模拟玩家 actor 异常/离线：AddMail 必然失败 → 兜底落库
            player_ref.kill();
            player_ref.wait_for_shutdown().await;

            let ok = deliver_collect_restore_fallback(
                &player_ref,
                &gate_ref,
                &db_pool,
                93,
                "CollectFC",
                300,
                vec![user_item(7772, 901)],
            )
            .await;
            assert!(ok, "落库投递必须成功");

            let gold: i64 =
                sqlx::query_scalar("SELECT gold FROM mail WHERE character_name = ?")
                    .bind("CollectFC")
                    .fetch_one(&db_pool)
                    .await
                    .expect("归还邮件必须落库（修复前仅 error!：无行）");
            assert_eq!(gold, 300);
            let items_json: String =
                sqlx::query_scalar("SELECT items_json FROM mail WHERE character_name = ?")
                    .bind("CollectFC")
                    .fetch_one(&db_pool)
                    .await
                    .expect("归还邮件必须落库");
            assert!(
                items_json.contains("7772"),
                "归还邮件附件必须含未收物品 uid：{}",
                items_json
            );
        });
    }

    /// 收取/删除连发不丢件（回归）：附件入包失败写回邮件期间客户端立即删邮件，
    /// 附件必须仍在邮件中（删除被拒），不得蒸发。
    /// 构造：count=0 附件绕过背包满预检（can_gain_items_for 跳过 0 计数件），
    /// 但 add_item 仍需空格 → 入包确定性失败 → 走写回分支。
    /// 注：kameo actor 顺序处理——DeleteMail 不会插入 CollectParcel 处理中途，
    /// 本用例钉住「连发时序下附件零丢失」这一客户端可见不变量。
    ///
    /// 红检：把收取失败分支的写回+兜底整块去掉（回退为仅记日志）→ 附件丢失、
    /// 删除成功（收不到「该邮件含有未收取的附件」拒绝提示），本用例红。
    #[test]
    fn e2e_collect_parcel_rapid_delete_keeps_attachment() {
        const MAIL_ID: u64 = 5003;
        const RECEIVER: &str = "MailRecvRace";

        let rt = tokio::runtime::Builder::new_multi_thread()
            .thread_stack_size(8 * 1024 * 1024)
            .enable_time()
            .build()
            .unwrap();
        rt.block_on(async {
            let gate_ref = GateActor::spawn(());
            let db_pool = db::init_db_pool("sqlite::memory:").await.expect("init_db");
            let account_ref = AccountActor::spawn((gate_ref.clone(), db_pool.clone()));
            let _ = gate_ref.ask(SetAccountRef { account_ref }).await;
            spawn_world(&gate_ref, &db_pool).await;

            let s1 = 83u64;
            let mut rx1 = session_created(&gate_ref, s1).await;
            login_and_new_char(&gate_ref, s1, &mut rx1, "mailrecvrace", RECEIVER).await;

            // 填满全部 46 格背包（max_dura=1 不可叠放）：附件入包必然失败
            for grid in 0..46i32 {
                let mut it = user_item(9100 + grid as u64, 902);
                it.max_dura = 1;
                it.current_dura = 1;
                sqlx::query(
                    "INSERT INTO inventory_backpack (character_name, grid, item_json) VALUES (?, ?, ?)",
                )
                .bind(RECEIVER)
                .bind(grid)
                .bind(serde_json::to_string(&it).unwrap())
                .execute(&db_pool)
                .await
                .expect("fill backpack");
            }
            // count=0 附件：预检 can_gain_items_for 跳过 0 计数件（通过），
            // add_item 仍需空格（背包满 → 失败）→ 确定性走写回分支
            let mut loot = user_item(8888, 902);
            loot.count = 0;
            loot.max_dura = 1;
            loot.current_dura = 1;
            db::insert_mail(
                &db_pool,
                RECEIVER,
                &crate::actors::mail::MailMessage {
                    mail_id: MAIL_ID,
                    sender_name: "sys".into(),
                    receiver_name: RECEIVER.into(),
                    subject: "race".into(),
                    body: "race".into(),
                    timestamp: 1,
                    read: false,
                    collected: true,
                    locked: false,
                    gold: 0,
                    items: vec![loot],
                },
            )
            .await
            .expect("seed mail");
            start_game(&gate_ref, s1, &mut rx1).await;

            // 收取（入包失败写回邮件）后立即连发删除
            let _ = collect_packets(&mut rx1, 1).await;
            let _ = gate_ref
                .ask(ClientData {
                    session_id: s1,
                    data: build_packet_bytes(
                        mir2_shared::enums::ClientPacketIds::CollectParcel as i16,
                        &MAIL_ID.to_le_bytes().to_vec(),
                    ),
                })
                .await;
            let (body, mut seen) = recv_until(&mut rx1, S_PARCEL_COLLECTED, 3)
                .await
                .expect("ParcelCollected missing");
            assert_eq!(
                body,
                vec![(-1i8) as u8],
                "入包失败必须 ParcelCollected=-1（附件写回邮件）"
            );
            seen.extend(collect_packets(&mut rx1, 1).await);
            assert!(
                chats_contain(&seen, "背包空间不足，附件已保留在邮件中"),
                "写回成功必须提示附件保留（实际包：{:?}）",
                seen.iter().map(|(op, _)| op).collect::<Vec<_>>()
            );

            // 连发删除：附件已写回 → 必须拒绝删除（附件丢失则删除成功 = 蒸发）
            let _ = gate_ref
                .ask(ClientData {
                    session_id: s1,
                    data: build_packet_bytes(
                        mir2_shared::enums::ClientPacketIds::DeleteMail as i16,
                        &MAIL_ID.to_le_bytes().to_vec(),
                    ),
                })
                .await;
            let pkts = collect_packets(&mut rx1, 2).await;
            assert!(
                chats_contain(&pkts, "该邮件含有未收取的附件，请先收取后再删除"),
                "连发删除必须被拒（附件仍在邮件中；实际包：{:?}）",
                pkts.iter().map(|(op, _)| op).collect::<Vec<_>>()
            );
            assert!(
                !chats_contain(&pkts, "邮件已删除"),
                "附件未收前不得删除成功（删除成功 = 附件蒸发）"
            );
        });
    }
}
