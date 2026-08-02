// SocialActor - 社交/组队/交易/好友/行会/婚姻/师徒
// 从 WorldActor 拆分出来，负责所有跨玩家社交逻辑

use std::collections::{HashMap, HashSet};

use kameo::actor::{Actor, ActorRef};
use kameo::message::Message;
use kameo::prelude::Context;
use tokio::sync::RwLock;
use std::sync::Arc;
use tracing::{debug, warn, info};

use crate::actors::player::{PlayerActor, GetPlayerState, SetGroupId, SetSpouse, SetGuildInfo, SetAllowMentor, SetMentor, SetPlayerPosition, SetLastRecallTime, SetEnableGroupRecall, AddFriendToSelf, RemoveFriendFromSelf, SetFriendMemo, AddGold, DeductGold, AddItemToInventory, RemoveItemFromInventory, GetItemInfo, CanGainItems, CanGainGold};
use crate::actors::inventory::EquipmentSlot;
use crate::actors::group::{Group, GroupMember};
use crate::actors::trade::TradeSession;
use crate::actors::guild::{Guild, GuildRank};
use crate::db::{self, DbPool};
use crate::gate::actor::{GateActor, SendToClient};
use crate::actors::social_packets::*;
use crate::util::wire::build_packet_bytes;
use mir2_shared::enums::{ServerPacketIds, ItemSet};

// ============================================================
// Message types (moved from WorldActor)
// ============================================================

// --- Group ---

pub struct SwitchGroupRequest {
    pub session_id: u64,
    pub allow_group: bool,
}

pub struct GroupInviteRequest {
    pub session_id: u64,
    pub target_name: String,
}

pub struct GroupInviteReply {
    pub session_id: u64,
    pub inviter_id: u64,
    pub accept: bool,
}

pub struct DellMemberRequest {
    pub session_id: u64,
    pub member_name: String,
}

// --- Trade ---

pub struct TradeStartRequest {
    pub session_id: u64,
}

pub struct TradeStartReply {
    pub session_id: u64,
    pub accept: bool,
}

pub struct TradeAddGold {
    pub session_id: u64,
    pub amount: u32,
}

pub struct TradeConfirmLock {
    pub session_id: u64,
    pub locked: bool,
}

pub struct TradeCancel {
    pub session_id: u64,
}

pub struct TradeAddItem {
    pub session_id: u64,
    pub unique_id: u64,
    pub grid: u8,
    pub count: u16,
}

pub struct TradeRemoveItem {
    pub session_id: u64,
    pub unique_id: u64,
}

pub struct DepositTradeItemBySlot {
    pub session_id: u64,
    pub from_slot: i32,
    pub to_slot: i32,
}

pub struct RetrieveTradeItemBySlot {
    pub session_id: u64,
    pub from_slot: i32,
    pub to_slot: i32,
}

// --- Friend ---

pub struct AddFriendRequest {
    pub session_id: u64,
    pub friend_name: String,
    pub blocked: bool,
}

pub struct RemoveFriendRequest {
    pub session_id: u64,
    pub friend_object_id: u32,
}

pub struct RefreshFriendsRequest {
    pub session_id: u64,
}

pub struct AddMemoRequest {
    pub session_id: u64,
    pub friend_object_id: u32,
    pub memo: String,
}

// --- Guild ---

pub const GUILD_MAX_MEMBERS: usize = 200;

pub struct CreateGuildRequest {
    pub session_id: u64,
    pub guild_name: String,
}

pub struct GuildInviteReply {
    pub session_id: u64,
    pub accept: bool,
}

pub struct RequestGuildInfo {
    pub session_id: u64,
    pub info_type: u8,
}

pub struct EditGuildMemberRequest {
    pub session_id: u64,
    pub change_type: u8,
    pub member_name: String,
}

pub struct EditGuildNoticeRequest {
    pub session_id: u64,
    pub notice: Vec<String>,
}

pub struct LeaveGuildRequest {
    pub session_id: u64,
}

pub struct GuildStorageGoldChangeRequest {
    pub session_id: u64,
    pub change_type: u8,
    pub amount: u32,
}

pub struct GuildStorageItemChangeRequest {
    pub session_id: u64,
    pub change_type: u8,
    pub grid: u8,
    pub unique_id: u64,
    pub count: u32,
}

// --- Marriage ---

pub struct MarriageRequest {
    pub session_id: u64,
    pub target_name: String,
}

pub struct MarriageReply {
    pub session_id: u64,
    pub accept: bool,
}

pub struct SocialDivorceRequest {
    pub session_id: u64,
    pub partner_name: String,
}

pub struct SocialDivorceReply {
    pub session_id: u64,
    pub accept: bool,
}

pub struct SocialChangeMarriage {
    pub session_id: u64,
}

// --- Mentor ---

pub struct SocialAddMentor {
    pub session_id: u64,
    pub mentor_name: String,
}

pub struct SocialMentorReply {
    pub session_id: u64,
    pub accept: bool,
}

pub struct SocialAllowMentor {
    pub session_id: u64,
    pub allow: bool,
}

pub struct SocialCancelMentor {
    pub session_id: u64,
}

// --- Trade helper struct ---
// ============================================================
// WorldActor -> SocialActor: 玩家上线同步
// ============================================================

pub struct SocialPlayerJoined {
    pub session_id: u64,
    pub actor_ref: ActorRef<PlayerActor>,
    pub name: String,
}

/// WorldActor -> SocialActor: 玩家下线同步
pub struct SocialPlayerLeft {
    pub session_id: u64,
}

/// WorldActor -> SocialActor: 聊天命令转发（社交类）
pub struct SocialChatCommand {
    pub session_id: u64,
    pub command: String,
    pub args: Vec<String>,
}

/// SocialActor 配置
pub struct SocialActorConfig {
    pub map_infos: Arc<RwLock<HashMap<i32, db::MapInfo>>>,
    pub item_infos: Arc<RwLock<HashMap<i32, db::ItemInfo>>>,
    /// 创建行会所需金币 (来自 cfg.server.toml 的 [social] section)
    /// 替代之前 hardcoded 1_000_000 常量。
    pub guild_creation_cost_gold: u64,
}

impl Default for SocialActorConfig {
    fn default() -> Self {
        Self {
            map_infos: Arc::new(RwLock::new(HashMap::new())),
            item_infos: Arc::new(RwLock::new(HashMap::new())),
            // default 与主流程 cfg 路径一致
            guild_creation_cost_gold: 1_000_000,
        }
    }
}

/// SocialActor 启动参数
pub struct SocialActorArgs {
    pub gate_ref: ActorRef<GateActor>,
    pub db_pool: DbPool,
    pub config: SocialActorConfig,
}

impl Actor for SocialActor {
    type Args = SocialActorArgs;
    type Error = anyhow::Error;

    async fn on_start(args: SocialActorArgs, _actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        // Load guilds from DB
        let guilds = match db::load_guilds(&args.db_pool).await {
            Ok(g) => {
                info!("SocialActor: loaded {} guilds from database", g.len());
                g
            }
            Err(e) => {
                warn!("SocialActor: failed to load guilds from DB: {}", e);
                HashMap::new()
            }
        };

        Ok(Self {
            players: HashMap::new(),
            groups: HashMap::new(),
            next_group_id: 1,
            pending_invites: HashMap::new(),
            active_trades: HashMap::new(),
            guilds,
            pending_guild_invites: HashMap::new(),
            pending_marriage_invites: HashMap::new(),
            pending_mentor_invites: HashMap::new(),
            gate_ref: args.gate_ref,
            db_pool: args.db_pool,
            config: args.config,
        })
    }
}

/// SocialActor 状态
pub struct SocialActor {
    /// 在线玩家镜像（session_id -> ActorRef）
    players: HashMap<u64, ActorRef<PlayerActor>>,

    // === 组队状态 ===
    groups: HashMap<u64, Group>,
    next_group_id: u64,
    pending_invites: HashMap<u64, u64>,

    // === 交易状态 ===
    active_trades: HashMap<u64, TradeSession>,

    // === 行会状态 ===
    guilds: HashMap<String, Guild>,
    pending_guild_invites: HashMap<u64, (u64, String)>,

    // === 婚姻状态 ===
    pending_marriage_invites: HashMap<u64, u64>,

    // === 师徒状态 ===
    pending_mentor_invites: HashMap<u64, u64>,

    // === 依赖 ===
    gate_ref: ActorRef<GateActor>,
    db_pool: DbPool,
    config: SocialActorConfig,
}

const TRADE_RANGE: i32 = 3;

/// 行会创建所需等级（对应 C# Settings.Guild_RequiredLevel）
const GUILD_REQUIRED_LEVEL: u16 = 7;

/// 是否允许配偶召回（对应 C# Settings.WeddingRingRecall）
const WEDDING_RING_RECALL_ENABLED: bool = true;

/// 行会创建费用：金币（对应 C# Settings.Guild_CreationCostList gold entry）
// 创建行会所需金币来自 cfg.server.toml (social.guild_creation_cost_gold),
// 不再需要 hardcoded 常量。config 在 SocialActor.config 字段里。

/// 新手行会名称（对应 C# Settings.NewbieGuild）
const NEWBIE_GUILD: &str = "新手村";

/// Mir 方向常量（对应 C# MirDirection 0..7）
const DIR_DX: [i32; 8] = [0, 1, 1, 1, 0, -1, -1, -1];
const DIR_DY: [i32; 8] = [-1, -1, 0, 1, 1, 1, 0, -1];

fn player_dir_x(dir: u8) -> i32 { DIR_DX[dir as usize & 7] }
fn player_dir_y(dir: u8) -> i32 { DIR_DY[dir as usize & 7] }

// ============================================================
// Helper methods
// ============================================================

impl SocialActor {
    /// 按名称查找在线玩家 session_id
    async fn find_player_by_name(&self, name: &str, exclude_session: u64) -> Option<u64> {
        for (sid, record) in &self.players {
            if *sid == exclude_session {
                continue;
            }
            if let Ok(Some(s)) = record.ask(GetPlayerState).await {
                if s.name == name {
                    return Some(*sid);
                }
            }
        }
        None
    }

    /// 查找交易（不可变）
    fn find_trade(&self, session_id: u64) -> Option<&TradeSession> {
        self.active_trades.values().find(|t| {
            t.side_a.session_id == session_id || t.side_b.session_id == session_id
        })
    }

