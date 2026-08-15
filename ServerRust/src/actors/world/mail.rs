use super::*;

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
        // #2538：C# PlayerObject.SendMail（11758-11792）——贴票消耗一张邮票（Nothings/Shape==1）
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
        if let Some(uid) = stamp_uid {
            let _ = record
                .actor_ref
                .ask(crate::actors::player::RemoveItemFromInventoryCount {
                    unique_id: uid,
                    count: 1,
                })
                .await;
            send_system_message(&self.gate_ref, msg.session_id, "消耗一张邮票");
        }
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

        // 检查金币是否足够（附件金币 + 寄送费用）
        let total_gold = msg.gold as u64;
        if sender_state.inventory.gold < total_gold + mail_cost {
            send_mail_sent_result(&self.gate_ref, msg.session_id, -1);
            send_system_message(&self.gate_ref, msg.session_id, "金币不足（含寄送费用）");
            return;
        }

        // 从发送者扣除物品（#2538：按贴票门控后的槽位）
        let mut items: Vec<mir2_shared::data::item::UserItem> = Vec::new();
        for uid in &item_uids {
            if let Some(item) = sender_state.inventory.get_item(*uid) {
                items.push(item.clone());
            }
        }

        // 从发送者扣除金币（附件 + 寄送费用）和物品
        if total_gold + mail_cost > 0 {
            let _ = record
                .actor_ref
                .ask(DeductGold {
                    amount: total_gold + mail_cost,
                })
                .await;
        }
        for uid in &item_uids {
            let _ = record
                .actor_ref
                .ask(RemoveItemFromInventory { unique_id: *uid })
                .await;
        }

        // C# MailInfo.Send()：发信初始 Collected 计算（无附件恒 true，包裹按 MailAutoSendGold/Items）
        let collected = crate::actors::mail::initial_collected(
            total_gold > 0,
            !items.is_empty(),
            auto_send_gold,
            auto_send_items,
        );

        // 创建邮件
        let mail = MailMessage {
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

        if let Some(target) = target_session {
            if let Some(target_record) = self.players.get(&target) {
                let _ = target_record
                    .actor_ref
                    .ask(crate::actors::player::AddMail { mail: mail.clone() })
                    .await;
                send_mail_received_packet(&self.gate_ref, target, &mail);
                // C# PlayerObject.Process（:499-504）：收到新邮件 → 系统消息提示
                send_system_message(&self.gate_ref, target, "你收到了一封新邮件");
                debug!(
                    "Mail delivered online: {} -> {}",
                    sender_state.name, msg.receiver_name
                );
            }
        } else {
            // 收件人不在线，保存到数据库
            if let Err(e) = db::insert_mail(&self.db_pool, &msg.receiver_name, &mail).await {
                warn!(
                    "Failed to save offline mail for {}: {}",
                    msg.receiver_name, e
                );
                send_mail_sent_result(&self.gate_ref, msg.session_id, -1);
                send_system_message(&self.gate_ref, msg.session_id, "邮件发送失败，请稍后重试");
                return;
            }
            debug!(
                "Mail saved offline: {} -> {}",
                sender_state.name, msg.receiver_name
            );
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
                let body = vec![-1i8 as u8];
                let _ = self
                    .gate_ref
                    .tell(SendToClient {
                        session_id: msg.session_id,
                        data: build_packet_bytes(
                            mir2_shared::enums::ServerPacketIds::ParcelCollected as i16,
                            &body,
                        ),
                    })
                    .await;
                return;
            }
        };

        let (gold, items) = result;
        if gold > 0 {
            let _ = record.actor_ref.ask(AddGold { amount: gold }).await;
        }
        for item in items {
            let _ = record.actor_ref.ask(AddItemToInventory { item }).await;
        }

        // C# S.ParcelCollected.Result=1：收取成功
        let body = vec![1i8 as u8];
        let _ = self
            .gate_ref
            .tell(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(
                    mir2_shared::enums::ServerPacketIds::ParcelCollected as i16,
                    &body,
                ),
            })
            .await;
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
}
