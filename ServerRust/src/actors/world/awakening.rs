use super::*;

/// 存入精炼物品（C# DepositRefineItem：From=背包格、To=精炼栏格）
pub struct DepositRefineItemRequest {
    pub session_id: u64,
    /// C# C.DepositRefineItem.From：背包格索引
    pub from: i32,
    /// C# C.DepositRefineItem.To：精炼栏格索引（Rust 单槽，须为 0）
    pub to: i32,
}

impl WorldActor {
    /// #2827：C# `S.DepositRefineItem` / `S.RetrieveRefineItem` 确认包（两者同布局
    /// `[from i32][to i32][success u8]`）；**成功与失败都要发**（C# `Enqueue(p)`，
    /// `PlayerObject.cs:12511-12562` / `:12564-12601`）——客户端靠它推进存入/取回状态机。
    async fn send_refine_slot_ack(
        &self,
        session_id: u64,
        opcode: mir2_shared::enums::ServerPacketIds,
        from: i32,
        to: i32,
        success: bool,
    ) {
        let data = build_packet_bytes(
            opcode as i16,
            &crate::actors::refine::refine_slot_ack_body(from, to, success),
        );
        if let Err(e) = self.gate_ref.tell(SendToClient { session_id, data }).try_send()
        {
            warn!(
                "gate mailbox full: SendToClient dropped (session={} opcode={:?} err={})",
                session_id,
                super::dropped_send_opcode(&e),
                e
            );
        }
    }

    /// 存入精炼物品/材料的实际逻辑（C# DepositRefineItem）；确认包由调用方统一发
    async fn deposit_refine_item_inner(&mut self, session_id: u64, from: i32, to: i32) -> bool {
        // #2843：C# DepositRefineItem（:12509）要求 NPCPage.Key == [@REFINE]，否则回 success=false 的确认包
        if !self.npc_page_allows(session_id, &["[@REFINE]"]) {
            debug!("DepositRefineItem rejected: not on [@REFINE] page");
            return false;
        }
        let record = match self.players.get(&session_id) {
            Some(r) => r,
            None => return false,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return false,
        };

        // C# DepositRefineItem（:12529-12559）：From 为背包格、To 为精炼栏格；To=0 武器槽（Rust 单槽），To=1..10 材料槽
        if to < 0 || to as usize > crate::actors::refine::REFINE_MATERIAL_SLOTS {
            send_system_message(&self.gate_ref, session_id, "精炼栏已满");
            return false;
        }
        if from < 0 || from as usize >= state.inventory.backpack.len() {
            send_system_message(&self.gate_ref, session_id, "物品不存在");
            return false;
        }
        let Some(item) = state.inventory.backpack[from as usize]
            .as_ref()
            .map(|s| s.item.clone())
        else {
            send_system_message(&self.gate_ref, session_id, "物品不存在");
            return false;
        };

        // 材料槽（To 1..=10）：C# Info.Refine 材料格（材料可为任意装备/矿，不做 DontUpgrade 限制）
        if to != 0 {
            let Some(item) = record
                .actor_ref
                .ask(crate::actors::player::RemoveItemFromInventory {
                    unique_id: item.unique_id,
                })
                .await
                .unwrap_or(None)
            else {
                send_system_message(&self.gate_ref, session_id, "物品不存在");
                return false;
            };
            let mut log = state.refine_log;
            if !log.deposit_material((to - 1) as usize, item) {
                send_system_message(&self.gate_ref, session_id, "该精炼栏格子已被占用");
                return false;
            }
            let _ = record.actor_ref.ask(SetRefineLog { refine_log: log }).await;
            send_system_message(&self.gate_ref, session_id, "精炼材料已存入");
            debug!(
                "DepositRefineItem(material): {} from={} to={}",
                state.name, from, to
            );
            return true;
        }

        // #926：C# BindMode.DontUpgrade(0x40)：不可精炼/升级（含租赁绑定，:12678）
        if self
            .item_infos
            .get(&item.item_index)
            .map(|i| {
                super::has_bind_flag(
                    i.bind_mode,
                    mir2_shared::enums::BindMode::DONT_UPGRADE.bits(),
                )
            })
            .unwrap_or(false)
            || super::rental_has_flag(&item, mir2_shared::enums::BindMode::DONT_UPGRADE.bits())
        {
            send_system_message(&self.gate_ref, session_id, "该物品无法精炼");
            return false;
        }

        // C# RefineItem（:12705）：从背包移除物品并存入精炼日志（含完整物品数据）
        let Some(item) = record
            .actor_ref
            .ask(crate::actors::player::RemoveItemFromInventory {
                unique_id: item.unique_id,
            })
            .await
            .unwrap_or(None)
        else {
            send_system_message(&self.gate_ref, session_id, "物品不存在");
            return false;
        };
        // 更新精炼日志
        let mut log = state.refine_log;
        if !log.deposit_item(item) {
            send_system_message(&self.gate_ref, session_id, "已有精炼进行中");
            return false;
        }

        let _ = record.actor_ref.ask(SetRefineLog { refine_log: log }).await;
        send_system_message(&self.gate_ref, session_id, "精炼物品已存入");
        debug!("DepositRefineItem: {} from={}", state.name, from);
        true
    }
}

impl Message<DepositRefineItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: DepositRefineItemRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let success = self
            .deposit_refine_item_inner(msg.session_id, msg.from, msg.to)
            .await;
        self.send_refine_slot_ack(
            msg.session_id,
            mir2_shared::enums::ServerPacketIds::DepositRefineItem,
            msg.from,
            msg.to,
            success,
        )
        .await;
    }
}

/// 取回精炼物品（C# RetrieveRefineItem：From=精炼栏格、To=背包格）
pub struct RetrieveRefineItemRequest {
    pub session_id: u64,
    /// C# C.RetrieveRefineItem.From：精炼栏格索引（Rust 单槽，须为 0）
    pub from: i32,
    /// C# C.RetrieveRefineItem.To：背包格索引
    pub to: i32,
}