    /// 查找交易（可变）
    fn find_trade_mut(&mut self, session_id: u64) -> Option<&mut TradeSession> {
        self.active_trades.values_mut().find(|t| {
            t.side_a.session_id == session_id || t.side_b.session_id == session_id
        })
    }

    /// 发送好友列表
    async fn send_friends_list(&self, session_id: u64) {
        let record = match self.players.get(&session_id) {
            Some(r) => r, None => return,
        };

        let state = match record.ask(GetPlayerState).await {
            Ok(Some(s)) => s, _ => return,
        };

        // 收集所有在线 object_ids
        let mut online_object_ids: Vec<u32> = Vec::new();
        for r in self.players.values() {
            if let Ok(Some(s)) = r.ask(GetPlayerState).await {
                online_object_ids.push(s.object_id);
            }
        }

        send_friends_list_packet(&self.gate_ref, session_id, &state.friend_list.friends, &online_object_ids);
    }

    // === 组队辅助方法 ===

    /// 加入或创建组队
    async fn join_or_create_group(&mut self, joiner_session: u64, target_session: u64, joiner_name: &str) {
        let joiner_name = joiner_name.to_string();

        // 获取加入者信息
        let joiner_member = {
            let record = match self.players.get(&joiner_session) {
                Some(r) => r,
                None => return,
            };
            let state = match record.ask(GetPlayerState).await {
                Ok(Some(s)) => s,
                _ => return,
            };
            GroupMember {
                session_id: state.session_id,
                name: state.name.clone(),
                is_leader: false,
                online: true,
            }
        };

        // 检查目标玩家是否在线
        let target_record = match self.players.get(&target_session) {
            Some(r) => r,
            None => {
                send_system_message(&self.gate_ref, joiner_session, "目标玩家不在线");
                return;
            }
        };

        let target_state = match target_record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        if let Some(target_group_id) = target_state.group_id {
            // 加入已有组队
            if let Some(group) = self.groups.get_mut(&target_group_id) {
                if !group.add_member(joiner_member) {
                    send_system_message(&self.gate_ref, joiner_session, "队伍已满或你已在队伍中");
                    return;
                }
                // 更新加入者的 group_id
                if let Some(record) = self.players.get(&joiner_session) {
                    let _ = record.ask(SetGroupId { group_id: Some(target_group_id) }).await;
                }
                send_system_message(&self.gate_ref, joiner_session, &format!("已加入队伍 #{}", target_group_id));
                self.broadcast_group_update(target_group_id);
                debug!("Player {} joined group #{}", joiner_name, target_group_id);
            }
        } else {
            // 创建新组队
            let group_id = self.next_group_id;
            self.next_group_id += 1;

            let target_member = GroupMember {
                session_id: target_session,
                name: target_state.name.clone(),
                is_leader: true,
                online: true,
            };

            let mut group = Group::new(group_id, target_member);
            group.add_member(joiner_member);

            // 更新两个玩家的 group_id
            if let Some(record) = self.players.get(&target_session) {
                let _ = record.ask(SetGroupId { group_id: Some(group_id) }).await;
            }
            if let Some(record) = self.players.get(&joiner_session) {
                let _ = record.ask(SetGroupId { group_id: Some(group_id) }).await;
            }

            self.groups.insert(group_id, group);
            send_system_message(&self.gate_ref, joiner_session, &format!("队伍 #{} 已创建", group_id));
            send_system_message(&self.gate_ref, target_session, &format!("队伍 #{} 已创建", group_id));
            // 创建后广播成员列表（C# 语义：双方立即看到组队面板）
            self.broadcast_group_update(group_id);
            debug!("Created group #{} with {} and {}", group_id, target_state.name, joiner_name);
        }
    }

    /// 离开组队
    async fn leave_group(&mut self, group_id: u64, session_id: u64, name: &str) {
        if let Some(group) = self.groups.get_mut(&group_id) {
            if group.remove_member(session_id).is_some() {
                if let Some(record) = self.players.get(&session_id) {
                    let _ = record.ask(SetGroupId { group_id: None }).await;
                }
                send_system_message(&self.gate_ref, session_id, "已离开队伍");
                debug!("Player {} left group #{}", name, group_id);

                if group.member_count() == 0 {
                    self.groups.remove(&group_id);
                } else {
                    self.broadcast_group_update(group_id);
                }
            }
        }
    }

    /// 广播组队更新给所有成员
    fn broadcast_group_update(&self, group_id: u64) {
        if let Some(group) = self.groups.get(&group_id) {
            for member in &group.members {
                if member.online {
                    send_group_members_map(&self.gate_ref, member.session_id, &group.members);
                }
            }
        }
    }

    // === 交易执行 ===

    /// 执行交易：实际扣除并转移物品/金币
    async fn execute_trade(&mut self, trigger_session: u64) {
        // 先克隆交易数据（避免借用冲突）
        let trade_data = match self.find_trade(trigger_session) {
            Some(t) => t.clone(),
            None => return,
        };

        let (s1, s2) = trade_data.participant_sessions();
        let gold_a = trade_data.side_a.gold;
        let gold_b = trade_data.side_b.gold;
        let items_a: Vec<_> = trade_data.side_a.items.to_vec();
        let items_b: Vec<_> = trade_data.side_b.items.to_vec();

        // 容量检查（对应 C# CanGainItems / CanGainGold）
        // A 能否接收 B 的物品和金币
        let a_can_receive = match self.players.get(&s1) {
            Some(rec) => {
                let items_ok = items_b.is_empty() || rec.ask(CanGainItems).await.unwrap_or(false);
                let gold_ok = gold_b == 0 || rec.ask(CanGainGold { amount: (gold_b as u32).min(u32::MAX) }).await.unwrap_or(false);
                items_ok && gold_ok
            }
            None => false,
        };
        // B 能否接收 A 的物品和金币
        let b_can_receive = match self.players.get(&s2) {
            Some(rec) => {
                let items_ok = items_a.is_empty() || rec.ask(CanGainItems).await.unwrap_or(false);
                let gold_ok = gold_a == 0 || rec.ask(CanGainGold { amount: (gold_a as u32).min(u32::MAX) }).await.unwrap_or(false);
                items_ok && gold_ok
            }
            None => false,
        };

        if !a_can_receive {
            send_system_message(&self.gate_ref, s1, "你的背包已满或金币已达上限，无法完成交易");
            send_trade_cancel_packet(&self.gate_ref, s1);
            send_trade_cancel_packet(&self.gate_ref, s2);
            send_trade_close_packet(&self.gate_ref, s1);
            send_trade_close_packet(&self.gate_ref, s2);
            self.active_trades.remove(&trade_data.side_a.session_id);
            return;
        }
        if !b_can_receive {
            send_system_message(&self.gate_ref, s2, "你的背包已满或金币已达上限，无法完成交易");
            send_trade_cancel_packet(&self.gate_ref, s1);
            send_trade_cancel_packet(&self.gate_ref, s2);
            send_trade_close_packet(&self.gate_ref, s1);
            send_trade_close_packet(&self.gate_ref, s2);
            self.active_trades.remove(&trade_data.side_a.session_id);
            return;
        }

        // 从 A 扣除金币和物品
        if let Some(rec) = self.players.get(&s1) {
            if gold_a > 0 {
                let _ = rec.ask(DeductGold { amount: gold_a }).await;
            }
            for item in &items_a {
                let _ = rec.ask(RemoveItemFromInventory { unique_id: item.uid }).await;
            }
        }

        // 从 B 扣除金币和物品
        if let Some(rec) = self.players.get(&s2) {
            if gold_b > 0 {
                let _ = rec.ask(DeductGold { amount: gold_b }).await;
            }
            for item in &items_b {
                let _ = rec.ask(RemoveItemFromInventory { unique_id: item.uid }).await;
            }
        }

        // 将 B 的金币和物品给 A
        if let Some(rec) = self.players.get(&s1) {
            if gold_b > 0 {
                let _ = rec.ask(AddGold { amount: gold_b }).await;
            }
            for item in &items_b {
                // 优先使用交易侧缓存的完整物品（DepositTradeItemBySlot 已从背包移除，不能再查询）
                if let Some(data) = item.item_data.clone() {
                    let _ = rec.ask(AddItemToInventory { item: data }).await;
                } else if let Some(rec2) = self.players.get(&s2) {
                    if let Ok(Some(item_data)) = rec2.ask(GetItemInfo { unique_id: item.uid }).await {
                        let _ = rec.ask(AddItemToInventory { item: item_data }).await;
                    }
                }
            }
        }

        // 将 A 的金币和物品给 B
        if let Some(rec) = self.players.get(&s2) {
            if gold_a > 0 {
                let _ = rec.ask(AddGold { amount: gold_a }).await;
            }
            for item in &items_a {
                if let Some(data) = item.item_data.clone() {
                    let _ = rec.ask(AddItemToInventory { item: data }).await;
                } else if let Some(rec2) = self.players.get(&s1) {
                    if let Ok(Some(item_data)) = rec2.ask(GetItemInfo { unique_id: item.uid }).await {
                        let _ = rec.ask(AddItemToInventory { item: item_data }).await;
                    }
                }
            }
        }

        // 移除交易会话
        self.active_trades.remove(&trade_data.side_a.session_id);

        send_trade_success_packet(&self.gate_ref, s1);
        send_trade_success_packet(&self.gate_ref, s2);

        debug!("Trade executed: {} gold + {} items <-> {} gold + {} items", gold_a, items_a.len(), gold_b, items_b.len());

        send_trade_close_packet(&self.gate_ref, s1);
        send_trade_close_packet(&self.gate_ref, s2);
    }

    // === 召回命令 ===

    /// 检查召回套装是否完整（需要4种不同类型的Recall装备）
    async fn check_recall_set(&self, session_id: u64) -> bool {
        let record = match self.players.get(&session_id) {
            Some(r) => r,
            None => return false,
        };
        let state = match record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return false,
        };

