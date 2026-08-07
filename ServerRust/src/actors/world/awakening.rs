use super::*;

/// 存入精炼物品
pub struct DepositRefineItemRequest {
    pub session_id: u64,
    pub unique_id: u64,
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

        // 检查物品是否在背包中
        let Some(item) = state.inventory.get_item(msg.unique_id) else {
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

        // 更新精炼日志
        let mut log = state.refine_log;
        if !log.deposit_item(msg.unique_id) {
            send_system_message(&self.gate_ref, msg.session_id, "已有精炼进行中");
            return;
        }

        let _ = record.actor_ref.ask(SetRefineLog { refine_log: log }).await;
        send_system_message(&self.gate_ref, msg.session_id, "精炼物品已存入");
        debug!("DepositRefineItem: {} uid={}", state.name, msg.unique_id);
    }
}

/// 取回精炼物品
pub struct RetrieveRefineItemRequest {
    pub session_id: u64,
    pub unique_id: u64,
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

        // 检查精炼是否完成或可取回
        if state.refine_log.active_refine.is_none() {
            send_system_message(&self.gate_ref, msg.session_id, "没有精炼物品可取回");
            return;
        }

        // 检查背包是否有空位
        if !state.inventory.has_space() {
            send_system_message(&self.gate_ref, msg.session_id, "背包已满");
            return;
        }

        let mut log = state.refine_log;
        if let Some(_item) = log.retrieve() {
            let _ = record.actor_ref.ask(SetRefineLog { refine_log: log }).await;
            send_system_message(&self.gate_ref, msg.session_id, "精炼物品已取回");
            debug!("RetrieveRefineItem: {} uid={}", state.name, msg.unique_id);
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
        log.cancel();
        let _ = record.actor_ref.ask(SetRefineLog { refine_log: log }).await;
        send_system_message(&self.gate_ref, msg.session_id, "精炼已取消");
        debug!("RefineCancel: {}", state.name);
    }
}

/// 开始精炼
pub struct RefineItemRequest {
    pub session_id: u64,
    pub item_id: u32,
    pub materials: u32,
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

        // 检查是否有待精炼物品
        if state.refine_log.active_refine.is_none() {
            send_system_message(&self.gate_ref, msg.session_id, "没有待精炼的物品");
            return;
        }

        // 开始精炼（60 秒完成，80% 成功率）
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let mut log = state.refine_log;
        let duration = 60u64; // 1 分钟
        let success_chance = 80u8; // 80%
        log.start_refine(msg.item_id, current_time, duration, success_chance);
        let _ = record.actor_ref.ask(SetRefineLog { refine_log: log }).await;

        send_system_message(&self.gate_ref, msg.session_id, "精炼已开始，请稍后查看");
        debug!("RefineItem: {} item={} materials={}", state.name, msg.item_id, msg.materials);
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
                    send_system_message(&self.gate_ref, msg.session_id, "精炼成功！物品已提升");
                } else {
                    send_system_message(&self.gate_ref, msg.session_id, "精炼失败，物品已损毁");
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

impl Message<ResetAddedItemRequest> for WorldActor {
    type Reply = ();
    async fn handle(&mut self, msg: ResetAddedItemRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) { Some(r) => r.clone(), None => return };
        let state = match record.actor_ref.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

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