impl WorldActor {
    /// 取回精炼物品/材料的实际逻辑（C# RetrieveRefineItem）；确认包由调用方统一发
    async fn retrieve_refine_item_inner(&mut self, session_id: u64, from: i32, to: i32) -> bool {
        let record = match self.players.get(&session_id) {
            Some(r) => r,
            None => return false,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return false,
        };

        // C# RetrieveRefineItem（:12568-12600）：From=精炼栏格（0=武器 / 1..=10 材料）、To=背包格
        if from < 0 || from as usize > crate::actors::refine::REFINE_MATERIAL_SLOTS {
            send_system_message(&self.gate_ref, session_id, "没有精炼物品可取回");
            return false;
        }
        if to < 0
            || to as usize >= state.inventory.backpack.len()
            || state.inventory.backpack[to as usize].is_some()
        {
            send_system_message(&self.gate_ref, session_id, "该背包格子已被占用");
            return false;
        }

        let mut log = state.refine_log.clone();
        if from == 0 {
            // 武器槽（C# :12590 返还到指定背包格）
            if state.refine_log.active_refine.is_none() {
                send_system_message(&self.gate_ref, session_id, "没有精炼物品可取回");
                return false;
            }
            if let Some(ri) = log.retrieve() {
                if let Some(item) = ri.item {
                    let ok = record
                        .actor_ref
                        .ask(crate::actors::player::PlaceItemAtSlot {
                            slot: to,
                            item: item.clone(),
                        })
                        .await
                        .unwrap_or(false);
                    if !ok {
                        // 放置失败回放精炼栏（避免物品丢失）
                        let mut log2 = state.refine_log.clone();
                        if log2.deposit_item(item) {
                            let _ = record
                                .actor_ref
                                .ask(SetRefineLog { refine_log: log2 })
                                .await;
                        }
                        send_system_message(&self.gate_ref, session_id, "背包已满");
                        return false;
                    }
                }
                let _ = record.actor_ref.ask(SetRefineLog { refine_log: log }).await;
                send_system_message(&self.gate_ref, session_id, "精炼物品已取回");
                debug!("RetrieveRefineItem: {} to={}", state.name, to);
                return true;
            }
            false
        } else {
            // 材料槽（C# :12590 返还材料到指定背包格）
            let Some(item) = log.retrieve_material((from - 1) as usize) else {
                send_system_message(&self.gate_ref, session_id, "该材料格为空");
                return false;
            };
            let ok = record
                .actor_ref
                .ask(crate::actors::player::PlaceItemAtSlot {
                    slot: to,
                    item: item.clone(),
                })
                .await
                .unwrap_or(false);
            if !ok {
                // 放置失败回放材料格（避免物品丢失）
                let _ = log.deposit_material((from - 1) as usize, item);
                let _ = record.actor_ref.ask(SetRefineLog { refine_log: log }).await;
                send_system_message(&self.gate_ref, session_id, "背包已满");
                return false;
            }
            let _ = record.actor_ref.ask(SetRefineLog { refine_log: log }).await;
            send_system_message(&self.gate_ref, session_id, "精炼材料已取回");
            debug!(
                "RetrieveRefineItem(material): {} from={} to={}",
                state.name, from, to
            );
            true
        }
    }
}

impl Message<RetrieveRefineItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: RetrieveRefineItemRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let success = self
            .retrieve_refine_item_inner(msg.session_id, msg.from, msg.to)
            .await;
        self.send_refine_slot_ack(
            msg.session_id,
            mir2_shared::enums::ServerPacketIds::RetrieveRefineItem,
            msg.from,
            msg.to,
            success,
        )
        .await;
    }
}

/// 取消精炼
pub struct RefineCancelRequest {
    pub session_id: u64,
}

impl Message<RefineCancelRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: RefineCancelRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        let mut log = state.refine_log;
        // 返还全部材料（C# RefineCancel :12603-12639）
        for m in log.take_all_materials().into_iter().flatten() {
            let _ = record
                .actor_ref
                .ask(crate::actors::player::AddItemToInventory { item: m })
                .await;
        }
        // 返还武器（如有）
        if let Some(ri) = log.cancel() {
            if let Some(item) = ri.item {
                let _ = record
                    .actor_ref
                    .ask(crate::actors::player::AddItemToInventory { item })
                    .await;
            }
        }
        let _ = record.actor_ref.ask(SetRefineLog { refine_log: log }).await;
        send_system_message(&self.gate_ref, msg.session_id, "精炼已取消");
        debug!("RefineCancel: {}", state.name);
    }
}

// #2392：精炼费用/时长改用 util::config::RefineConfig（C# Settings.RefineCost/RefineTime）

/// #2034：C# RefineItem（12676）——费用 (RequiredAmount*10)*RefineCost
fn refine_cost(required_amount: i32, cost: u32) -> u64 {
    (required_amount.max(0) as i64 * 10 * cost as i64) as u64
}

/// 开始精炼（C# RefineItem：UniqueID 为精炼栏内物品 uid）
pub struct RefineItemRequest {
    pub session_id: u64,
    /// C# C.RefineItem.UniqueID：精炼栏内待精炼物品 uid
    pub unique_id: u64,
}

impl Message<RefineItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: RefineItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        // #2843：C# RefineItem（:12647）要求 NPCPage.Key == [@REFINE]（防远程发包）
        if !self.npc_page_allows(msg.session_id, &["[@REFINE]"]) {
            send_system_message(&self.gate_ref, msg.session_id, "请先打开精炼页");
            return;
        }
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // C# RefineItem（:12649-12658）：按 unique_id 校验精炼栏物品
        let deposited = state
            .refine_log
            .active_refine
            .as_ref()
            .and_then(|ri| ri.item.clone());
        let Some(deposited) = deposited else {
            send_system_message(&self.gate_ref, msg.session_id, "没有待精炼的物品");
            return;
        };
        if msg.unique_id != 0 && deposited.unique_id != msg.unique_id {
            send_system_message(&self.gate_ref, msg.session_id, "物品不存在");
            return;
        }

        // #2034：C# RefineItem（12676-12683）——费用 (RequiredAmount*10)*RefineCost
        let item_db = match self.item_infos.get(&deposited.item_index) {
            Some(i) => i.clone(),
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "物品不存在");
                return;
            }
        };
        // C# Settings.OnlyRefineWeapon（RefineSystem.ini `[Config] OnlyRefineWeapon`）+（ItemType.Weapon=1）
        if item_db.item_type != 1 && self.refine_cfg.only_refine_weapon {
            send_system_message(&self.gate_ref, msg.session_id, "只有武器可以精炼");
            return;
        }
        // C# BindMode.DontUpgrade(0x40)：不可精炼（与 DepositRefineItem #926 一致；含租赁绑定 :12678）
        if super::has_bind_flag(
            item_db.bind_mode,
            mir2_shared::enums::BindMode::DONT_UPGRADE.bits(),
        ) || super::rental_has_flag(
            &deposited,
            mir2_shared::enums::BindMode::DONT_UPGRADE.bits(),
        ) {
            send_system_message(&self.gate_ref, msg.session_id, "该物品无法精炼");
            return;
        }
        let cost = refine_cost(item_db.required_amount, self.refine_cfg.cost);
        if state.inventory.gold < cost {
            send_system_message(&self.gate_ref, msg.session_id, "金币不足，无法精炼");
            return;
        }
        let _ = record
            .actor_ref
            .ask(crate::actors::player::DeductGold { amount: cost })
            .await;
        super::send_gold_changed_packet(&self.gate_ref, msg.session_id, cost);

        // 开始精炼（C# Settings.RefineTime=20 分钟完成；成功率公式待材料系统，暂保持 80%）
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let duration = self.refine_cfg.time_minutes as u64 * 60; // C# Settings.RefineTime=20 分钟
                                                                 // C# RefineItem 材料聚合（:12710-12751）→ RefinedValue/refineStat（:12790-12806）→ 成功率（:12811-12845）
        let aggregates = crate::actors::refine::refine_material_aggregates(
            &state.refine_log.materials,
            &self.item_infos,
            &self.refine_cfg.ore_name,
        );
        // C# :12753/:12771 前置（三属性全 0 / 无矿）→ RefinedValue=None + 成功率 0（必碎）；
        // 其余按严格大于选 DC/MC/SC（:12790-12806）
        let (refined_value, refine_stat) = crate::actors::refine::refine_target_value(
            aggregates.ore_amount,
            aggregates.total_dc,
            aggregates.total_mc,
            aggregates.total_sc,
        );
        let luck = deposited.added_stats.get(mir2_shared::enums::Stat::Luck);
        let added_dc = deposited.added_stats.get(mir2_shared::enums::Stat::MaxDC);
        let added_mc = deposited.added_stats.get(mir2_shared::enums::Stat::MaxMC);
        let added_sc = deposited.added_stats.get(mir2_shared::enums::Stat::MaxSC);
        let success_chance = crate::actors::refine::refine_success_chance(
            refine_stat,
            item_db.required_amount,
            aggregates.required_level,
            aggregates.item_amount,
            aggregates.durability_count,
            aggregates.current_dura_count,
            aggregates.ore_amount,
            aggregates.ore_purity,
            luck,
            added_dc,
            added_mc,
            added_sc,
            true,
            self.refine_cfg.base_chance as i32,
            self.refine_cfg.wep_stat_reduce as i32,
            self.refine_cfg.item_stat_reduce as i32,
        )
        .clamp(0, 255) as u8;
        let mut log = state.refine_log;
        if !log.begin_refine(current_time, duration, success_chance) {
            send_system_message(&self.gate_ref, msg.session_id, "没有待精炼的物品");
            return;
        }
        // 设置精炼字段（C# :12808 RefineAdded=RefineIncrease；:12845 RefineSuccessChance）
        if let Some(ri) = log.active_refine.as_mut() {
            if let Some(item) = ri.item.as_mut() {
                item.refined_value = refined_value;
                item.refine_added = self.refine_cfg.increase;
                item.refine_success_chance = success_chance as i32;
            }
        }
        // 消耗材料（C# :12729 / :12750 清空 Info.Refine）
        log.take_all_materials();
        let _ = record.actor_ref.ask(SetRefineLog { refine_log: log }).await;

        // C# RefineItem（:12703）：开始时发 S.RefineItem { UniqueID }
        let mut rb = Vec::new();
        rb.extend_from_slice(&deposited.unique_id.to_le_bytes());
        if let Err(_) = self
            .gate_ref
            .tell(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(
                    mir2_shared::enums::ServerPacketIds::RefineItem as i16,
                    &rb,
                ),
            })
            .try_send()
        {
            warn!("gate mailbox full: SendToClient dropped (session={} packet=RefineItem)", msg.session_id);
        }

        send_system_message(&self.gate_ref, msg.session_id, "精炼已开始，请稍后查看");
        debug!("RefineItem: {} uid={}", state.name, msg.unique_id);
    }
}