        let item_infos = self.config.item_infos.read().await;
        let recall_types: HashSet<i32> = state.inventory.equipment.iter()
            .filter_map(|e| e.as_ref())
            .filter(|e| {
                item_infos.get(&e.item_index)
                    .map(|info| info.set_type == ItemSet::Recall as i32)
                    .unwrap_or(false)
            })
            .filter_map(|e| item_infos.get(&e.item_index))
            .map(|info| info.item_type)
            .collect();
        recall_types.len() >= 4
    }

    /// GROUPRECALL - 召回所有组队成员
    async fn handle_group_recall(&mut self, leader_session: u64) {
        let record = match self.players.get(&leader_session) { Some(r) => r.clone(), None => return };
        let state = match record.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        // Must be group leader
        let Some(group_id) = state.group_id else { return; };
        let Some(group) = self.groups.get(&group_id) else { return; };
        if group.leader_session() != Some(leader_session) {
            send_system_message(&self.gate_ref, leader_session, "你不是队长");
            return;
        }

        // Check dead
        if state.is_dead {
            send_system_message(&self.gate_ref, leader_session, "你无法在死亡状态下使用组队召回");
            return;
        }

        // Check no_recall
        {
            let map_infos = self.config.map_infos.read().await;
            if let Some(mi) = map_infos.get(&(state.map_index as i32)) {
                if mi.no_recall {
                    send_system_message(&self.gate_ref, leader_session, "该地图无法使用组队召回");
                    return;
                }
            }
        }

        // Check 180s cooldown
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        if now_ms < state.last_recall_time {
            let remaining = (state.last_recall_time - now_ms + 999) / 1000;
            send_system_message(&self.gate_ref, leader_session, &format!("你还需要等待 {} 秒才能再次使用组队召回", remaining));
            return;
        }

        // Check Recall item set
        if !self.check_recall_set(leader_session).await {
            send_system_message(&self.gate_ref, leader_session, "你需要装备完整的召回套装才能使用组队召回");
            return;
        }

        let target_map = state.map_index;
        let target_x = state.x;
        let target_y = state.y;

        // Set cooldown BEFORE loop
        let new_recall_time = now_ms + 180_000;
        let _ = record.ask(SetLastRecallTime { last_recall_time: new_recall_time }).await;

        // Teleport all group members (only those with EnableGroupRecall=true)
        // Clone group to avoid borrowing self while iterating + calling ask()
        let group = self.groups.get(&group_id).unwrap().clone();
        for member in &group.members {
            if member.session_id == leader_session { continue; }
            if !member.online { continue; }
            if let Some(mem_record) = self.players.get(&member.session_id) {
                if let Ok(Some(mem_state)) = mem_record.ask(GetPlayerState).await {
                    if !mem_state.enable_group_recall {
                        send_system_message(&self.gate_ref, mem_state.session_id, "有人试图未经你允许进行组队召回");
                        send_system_message(&self.gate_ref, leader_session, &format!("{} 拒绝了组队召回", mem_state.name));
                        continue;
                    }
                    let _ = mem_record.ask(SetPlayerPosition {
                        x: target_x, y: target_y,
                        direction: mem_state.direction,
                        map_index: Some(target_map),
                        is_mounted: None,
                    }).await;
                    let mut body = Vec::new();
                    body.extend_from_slice(&target_x.to_le_bytes());
                    body.extend_from_slice(&target_y.to_le_bytes());
                    body.push(mem_state.direction);
                    let _ = self.gate_ref.tell(SendToClient {
                        session_id: member.session_id,
                        data: build_packet_bytes(ServerPacketIds::UserLocation as i16, &body),
                    }).await;
                    debug!("GROUPRECALL: {} recalled to ({}, {}) on map {}", mem_state.name, target_x, target_y, target_map);
                }
            }
        }
    }

    /// RECALLMEMBER <name> - 召回指定成员
    async fn handle_recall_member(&mut self, leader_session: u64, member_name: &str) {
        let record = match self.players.get(&leader_session) { Some(r) => r.clone(), None => return };
        let state = match record.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        let Some(group_id) = state.group_id else { return; };
        let Some(group) = self.groups.get(&group_id) else { return; };
        if group.leader_session() != Some(leader_session) {
            send_system_message(&self.gate_ref, leader_session, "你不是队长");
            return;
        }

        if state.is_dead {
            send_system_message(&self.gate_ref, leader_session, "你无法在死亡状态下使用组队召回");
            return;
        }

        {
            let map_infos = self.config.map_infos.read().await;
            if let Some(mi) = map_infos.get(&(state.map_index as i32)) {
                if mi.no_recall {
                    send_system_message(&self.gate_ref, leader_session, "该地图无法使用组队召回");
                    return;
                }
            }
        }

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        if now_ms < state.last_recall_time {
            let remaining = (state.last_recall_time - now_ms + 999) / 1000;
            send_system_message(&self.gate_ref, leader_session, &format!("你还需要等待 {} 秒才能再次使用组队召回", remaining));
            return;
        }

        if !self.check_recall_set(leader_session).await {
            send_system_message(&self.gate_ref, leader_session, "你需要装备完整的召回套装才能使用组队召回");
            return;
        }

        let target_map = state.map_index;
        let target_x = state.x;
        let target_y = state.y;

        // Find and teleport the named member
        // Clone group to avoid borrowing self while iterating + calling ask()
        let group = self.groups.get(&group_id).unwrap().clone();
        for member in &group.members {
            if member.session_id == leader_session { continue; }
            if !member.online { continue; }
            if let Some(mem_record) = self.players.get(&member.session_id) {
                if let Ok(Some(mem_state)) = mem_record.ask(GetPlayerState).await {
                    if mem_state.name.eq_ignore_ascii_case(member_name) {
                        if !mem_state.enable_group_recall {
                            send_system_message(&self.gate_ref, mem_state.session_id, "有人试图未经你允许进行组队召回");
                            send_system_message(&self.gate_ref, leader_session, &format!("{} 拒绝了组队召回", mem_state.name));
                            return;
                        }
                        let _ = record.ask(SetLastRecallTime { last_recall_time: now_ms + 60_000 }).await;
                        let _ = mem_record.ask(SetPlayerPosition {
                            x: target_x, y: target_y,
                            direction: mem_state.direction,
                            map_index: Some(target_map),
                            is_mounted: None,
                        }).await;
                        let mut body = Vec::new();
                        body.extend_from_slice(&target_x.to_le_bytes());
                        body.extend_from_slice(&target_y.to_le_bytes());
                        body.push(mem_state.direction);
                        let _ = self.gate_ref.tell(SendToClient {
                            session_id: member.session_id,
                            data: build_packet_bytes(ServerPacketIds::UserLocation as i16, &body),
                        }).await;
                        debug!("RECALLMEMBER: {} recalled to ({}, {})", mem_state.name, target_x, target_y);
                        return;
                    }
                }
            }
        }
        send_system_message(&self.gate_ref, leader_session, "玩家未找到");
    }

    /// RECALL - 召回配偶（对应 C# PlayerObject.cs:2439 RECALLLOVER）
    async fn handle_recall_lover(&mut self, session_id: u64) {
        let record = match self.players.get(&session_id) { Some(r) => r.clone(), None => return };
        let state = match record.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        let Some(spouse_name) = &state.spouse_name else {
            send_system_message(&self.gate_ref, session_id, "你需要有配偶才能使用召回");
            return;
        };

        if state.is_dead {
            send_system_message(&self.gate_ref, session_id, "你无法在死亡状态下使用配偶召回");
            return;
        }

        {
            let map_infos = self.config.map_infos.read().await;
            if let Some(mi) = map_infos.get(&(state.map_index as i32)) {
                if mi.no_recall {
                    send_system_message(&self.gate_ref, session_id, "该地图无法使用配偶召回");
                    return;
                }
            }
        }

        let ring_l = state.inventory.get_equipment(EquipmentSlot::RingL);
        if ring_l.is_none() {
            send_system_message(&self.gate_ref, session_id, "你需要佩戴结婚戒指才能使用配偶召回");
            return;
        }

        let ring = ring_l.unwrap();
        if ring.wedding_ring == 0 {
            send_system_message(&self.gate_ref, session_id, "你的结婚戒指未绑定配偶");
            return;
        }

        // 全局开关（对应 C# Settings.WeddingRingRecall）
        if !WEDDING_RING_RECALL_ENABLED {
            send_system_message(&self.gate_ref, session_id, "结婚戒指召回功能已关闭");
            return;
        }

        let target_map = state.map_index;
        let target_x = state.x;
        let target_y = state.y;
        let spouse_name = spouse_name.clone();

        // Find spouse and teleport
        for (other_session, other_record) in &self.players {
            if *other_session == session_id { continue; }
            if let Ok(Some(other_state)) = other_record.ask(GetPlayerState).await {
                if other_state.name.eq_ignore_ascii_case(&spouse_name) {
                    // 检查配偶是否死亡（对应 C# player.Dead）
                    if other_state.is_dead {
                        send_system_message(&self.gate_ref, session_id, "配偶已死亡，无法召回");
                        return;
                    }

                    // 检查配偶是否也佩戴了结婚戒指（对应 C# player.Info.Equipment[RingL] == null）
                    let spouse_ring_l = other_state.inventory.get_equipment(EquipmentSlot::RingL);
                    if spouse_ring_l.is_none() {
                        send_system_message(&self.gate_ref, *other_session, "你需要佩戴结婚戒指才能被召回");
                        send_system_message(&self.gate_ref, session_id, &format!("{} 没有佩戴结婚戒指", other_state.name));
                        return;
                    }
                    let spouse_ring = spouse_ring_l.unwrap();
                    // 检查配偶戒指绑定是否正确（对应 C# player.Info.Equipment[RingL].WeddingRing != player.Info.Married）
                    if spouse_ring.wedding_ring == 0 {
                        send_system_message(&self.gate_ref, *other_session, "你需要佩戴已绑定的结婚戒指才能被召回");
                        send_system_message(&self.gate_ref, session_id, &format!("{} 没有佩戴已绑定的结婚戒指", other_state.name));
                        return;
                    }

                    // 检查配偶是否允许配偶召回（对应 C# player.AllowLoverRecall）
                    if !other_state.allow_lover_recall {
                        send_system_message(&self.gate_ref, *other_session, "有人试图未经你允许进行配偶召回");
                        send_system_message(&self.gate_ref, session_id, &format!("{} 拒绝了配偶召回", other_state.name));
                        return;
                    }

                    // 检查冷却时间（对应 C# Envir.Time < LastRecallTime && Envir.Time < player.LastRecallTime）
                    let now_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0);
                    if now_ms < state.last_recall_time {
                        let remaining = (state.last_recall_time - now_ms + 999) / 1000;
                        send_system_message(&self.gate_ref, session_id, &format!("你还需要等待 {} 秒才能再次使用配偶召回", remaining));
                        return;
                    }
                    if now_ms < other_state.last_recall_time {
                        let remaining = (other_state.last_recall_time - now_ms + 999) / 1000;
                        send_system_message(&self.gate_ref, session_id, &format!("配偶还需要等待 {} 秒才能再次使用召回", remaining));
                        return;
                    }

                    // 设置冷却（60s，对应 C# LastRecallTime = Envir.Time + 60000; player.LastRecallTime = Envir.Time + 60000）
                    let new_recall_time = now_ms + 60_000;
                    let _ = record.ask(SetLastRecallTime { last_recall_time: new_recall_time }).await;
                    let _ = other_record.ask(SetLastRecallTime { last_recall_time: new_recall_time }).await;

                    // 尝试 Teleport（对应 C# player.Teleport(CurrentMap, Front)，失败则 CurrentLocation）
                    // Front = 发起者当前位置前方一格
                    let front_x = target_x + player_dir_x(state.direction);
                    let front_y = target_y + player_dir_y(state.direction);
                    let _ = other_record.ask(SetPlayerPosition {
                        x: front_x, y: front_y,
                        direction: other_state.direction,
                        map_index: Some(target_map),
                        is_mounted: None,
                    }).await;
                    let mut body = Vec::new();
                    body.extend_from_slice(&front_x.to_le_bytes());
                    body.extend_from_slice(&front_y.to_le_bytes());
                    body.push(other_state.direction);
                    let _ = self.gate_ref.tell(SendToClient {
                        session_id: *other_session,
                        data: build_packet_bytes(ServerPacketIds::UserLocation as i16, &body),
                    }).await;
                    debug!("RECALL: {} recalled {} to ({}, {})", state.name, spouse_name, front_x, front_y);
                    return;
                }
            }
        }
        send_system_message(&self.gate_ref, session_id, "配偶不在线");
    }

    /// RIDE - 切换骑乘状态
    async fn handle_toggle_ride(&mut self, session_id: u64) {
        let record = match self.players.get(&session_id) { Some(r) => r.clone(), None => return };
        let state = match record.ask(GetPlayerState).await { Ok(Some(s)) => s, _ => return };

        let mount_item = match state.inventory.get_equipment(EquipmentSlot::Mount) {
            Some(m) => m.clone(),
            None => {
                send_system_message(&self.gate_ref, session_id, "你没有装备坐骑");
                return;
            }
        };

        let has_saddle = mount_item.slots.get(2).and_then(|s| s.as_ref()).is_some();
        if !has_saddle {
            send_system_message(&self.gate_ref, session_id, "你必须给坐骑装备鞍才能骑乘");
            return;
        }

        let map_info;
        {
            let map_infos = self.config.map_infos.read().await;
            map_info = map_infos.get(&(state.map_index as i32)).cloned();
        }

        if state.is_mounted {
            let _ = record.ask(SetPlayerPosition {
                x: state.x, y: state.y,
                direction: state.direction,
                map_index: None,
                is_mounted: Some(false),
            }).await;
            debug!("RIDE: {} dismounted", state.name);
        } else {
            if let Some(ref mi) = map_info {
                if mi.no_mount {
                    send_system_message(&self.gate_ref, session_id, "该地图无法骑乘坐骑");
                    return;
                }
                if mi.need_bridle {
                    let has_reins = mount_item.slots.first().and_then(|s| s.as_ref()).is_some();
                    if !has_reins {
                        send_system_message(&self.gate_ref, session_id, "该地图需要给坐骑装备缰绳才能骑乘");
                        return;
                    }
                }
            }

            let _ = record.ask(SetPlayerPosition {
                x: state.x, y: state.y,
                direction: state.direction,
                map_index: None,
                is_mounted: Some(true),
            }).await;
            debug!("RIDE: {} mounted", state.name);
        }
    }
}

