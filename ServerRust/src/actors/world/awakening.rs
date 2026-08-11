use super::*;

/// 存入精炼物品（C# DepositRefineItem：From=背包格、To=精炼栏格）
pub struct DepositRefineItemRequest {
    pub session_id: u64,
    /// C# C.DepositRefineItem.From：背包格索引
    pub from: i32,
    /// C# C.DepositRefineItem.To：精炼栏格索引（Rust 单槽，须为 0）
    pub to: i32,
}

impl Message<DepositRefineItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: DepositRefineItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // C# DepositRefineItem（:12529-12559）：From 为背包格、To 为精炼栏格（Rust 单槽 to=0）
        let from = msg.from;
        if from < 0 || from as usize >= state.inventory.backpack.len() {
            send_system_message(&self.gate_ref, msg.session_id, "物品不存在");
            return;
        }
        if msg.to != 0 {
            send_system_message(&self.gate_ref, msg.session_id, "精炼栏已满");
            return;
        }
        let Some(item) = state.inventory.backpack[from as usize].as_ref().map(|s| s.item.clone()) else {
            send_system_message(&self.gate_ref, msg.session_id, "物品不存在");
            return;
        };

        // #926：C# BindMode.DontUpgrade(0x40)：不可精炼/升级
        if self.item_infos.get(&item.item_index)
            .map(|i| super::has_bind_flag(i.bind_mode, mir2_shared::enums::BindMode::DONT_UPGRADE.bits()))
            .unwrap_or(false)
        {
            send_system_message(&self.gate_ref, msg.session_id, "该物品无法精炼");
            return;
        }

        // C# RefineItem（:12705）：从背包移除物品并存入精炼日志（含完整物品数据）
        let Some(item) = record.actor_ref.ask(crate::actors::player::RemoveItemFromInventory { unique_id: item.unique_id }).await.unwrap_or(None) else {
            send_system_message(&self.gate_ref, msg.session_id, "物品不存在");
            return;
        };
        // 更新精炼日志
        let mut log = state.refine_log;
        if !log.deposit_item(item) {
            send_system_message(&self.gate_ref, msg.session_id, "已有精炼进行中");
            return;
        }

        let _ = record.actor_ref.ask(SetRefineLog { refine_log: log }).await;
        send_system_message(&self.gate_ref, msg.session_id, "精炼物品已存入");
        debug!("DepositRefineItem: {} from={}", state.name, from);
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

impl Message<RetrieveRefineItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: RetrieveRefineItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // C# RetrieveRefineItem（:12568-12600）：From=精炼栏格（Rust 单槽 0）、To=背包格
        if msg.from != 0 || state.refine_log.active_refine.is_none() {
            send_system_message(&self.gate_ref, msg.session_id, "没有精炼物品可取回");
            return;
        }
        let to = msg.to;
        if to < 0 || to as usize >= state.inventory.backpack.len() || state.inventory.backpack[to as usize].is_some() {
            send_system_message(&self.gate_ref, msg.session_id, "该背包格子已被占用");
            return;
        }

        let mut log = state.refine_log.clone();
        if let Some(ri) = log.retrieve() {
            // C# RetrieveRefineItem（:12590）：返还到指定背包格
            if let Some(item) = ri.item {
                let ok = record.actor_ref.ask(crate::actors::player::PlaceItemAtSlot { slot: to, item: item.clone() }).await.unwrap_or(false);
                if !ok {
                    // 放置失败回放精炼栏（避免物品丢失）
                    let mut log2 = state.refine_log.clone();
                    if log2.deposit_item(item) {
                        let _ = record.actor_ref.ask(SetRefineLog { refine_log: log2 }).await;
                    }
                    send_system_message(&self.gate_ref, msg.session_id, "背包已满");
                    return;
                }
            }
            let _ = record.actor_ref.ask(SetRefineLog { refine_log: log }).await;
            send_system_message(&self.gate_ref, msg.session_id, "精炼物品已取回");
            debug!("RetrieveRefineItem: {} to={}", state.name, to);
        }
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

        if state.refine_log.active_refine.is_none() {
            send_system_message(&self.gate_ref, msg.session_id, "没有精炼可取消");
            return;
        }

        let mut log = state.refine_log;
        if let Some(ri) = log.cancel() {
            // 取消同样返还物品（C# CollectRefine 返还语义）
            if let Some(item) = ri.item {
                let _ = record.actor_ref.ask(crate::actors::player::AddItemToInventory { item }).await;
            }
        }
        let _ = record.actor_ref.ask(SetRefineLog { refine_log: log }).await;
        send_system_message(&self.gate_ref, msg.session_id, "精炼已取消");
        debug!("RefineCancel: {}", state.name);
    }
}