/// 检查精炼状态
pub struct CheckRefineRequest {
    pub session_id: u64,
    pub unique_id: u64,
}

impl Message<CheckRefineRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: CheckRefineRequest, _ctx: &mut Context<Self, Self::Reply>) {
        // #2843：C# CheckRefine（:12905）要求 NPCPage.Key == [@REFINECHECK]（防远程发包）
        if !self.npc_page_allows(msg.session_id, &["[@REFINECHECK]"]) {
            send_system_message(&self.gate_ref, msg.session_id, "请先打开精炼查看页");
            return;
        }
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        if let Some(ref item) = state.refine_log.active_refine {
            if item.status == RefineStatus::Pending && current_time >= item.finish_time {
                let mut log = state.refine_log;
                match log.settle_check(self.refine_cfg.crit_chance, self.refine_cfg.crit_increase) {
                    Some(crate::actors::refine::RefineCheckResult::Applied) => {
                        let refined_item =
                            log.active_refine.as_ref().and_then(|ri| ri.item.clone());
                        let _ = record.actor_ref.ask(SetRefineLog { refine_log: log }).await;
                        // C# CheckRefine（:12970）：成功发 S.ItemUpgraded { Item }
                        if let Some(item) = refined_item {
                            let pkt = mir2_shared::packets::server::ItemUpgraded { item };
                            let mut body = Vec::new();
                            if mir2_shared::packets::base::serialize_packet(
                                &mut std::io::Cursor::new(&mut body),
                                &pkt,
                            )
                            .is_ok()
                            {
                                if let Err(e) = self
                                    .gate_ref
                                    .tell(SendToClient {
                                        session_id: msg.session_id,
                                        data: body,
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
                        send_system_message(&self.gate_ref, msg.session_id, "精炼成功！请取回物品");
                        debug!("CheckRefine: {} applied", state.name);
                    }
                    Some(crate::actors::refine::RefineCheckResult::Destroyed) => {
                        // C# :12961-12967：失败/无 RefinedValue → 先发 S.RefineItem（客户端 RefineReset）再粉碎
                        let destroyed_uid = log
                            .active_refine
                            .as_ref()
                            .and_then(|ri| ri.item.as_ref())
                            .map(|i| i.unique_id)
                            .unwrap_or(msg.unique_id);
                        let _ = log.cancel();
                        let _ = record.actor_ref.ask(SetRefineLog { refine_log: log }).await;
                        if let Err(_) = self
                            .gate_ref
                            .tell(SendToClient {
                                session_id: msg.session_id,
                                data: build_packet_bytes(
                                    mir2_shared::enums::ServerPacketIds::RefineItem as i16,
                                    &destroyed_uid.to_le_bytes(),
                                ),
                            })
                            .try_send()
                        {
                            warn!("gate mailbox full: SendToClient dropped (session={} packet=RefineItem)", msg.session_id);
                        }
                        send_system_message(&self.gate_ref, msg.session_id, "精炼失败，物品已粉碎");
                        debug!("CheckRefine: {} destroyed", state.name);
                    }
                    None => {
                        let _ = record.actor_ref.ask(SetRefineLog { refine_log: log }).await;
                        send_system_message(&self.gate_ref, msg.session_id, "精炼进行中");
                    }
                }
            } else if item.status == RefineStatus::Ready {
                send_system_message(&self.gate_ref, msg.session_id, "精炼已完成，请取回物品");
            } else {
                let remaining = item.finish_time.saturating_sub(current_time);
                send_system_message(
                    &self.gate_ref,
                    msg.session_id,
                    &format!("精炼进行中，剩余 {} 秒", remaining),
                );
            }
        } else {
            send_system_message(&self.gate_ref, msg.session_id, "没有精炼进行中");
        }
    }
}

pub struct AwakeningNeedMaterialsRequest {
    pub session_id: u64,
    pub unique_id: u64,
    pub awake_type: u8,
}

impl Message<AwakeningNeedMaterialsRequest> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: AwakeningNeedMaterialsRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        // #2573：页 key 门槛（C# PlayerObject.cs:8808 觉醒要求 AwakeningKey，防远程发包）
        if !self.npc_page_allows(msg.session_id, &["[@AWAKENING]"]) {
            send_system_message(&self.gate_ref, msg.session_id, "请先打开觉醒页");
            return;
        }

        let awake_type = match mir2_shared::enums::AwakeType::try_from(msg.awake_type) {
            Ok(t) => t,
            Err(_) => {
                send_system_message(&self.gate_ref, msg.session_id, "无效的觉醒类型");
                return;
            }
        };

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

        if !item_info.can_awakening {
            send_system_message(&self.gate_ref, msg.session_id, "该物品不支持觉醒");
            return;
        }

        // 计算所需材料：觉醒材料是 item_type=35 的物品
        // shape 编码（C# 约定，见真实物品库 BraveryGlyph/MagicGlyph/…/AwakeningSoul）：
        // 0=DC, 1=MC, 2=SC, 3=AC, 4=MAC, 5=HPMP, 100=通用
        let awake_level = item.awake.awake_level();
        if !(1..=4).contains(&item_info.grade) {
            send_system_message(&self.gate_ref, msg.session_id, "该物品品级不支持觉醒");
            return;
        }

        // C# Awake.AwakeMaterials + AwakeMaterialRate：
        // need[slot] = base[type][slot][grade] + (byte)(material_rate[grade] * level)（PlayerObject.cs:9061-9067）
        let type_shape = awake_type_shape(awake_type);
        let need = awakening_material_need(
            &self.awakening_cfg,
            awake_type,
            item_info.grade,
            awake_level as u32,
        );

        // 查找匹配的觉醒材料物品：品级必须与物品相同（C# :9073-9084）
        let mut materials = Vec::new();
        for (slot, shape) in [type_shape, 100].iter().enumerate() {
            let mut found: Option<i32> = None;
            for (idx, info) in self.item_infos.iter() {
                if info.item_type == 35 /* ItemType::Awakening */
                    && info.grade == item_info.grade
                    && info.shape == *shape
                {
                    found = Some(*idx);
                    break;
                }
            }
            if let Some(item_id) = found {
                materials.push(
                    mir2_shared::packets::server::awakening_system::MaterialInfo {
                        item_id,
                        count: need[slot] as i32,
                    },
                );
            }
        }

        let packet = mir2_shared::packets::server::awakening_system::AwakeningNeedMaterials {
            item_id: item.item_index,
            materials,
        };
        let mut body = Vec::new();
        if let Err(e) = packet.write_body(&mut body) {
            warn!("Failed to serialize AwakeningNeedMaterials: {}", e);
            return;
        }
        if self
            .gate_ref
            .tell(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(
                    mir2_shared::enums::ServerPacketIds::AwakeningNeedMaterials as i16,
                    &body,
                ),
            })
            .try_send()
            .is_err()
        {
            warn!("gate mailbox full: SendToClient dropped (session={} packet=AwakeningNeedMaterials)", msg.session_id);
        }
    }
}