// ============================================================
// Message: SocialPlayerJoined
// ============================================================

impl Message<SocialPlayerJoined> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: SocialPlayerJoined, _ctx: &mut Context<Self, Self::Reply>) {
        self.players.insert(msg.session_id, msg.actor_ref);
        debug!("SocialActor: player {} joined", msg.name);
    }
}

// ============================================================
// Message: SocialPlayerLeft
// ============================================================

impl Message<SocialPlayerLeft> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: SocialPlayerLeft, _ctx: &mut Context<Self, Self::Reply>) {
        self.players.remove(&msg.session_id);

        // 处理组队离线标记
        for group in self.groups.values_mut() {
            if let Some(member) = group.members.iter_mut().find(|m| m.session_id == msg.session_id) {
                member.online = false;
            }
        }

        // 清理交易
        self.active_trades.retain(|_, trade| {
            trade.side_a.session_id != msg.session_id && trade.side_b.session_id != msg.session_id
        });

        // 清理邀请
        self.pending_invites.retain(|&k, &mut v| k != msg.session_id && v != msg.session_id);
        self.pending_guild_invites.remove(&msg.session_id);
        self.pending_marriage_invites.retain(|&k, &mut v| k != msg.session_id && v != msg.session_id);
        self.pending_mentor_invites.retain(|&k, &mut v| k != msg.session_id && v != msg.session_id);

        debug!("SocialActor: player left (session={})", msg.session_id);
    }
}

// ============================================================
// Message: SocialChatCommand
// ============================================================

impl Message<SocialChatCommand> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: SocialChatCommand, _ctx: &mut Context<Self, Self::Reply>) {
        match msg.command.as_str() {
            "GROUPRECALL" => {
                self.handle_group_recall(msg.session_id).await;
            }
            "RECALLMEMBER" => {
                if !msg.args.is_empty() {
                    self.handle_recall_member(msg.session_id, &msg.args[0]).await;
                }
            }
            "RECALL" => {
                self.handle_recall_lover(msg.session_id).await;
            }
            "ENABLEGROUPRECALL" => {
                if let Some(record) = self.players.get(&msg.session_id) {
                    let _ = record.ask(SetEnableGroupRecall { enable: true }).await;
                }
                send_system_message(&self.gate_ref, msg.session_id, "Group Recall Enabled.");
            }
            "DISABLEGROUPRECALL" => {
                if let Some(record) = self.players.get(&msg.session_id) {
                    let _ = record.ask(SetEnableGroupRecall { enable: false }).await;
                }
                send_system_message(&self.gate_ref, msg.session_id, "Group Recall Disabled.");
            }
            "RIDE" => {
                self.handle_toggle_ride(msg.session_id).await;
            }
            _ => warn!("SocialActor: unknown social command: {}", msg.command),
        }
    }
}

// ============================================================
// 组队系统 Handler
// ============================================================

impl Message<SwitchGroupRequest> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: SwitchGroupRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        let state = match record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        if !msg.allow_group {
            // 禁止组队 → 离开当前组队
            if let Some(group_id) = state.group_id {
                self.leave_group(group_id, msg.session_id, &state.name).await;
            }
            send_system_message(&self.gate_ref, msg.session_id, "已关闭组队");
        } else {
            send_system_message(&self.gate_ref, msg.session_id, "已开启组队");
        }
    }
}

impl Message<GroupInviteRequest> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: GroupInviteRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let inviter_record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        let inviter_state = match inviter_record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 通过名称查找目标玩家
        let Some(target_session) = self.find_player_by_name(&msg.target_name, msg.session_id).await else {
            send_system_message(&self.gate_ref, msg.session_id, "目标玩家不在线");
            return;
        };

        let target_record = match self.players.get(&target_session) {
            Some(r) => r,
            None => return,
        };

        let target_state = match target_record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 检查是否已在同一组队
        if let (Some(g1), Some(g2)) = (inviter_state.group_id, target_state.group_id) {
            if g1 == g2 {
                send_system_message(&self.gate_ref, msg.session_id, "你们已在同一组队中");
                return;
            }
        }

        // 发送邀请给目标玩家
        send_group_invite_packet(&self.gate_ref, target_session, &inviter_state.name, msg.session_id);
        // 记录待处理邀请
        self.pending_invites.insert(target_session, msg.session_id);
        debug!("Group invite: {} -> {}", inviter_state.name, target_state.name);
    }
}

impl Message<GroupInviteReply> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: GroupInviteReply, _ctx: &mut Context<Self, Self::Reply>) {
        // 解析邀请者 ID
        let inviter_id = self.pending_invites.remove(&msg.session_id)
            .unwrap_or(msg.inviter_id);

        if !msg.accept {
            send_system_message(&self.gate_ref, inviter_id, "对方拒绝了组队邀请");
            return;
        }

        // 获取邀请者状态
        let inviter_record = match self.players.get(&inviter_id) {
            Some(r) => r,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "邀请者已离线");
                return;
            }
        };

        let _inviter_state = match inviter_record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 邀请者接受：将回复者加入邀请者的组队
        let reply_name = {
            let record = match self.players.get(&msg.session_id) {
                Some(r) => r,
                None => return,
            };
            match record.ask(GetPlayerState).await {
                Ok(Some(s)) => s.name.clone(),
                _ => return,
            }
        };

        self.join_or_create_group(msg.session_id, inviter_id, &reply_name).await;
    }
}

impl Message<DellMemberRequest> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: DellMemberRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        let state = match record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        let group_id = match state.group_id {
            Some(g) => g,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "你不在任何组队中");
                return;
            }
        };

        // 检查是否是队长
        let is_leader = {
            match self.groups.get(&group_id) {
                Some(g) => g.leader_session() == Some(msg.session_id),
                None => {
                    send_system_message(&self.gate_ref, msg.session_id, "组队不存在");
                    return;
                }
            }
        };

        if !is_leader {
            send_system_message(&self.gate_ref, msg.session_id, "只有队长可以踢出成员");
            return;
        }

        // 通过名称查找成员的 session_id
        let member_session = {
            let group = match self.groups.get(&group_id) {
                Some(g) => g,
                None => return,
            };
            group.members.iter().find(|m| m.name == msg.member_name).map(|m| m.session_id)
        };

        let Some(member_session) = member_session else {
            send_system_message(&self.gate_ref, msg.session_id, &format!("找不到名为 '{}' 的队员", msg.member_name));
            return;
        };

        // 踢出成员
        if let Some(group) = self.groups.get_mut(&group_id) {
            if group.remove_member(member_session).is_some() {
                // 更新被踢出玩家的 group_id
                if let Some(target_record) = self.players.get(&member_session) {
                    let _ = target_record.ask(SetGroupId { group_id: None }).await;
                }

                debug!("Kicked {} from group #{}", msg.member_name, group_id);
                send_system_message(&self.gate_ref, msg.session_id, &format!("{} 已被踢出队伍", msg.member_name));

                // 如果组队空了，删除
                if group.member_count() == 0 {
                    self.groups.remove(&group_id);
                } else {
                    // 广播更新
                    self.broadcast_group_update(group_id);
                }
            }
        }
    }
}