/// 精炼费用系数（C# Settings.RefineCost = 125）
const REFINE_COST: i32 = 125;
/// 精炼时长（分钟，C# Settings.RefineTime = 20）
const REFINE_TIME_MINUTES: u64 = 20;

/// #2034：C# RefineItem（12676）——费用 (RequiredAmount*10)*RefineCost
fn refine_cost(required_amount: i32) -> u64 {
    ((required_amount.max(0) * 10) * REFINE_COST) as u64
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
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.actor_ref.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // C# RefineItem（:12649-12658）：按 unique_id 校验精炼栏物品
        let deposited = state.refine_log.active_refine.as_ref().and_then(|ri| ri.item.clone());
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
        // C# Settings.OnlyRefineWeapon = true：仅武器（ItemType.Weapon=1）
        if item_db.item_type != 1 {
            send_system_message(&self.gate_ref, msg.session_id, "只有武器可以精炼");
            return;
        }
        // C# BindMode.DontUpgrade(0x40)：不可精炼（与 DepositRefineItem #926 一致）
        if super::has_bind_flag(item_db.bind_mode, mir2_shared::enums::BindMode::DONT_UPGRADE.bits()) {
            send_system_message(&self.gate_ref, msg.session_id, "该物品无法精炼");
            return;
        }
        let cost = refine_cost(item_db.required_amount);
        if state.inventory.gold < cost {
            send_system_message(&self.gate_ref, msg.session_id, "金币不足，无法精炼");
            return;
        }
        let _ = record.actor_ref.ask(crate::actors::player::DeductGold { amount: cost }).await;
        super::send_gold_changed_packet(&self.gate_ref, msg.session_id, cost);

        // 开始精炼（C# Settings.RefineTime=20 分钟完成；成功率公式待材料系统，暂保持 80%）
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let mut log = state.refine_log;
        let duration = REFINE_TIME_MINUTES * 60; // C# Settings.RefineTime=20 分钟
        // C# RefineItem 成功率公式（:12811-12845）；材料槽未实现时材料输入全 0 → 无材料恒失败（C# :12753）
        let luck = deposited.added_stats.get(mir2_shared::enums::Stat::Luck);
        let added_dc = deposited.added_stats.get(mir2_shared::enums::Stat::MaxDC);
        let added_mc = deposited.added_stats.get(mir2_shared::enums::Stat::MaxMC);
        let added_sc = deposited.added_stats.get(mir2_shared::enums::Stat::MaxSC);
        let success_chance = crate::actors::refine::refine_success_chance(
            0, item_db.required_amount, 0, 0, 0, 0, 0, 0,
            luck, added_dc, added_mc, added_sc, true,
        ).clamp(0, 255) as u8;
        if !log.begin_refine(current_time, duration, success_chance) {
            send_system_message(&self.gate_ref, msg.session_id, "没有待精炼的物品");
            return;
        }
        let _ = record.actor_ref.ask(SetRefineLog { refine_log: log }).await;

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
                // 精炼完成，自动标记为完成
                let mut log = state.refine_log;
                let success = log.finish();
                let _ = record.actor_ref.ask(SetRefineLog { refine_log: log }).await;
                if success {
                    send_system_message(&self.gate_ref, msg.session_id, "精炼成功！请取回物品");
                } else {
                    send_system_message(&self.gate_ref, msg.session_id, "精炼失败，请取回物品");
                }
                debug!("CheckRefine: {} result={}", state.name, success);
            } else if item.status == RefineStatus::Ready {
                send_system_message(&self.gate_ref, msg.session_id, "精炼已完成，请取回物品");
            } else {
                let remaining = item.finish_time.saturating_sub(current_time);
                send_system_message(&self.gate_ref, msg.session_id, &format!("精炼进行中，剩余 {} 秒", remaining));
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

    async fn handle(&mut self, msg: AwakeningNeedMaterialsRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };

        let _awake_type = match mir2_shared::enums::AwakeType::try_from(msg.awake_type) {
            Ok(t) => t,
            Err(_) => {
                send_system_message(&self.gate_ref, msg.session_id, "无效的觉醒类型");
                return;
            }
        };

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

        if !item_info.can_awakening {
            send_system_message(&self.gate_ref, msg.session_id, "该物品不支持觉醒");
            return;
        }

        // 计算所需材料：觉醒材料是 item_type=35 的物品
        // shape 编码：0=DC, 1=MC, 2=SC, 3=AC, 4=MAC, 5=HpMp, 100=通用
        let awake_level = item.awake.awake_level();
        let grade_index = match item_info.grade {
            1..=4 => item_info.grade - 1,
            _ => {
                send_system_message(&self.gate_ref, msg.session_id, "该物品品级不支持觉醒");
                return;
            }
        };

        // 材料数量 = 基础值 * (1 + 已觉醒等级)
        let base_count: i32 = match grade_index {
            0 => 3,
            1 => 5,
            2 => 8,
            _ => 12,
        };
        let needed = base_count * (1 + awake_level as i32);

        // 查找匹配的觉醒材料物品
        let type_shape = msg.awake_type.saturating_sub(1) as i32;
        let mut materials = Vec::new();
        for (idx, info) in self.item_infos.iter() {
            if info.item_type != 35 { continue; } // ItemType::Awakening
            if info.shape == type_shape || info.shape == 100 {
                materials.push(mir2_shared::packets::server::awakening_system::MaterialInfo {
                    item_id: *idx,
                    count: needed,
                });
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
        let _ = self.gate_ref.tell(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::AwakeningNeedMaterials as i16, &body),
        }).await;
    }
}