pub struct AwakeningLockedItemRequest {
    pub session_id: u64,
    pub unique_id: u64,
    pub locked: bool,
}

impl Message<AwakeningLockedItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: AwakeningLockedItemRequest,
        _ctx: &mut Context<Self, Self::Reply>,
    ) {
        let packet = mir2_shared::packets::server::awakening_system::AwakeningLockedItem {
            unique_id: msg.unique_id,
            locked: msg.locked,
        };
        let mut body = Vec::new();
        if let Err(e) = packet.write_body(&mut body) {
            warn!("Failed to serialize AwakeningLockedItem: {}", e);
            return;
        }
        if self
            .gate_ref
            .tell(SendToClient {
                session_id: msg.session_id,
                data: build_packet_bytes(
                    mir2_shared::enums::ServerPacketIds::AwakeningLockedItem as i16,
                    &body,
                ),
            })
            .try_send()
            .is_err()
        {
            warn!("gate mailbox full: SendToClient dropped (session={} packet=AwakeningLockedItem)", msg.session_id);
        }
    }
}

pub struct AwakeningRequest {
    pub session_id: u64,
    pub unique_id: u64,
    pub awake_type: u8,
}

impl WorldActor {
    /// C# `PlayerObject.AwakeningEffect`（`PlayerObject.cs:9094-9104`）：
    /// 5 次 `S.ObjectEffect{Effect = isHit[i] ? AwakeningHit : AwakeningMiss, EffectType = 0, DelayTime = i*500}`
    /// + 1 次 `S.ObjectEffect{Effect = isSuccess ? AwakeningSuccess : AwakeningFail, DelayTime = 2500}`；
    /// C# 是 `Enqueue`（自己）+ `Broadcast`（同图他人）——这里折叠成「同图全体各一次」。
    async fn send_awakening_effects(
        &self,
        object_id: u32,
        map_index: u16,
        is_success: bool,
        hits: &[bool; 5],
    ) {
        use mir2_shared::enums::SpellEffect;
        for (i, hit) in hits.iter().enumerate() {
            let effect = if *hit {
                SpellEffect::AwakeningHit
            } else {
                SpellEffect::AwakeningMiss
            };
            self.send_object_effect_to_map(object_id, effect, i as u32 * 500, map_index)
                .await;
        }
        let final_effect = if is_success {
            SpellEffect::AwakeningSuccess
        } else {
            SpellEffect::AwakeningFail
        };
        self.send_object_effect_to_map(object_id, final_effect, 2500, map_index)
            .await;
    }

    /// 发一条 `S.ObjectEffect` 给同图全体（含自己一次；C# Enqueue + Broadcast 去重的等价结果）
    async fn send_object_effect_to_map(
        &self,
        object_id: u32,
        effect: mir2_shared::enums::SpellEffect,
        delay_time: u32,
        map_index: u16,
    ) {
        let packet = mir2_shared::packets::server::magic_combat::ObjectEffect {
            object_id,
            effect,
            effect_type: 0,
            delay_time,
            time: 0,
        };
        let mut body = Vec::new();
        if packet.write_body(&mut body).is_err() {
            return;
        }
        let data = build_packet_bytes(
            mir2_shared::enums::ServerPacketIds::ObjectEffect as i16,
            &body,
        );
        for (sid, r) in &self.players {
            if let Ok(Some(os)) = r.actor_ref.ask(GetPlayerState).await {
                if os.map_index == map_index {
                    if let Err(e) = self
                        .gate_ref
                        .tell(SendToClient {
                            session_id: *sid,
                            data: data.clone(),
                        })
                        .try_send()
                    {
                        warn!(
                            "gate mailbox full: SendToClient dropped (session={} opcode={:?} err={})",
                            *sid,
                            super::dropped_send_opcode(&e),
                            e
                        );
                    }
                }
            }
        }
    }
}