// ============================================================
// 交易系统 Handler
// ============================================================

impl Message<TradeStartRequest> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: TradeStartRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };

        let state = match record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 检查是否已有交易
        if self.active_trades.contains_key(&msg.session_id) {
            send_system_message(&self.gate_ref, msg.session_id, "你已经在交易中");
            return;
        }

        // 检查是否死亡（对应 C# player.Dead || Dead）
        if state.is_dead {
            send_system_message(&self.gate_ref, msg.session_id, "死亡状态下无法交易");
            return;
        }

        // 发送交易请求给附近的玩家（距离校验）
        let player_pos = (state.x, state.y);

        let mut nearest_target: Option<(u64, i32)> = None;
        for sid in self.players.keys() {
            if *sid == msg.session_id {
                continue;
            }
            if self.active_trades.contains_key(sid) {
                continue; // 对方已在交易中
            }
            if let Some(rec) = self.players.get(sid) {
                if let Ok(Some(other_state)) = rec.ask(GetPlayerState).await {
                    if other_state.is_dead { continue; } // 死亡状态下无法交易
                    let dist = (other_state.x - player_pos.0).abs() + (other_state.y - player_pos.1).abs();
                    if dist <= TRADE_RANGE {
                        nearest_target = Some((*sid, dist));
                        break;
                    }
                }
            }
        }

        if let Some((target, _dist)) = nearest_target {
            // 记录待处理交易请求
            self.pending_invites.insert(target, msg.session_id);
            send_trade_invite_packet(&self.gate_ref, target, &state.name);
            debug!("Trade request: {} -> session {} (dist={})", state.name, target, _dist);
        } else {
            send_system_message(&self.gate_ref, msg.session_id, "附近没有其他玩家");
        }
    }
}

impl Message<TradeStartReply> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: TradeStartReply, _ctx: &mut Context<Self, Self::Reply>) {
        // 解析发起者
        let initiator_id = self.pending_invites.remove(&msg.session_id)
            .or_else(|| self.active_trades.get(&msg.session_id).map(|t| {
                if t.side_a.session_id == msg.session_id { t.side_b.session_id }
                else { t.side_a.session_id }
            }));

        let Some(initiator_id) = initiator_id else {
            return;
        };

        if !msg.accept {
            send_system_message(&self.gate_ref, initiator_id, "对方拒绝了交易请求");
            return;
        }

        // 创建交易会话
        let initiator_record = match self.players.get(&initiator_id) {
            Some(r) => r, None => return,
        };
        let initiator_name = match initiator_record.ask(GetPlayerState).await {
            Ok(Some(s)) => s.name.clone(), _ => return,
        };
        let target_record = match self.players.get(&msg.session_id) {
            Some(r) => r, None => return,
        };
        let target_name = match target_record.ask(GetPlayerState).await {
            Ok(Some(s)) => s.name.clone(), _ => return,
        };

        let trade = TradeSession::new(initiator_id, initiator_name.clone(), msg.session_id, target_name.clone());
        self.active_trades.insert(initiator_id, trade);

        // 通知双方打开交易窗口
        send_trade_open_packet(&self.gate_ref, initiator_id, &target_name);
        send_trade_open_packet(&self.gate_ref, msg.session_id, &initiator_name);
        debug!("Trade session created: {} <-> {}", initiator_name, target_name);
    }
}

impl Message<TradeAddGold> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: TradeAddGold, _ctx: &mut Context<Self, Self::Reply>) {
        // 先检查金币（避免可变借用冲突）
        let has_enough_gold = {
            let record = match self.players.get(&msg.session_id) {
                Some(r) => r, None => return,
            };
            match record.ask(GetPlayerState).await {
                Ok(Some(s)) => s.inventory.gold >= msg.amount as u64,
                _ => return,
            }
        };
        if !has_enough_gold {
            send_system_message(&self.gate_ref, msg.session_id, "金币不足");
            return;
        }

        let trade = match self.find_trade_mut(msg.session_id) {
            Some(t) => t,
            None => { send_system_message(&self.gate_ref, msg.session_id, "你不在交易中"); return; }
        };

        let side = match trade.side_of_mut(msg.session_id) {
            Some(s) => s, None => return,
        };
        side.gold = msg.amount as u64;
        side.unlock();

        let other_session = trade.other_session(msg.session_id);
        if let Some(other) = other_session {
            send_trade_gold_update_packet(&self.gate_ref, other, msg.session_id, msg.amount as u64);
        }
    }
}

impl Message<TradeConfirmLock> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: TradeConfirmLock, _ctx: &mut Context<Self, Self::Reply>) {
        let trade = match self.find_trade_mut(msg.session_id) {
            Some(t) => t, None => return,
        };

        let side = match trade.side_of_mut(msg.session_id) {
            Some(s) => s, None => return,
        };
        side.locked = msg.locked;

        let both_locked = trade.both_locked();
        let (s1, s2) = trade.participant_sessions();
        let side_a = trade.side_a.clone();
        let side_b = trade.side_b.clone();

        // 通知双方确认状态
        send_trade_confirm_packet(&self.gate_ref, s1, &side_a, &side_b);
        send_trade_confirm_packet(&self.gate_ref, s2, &side_a, &side_b);

        // 双方都锁定 -> 执行交易
        if both_locked {
            self.execute_trade(msg.session_id).await;
        }
    }
}

impl Message<TradeCancel> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: TradeCancel, _ctx: &mut Context<Self, Self::Reply>) {
        let trade = match self.find_trade(msg.session_id) {
            Some(t) => t.clone(),
            None => return,
        };

        let (s1, s2) = trade.participant_sessions();
        self.active_trades.remove(&trade.side_a.session_id);

        // 通知双方
        send_trade_cancel_packet(&self.gate_ref, s1);
        send_trade_cancel_packet(&self.gate_ref, s2);
        debug!("Trade cancelled: session {}", s1);
    }
}

impl Message<TradeAddItem> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: TradeAddItem, _ctx: &mut Context<Self, Self::Reply>) {
        let trade = match self.find_trade_mut(msg.session_id) {
            Some(t) => t, None => return,
        };

        let side = match trade.side_of_mut(msg.session_id) {
            Some(s) => s, None => return,
        };
        side.add_item(msg.unique_id, msg.grid, msg.count, None);
        side.unlock();

        // 通知对方
        if let Some(other) = trade.other_session(msg.session_id) {
            send_trade_item_update_packet(&self.gate_ref, other, msg.unique_id, msg.grid, msg.count, true);
        }
    }
}

impl Message<TradeRemoveItem> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: TradeRemoveItem, _ctx: &mut Context<Self, Self::Reply>) {
        let trade = match self.find_trade_mut(msg.session_id) {
            Some(t) => t, None => return,
        };

        let side = match trade.side_of_mut(msg.session_id) {
            Some(s) => s, None => return,
        };
        side.remove_item(msg.unique_id);
        side.unlock();

        // 通知对方
        if let Some(other) = trade.other_session(msg.session_id) {
            send_trade_item_update_packet(&self.gate_ref, other, msg.unique_id, 0, 0, false);
        }
    }
}

impl Message<DepositTradeItemBySlot> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: DepositTradeItemBySlot, _ctx: &mut Context<Self, Self::Reply>) {
        // Resolve from_slot → unique_id from player inventory
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(), None => return,
        };
        let state = match record.ask(GetPlayerState).await {
            Ok(Some(s)) => s, _ => return,
        };

        let slot = msg.from_slot as usize;
        let slot_data = state.inventory.backpack.get(slot).and_then(|s| s.as_ref());
        let uid = match slot_data {
            Some(s) => s.item.unique_id,
            None => {
                send_deposit_trade_item_packet(&self.gate_ref, msg.session_id, msg.from_slot, false);
                return;
            }
        };

        // Check trade exists and not locked
        {
            let trade = match self.find_trade_mut(msg.session_id) {
                Some(t) => t, None => {
                    send_deposit_trade_item_packet(&self.gate_ref, msg.session_id, msg.from_slot, false);
                    return;
                }
            };
            let side = match trade.side_of_mut(msg.session_id) {
                Some(s) => s, None => return,
            };
            if side.locked {
                send_deposit_trade_item_packet(&self.gate_ref, msg.session_id, msg.from_slot, false);
                return;
            }
        }

        // Remove item from player inventory
        let removed = record.ask(RemoveItemFromInventory { unique_id: uid }).await.ok().flatten();
        let item_data = removed.clone();

        // Add to trade side
        let other_session = {
            let trade = match self.find_trade_mut(msg.session_id) {
                Some(t) => t, None => {
                    // Rollback: return item to player
                    if let Some(item) = removed {
                        let _ = record.ask(AddItemToInventory { item }).await;
                    }
                    send_deposit_trade_item_packet(&self.gate_ref, msg.session_id, msg.from_slot, false);
                    return;
                }
            };
            let side = match trade.side_of_mut(msg.session_id) {
                Some(s) => s, None => {
                    if let Some(item) = removed {
                        let _ = record.ask(AddItemToInventory { item }).await;
                    }
                    return;
                }
            };
            side.add_item(uid, msg.to_slot as u8, 1, item_data);
            side.unlock();
            trade.other_session(msg.session_id)
        };

        if let Some(other) = other_session {
            send_trade_item_update_packet(&self.gate_ref, other, uid, msg.to_slot as u8, 1, true);
        }
        send_deposit_trade_item_packet(&self.gate_ref, msg.session_id, msg.from_slot, true);
    }
}

