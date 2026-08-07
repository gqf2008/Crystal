use super::*;

/// 发送邮件
pub struct SendMailRequest {
    pub session_id: u64,
    pub receiver_name: String,
    pub subject: String,
    pub body: String,
    pub gold: u32,
    pub item_uids: Vec<u64>,
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
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let mut state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        if let Some(mail) = state.mailbox.get_mail_mut(msg.mail_id) {
            mail.locked = msg.lock;
            let _ = record.actor_ref.ask(SetPlayerState { state: state.clone() }).await;
            debug!("LockMail: {} mail_id={} lock={}", state.name, msg.mail_id, msg.lock);
        }
    }
}

pub struct MailLockedItemRequest {
    pub session_id: u64,
    pub mail_id: u64,
    pub item_index: u32,
}

impl Message<MailLockedItemRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: MailLockedItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let mut state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        if let Some(mail) = state.mailbox.get_mail_mut(msg.mail_id) {
            mail.locked = true;
            let _ = record.actor_ref.ask(SetPlayerState { state: state.clone() }).await;
            debug!("MailLockedItem: {} mail_id={} item_index={}", state.name, msg.mail_id, msg.item_index);
        }
    }
}

impl Message<SendMailRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: SendMailRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r, None => return,
        };
        let sender_state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s, _ => return,
        };

        if msg.receiver_name == sender_state.name {
            send_system_message(&self.gate_ref, msg.session_id, "不能给自己发送邮件");
            return;
        }

        // C# PlayerObject.GetMailCost：金币费用 floor(gold/1000)*CostPer1K + 物品保险 floor(price/100*Insurance)
        let (mail_cost_per_1k, mail_insurance_pct, mail_free_with_stamp) = self.social_ref
            .ask(crate::actors::social::NpcGetMailSettings)
            .await
            .unwrap_or((100, 5, true));
        // Rust 暂无邮票系统：无邮票 → 不免费（对齐 C# 无 stamp 时收费）
        let mail_cost: u64 = if mail_free_with_stamp {
            0
        } else {
            let gold_fee = (msg.gold as u64 / 1000) * mail_cost_per_1k as u64;
            let mut item_fee: u64 = 0;
            for uid in &msg.item_uids {
                if let Some(item) = sender_state.inventory.get_item(*uid) {
                    if let Some(info) = self.item_infos.get(&item.item_index) {
                        // C# GetMailCost：item.Price()（含耐久比例/附加属性）× Count
                        let price = super::item::compute_item_price_per_unit(&item, info)
                            .saturating_mul(item.count as u64);
                        item_fee += price / 100 * mail_insurance_pct as u64;
                    }
                }
            }
            gold_fee + item_fee
        };

        // 检查金币是否足够（附件金币 + 寄送费用）
        let total_gold = msg.gold as u64;
        if sender_state.inventory.gold < total_gold + mail_cost {
            send_system_message(&self.gate_ref, msg.session_id, "金币不足（含寄送费用）");
            return;
        }

        // 从发送者扣除物品
        let mut items: Vec<mir2_shared::data::item::UserItem> = Vec::new();
        for uid in &msg.item_uids {
            if let Some(item) = sender_state.inventory.get_item(*uid) {
                items.push(item.clone());
            }
        }

        // 从发送者扣除金币（附件 + 寄送费用）和物品
        if total_gold + mail_cost > 0 {
            let _ = record.actor_ref.ask(DeductGold { amount: total_gold + mail_cost }).await;
        }
        for uid in &msg.item_uids {
            let _ = record.actor_ref.ask(RemoveItemFromInventory { unique_id: *uid }).await;
        }

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
            collected: false,
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
                let _ = target_record.actor_ref.ask(crate::actors::player::AddMail { mail: mail.clone() }).await;
                send_mail_received_packet(&self.gate_ref, target, &mail);
                debug!("Mail delivered online: {} -> {}", sender_state.name, msg.receiver_name);
            }
        } else {
            // 收件人不在线，保存到数据库
            if let Err(e) = db::insert_mail(&self.db_pool, &msg.receiver_name, &mail).await {
                warn!("Failed to save offline mail for {}: {}", msg.receiver_name, e);
                send_system_message(&self.gate_ref, msg.session_id, "邮件发送失败，请稍后重试");
                return;
            }
            debug!("Mail saved offline: {} -> {}", sender_state.name, msg.receiver_name);
        }

        send_system_message(&self.gate_ref, msg.session_id, "邮件已发送");
    }
}

impl Message<ReadMailRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: ReadMailRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r, None => return,
        };

        let mail = match record.actor_ref.ask(crate::actors::player::GetMail { mail_id: msg.mail_id }).await {
            Ok(Some(m)) => m, _ => return,
        };

        send_mail_content_packet(&self.gate_ref, msg.session_id, &mail);
        let _ = record.actor_ref.ask(crate::actors::player::MarkMailRead { mail_id: msg.mail_id }).await;
    }
}

impl Message<CollectParcelRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: CollectParcelRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r, None => return,
        };

        let result = match record.actor_ref.ask(crate::actors::player::CollectMailAttachment { mail_id: msg.mail_id }).await {
            Ok(Some(r)) => r, _ => {
                // C# S.ParcelCollected.Result=-1：无可收取包裹
                let body = vec![-1i8 as u8];
                let _ = self.gate_ref.tell(SendToClient {
                    session_id: msg.session_id,
                    data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::ParcelCollected as i16, &body),
                }).await;
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
        let _ = self.gate_ref.tell(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::ParcelCollected as i16, &body),
        }).await;
        debug!("CollectParcel: session={} mail_id={} gold={}", msg.session_id, msg.mail_id, gold);
    }
}

impl Message<DeleteMailRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: DeleteMailRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r, None => return,
        };

        let deleted = match record.actor_ref.ask(crate::actors::player::DeleteMail { mail_id: msg.mail_id }).await {
            Ok(d) => d, _ => return,
        };

        if deleted {
            send_system_message(&self.gate_ref, msg.session_id, "邮件已删除");
        }
    }
}