impl Message<AwakeningRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: AwakeningRequest, _ctx: &mut Context<Self, Self::Reply>) {
        use mir2_shared::packets::server::awakening_system::*;

        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };
        // #2573：页 key 门槛（C# PlayerObject.cs:8808 觉醒要求 AwakeningKey，防远程发包）
        if !self.npc_page_allows(msg.session_id, &["[@AWAKENING]"]) {
            self.send_awakening_result(msg.session_id, AWAKE_RESULT_FAIL, -1);
            return;
        }

        let awake_type = match mir2_shared::enums::AwakeType::try_from(msg.awake_type) {
            Ok(t) => t,
            Err(_) => {
                self.send_awakening_result(msg.session_id, AWAKE_RESULT_FAIL, -1);
                return;
            }
        };

        let item = match record
            .actor_ref
            .ask(crate::actors::player::GetItemInfo {
                unique_id: msg.unique_id,
            })
            .await
        {
            Ok(Some(i)) => i,
            _ => {
                self.send_awakening_result(msg.session_id, AWAKE_RESULT_FAIL, -1);
                return;
            }
        };

        let item_info = match self.item_infos.get(&item.item_index) {
            Some(i) => i,
            None => {
                self.send_awakening_result(msg.session_id, AWAKE_RESULT_FAIL, -1);
                return;
            }
        };

        // C# AwakeningItem（:8826）：租赁 DontUpgrade 物品不可觉醒
        if super::rental_has_flag(&item, mir2_shared::enums::BindMode::DONT_UPGRADE.bits()) {
            self.send_awakening_result(msg.session_id, AWAKE_RESULT_FAIL, -1);
            return;
        }

        // 验证：物品可觉醒
        if !item_info.can_awakening {
            self.send_awakening_result(msg.session_id, AWAKE_RESULT_FAIL, -1);
            return;
        }

        // 验证：未达最大等级（#2416：MaxUpgradeLevel 配置化）
        if item.awake.awake_level() >= self.awakening_cfg.max_awake_level {
            self.send_awakening_result(msg.session_id, AWAKE_RESULT_MAX_LEVEL, -1);
            return;
        }

        // 验证：觉醒类型匹配（已觉醒的物品不能换类型）
        if item.awake.awake_type != mir2_shared::enums::AwakeType::None
            && item.awake.awake_type != awake_type
        {
            self.send_awakening_result(msg.session_id, AWAKE_RESULT_FAIL, -1);
            return;
        }

        // 验证物品类型与觉醒类型兼容性 (Weapon=1, Armour=2, Helmet=4)
        let compatible = match item_info.item_type {
            1 => matches!(
                awake_type,
                mir2_shared::enums::AwakeType::Dc
                    | mir2_shared::enums::AwakeType::Mc
                    | mir2_shared::enums::AwakeType::Sc
            ),
            4 => matches!(
                awake_type,
                mir2_shared::enums::AwakeType::Ac | mir2_shared::enums::AwakeType::Mac
            ),
            2 => awake_type == mir2_shared::enums::AwakeType::HpMp,
            _ => false,
        };
        if !compatible {
            self.send_awakening_result(msg.session_id, AWAKE_RESULT_FAIL, -1);
            return;
        }

        // 品级 (Common=1, Rare=2, Legendary=3, Mythical=4)
        let grade = match item_info.grade {
            1..=4 => item_info.grade,
            _ => {
                self.send_awakening_result(msg.session_id, AWAKE_RESULT_FAIL, -1);
                return;
            }
        };
        let awake_level = item.awake.awake_level();

        // 检查金币：费用 = 1500 * (1 + awakeLevel * 2) * grade
        let gold_cost = 1500u64 * (1 + awake_level as u64 * 2) * grade as u64;
        let has_gold = record
            .actor_ref
            .ask(crate::actors::player::HasGold { amount: gold_cost })
            .await
            .unwrap_or(false);
        if !has_gold {
            self.send_awakening_result(msg.session_id, AWAKE_RESULT_NO_GOLD, -1);
            return;
        }

        // 检查材料（C# HasAwakeningNeedMaterials :9123-9166）：两类 shape 分别统计、
        // 数量必须分别精确等于需求（计数按需求上限截断 → 「可用 >= 需求」等价），
        // 品级必须与物品相同（None 品级不参与），再按需求逐项消耗。
        let type_shape = awake_type_shape(awake_type);
        let need =
            awakening_material_need(&self.awakening_cfg, awake_type, grade, awake_level as u32);
        let mut material_indices: [Vec<i32>; 2] = [Vec::new(), Vec::new()];
        for (idx, info) in self.item_infos.iter() {
            if info.item_type == 35 /* ItemType::Awakening */ && info.grade == grade {
                if info.shape == type_shape {
                    material_indices[0].push(*idx);
                } else if info.shape == 100 {
                    material_indices[1].push(*idx);
                }
            }
        }

        let mut available = [0i32; 2];
        let mut per_index: [Vec<(i32, i32)>; 2] = [Vec::new(), Vec::new()];
        for slot in 0..2 {
            for idx in &material_indices[slot] {
                let have = record
                    .actor_ref
                    .ask(crate::actors::player::CountItemsByIndex { item_index: *idx })
                    .await
                    .unwrap_or(0) as i32;
                available[slot] += have;
                per_index[slot].push((*idx, have));
            }
        }
        for slot in 0..2 {
            if available[slot] < need[slot] as i32 {
                self.send_awakening_result(msg.session_id, AWAKE_RESULT_NO_MATERIALS, -1);
                return;
            }
        }

        for slot in 0..2 {
            let mut remaining = need[slot] as i32;
            for (idx, have) in &per_index[slot] {
                if remaining <= 0 {
                    break;
                }
                let take = (*have).min(remaining);
                if take <= 0 {
                    continue;
                }
                let consumed = record
                    .actor_ref
                    .ask(crate::actors::player::ConsumeItemsByIndex {
                        item_index: *idx,
                        count: take as u16,
                    })
                    .await
                    .unwrap_or(false);
                if !consumed {
                    self.send_awakening_result(msg.session_id, AWAKE_RESULT_NO_MATERIALS, -1);
                    return;
                }
                remaining -= take;
            }
        }

        // 扣除金币
        let gold_deducted = record
            .actor_ref
            .ask(crate::actors::player::DeductGold { amount: gold_cost })
            .await;
        if !gold_deducted.unwrap_or(false) {
            self.send_awakening_result(msg.session_id, AWAKE_RESULT_NO_GOLD, -1);
            return;
        }

        // 执行觉醒：70% 成功率
        let roll = fastrand::u8(0..100);
        if awakening_roll_succeeds(roll, self.awakening_cfg.success_rate) {
            // 成功：计算觉醒值（#2416：AwakeningSystem.ini 配置化）
            let chance_max = self
                .awakening_cfg
                .chance_max
                .get(grade.saturating_sub(1) as usize)
                .copied()
                .unwrap_or(1);
            let rate = match item_info.item_type {
                1 => self.awakening_cfg.weapon_rate, // Weapon
                4 => self.awakening_cfg.helmet_rate, // Helmet
                2 => self.awakening_cfg.armor_rate,  // Armour
                _ => 1,
            };
            let (value, hits) = awake_roll(chance_max, self.awakening_cfg.hit_rate, rate);

            let mut awake = item.awake.clone();
            awake.awake_type = awake_type;
            awake.levels.push(value);

            let set = record
                .actor_ref
                .ask(crate::actors::player::SetItemAwake {
                    unique_id: msg.unique_id,
                    awake,
                })
                .await
                .unwrap_or(false);

            if set {
                debug!(
                    "Awakening success: {} item={} type={:?} value={}",
                    state.name, msg.unique_id, awake_type, value
                );
                // C# :8869 AwakeningEffect(true, isHit)——命中/成败特效广播
                self.send_awakening_effects(state.object_id, state.map_index, true, &hits)
                    .await;
                self.send_awakening_result(msg.session_id, AWAKE_RESULT_SUCCESS, -1);
                send_system_message(
                    &self.gate_ref,
                    msg.session_id,
                    &format!("觉醒成功！{} +{}", awake_type_name(awake_type), value),
                );
            } else {
                self.send_awakening_result(msg.session_id, AWAKE_RESULT_FAIL, -1);
            }
        } else {
            // 失败：物品被摧毁
            // C# `UpgradeAwake` 失败分支 `isHit = MakeHit(1, out _)`（ItemData.cs:1004）→ 同样播 5 次命中/未命中
            let (_, hits) = awake_roll(1, self.awakening_cfg.hit_rate, 1);
            let removed = record
                .actor_ref
                .ask(crate::actors::player::RemoveItemFromInventory {
                    unique_id: msg.unique_id,
                })
                .await
                .ok()
                .flatten();
            if removed.is_some() {
                debug!(
                    "Awakening destroy: {} item={} destroyed",
                    state.name, msg.unique_id
                );
                // C# :8863 AwakeningEffect(false, isHit)
                self.send_awakening_effects(state.object_id, state.map_index, false, &hits)
                    .await;
                self.send_awakening_result(
                    msg.session_id,
                    AWAKE_RESULT_DESTROYED,
                    msg.unique_id as i64,
                );
                send_system_message(&self.gate_ref, msg.session_id, "觉醒失败，物品已损毁！");
            } else {
                self.send_awakening_result(msg.session_id, AWAKE_RESULT_FAIL, -1);
            }
        }
    }
}