impl Message<RetrieveTradeItemBySlot> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: RetrieveTradeItemBySlot, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r.clone(), None => return,
        };

        // Find trade item by grid slot and extract
        let removed_trade_item = {
            let trade = match self.find_trade_mut(msg.session_id) {
                Some(t) => t, None => {
                    send_retrieve_trade_item_packet(&self.gate_ref, msg.session_id, msg.from_slot, false);
                    return;
                }
            };
            let side = match trade.side_of_mut(msg.session_id) {
                Some(s) => s, None => {
                    send_retrieve_trade_item_packet(&self.gate_ref, msg.session_id, msg.from_slot, false);
                    return;
                }
            };
            if side.locked {
                send_retrieve_trade_item_packet(&self.gate_ref, msg.session_id, msg.from_slot, false);
                return;
            }
            let uid = side.items.iter()
                .find(|i| i.grid == msg.from_slot as u8)
                .map(|i| i.uid);
            match uid {
                Some(uid) => {
                    let removed = side.remove_item(uid);
                    side.unlock();
                    removed
                }
                None => {
                    send_retrieve_trade_item_packet(&self.gate_ref, msg.session_id, msg.from_slot, false);
                    return;
                }
            }
        };

        // Add item back to player inventory
        if let Some(trade_item) = &removed_trade_item {
            if let Some(item_data) = &trade_item.item_data {
                let _ = record.ask(AddItemToInventory { item: item_data.clone() }).await;
            }
        }

        // Notify other party
        if let Some(trade_item) = &removed_trade_item {
            let trade = match self.find_trade(msg.session_id) {
                Some(t) => t, None => { return; }
            };
            if let Some(other) = trade.other_session(msg.session_id) {
                send_trade_item_update_packet(&self.gate_ref, other, trade_item.uid, 0, 0, false);
            }
        }
        send_retrieve_trade_item_packet(&self.gate_ref, msg.session_id, msg.from_slot, true);
    }
}

// ============================================================
// 好友系统 Handler
// ============================================================

impl Message<AddFriendRequest> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: AddFriendRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r, None => return,
        };

        let state = match record.ask(GetPlayerState).await {
            Ok(Some(s)) => s, _ => return,
        };

        if msg.friend_name == state.name {
            send_system_message(&self.gate_ref, msg.session_id, "不能添加自己为好友");
            return;
        }

        // 遍历所有玩家查找匹配名称
        let mut found: Option<(u64, u32, String)> = None;
        for (sid, r) in &self.players {
            if let Ok(Some(s)) = r.ask(GetPlayerState).await {
                if s.name == msg.friend_name {
                    found = Some((*sid, s.object_id, s.name));
                    break;
                }
            }
        }

        if let Some((target_session, target_oid, target_name)) = found {
            // 检查是否已在黑名单
            if msg.blocked {
                let record = match self.players.get(&msg.session_id) {
                    Some(r) => r, None => return,
                };
                let mut state = match record.ask(GetPlayerState).await {
                    Ok(Some(s)) => s, _ => return,
                };
                state.friend_list.add_blocked(target_oid, target_name.clone());
                send_system_message(&self.gate_ref, msg.session_id, &format!("已将 {} 加入黑名单", target_name));
                return;
            }

            // 检查是否已是好友
            let is_already_friend = {
                let record = match self.players.get(&msg.session_id) {
                    Some(r) => r, None => return,
                };
                match record.ask(GetPlayerState).await {
                    Ok(Some(s)) => s.friend_list.is_friend(target_oid),
                    _ => return,
                }
            };
            if is_already_friend {
                send_system_message(&self.gate_ref, msg.session_id, "已是你的好友");
                return;
            }

            // 添加好友（双方互相添加）
            {
                let record = match self.players.get(&msg.session_id) {
                    Some(r) => r, None => return,
                };
                let _ = record.ask(AddFriendToSelf { friend_oid: target_oid, friend_name: target_name.clone() }).await;
            }
            {
                let target_r = match self.players.get(&target_session) {
                    Some(r) => r, None => return,
                };
                let _ = target_r.ask(AddFriendToSelf { friend_oid: state.object_id, friend_name: state.name.clone() }).await;
            }

            // 通知双方
            self.send_friends_list(msg.session_id).await;
            self.send_friends_list(target_session).await;

            send_system_message(&self.gate_ref, msg.session_id, &format!("已将 {} 添加为好友", target_name));
        } else {
            send_system_message(&self.gate_ref, msg.session_id, &format!("找不到名为 '{}' 的玩家", msg.friend_name));
        }
    }
}

impl Message<RemoveFriendRequest> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: RemoveFriendRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r, None => return,
        };

        let success = match record.ask(RemoveFriendFromSelf { friend_oid: msg.friend_object_id }).await {
            Ok(s) => s, _ => return,
        };

        if success {
            send_system_message(&self.gate_ref, msg.session_id, "已移除好友");
            self.send_friends_list(msg.session_id).await;
        }
    }
}

impl Message<RefreshFriendsRequest> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: RefreshFriendsRequest, _ctx: &mut Context<Self, Self::Reply>) {
        self.send_friends_list(msg.session_id).await;
    }
}

impl Message<AddMemoRequest> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: AddMemoRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r, None => return,
        };

        let success = match record.ask(SetFriendMemo { friend_oid: msg.friend_object_id, memo: msg.memo.clone() }).await {
            Ok(s) => s, _ => return,
        };

        if success {
            send_system_message(&self.gate_ref, msg.session_id, "备注已更新");
            self.send_friends_list(msg.session_id).await;
        }
    }
}

// ============================================================
// 行会系统 Handler
// ============================================================

impl Message<CreateGuildRequest> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: CreateGuildRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r, None => return,
        };
        let state = match record.ask(GetPlayerState).await {
            Ok(Some(s)) => s, _ => return,
        };

        // 检查是否已在行会
        if state.guild_name.is_some() {
            send_system_message(&self.gate_ref, msg.session_id, "你已经有行会了");
            return;
        }

        // 等级检查（对应 C# Info.Level < Settings.Guild_RequiredLevel）
        if state.level < GUILD_REQUIRED_LEVEL {
            send_system_message(&self.gate_ref, msg.session_id, &format!("等级不足，创建行会需要 {} 级", GUILD_REQUIRED_LEVEL));
            return;
        }

        // 名称唯一性检查
        if self.guilds.contains_key(&msg.guild_name) {
            send_system_message(&self.gate_ref, msg.session_id, "行会名称已存在");
            return;
        }

        // 新手行会名称限制（对应 C# !Info.AccountInfo.AdminAccount && guildName == Settings.NewbieGuild）
        if !state.is_gm && msg.guild_name.eq_ignore_ascii_case(NEWBIE_GUILD) {
            send_system_message(&self.gate_ref, msg.session_id, "不能创建该名称的行会");
            return;
        }

        // 名称为空检查
        if msg.guild_name.trim().is_empty() || msg.guild_name.len() > 20 {
            send_system_message(&self.gate_ref, msg.session_id, "行会名称无效");
            return;
        }

        // 创建行会（扣除费用，对应 C# Settings.Guild_CreationCost / GuildCreationCost）
        // 调查发现:master C# 端 **没有** Guild_CreationCostList 字段(2026-06
        // 搜索 Server/Settings.cs 和 Server/MirEnvir/ 0 匹配);旧代码的
        // "支持物品+金币混合消耗" 注释原本基于对 master 的误解。本分支只
        // 支持金币扣除,这是 master 端实际的等价行为。如果未来 master 加
        // 混合消耗,可在此扩展。
        if state.inventory.gold < self.config.guild_creation_cost_gold as u64 {
            send_system_message(&self.gate_ref, msg.session_id, &format!("金币不足，创建行会需要 {} 金币", self.config.guild_creation_cost_gold));
            return;
        }
        let _ = record.ask(DeductGold { amount: self.config.guild_creation_cost_gold }).await;

        let guild = Guild::new(msg.guild_name.clone(), state.name.clone(), msg.session_id);
        self.guilds.insert(msg.guild_name.clone(), guild);

        // 保存行会到数据库
        if let Some(guild) = self.guilds.get(&msg.guild_name) {
            if let Err(e) = db::save_guild(&self.db_pool, guild).await {
                warn!("Failed to save guild '{}' to DB: {}", msg.guild_name, e);
            }
        }

        // 更新玩家行会信息
        let _ = record.ask(SetGuildInfo {
            guild_name: Some(msg.guild_name.clone()),
            rank: GuildRank::Leader,
        }).await;

        send_system_message(&self.gate_ref, msg.session_id, &format!("行会 \"{}\" 已创建", msg.guild_name));
        send_guild_status_packet(&self.gate_ref, msg.session_id, true);
        debug!("Guild created: {} by {}", msg.guild_name, state.name);
    }
}

impl Message<GuildInviteReply> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: GuildInviteReply, _ctx: &mut Context<Self, Self::Reply>) {
        // 查找邀请
        let invite = match self.pending_guild_invites.remove(&msg.session_id) {
            Some(inv) => inv,
            None => return,
        };
        let (inviter_session, guild_name) = invite;

        if !msg.accept {
            send_system_message(&self.gate_ref, inviter_session, "对方拒绝了行会邀请");
            return;
        }

        // 获取行会
        let guild = match self.guilds.get_mut(&guild_name) {
            Some(g) => g, None => return,
        };

        // 检查行会人数上限
        if guild.member_count() >= GUILD_MAX_MEMBERS {
            send_system_message(&self.gate_ref, msg.session_id, "行会已满");
            return;
        }

        // 获取被邀请者名称
        let invitee_name = match self.players.get(&msg.session_id) {
            Some(r) => match r.ask(GetPlayerState).await {
                Ok(Some(s)) => s.name.clone(),
                _ => return,
            },
            None => return,
        };

        // 检查是否已在行会
        {
            let record = match self.players.get(&msg.session_id) {
                Some(r) => r, None => return,
            };
            let state = match record.ask(GetPlayerState).await {
                Ok(Some(s)) => s, _ => return,
            };
            if state.guild_name.is_some() {
                send_system_message(&self.gate_ref, msg.session_id, "你已经有行会了");
                return;
            }
        }

        // 添加到行会
        guild.add_member(invitee_name.clone(), Some(msg.session_id));