pub struct AwakeningLockedItemRequest {
    pub session_id: u64,
    pub unique_id: u64,
    pub locked: bool,
}

impl Message<AwakeningLockedItemRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: AwakeningLockedItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let packet = mir2_shared::packets::server::awakening_system::AwakeningLockedItem {
            unique_id: msg.unique_id,
            locked: msg.locked,
        };
        let mut body = Vec::new();
        if let Err(e) = packet.write_body(&mut body) {
            warn!("Failed to serialize AwakeningLockedItem: {}", e);
            return;
        }
        let _ = self.gate_ref.tell(SendToClient {
            session_id: msg.session_id,
            data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::AwakeningLockedItem as i16, &body),
        }).await;
    }
}

pub struct AwakeningRequest {
    pub session_id: u64,
    pub unique_id: u64,
    pub awake_type: u8,
}

impl Message<AwakeningRequest> for WorldActor {
    type Reply = ();

    async fn handle(&mut self, msg: AwakeningRequest, _ctx: &mut Context<Self, Self::Reply>) {
        use mir2_shared::packets::server::awakening_system::*;

        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        let awake_type = match mir2_shared::enums::AwakeType::try_from(msg.awake_type) {
            Ok(t) => t,
            Err(_) => {
                self.send_awakening_result(msg.session_id, AWAKE_RESULT_FAIL, -1);
                return;
            }
        };

        let item = match record.actor_ref.ask(crate::actors::player::GetItemInfo { unique_id: msg.unique_id }).await {
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

        // 验证：物品可觉醒
        if !item_info.can_awakening {
            self.send_awakening_result(msg.session_id, AWAKE_RESULT_FAIL, -1);
            return;
        }

        // 验证：未达最大等级
        if item.awake.is_max_level() {
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
            1 => matches!(awake_type, mir2_shared::enums::AwakeType::Dc | mir2_shared::enums::AwakeType::Mc | mir2_shared::enums::AwakeType::Sc),
            4 => matches!(awake_type, mir2_shared::enums::AwakeType::Ac | mir2_shared::enums::AwakeType::Mac),
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
        let has_gold = record.actor_ref.ask(crate::actors::player::HasGold { amount: gold_cost }).await.unwrap_or(false);
        if !has_gold {
            self.send_awakening_result(msg.session_id, AWAKE_RESULT_NO_GOLD, -1);
            return;
        }

        // 检查材料：计算所需数量
        let base_count: u16 = match grade {
            1 => 3,
            2 => 5,
            3 => 8,
            _ => 12,
        };
        let needed = base_count * (1 + awake_level as u16);

        // 查找匹配的觉醒材料
        let type_shape = msg.awake_type.saturating_sub(1) as i32;
        let mut material_index: Option<i32> = None;
        for (idx, info) in self.item_infos.iter() {
            if info.item_type == 35 // ItemType::Awakening
                && (info.shape == type_shape || info.shape == 100)
            {
                material_index = Some(*idx);
                break;
            }
        }
        let mat_idx = match material_index {
            Some(idx) => idx,
            None => {
                // 没有配置觉醒材料，跳过材料检查
                0
            }
        };

        // 检查材料数量
        if mat_idx > 0 {
            let available = record.actor_ref.ask(crate::actors::player::CountItemsByIndex {
                item_index: mat_idx,
            }).await.unwrap_or(0);
            if available < needed {
                self.send_awakening_result(msg.session_id, AWAKE_RESULT_NO_MATERIALS, -1);
                return;
            }
        }

        // 扣除材料
        if mat_idx > 0 {
            let consumed = record.actor_ref.ask(crate::actors::player::ConsumeItemsByIndex {
                item_index: mat_idx,
                count: needed,
            }).await.unwrap_or(false);
            if !consumed {
                self.send_awakening_result(msg.session_id, AWAKE_RESULT_NO_MATERIALS, -1);
                return;
            }
        }

        // 扣除金币
        let gold_deducted = record.actor_ref.ask(crate::actors::player::DeductGold { amount: gold_cost }).await;
        if !gold_deducted.unwrap_or(false) {
            self.send_awakening_result(msg.session_id, AWAKE_RESULT_NO_GOLD, -1);
            return;
        }

        // 执行觉醒：70% 成功率
        let roll = fastrand::u8(0..100);
        if roll < mir2_shared::data::item::Awake::SUCCESS_RATE {
            // 成功：计算觉醒值
            let chance_max = mir2_shared::data::item::Awake::CHANCE_MAX
                .get(grade.saturating_sub(1) as usize)
                .copied()
                .unwrap_or(1);
            let rate = match item_info.item_type {
                1 => mir2_shared::data::item::Awake::WEAPON_RATE,  // Weapon
                4 => mir2_shared::data::item::Awake::HELMET_RATE,  // Helmet
                2 => mir2_shared::data::item::Awake::ARMOUR_RATE,  // Armour
                _ => 1,
            };
            let value = (fastrand::u8(1..=chance_max) as i32 * rate as i32).max(1) as u8;

            let mut awake = item.awake.clone();
            awake.awake_type = awake_type;
            awake.levels.push(value);

            let set = record.actor_ref.ask(crate::actors::player::SetItemAwake {
                unique_id: msg.unique_id,
                awake,
            }).await.unwrap_or(false);

            if set {
                debug!("Awakening success: {} item={} type={:?} value={}", state.name, msg.unique_id, awake_type, value);
                self.send_awakening_result(msg.session_id, AWAKE_RESULT_SUCCESS, -1);
                send_system_message(&self.gate_ref, msg.session_id,
                    &format!("觉醒成功！{} +{}", awake_type_name(awake_type), value));
            } else {
                self.send_awakening_result(msg.session_id, AWAKE_RESULT_FAIL, -1);
            }
        } else {
            // 失败：物品被摧毁
            let removed = record.actor_ref.ask(crate::actors::player::RemoveItemFromInventory {
                unique_id: msg.unique_id,
            }).await.ok().flatten();
            if removed.is_some() {
                debug!("Awakening destroy: {} item={} destroyed", state.name, msg.unique_id);
                self.send_awakening_result(msg.session_id, AWAKE_RESULT_DESTROYED, msg.unique_id as i64);
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

    async fn handle(&mut self, msg: DowngradeAwakeningRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        let item = match record.actor_ref.ask(crate::actors::player::GetItemInfo { unique_id: msg.unique_id }).await {
            Ok(Some(i)) => i,
            _ => {
                send_system_message(&self.gate_ref, msg.session_id, "找不到该物品");
                return;
            }
        };

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

        let has_gold = record.actor_ref.ask(crate::actors::player::HasGold { amount: gold_cost }).await.unwrap_or(false);
        if !has_gold {
            send_system_message(&self.gate_ref, msg.session_id, &format!("金币不足，降级需要 {} 金币", gold_cost));
            return;
        }

        let gold_deducted = record.actor_ref.ask(crate::actors::player::DeductGold { amount: gold_cost }).await;
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

        let set = record.actor_ref.ask(crate::actors::player::SetItemAwake {
            unique_id: msg.unique_id,
            awake,
        }).await.unwrap_or(false);

        if set {
            debug!("DowngradeAwakening: {} item={} new_level={}", state.name, msg.unique_id, item.awake.awake_level().saturating_sub(1));
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
        use mir2_shared::data::item::Awake;
        let record = match self.players.get(&session_id) { Some(r) => r.clone(), None => return };
        let state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        // C#：遍历 Info.Equipment，真实类型匹配则 UpgradeAwake
        for item in state.inventory.equipment.iter().flatten() {
            let Some(info) = item.info.as_ref() else { continue };
            if info.item_type != item_type { continue; }
            let Some(item_info) = self.item_infos.get(&item.item_index) else { continue };

            // C# CheckAwakening：可觉醒 / 未满级 / 类型兼容 / 品级
            if !item_info.can_awakening || item.awake.is_max_level() {
                send_system_message(&self.gate_ref, session_id, &format!("条件不符：{}", item_info.name));
                continue;
            }
            if item.awake.awake_type != mir2_shared::enums::AwakeType::None
                && item.awake.awake_type != awake_type
            {
                send_system_message(&self.gate_ref, session_id, &format!("条件不符：{}", item_info.name));
                continue;
            }
            let compatible = match info.item_type {
                mir2_shared::enums::ItemType::Weapon => matches!(awake_type,
                    mir2_shared::enums::AwakeType::Dc | mir2_shared::enums::AwakeType::Mc | mir2_shared::enums::AwakeType::Sc),
                mir2_shared::enums::ItemType::Helmet => matches!(awake_type,
                    mir2_shared::enums::AwakeType::Ac | mir2_shared::enums::AwakeType::Mac),
                mir2_shared::enums::ItemType::Armour => awake_type == mir2_shared::enums::AwakeType::HpMp,
                _ => false,
            };
            let grade = item_info.grade;
            if !compatible || !(1..=4).contains(&grade) {
                send_system_message(&self.gate_ref, session_id, &format!("条件不符：{}", item_info.name));
                continue;
            }

            // C# UpgradeAwake：70% 成功；失败仅提示（GM 语义不销毁物品）
            let roll = fastrand::u8(0..100);
            if roll >= Awake::SUCCESS_RATE {
                send_system_message(&self.gate_ref, session_id, &format!("觉醒失败：{}", item_info.name));
                continue;
            }

            // 成功：计算觉醒值（与 NPC 觉醒一致：grade→chance_max，item_type→rate）
            let chance_max = Awake::CHANCE_MAX.get(grade.saturating_sub(1) as usize).copied().unwrap_or(1);
            let rate = match info.item_type {
                mir2_shared::enums::ItemType::Weapon => Awake::WEAPON_RATE,
                mir2_shared::enums::ItemType::Helmet => Awake::HELMET_RATE,
                mir2_shared::enums::ItemType::Armour => Awake::ARMOUR_RATE,
                _ => 1,
            };
            let value = (fastrand::u8(1..=chance_max) as i32 * rate as i32).max(1) as u8;

            let mut new_awake = item.awake.clone();
            new_awake.awake_type = awake_type;
            new_awake.levels.push(value);
            let mut snapshot = item.clone();
            snapshot.awake = new_awake.clone();
            let ok = record.actor_ref.ask(crate::actors::player::SetItemAwake {
                unique_id: item.unique_id,
                awake: new_awake,
            }).await.unwrap_or(false);
            if ok {
                // C#：Enqueue(new S.RefreshItem { Item = temp })
                let pkt = mir2_shared::packets::server::item::RefreshItem { item: snapshot };
                let mut body = Vec::new();
                if pkt.write_body(&mut body).is_ok() {
                    let _ = self.gate_ref.tell(SendToClient {
                        session_id,
                        data: build_packet_bytes(mir2_shared::enums::ServerPacketIds::RefreshItem as i16, &body),
                    }).await;
                }
                send_system_message(&self.gate_ref, session_id,
                    &format!("觉醒成功！{} +{}", awake_type_name(awake_type), value));
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
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        // #2058：C# ResetPrice——费用 3000*Grade*(AddedStats.Count*0.2+1)
        let item = match record.actor_ref.ask(GetItemInfo { unique_id: msg.unique_id }).await {
            Ok(Some(i)) => i,
            _ => {
                send_system_message(&self.gate_ref, msg.session_id, "找不到该物品");
                return;
            }
        };
        let Some(item_info) = self.item_infos.get(&item.item_index).cloned() else {
            send_system_message(&self.gate_ref, msg.session_id, "物品信息不存在");
            return;
        };
        let gold_cost = reset_price(&item, &item_info);
        if state.inventory.gold < gold_cost {
            send_system_message(&self.gate_ref, msg.session_id, &format!("金币不足，重置需要 {} 金币", gold_cost));
            return;
        }
        let _ = record.actor_ref.ask(crate::actors::player::DeductGold { amount: gold_cost }).await;
        super::send_gold_changed_packet(&self.gate_ref, msg.session_id, gold_cost);

        let success = record.actor_ref.ask(crate::actors::player::ResetItemAddedStats {
            unique_id: msg.unique_id,
        }).await.unwrap_or(false);
        if success {
            send_system_message(&self.gate_ref, msg.session_id, "物品附加属性已重置");
            debug!("ResetAddedItem: {} uid={} - success", state.name, msg.unique_id);
        } else {
            send_system_message(&self.gate_ref, msg.session_id, "找不到该物品或无法重置");
            debug!("ResetAddedItem: {} uid={} - failed", state.name, msg.unique_id);
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
        let info = crate::db::ItemInfo { index: 1, grade: 2, ..Default::default() };
        // 3000*2 * (2*0.2+1) = 6000 * 1.4 = 8400
        assert_eq!(reset_price(&item, &info), 8400);
        // 无附加：3000*2*1.0 = 6000
        let plain = UserItem { item_index: 1, ..Default::default() };
        assert_eq!(reset_price(&plain, &info), 6000);
    }

    #[test]
    fn refine_cost_matches_csharp() {
        // #2034：C# RefineItem——cost = (RequiredAmount*10)*RefineCost(125)
        assert_eq!(refine_cost(0), 0);
        assert_eq!(refine_cost(1), 1250);
        assert_eq!(refine_cost(10), 12500);
        assert_eq!(refine_cost(-5), 0); // RequiredAmount 负数按 0
    }
}