pub struct DowngradeAwakeningRequest {
    pub session_id: u64,
    pub unique_id: u64,
}

impl Message<DowngradeAwakeningRequest> for WorldActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: DowngradeAwakeningRequest,
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

        // C# Downgrade（:8891）：租赁物品不可降级
        if item.rental_information.is_some() {
            let owner = item
                .rental_information
                .as_ref()
                .map(|r| r.owner_name.clone())
                .unwrap_or_default();
            send_system_message(
                &self.gate_ref,
                msg.session_id,
                &format!("该物品属于 {}，无法降级", owner),
            );
            return;
        }

        if item.awake.awake_level() == 0 {
            send_system_message(&self.gate_ref, msg.session_id, "该物品没有觉醒等级");
            return;
        }

        let item_info = match self.item_infos.get(&item.item_index) {
            Some(i) => i,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "物品信息不存在");
                return;
            }
        };

        let grade = match item_info.grade {
            1..=4 => item_info.grade,
            _ => 1,
        };

        // 降级费用 = 3000 * (1 + (awakeLevel+1) * 2) * grade
        let awake_level = item.awake.awake_level() as u64;
        let gold_cost = 3000u64 * (1 + (awake_level + 1) * 2) * grade as u64;

        let has_gold = record
            .actor_ref
            .ask(crate::actors::player::HasGold { amount: gold_cost })
            .await
            .unwrap_or(false);
        if !has_gold {
            send_system_message(
                &self.gate_ref,
                msg.session_id,
                &format!("金币不足，降级需要 {} 金币", gold_cost),
            );
            return;
        }

        let gold_deducted = record
            .actor_ref
            .ask(crate::actors::player::DeductGold { amount: gold_cost })
            .await;
        if !gold_deducted.unwrap_or(false) {
            send_system_message(&self.gate_ref, msg.session_id, "金币扣除失败");
            return;
        }

        // 移除最后一级觉醒
        let mut awake = item.awake.clone();
        awake.levels.pop();
        if awake.levels.is_empty() {
            awake.awake_type = mir2_shared::enums::AwakeType::None;
        }

        let set = record
            .actor_ref
            .ask(crate::actors::player::SetItemAwake {
                unique_id: msg.unique_id,
                awake,
            })
            .await
            .unwrap_or(false);

        if set {
            debug!(
                "DowngradeAwakening: {} item={} new_level={}",
                state.name,
                msg.unique_id,
                item.awake.awake_level().saturating_sub(1)
            );
            send_system_message(&self.gate_ref, msg.session_id, "觉醒降级成功");
        } else {
            send_system_message(&self.gate_ref, msg.session_id, "降级失败");
        }
    }
}

pub fn awake_type_name(t: mir2_shared::enums::AwakeType) -> &'static str {
    match t {
        mir2_shared::enums::AwakeType::Dc => "攻击",
        mir2_shared::enums::AwakeType::Mc => "魔法",
        mir2_shared::enums::AwakeType::Sc => "道术",
        mir2_shared::enums::AwakeType::Ac => "防御",
        mir2_shared::enums::AwakeType::Mac => "魔防",
        mir2_shared::enums::AwakeType::HpMp => "生命/魔法",
        _ => "未知",
    }
}

/// C# Awake.MakeHit：给定命中次数 / chance_max / itemRate，确定性计算觉醒值。
/// stepValue = chance_max / 5；makeValue = total <= 1 ? 1 : floor(total)；value = max(1, makeValue * rate)。
/// SharedRust `AwakeType`（None=3 … HpMp=9）→ C# `AwakeType`（None=0 … HPMP=6）
/// （C# 枚举 `Shared/Enums.cs:105-114` 为 0-based 1..=6 对应 DC..HPMP）
pub(crate) fn awake_type_csharp(awake_type: mir2_shared::enums::AwakeType) -> i32 {
    (awake_type as u8).saturating_sub(3) as i32
}

/// C# 觉醒材料 shape 约定 = `(int)AwakeType - 1`（`PlayerObject.cs:9076`/`:9131`），
/// 与真实物品库一致：0=DC(BraveryGlyph) / 1=MC / 2=SC / 3=AC / 4=MAC / 5=HPMP(BodyGlyph)，100=通用。
pub(crate) fn awake_type_shape(awake_type: mir2_shared::enums::AwakeType) -> i32 {
    awake_type_csharp(awake_type) - 1
}

/// C# `Awake.AwakeMaterials` + `AwakeMaterialRate`（`PlayerObject.cs:9110-9119` / `:9053-9067`）：
/// `need[slot] = base[type-1][slot][grade-1] + (byte)(material_rate[grade-1] * level)`
/// （C# 先 float 相乘再截断成 byte，再按 byte 相加；本端用 `wrapping_add` 保持同样的溢出语义）。
pub(crate) fn awakening_material_need(
    cfg: &crate::util::ini::AwakeningIniSettings,
    awake_type: mir2_shared::enums::AwakeType,
    grade: i32,
    level: u32,
) -> [u8; 2] {
    let type_idx = (awake_type_csharp(awake_type) - 1).clamp(0, 5) as usize;
    let grade_idx = (grade - 1).clamp(0, 4) as usize;
    let extra = (cfg.material_rate[grade_idx] * level as f32) as u8; // C# (byte) 截断
    [
        cfg.materials_base[type_idx][0][grade_idx].wrapping_add(extra),
        cfg.materials_base[type_idx][1][grade_idx].wrapping_add(extra),
    ]
}

/// C# `rand.Next(0,100) <= AwakeSuccessRate`（`Shared/Data/ItemData.cs:997`）：
/// roll ∈ 0..=99，命中区间 = `0..=rate`（rate=70 → 71/100 成功）。
pub(crate) fn awakening_roll_succeeds(roll: u8, success_rate: u8) -> bool {
    roll <= success_rate
}

pub(crate) fn awake_value_from_hits(hit_count: u8, chance_max: u8, rate: u8) -> u8 {
    let step = chance_max as f32 / 5.0;
    let total = step * hit_count as f32;
    let make = if total <= 1.0 { 1 } else { total as i32 };
    (make * rate as i32).max(1) as u8
}

/// C# Awake.MakeHit：5 次 Bernoulli(hit_rate) 命中累加，返回最终觉醒值。
pub(crate) fn awake_roll_value(chance_max: u8, hit_rate: u8, rate: u8) -> u8 {
    awake_roll(chance_max, hit_rate, rate).0
}