        // 更新玩家行会信息
        if let Some(record) = self.players.get(&msg.session_id) {
            let _ = record.ask(SetGuildInfo {
                guild_name: Some(guild_name.clone()),
                rank: GuildRank::Member,
            }).await;
            send_guild_status_packet(&self.gate_ref, msg.session_id, true);
        }

        // 通知行会成员
        for sid in guild.online_sessions(0) {
            send_guild_member_change_packet(&self.gate_ref, sid, &invitee_name, true);
        }

        send_system_message(&self.gate_ref, msg.session_id, &format!("已加入行会 \"{}\"", guild_name));
        if let Some(inv_record) = self.players.get(&inviter_session) {
            if let Ok(Some(_inv_state)) = inv_record.ask(GetPlayerState).await {
                send_system_message(&self.gate_ref, inviter_session, &format!("{} 加入了行会", invitee_name));
                send_guild_member_change_packet(&self.gate_ref, inviter_session, &invitee_name, true);
            }
        }

        debug!("Guild invite accepted: {} joined {}", invitee_name, guild_name);
    }
}

impl Message<RequestGuildInfo> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: RequestGuildInfo, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r, None => return,
        };
        let state = match record.ask(GetPlayerState).await {
            Ok(Some(s)) => s, _ => return,
        };

        let guild_name = match &state.guild_name {
            Some(n) => n.clone(),
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "你没有行会");
                return;
            }
        };

        let guild = match self.guilds.get(&guild_name) {
            Some(g) => g, None => return,
        };

        // 发送行会信息
        send_guild_info_packet(&self.gate_ref, msg.session_id, guild);
        debug!("Guild info requested by {}", state.name);
    }
}

impl Message<EditGuildMemberRequest> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: EditGuildMemberRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r, None => return,
        };
        let state = match record.ask(GetPlayerState).await {
            Ok(Some(s)) => s, _ => return,
        };

        let guild_name = match &state.guild_name {
            Some(n) => n.clone(), None => return,
        };
        let my_rank = state.guild_rank;

        let guild = match self.guilds.get_mut(&guild_name) {
            Some(g) => g, None => return,
        };

        // 只有会长和副会长可以管理成员
        if my_rank != GuildRank::Leader && my_rank != GuildRank::Officer {
            send_system_message(&self.gate_ref, msg.session_id, "权限不足");
            return;
        }

        match msg.change_type {
            0 => { // 踢出
                // 不能踢会长
                if guild.members.iter().any(|m| m.name == msg.member_name && m.rank == GuildRank::Leader) {
                    send_system_message(&self.gate_ref, msg.session_id, "不能踢出会长");
                    return;
                }
                // 副会长不能踢出其他副会长/会长
                if my_rank == GuildRank::Officer {
                    if let Some(m) = guild.members.iter().find(|m| m.name == msg.member_name) {
                        if m.rank != GuildRank::Member {
                            send_system_message(&self.gate_ref, msg.session_id, "权限不足");
                            return;
                        }
                    }
                }

                let kicked_session = guild.members.iter().find_map(|m| {
                    if m.name == msg.member_name { m.session_id } else { None }
                });

                if guild.remove_member(&msg.member_name) {
                    // 更新被踢玩家
                    if let Some(sid) = kicked_session {
                        if let Some(rec) = self.players.get(&sid) {
                            let _ = rec.ask(SetGuildInfo {
                                guild_name: None, rank: GuildRank::Member,
                            }).await;
                            send_guild_status_packet(&self.gate_ref, sid, false);
                        }
                    }
                    // 通知行会成员
                    for sid in guild.online_sessions(0) {
                        send_guild_member_change_packet(&self.gate_ref, sid, &msg.member_name, false);
                    }
                    send_system_message(&self.gate_ref, msg.session_id, &format!("{} 已被踢出行会", msg.member_name));
                }
            }
            1 => { // 升职
                if my_rank != GuildRank::Leader {
                    send_system_message(&self.gate_ref, msg.session_id, "只有会长可以升职成员");
                    return;
                }
                if guild.set_rank(&msg.member_name, GuildRank::Officer) {
                    send_system_message(&self.gate_ref, msg.session_id, &format!("{} 已升职为副会长", msg.member_name));
                }
            }
            2 => { // 降职
                if my_rank != GuildRank::Leader {
                    send_system_message(&self.gate_ref, msg.session_id, "只有会长可以降职成员");
                    return;
                }
                if guild.set_rank(&msg.member_name, GuildRank::Member) {
                    send_system_message(&self.gate_ref, msg.session_id, &format!("{} 已降职为成员", msg.member_name));
                }
            }
            _ => {}
        }
    }
}

impl Message<EditGuildNoticeRequest> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: EditGuildNoticeRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r, None => return,
        };
        let state = match record.ask(GetPlayerState).await {
            Ok(Some(s)) => s, _ => return,
        };

        let guild_name = match &state.guild_name {
            Some(n) => n.clone(), None => return,
        };

        let guild = match self.guilds.get_mut(&guild_name) {
            Some(g) => g, None => return,
        };

        guild.notice = msg.notice.clone();

        // 通知所有在线行会成员
        for sid in guild.online_sessions(0) {
            send_guild_notice_change_packet(&self.gate_ref, sid, &guild.notice);
        }

        send_system_message(&self.gate_ref, msg.session_id, "行会公告已更新");
    }
}

impl Message<LeaveGuildRequest> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: LeaveGuildRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r, None => return,
        };
        let state = match record.ask(GetPlayerState).await {
            Ok(Some(s)) => s, _ => return,
        };

        let guild_name = match &state.guild_name {
            Some(n) => n.clone(), None => return,
        };

        // 会长不能离开行会（必须先解散或转让）
        if state.guild_rank == GuildRank::Leader {
            send_system_message(&self.gate_ref, msg.session_id, "会长不能离开行会");
            return;
        }

        let guild = match self.guilds.get_mut(&guild_name) {
            Some(g) => g, None => return,
        };

        guild.remove_member(&state.name);
        let _ = record.ask(SetGuildInfo {
            guild_name: None, rank: GuildRank::Member,
        }).await;
        send_guild_status_packet(&self.gate_ref, msg.session_id, false);

        // 通知其他行会成员
        for sid in guild.online_sessions(0) {
            send_guild_member_change_packet(&self.gate_ref, sid, &state.name, false);
        }

        send_system_message(&self.gate_ref, msg.session_id, &format!("已离开行会 \"{}\"", guild_name));
    }
}

impl Message<GuildStorageGoldChangeRequest> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: GuildStorageGoldChangeRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r, None => return,
        };
        let state = match record.ask(GetPlayerState).await {
            Ok(Some(s)) => s, _ => return,
        };

        let guild_name = match &state.guild_name {
            Some(n) => n.clone(), None => return,
        };

        let guild = match self.guilds.get_mut(&guild_name) {
            Some(g) => g, None => return,
        };

        match msg.change_type {
            0 => { // 存入
                let has_gold = { state.inventory.gold >= msg.amount as u64 };
                if !has_gold {
                    send_system_message(&self.gate_ref, msg.session_id, "金币不足");
                    return;
                }
                let _ = record.ask(DeductGold { amount: msg.amount as u64 }).await;
                guild.gold += msg.amount as u64;
                send_system_message(&self.gate_ref, msg.session_id, &format!("已存入 {} 金币到行会仓库", msg.amount));
            }
            1 => { // 取出
                // 只有会长和副会长可以取出
                if state.guild_rank != GuildRank::Leader && state.guild_rank != GuildRank::Officer {
                    send_system_message(&self.gate_ref, msg.session_id, "权限不足");
                    return;
                }
                if guild.gold < msg.amount as u64 {
                    send_system_message(&self.gate_ref, msg.session_id, "行会仓库金币不足");
                    return;
                }
                guild.gold -= msg.amount as u64;
                let _ = record.ask(AddGold { amount: msg.amount as u64 }).await;
                send_system_message(&self.gate_ref, msg.session_id, &format!("已从行会仓库取出 {} 金币", msg.amount));
            }
            _ => {}
        }
    }
}

impl Message<GuildStorageItemChangeRequest> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: GuildStorageItemChangeRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r, None => return,
        };
        let state = match record.ask(GetPlayerState).await {
            Ok(Some(s)) => s, _ => return,
        };

        let guild_name = match &state.guild_name {
            Some(n) => n.clone(), None => {
                send_system_message(&self.gate_ref, msg.session_id, "你还没有加入行会");
                return;
            }
        };

        let guild = match self.guilds.get_mut(&guild_name) {
            Some(g) => g, None => return,
        };

        match msg.change_type {
            0 => { // 存入物品
                if !guild.storage_has_space() {
                    send_system_message(&self.gate_ref, msg.session_id, "行会仓库已满");
                    return;
                }

                if state.inventory.get_item(msg.unique_id).is_none() {
                    send_system_message(&self.gate_ref, msg.session_id, "物品不存在");
                    return;
                }

                let removed = record.ask(RemoveItemFromInventory { unique_id: msg.unique_id }).await.unwrap_or(None);
                if let Some(removed_item) = removed {
                    let item_index = removed_item.item_index;
                    let slot = guild.deposit_item(removed_item.clone(), msg.count);
                    if let Some(slot_val) = slot {
                        send_system_message(&self.gate_ref, msg.session_id, "物品已存入行会仓库");
                        debug!("GuildStorageItem: {} deposited item={} slot={}", state.name, item_index, slot_val);
                    } else {
                        let _ = record.ask(AddItemToInventory { item: removed_item }).await;
                        send_system_message(&self.gate_ref, msg.session_id, "行会仓库已满");
                    }
                }
            }
            1 => { // 取出物品
                if state.guild_rank != GuildRank::Leader && state.guild_rank != GuildRank::Officer {
                    send_system_message(&self.gate_ref, msg.session_id, "权限不足");
                    return;
                }

                if !state.inventory.has_space() {
                    send_system_message(&self.gate_ref, msg.session_id, "背包已满");
                    return;
                }

                let result = guild.withdraw_item(msg.grid);
                match result {
                    Some((item_data, qty, _slot)) => {
                        let added = record.ask(AddItemToInventory { item: item_data.clone() }).await.unwrap_or(false);
                        if added {
                            send_system_message(&self.gate_ref, msg.session_id, "物品已取出");
                        } else {
                            guild.storage_items[msg.grid as usize] = Some((item_data, qty));
                            send_system_message(&self.gate_ref, msg.session_id, "背包已满");
                        }
                    }
                    None => {
                        send_system_message(&self.gate_ref, msg.session_id, "该仓库格子没有物品");
                    }
                }
            }
            _ => {}
        }
    }
}

// ============================================================
// 婚姻系统 Handler
// ============================================================

impl Message<MarriageRequest> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: MarriageRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let requester_record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let requester_state = match requester_record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 检查是否已有配偶
        if requester_state.spouse_name.is_some() {
            send_system_message(&self.gate_ref, msg.session_id, "你已经结婚了");
            return;
        }

        // 查找目标玩家
        let target_session = match self.find_player_by_name(&msg.target_name, msg.session_id).await {
            Some(sid) => sid,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "目标玩家不在线");
                return;
            }
        };

        let target_record = match self.players.get(&target_session) {
            Some(r) => r,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "目标玩家已下线");
                return;
            }
        };
        let target_state = match target_record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 检查目标是否已有配偶
        if target_state.spouse_name.is_some() {
            send_system_message(&self.gate_ref, msg.session_id, "目标玩家已经结婚了");
            return;
        }

        // 发送结婚请求给目标
        self.pending_marriage_invites.insert(target_session, msg.session_id);
        send_marriage_invite_packet(&self.gate_ref, target_session, &requester_state.name);
        debug!("MarriageRequest: {} -> {} (session {})", requester_state.name, msg.target_name, target_session);
    }
}

impl Message<MarriageReply> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: MarriageReply, _ctx: &mut Context<Self, Self::Reply>) {
        let replier_session = msg.session_id;

        // 查找谁发送了邀请
        let requester_session = match self.pending_marriage_invites.remove(&replier_session) {
            Some(sid) => sid,
            None => {
                send_system_message(&self.gate_ref, replier_session, "没有待处理的结婚请求");
                return;
            }
        };

        let replier_record = match self.players.get(&replier_session) {
            Some(r) => r,
            None => return,
        };
        let replier_state = match replier_record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        if !msg.accept {
            send_system_message(&self.gate_ref, replier_session, "你拒绝了结婚请求");
            // 通知邀请方被拒绝
            if let Some(req_record) = self.players.get(&requester_session) {
                let req_state = match req_record.ask(GetPlayerState).await {
                    Ok(Some(s)) => s,
                    _ => return,
                };
                send_system_message(&self.gate_ref, requester_session, &format!("{} 拒绝了结婚请求", replier_state.name));
                debug!("MarriageReply: {} rejected {}'s proposal", replier_state.name, req_state.name);
            }
            return;
        }

        // 双方确认婚姻关系
        let requester_record = match self.players.get(&requester_session) {
            Some(r) => r,
            None => {
                send_system_message(&self.gate_ref, replier_session, "对方已不在线");
                return;
            }
        };
        let requester_state = match requester_record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        let _ = replier_record.ask(SetSpouse { spouse_name: Some(requester_state.name.clone()) }).await;
        let _ = requester_record.ask(SetSpouse { spouse_name: Some(replier_state.name.clone()) }).await;

        send_system_message(&self.gate_ref, replier_session, &format!("结婚成功，你的配偶是: {}", requester_state.name));
        send_system_message(&self.gate_ref, requester_session, &format!("结婚成功，你的配偶是: {}", replier_state.name));
        send_marriage_status_packet(&self.gate_ref, replier_session, true);
        send_marriage_status_packet(&self.gate_ref, requester_session, true);
        debug!("Marriage: {} <-> {} married", requester_state.name, replier_state.name);
    }
}

impl Message<SocialDivorceRequest> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: SocialDivorceRequest, _ctx: &mut Context<Self, Self::Reply>) {
        let requester_record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let requester_state = match requester_record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 检查是否已婚
        if requester_state.spouse_name.is_none() {
            send_system_message(&self.gate_ref, msg.session_id, "你还没有结婚");
            return;
        }

        // 查找配偶是否在线
        let target_session = match self.find_player_by_name(&msg.partner_name, msg.session_id).await {
            Some(sid) => sid,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "目标玩家不在线");
                return;
            }
        };

        let target_record = match self.players.get(&target_session) {
            Some(r) => r,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "目标玩家已下线");
                return;
            }
        };
        let target_state = match target_record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 确认确实是配偶关系
        if target_state.spouse_name.as_deref() != Some(&requester_state.name) {
            send_system_message(&self.gate_ref, msg.session_id, "对方不是你的配偶");
            return;
        }

        // 发送离婚请求
        send_divorce_request_packet(&self.gate_ref, target_session, &requester_state.name);
        debug!("DivorceRequest: {} -> {}", requester_state.name, msg.partner_name);
    }
}

impl Message<SocialDivorceReply> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: SocialDivorceReply, _ctx: &mut Context<Self, Self::Reply>) {
        let replier_record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let replier_state = match replier_record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        if !msg.accept {
            send_system_message(&self.gate_ref, msg.session_id, "你拒绝了离婚请求");
            return;
        }

        // 双方解除婚姻关系
        let spouse_name = replier_state.spouse_name.clone();
        let _ = replier_record.ask(SetSpouse { spouse_name: None }).await;

        // 通知前配偶
        if let Some(ref name) = spouse_name {
            if let Some(target_session) = self.find_player_by_name(name, msg.session_id).await {
                if let Some(target_record) = self.players.get(&target_session) {
                    let _ = target_record.ask(SetSpouse { spouse_name: None }).await;
                    send_system_message(&self.gate_ref, target_session, "你已离婚");
                }
            }
        }

        send_system_message(&self.gate_ref, msg.session_id, "离婚成功");
        send_marriage_status_packet(&self.gate_ref, msg.session_id, false);
        debug!("DivorceReply: {} divorced", replier_state.name);
    }
}

impl Message<SocialChangeMarriage> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: SocialChangeMarriage, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        let in_marriage = state.spouse_name.is_some();
        send_marriage_status_packet(&self.gate_ref, msg.session_id, in_marriage);
    }
}

// ============================================================
// 师徒系统 Handler
// ============================================================

impl Message<SocialAddMentor> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: SocialAddMentor, _ctx: &mut Context<Self, Self::Reply>) {
        let requester_record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let requester_state = match requester_record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 检查是否已有导师
        if requester_state.mentor_name.is_some() {
            send_system_message(&self.gate_ref, msg.session_id, "你已经有导师了");
            return;
        }

        // 查找目标玩家
        let target_session = match self.find_player_by_name(&msg.mentor_name, msg.session_id).await {
            Some(sid) => sid,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "目标玩家不在线");
                return;
            }
        };

        let target_record = match self.players.get(&target_session) {
            Some(r) => r,
            None => {
                send_system_message(&self.gate_ref, msg.session_id, "目标玩家已下线");
                return;
            }
        };
        let target_state = match target_record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // 检查目标是否允许拜师
        if !target_state.allow_mentor {
            send_system_message(&self.gate_ref, msg.session_id, "目标玩家当前不允许拜师");
            return;
        }

        // 发送拜师请求给目标
        self.pending_mentor_invites.insert(target_session, msg.session_id);
        send_mentor_invite_packet(&self.gate_ref, target_session, &requester_state.name);
        debug!("AddMentor: {} -> {}", requester_state.name, msg.mentor_name);
    }
}

impl Message<SocialMentorReply> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: SocialMentorReply, _ctx: &mut Context<Self, Self::Reply>) {
        let replier_session = msg.session_id;

        // 查找谁发送了邀请
        let requester_session = match self.pending_mentor_invites.remove(&replier_session) {
            Some(sid) => sid,
            None => {
                send_system_message(&self.gate_ref, replier_session, "没有待处理的拜师请求");
                return;
            }
        };

        let replier_record = match self.players.get(&replier_session) {
            Some(r) => r,
            None => return,
        };
        let replier_state = match replier_record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        if !msg.accept {
            send_system_message(&self.gate_ref, replier_session, "你拒绝了拜师请求");
            if let Some(req_record) = self.players.get(&requester_session) {
                let req_state = match req_record.ask(GetPlayerState).await {
                    Ok(Some(s)) => s,
                    _ => return,
                };
                send_system_message(&self.gate_ref, requester_session, &format!("{} 拒绝了拜师请求", replier_state.name));
                debug!("MentorReply: {} rejected {}'s request", replier_state.name, req_state.name);
            }
            return;
        }

        // 确认师徒关系
        let requester_record = match self.players.get(&requester_session) {
            Some(r) => r,
            None => {
                send_system_message(&self.gate_ref, replier_session, "对方已不在线");
                return;
            }
        };
        let requester_state = match requester_record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        // replier = 导师，requester = 徒弟
        let _ = replier_record.ask(SetMentor { mentor_name: None }).await;
        let _ = requester_record.ask(SetMentor { mentor_name: Some(replier_state.name.clone()) }).await;

        send_system_message(&self.gate_ref, replier_session, &format!("收徒成功，你的徒弟是: {}", requester_state.name));
        send_system_message(&self.gate_ref, requester_session, &format!("拜师成功，你的导师是: {}", replier_state.name));
        debug!("Mentor: {} is mentor of {}", replier_state.name, requester_state.name);
    }
}

impl Message<SocialAllowMentor> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: SocialAllowMentor, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let _ = record.ask(SetAllowMentor { allow: msg.allow }).await;
        send_system_message(
            &self.gate_ref,
            msg.session_id,
            if msg.allow { "已允许拜师" } else { "已禁止拜师" },
        );
        debug!("AllowMentor: session={} allow={}", msg.session_id, msg.allow);
    }
}

impl Message<SocialCancelMentor> for SocialActor {
    type Reply = ();

    async fn handle(&mut self, msg: SocialCancelMentor, _ctx: &mut Context<Self, Self::Reply>) {
        let record = match self.players.get(&msg.session_id) {
            Some(r) => r,
            None => return,
        };
        let state = match record.ask(GetPlayerState).await {
            Ok(Some(s)) => s,
            _ => return,
        };

        if state.mentor_name.is_none() {
            send_system_message(&self.gate_ref, msg.session_id, "你没有导师");
            return;
        }

        let _ = record.ask(SetMentor { mentor_name: None }).await;
        send_system_message(&self.gate_ref, msg.session_id, "已解除师徒关系");
        debug!("CancelMentor: {} removed mentor", state.name);
    }
}