/// C# `Awake.MakeHit`（`Shared/Data/ItemData.cs:1036-1058`）：5 次 Bernoulli(hit_rate) 命中，
/// 返回 `(觉醒值, 逐次命中表)`——命中表用于客户端表现（`AwakeningEffect` 的 Hit/Miss 序列）。
pub(crate) fn awake_roll(chance_max: u8, hit_rate: u8, rate: u8) -> (u8, [bool; 5]) {
    let mut ordered = [false; 5];
    let mut count = 0u8;
    for slot in ordered.iter_mut() {
        if fastrand::u8(0..100) < hit_rate {
            *slot = true;
            count += 1;
        }
    }
    (awake_value_from_hits(count, chance_max, rate), ordered)
}

pub struct ResetAddedItemRequest {
    pub session_id: u64,
    pub unique_id: u64,
}
impl WorldActor {
    /// C# @AWAKENING（PlayerObject.cs:3518-3559）：GM/TestServer 按 ItemType 直接升级装备觉醒（无金币/材料，失败不销毁）
    pub(crate) async fn gm_awakening(
        &mut self,
        session_id: u64,
        item_type: mir2_shared::enums::ItemType,
        awake_type: mir2_shared::enums::AwakeType,
    ) {
        let record = match self.players.get(&session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // C#：遍历 Info.Equipment，真实类型匹配则 UpgradeAwake
        for item in state.inventory.equipment.iter().flatten() {
            let Some(info) = item.info.as_ref() else {
                continue;
            };
            if info.item_type != item_type {
                continue;
            }
            let Some(item_info) = self.item_infos.get(&item.item_index) else {
                continue;
            };

            // C# CheckAwakening：可觉醒 / 未满级 / 类型兼容 / 品级（#2416：MaxUpgradeLevel 配置化）
            if !item_info.can_awakening
                || item.awake.awake_level() >= self.awakening_cfg.max_awake_level
            {
                send_system_message(
                    &self.gate_ref,
                    session_id,
                    &format!("条件不符：{}", item_info.name),
                );
                continue;
            }
            if item.awake.awake_type != mir2_shared::enums::AwakeType::None
                && item.awake.awake_type != awake_type
            {
                send_system_message(
                    &self.gate_ref,
                    session_id,
                    &format!("条件不符：{}", item_info.name),
                );
                continue;
            }
            let compatible = match info.item_type {
                mir2_shared::enums::ItemType::Weapon => matches!(
                    awake_type,
                    mir2_shared::enums::AwakeType::Dc
                        | mir2_shared::enums::AwakeType::Mc
                        | mir2_shared::enums::AwakeType::Sc
                ),
                mir2_shared::enums::ItemType::Helmet => matches!(
                    awake_type,
                    mir2_shared::enums::AwakeType::Ac | mir2_shared::enums::AwakeType::Mac
                ),
                mir2_shared::enums::ItemType::Armour => {
                    awake_type == mir2_shared::enums::AwakeType::HpMp
                }
                _ => false,
            };
            let grade = item_info.grade;
            if !compatible || !(1..=4).contains(&grade) {
                send_system_message(
                    &self.gate_ref,
                    session_id,
                    &format!("条件不符：{}", item_info.name),
                );
                continue;
            }

            // C# UpgradeAwake：70% 成功；失败仅提示（GM 语义不销毁物品，#2416 配置化）
            let roll = fastrand::u8(0..100);
            if roll >= self.awakening_cfg.success_rate {
                send_system_message(
                    &self.gate_ref,
                    session_id,
                    &format!("觉醒失败：{}", item_info.name),
                );
                continue;
            }

            // 成功：计算觉醒值（与 NPC 觉醒一致：grade→chance_max，item_type→rate）
            let chance_max = self
                .awakening_cfg
                .chance_max
                .get(grade.saturating_sub(1) as usize)
                .copied()
                .unwrap_or(1);
            let rate = match info.item_type {
                mir2_shared::enums::ItemType::Weapon => self.awakening_cfg.weapon_rate,
                mir2_shared::enums::ItemType::Helmet => self.awakening_cfg.helmet_rate,
                mir2_shared::enums::ItemType::Armour => self.awakening_cfg.armor_rate,
                _ => 1,
            };
            let value = awake_roll_value(chance_max, self.awakening_cfg.hit_rate, rate);

            let mut new_awake = item.awake.clone();
            new_awake.awake_type = awake_type;
            new_awake.levels.push(value);
            let mut snapshot = item.clone();
            snapshot.awake = new_awake.clone();
            let ok = record
                .actor_ref
                .ask(crate::actors::player::SetItemAwake {
                    unique_id: item.unique_id,
                    awake: new_awake,
                })
                .await
                .unwrap_or(false);
            if ok {
                // C#：Enqueue(new S.RefreshItem { Item = temp })
                let pkt = mir2_shared::packets::server::item::RefreshItem { item: snapshot };
                let mut body = Vec::new();
                if pkt.write_body(&mut body).is_ok() {
                    if self
                        .gate_ref
                        .tell(SendToClient {
                            session_id,
                            data: build_packet_bytes(
                                mir2_shared::enums::ServerPacketIds::RefreshItem as i16,
                                &body,
                            ),
                        })
                        .try_send()
                        .is_err()
                    {
                        warn!("gate mailbox full: SendToClient dropped (session={} packet=RefreshItem)", session_id);
                    }
                }
                send_system_message(
                    &self.gate_ref,
                    session_id,
                    &format!("觉醒成功！{} +{}", awake_type_name(awake_type), value),
                );
            }
        }
    }
}

/// #2058：C# ResetPrice（ItemData.cs:605-611）——3000*Grade*(AddedStats.Count*0.2+1)
fn reset_price(item: &mir2_shared::data::item::UserItem, info: &db::ItemInfo) -> u64 {
    let grade = info.grade.max(1) as u64;
    let stats = item.added_stats.len() as u64;
    ((3000u64 * grade) as f64 * (stats as f64 * 0.2 + 1.0)) as u64
}

impl Message<ResetAddedItemRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: ResetAddedItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(),
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // #2058：C# ResetPrice——费用 3000*Grade*(AddedStats.Count*0.2+1)
        let item = match record
            .actor_ref
            .ask(GetItemInfo {
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

        // C# Reset（:9001）：租赁物品不可重置
        if item.rental_information.is_some() {
            let owner = item
                .rental_information
                .as_ref()
                .map(|r| r.owner_name.clone())
                .unwrap_or_default();
            send_system_message(
                &self.gate_ref,
                msg.session_id,
                &format!("该物品属于 {}，无法重置", owner),
            );
            return;
        }

        let Some(item_info) = self.item_infos.get(&item.item_index).cloned() else {
            send_system_message(&self.gate_ref, msg.session_id, "物品信息不存在");
            return;
        };
        let gold_cost = reset_price(&item, &item_info);
        if state.inventory.gold < gold_cost {
            send_system_message(
                &self.gate_ref,
                msg.session_id,
                &format!("金币不足，重置需要 {} 金币", gold_cost),
            );
            return;
        }
        let _ = record
            .actor_ref
            .ask(crate::actors::player::DeductGold { amount: gold_cost })
            .await;
        super::send_gold_changed_packet(&self.gate_ref, msg.session_id, gold_cost);

        let success = record
            .actor_ref
            .ask(crate::actors::player::ResetItemAddedStats {
                unique_id: msg.unique_id,
            })
            .await
            .unwrap_or(false);
        if success {
            send_system_message(&self.gate_ref, msg.session_id, "物品附加属性已重置");
            debug!(
                "ResetAddedItem: {} uid={} - success",
                state.name, msg.unique_id
            );
        } else {
            send_system_message(&self.gate_ref, msg.session_id, "找不到该物品或无法重置");
            debug!(
                "ResetAddedItem: {} uid={} - failed",
                state.name, msg.unique_id
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_price_matches_csharp() {
        // #2058：C# ResetPrice = 3000*Grade*(AddedStats.Count*0.2+1)
        use mir2_shared::data::item::UserItem;
        use mir2_shared::data::stats::Stats;
        use mir2_shared::enums::Stat;
        let mut added = Stats::new();
        added.set(Stat::Luck, 50);
        added.set(Stat::Agility, 30); // 2 条
        let item = UserItem {
            item_index: 1,
            added_stats: added,
            ..Default::default()
        };
        let info = crate::db::ItemInfo {
            index: 1,
            grade: 2,
            ..Default::default()
        };
        // 3000*2 * (2*0.2+1) = 6000 * 1.4 = 8400
        assert_eq!(reset_price(&item, &info), 8400);
        // 无附加：3000*2*1.0 = 6000
        let plain = UserItem {
            item_index: 1,
            ..Default::default()
        };
        assert_eq!(reset_price(&plain, &info), 6000);
    }

    #[test]
    fn refine_cost_matches_csharp() {
        // #2034：C# RefineItem——cost = (RequiredAmount*10)*RefineCost(125)
        assert_eq!(refine_cost(0, 125), 0);
        assert_eq!(refine_cost(1, 125), 1250);
        assert_eq!(refine_cost(10, 125), 12500);
        assert_eq!(refine_cost(-5, 125), 0); // RequiredAmount 负数按 0
    }

    #[test]
    fn awake_value_from_hits_matches_csharp_makehit() {
        // 0 命中：total=0 → makeValue=1 → value=rate
        assert_eq!(awake_value_from_hits(0, 5, 1), 1);
        assert_eq!(awake_value_from_hits(0, 5, 5), 5);
        // 1 命中（step=1.0）：total=1.0 → makeValue=1 → value=rate
        assert_eq!(awake_value_from_hits(1, 5, 1), 1);
        // 2 命中：total=2.0 → makeValue=2
        assert_eq!(awake_value_from_hits(2, 5, 1), 2);
        // 5 命中：total=5.0 → makeValue=5；rate=5 → 25
        assert_eq!(awake_value_from_hits(5, 5, 1), 5);
        assert_eq!(awake_value_from_hits(5, 5, 5), 25);
        // chance_max=1（step=0.2）：全命中 total=1.0 → makeValue=1
        assert_eq!(awake_value_from_hits(5, 1, 5), 5);
        // chance_max=3（step=0.6）：3 命中 total=1.8 → floor=1
        assert_eq!(awake_value_from_hits(3, 3, 1), 1);
        // chance_max=3（step=0.6）：4 命中 total=2.4 → floor=2
        assert_eq!(awake_value_from_hits(4, 3, 1), 2);
    }

    /// #2829：C# 材料 shape 约定 = `(int)AwakeType - 1`（真实物品库与之一致）
    #[test]
    fn awake_type_shape_matches_csharp_material_shapes() {
        use mir2_shared::enums::AwakeType;
        assert_eq!(awake_type_csharp(AwakeType::Dc), 1);
        assert_eq!(awake_type_csharp(AwakeType::HpMp), 6);
        assert_eq!(awake_type_shape(AwakeType::Dc), 0); // BraveryGlyph
        assert_eq!(awake_type_shape(AwakeType::Mc), 1); // MagicGlyph
        assert_eq!(awake_type_shape(AwakeType::Sc), 2); // SoulGlyph
        assert_eq!(awake_type_shape(AwakeType::Ac), 3); // ProtectionGlyph
        assert_eq!(awake_type_shape(AwakeType::Mac), 4); // EvilSlayerGlyph
        assert_eq!(awake_type_shape(AwakeType::HpMp), 5); // BodyGlyph
    }

    /// #2829：需求数量 = ini 表 + 品级系数 × 等级（C# `:9115`）
    #[test]
    fn awakening_material_need_matches_csharp_table() {
        use crate::util::ini::AwakeningIniSettings;
        use mir2_shared::enums::AwakeType;

        // 默认表（base=1、rate=1.0）：Lv0 → [1,1]，Lv3 → [4,4]
        let cfg = AwakeningIniSettings::default();
        assert_eq!(awakening_material_need(&cfg, AwakeType::Dc, 1, 0), [1, 1]);
        assert_eq!(awakening_material_need(&cfg, AwakeType::Dc, 4, 3), [4, 4]);
        // 不同类型/品级取各自表项：DC 与 HPMP 独立
        let mut cfg = AwakeningIniSettings::default();
        cfg.materials_base[0][0][0] = 5; // DC / slot0 / Common
        cfg.materials_base[0][1][0] = 2; // DC / slot1 / Common
        cfg.materials_base[5][0][3] = 7; // HPMP / slot0 / Mythical
        cfg.material_rate[0] = 2.0; // Common 每级 +2
        assert_eq!(awakening_material_need(&cfg, AwakeType::Dc, 1, 0), [5, 2]);
        assert_eq!(awakening_material_need(&cfg, AwakeType::Dc, 1, 3), [11, 8]);
        assert_eq!(awakening_material_need(&cfg, AwakeType::HpMp, 4, 0), [7, 1]);
        // rate 非整数：截断（C# (byte)(float)）
        cfg.material_rate[1] = 0.5; // Rare 每级 +0.5
        assert_eq!(awakening_material_need(&cfg, AwakeType::Mc, 2, 1), [1, 1]);
        assert_eq!(awakening_material_need(&cfg, AwakeType::Mc, 2, 3), [2, 2]);
    }

    /// #2829：成功率边界对齐 C# `rand.Next(0,100) <= rate`（rate=70 → 71/100）
    #[test]
    fn awakening_roll_succeeds_matches_csharp_boundary() {
        assert!(awakening_roll_succeeds(0, 0)); // rate=0 仍有 1%（roll==0）
        assert!(!awakening_roll_succeeds(1, 0));
        assert!(awakening_roll_succeeds(70, 70)); // 边界：70 成功（C# <=）
        assert!(!awakening_roll_succeeds(71, 70));
        assert!(awakening_roll_succeeds(99, 99));
        assert!(!awakening_roll_succeeds(99, 98));
    }

    /// #2832：`awake_roll` 的命中表与觉醒值自洽（表现层 Hit/Miss 序列直接用这张表）
    #[test]
    fn awake_roll_hits_match_value() {
        for _ in 0..200 {
            let (value, hits) = awake_roll(5, 70, 1);
            let count = hits.iter().filter(|h| **h).count() as u8;
            assert_eq!(value, awake_value_from_hits(count, 5, 1));
            assert!(value >= 1);
        }
        // hit_rate=0 → 全不命中；hit_rate=100 → 全命中（C# `rand < rate` 语义）
        let (value, hits) = awake_roll(5, 0, 1);
        assert!(hits.iter().all(|h| !*h));
        assert_eq!(value, awake_value_from_hits(0, 5, 1));
        let (value, hits) = awake_roll(5, 100, 1);
        assert!(hits.iter().all(|h| *h));
        assert_eq!(value, awake_value_from_hits(5, 5, 1));
    }
}
