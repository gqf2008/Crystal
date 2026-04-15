// GateActor - TCP 接入层
// 对应 C# LoginGate/AppServer.cs + SelGate + GameGate
// 职责：接受 TCP 连接，解析帧，转发到 AccountActor/WorldActor

use std::collections::HashMap;

use kameo::actor::{Actor, ActorRef};
use kameo::message::Message;
use kameo::prelude::Context;
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use tracing::{info, warn, error, debug};

use mir2_shared::enums::{ClientPacketIds, ServerPacketIds};

use super::codec::{encode, decode};
use crate::util::wire::{build_packet_bytes, write_dotnet_string};

/// 会话 ID
pub type SessionId = u64;

/// 发送到客户端的数据通道
type SendChannel = mpsc::UnboundedSender<Vec<u8>>;

/// GateActor 状态
pub struct GateActor {
    /// 活跃会话的发送通道
    sessions: HashMap<SessionId, SendChannel>,
    /// AccountActor 引用
    account_ref: Option<ActorRef<crate::actors::account::AccountActor>>,
    /// WorldActor 引用
    world_ref: Option<ActorRef<crate::actors::world::WorldActor>>,
}

impl GateActor {
    pub fn new() -> Self {
        Self {
            sessions: HashMap::new(),
            account_ref: None,
            world_ref: None,
        }
    }

    pub fn set_account_ref(&mut self, account_ref: ActorRef<crate::actors::account::AccountActor>) {
        self.account_ref = Some(account_ref);
    }

    pub fn set_world_ref(&mut self, world_ref: ActorRef<crate::actors::world::WorldActor>) {
        self.world_ref = Some(world_ref);
    }
}

impl Actor for GateActor {
    type Args = ();
    type Error = anyhow::Error;

    async fn on_start(_args: (), _actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        info!("GateActor started");
        Ok(Self::new())
    }
}

impl Default for GateActor {
    fn default() -> Self {
        Self::new()
    }
}

/// 启动 TCP 监听并处理连接
pub async fn run_gate_listener(addr: String, actor_ref: ActorRef<GateActor>) -> anyhow::Result<()> {
    let listener = TcpListener::bind(&addr).await?;
    info!("Gate listening on {}", addr);

    let mut session_id: SessionId = 1;

    loop {
        let (mut stream, peer_addr) = listener.accept().await?;
        debug!("New connection from {}", peer_addr);

        let sid = session_id;
        session_id += 1;

        // 为每个会话创建发送通道
        let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();

        // 注册会话到 GateActor
        let _ = actor_ref.ask(SessionCreated {
            session_id: sid,
            sender: tx,
        }).await;

        let gate_ref = actor_ref.clone();

        tokio::spawn(async move {
            let mut buf = Vec::with_capacity(4096);
            let mut temp = [0u8; 4096];

            loop {
                tokio::select! {
                    // 从网络读取数据
                    read_result = stream.read(&mut temp) => {
                        match read_result {
                            Ok(0) => {
                                debug!("Session {} disconnected", sid);
                                let _ = gate_ref.ask(ClientDisconnected { session_id: sid }).await;
                                return;
                            }
                            Ok(n) => {
                                buf.extend_from_slice(&temp[..n]);

                                // 尝试解码所有完整帧
                                while let Some((payload, consumed)) = decode(&buf) {
                                    let _ = gate_ref.ask(ClientData {
                                        session_id: sid,
                                        data: payload,
                                    }).await;
                                    buf.drain(..consumed);
                                }
                            }
                            Err(e) => {
                                error!("Session {} read error: {}", sid, e);
                                let _ = gate_ref.ask(ClientDisconnected { session_id: sid }).await;
                                return;
                            }
                        }
                    }
                    // 发送数据到客户端
                    Some(data) = rx.recv() => {
                        let mut encoded = Vec::new();
                        encode(&data, &mut encoded);
                        if let Err(e) = stream.write_all(&encoded).await {
                            error!("Session {} write error: {}", sid, e);
                            let _ = gate_ref.ask(ClientDisconnected { session_id: sid }).await;
                            return;
                        }
                    }
                }
            }
        });
    }
}

// ============================================================
// 消息定义
// ============================================================

/// 会话创建（内部）
pub struct SessionCreated {
    pub session_id: SessionId,
    pub sender: SendChannel,
}

/// 收到客户端数据
pub struct ClientData {
    pub session_id: SessionId,
    pub data: Vec<u8>,
}

/// 向客户端发送数据
pub struct SendToClient {
    pub session_id: SessionId,
    pub data: Vec<u8>,
}

/// 客户端断开连接
pub struct ClientDisconnected {
    pub session_id: SessionId,
}

/// 登录结果（从 AccountActor 返回）
pub struct LoginResult {
    pub session_id: SessionId,
    pub success: bool,
}

/// 设置 AccountActor 引用
pub struct SetAccountRef {
    pub account_ref: ActorRef<crate::actors::account::AccountActor>,
}

/// 设置 WorldActor 引用
pub struct SetWorldRef {
    pub world_ref: ActorRef<crate::actors::world::WorldActor>,
}

// ============================================================
// Handler 实现
// ============================================================

impl Message<SessionCreated> for GateActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SessionCreated,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.sessions.insert(msg.session_id, msg.sender);
        debug!("Session {} created", msg.session_id);

        // 发送 Connected 包给客户端（客户端收到后会自动发送 ClientVersion）
        let connected_data = build_packet_bytes(ServerPacketIds::Connected as i16, &[]);
        let gate_ref = _ctx.actor_ref().clone();
        let _ = gate_ref.ask(SendToClient {
            session_id: msg.session_id,
            data: connected_data,
        });
    }
}

impl Message<ClientData> for GateActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: ClientData,
        ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        debug!(
            "Session {} received {} bytes (decoded)",
            msg.session_id,
            msg.data.len()
        );

        // 解析内层 PacketHeader (4 bytes: length u16 + opcode i16)
        const HEADER_SIZE: usize = 4;
        if msg.data.len() < HEADER_SIZE {
            warn!("Session {} received data too short for packet header", msg.session_id);
            return;
        }

        let length = u16::from_le_bytes([msg.data[0], msg.data[1]]) as usize;
        let opcode = i16::from_le_bytes([msg.data[2], msg.data[3]]);

        debug!("Session {} packet: length={}, opcode={}", msg.session_id, length, opcode);

        // 验证长度一致性
        if length > msg.data.len() || length < HEADER_SIZE {
            warn!("Session {} packet length mismatch: declared={}, available={}",
                  msg.session_id, length, msg.data.len());
            return;
        }

        let payload = &msg.data[HEADER_SIZE..length];
        let gate_ref = ctx.actor_ref().clone();

        match opcode {
            x if x == ClientPacketIds::ClientVersion as i16 => {
                // ClientVersion - 验证 payload 后回复 accepted
                handle_client_version(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::NewAccount as i16 => {
                // NewAccount - Phase 1: 自动成功
                handle_new_account(&gate_ref, msg.session_id);
            }
            x if x == ClientPacketIds::Login as i16 => {
                // Login - 转发到 AccountActor
                if let Some(account_ref) = &self.account_ref {
                    if let Some((username, password)) = parse_login_payload(payload) {
                        debug!("Login request: username={}", username);
                        let _ = account_ref.ask(crate::actors::account::LoginRequest {
                            session_id: msg.session_id,
                            username,
                            password,
                        }).await;
                    }
                } else {
                    warn!("AccountActor not linked");
                }
            }
            x if x == ClientPacketIds::StartGame as i16 => {
                // StartGame - 转发到 WorldActor
                if let Some(world_ref) = &self.world_ref {
                    if payload.len() >= 4 {
                        let character_index = i32::from_le_bytes([payload[0], payload[1], payload[2], payload[3]]);
                        debug!("StartGame request: character_index={}", character_index);
                        let _ = world_ref.ask(crate::actors::world::StartGameRequest {
                            session_id: msg.session_id,
                            character_index,
                        }).await;
                    }
                } else {
                    warn!("WorldActor not linked");
                }
            }
            x if x == ClientPacketIds::Turn as i16 => {
                // Turn - 转发到 WorldActor
                if let Some(world_ref) = &self.world_ref {
                    if !payload.is_empty() {
                        let direction = payload[0];
                        let _ = world_ref.ask(crate::actors::world::WorldTurnRequest {
                            session_id: msg.session_id,
                            direction,
                        }).await;
                    }
                }
            }
            x if x == ClientPacketIds::Walk as i16 => {
                // Walk - 转发到 WorldActor
                if let Some(world_ref) = &self.world_ref {
                    if !payload.is_empty() {
                        let direction = payload[0];
                        let _ = world_ref.ask(crate::actors::world::WorldMoveRequest {
                            session_id: msg.session_id,
                            direction,
                            is_run: false,
                        }).await;
                    }
                }
            }
            x if x == ClientPacketIds::Run as i16 => {
                // Run - 转发到 WorldActor
                if let Some(world_ref) = &self.world_ref {
                    if !payload.is_empty() {
                        let direction = payload[0];
                        let _ = world_ref.ask(crate::actors::world::WorldMoveRequest {
                            session_id: msg.session_id,
                            direction,
                            is_run: true,
                        }).await;
                    }
                }
            }
            x if x == ClientPacketIds::Attack as i16 => {
                // Attack - 转发到 WorldActor
                if let Some(world_ref) = &self.world_ref {
                    if payload.len() >= 2 {
                        let direction = payload[0];
                        let spell = payload[1];
                        debug!("Attack: session={} dir={} spell={}", msg.session_id, direction, spell);
                        let _ = world_ref.ask(crate::actors::world::WorldAttackRequest {
                            session_id: msg.session_id,
                            direction,
                            spell,
                        }).await;
                    }
                }
            }
            x if x == ClientPacketIds::KeepAlive as i16 => {
                // KeepAlive - 回复心跳
                handle_keep_alive(&gate_ref, msg.session_id);
            }
            x if x == ClientPacketIds::LogOut as i16 => {
                // LogOut - 通知 WorldActor 清理并断开
                if let Some(world_ref) = &self.world_ref {
                    let _ = world_ref.ask(crate::actors::world::PlayerLogOut {
                        session_id: msg.session_id,
                    }).await;
                }
            }
            x if x == ClientPacketIds::Disconnect as i16 => {
                // Disconnect - 客户端主动断开，由 kameo ClientDisconnected 处理清理
                debug!("Client disconnect request from session {}", msg.session_id);
            }
            x if x == ClientPacketIds::Chat as i16 => {
                // Chat - 解析并广播
                if let Some(world_ref) = &self.world_ref {
                    if let Some(message) = parse_chat_payload(payload) {
                        let _ = world_ref.ask(crate::actors::world::ChatRequest {
                            session_id: msg.session_id,
                            message,
                        }).await;
                    }
                }
            }
            x if x == ClientPacketIds::CallNPC as i16 => {
                // CallNPC - 与 NPC 对话
                if let Some(world_ref) = &self.world_ref {
                    if let Some((npc_object_id, key)) = parse_call_npc_payload(payload) {
                        let _ = world_ref.ask(crate::actors::world::NPCCallRequest {
                            session_id: msg.session_id,
                            npc_object_id,
                            key,
                        }).await;
                    }
                }
            }
            x if x == ClientPacketIds::PickUp as i16 => {
                if let Some(world_ref) = &self.world_ref {
                    let _ = world_ref.ask(crate::actors::world::PickUpRequest {
                        session_id: msg.session_id,
                    });
                }
            }
            x if x == ClientPacketIds::MoveItem as i16 => {
                forward_move_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::UseItem as i16 => {
                forward_use_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::EquipItem as i16 => {
                forward_equip_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RemoveItem as i16 => {
                forward_remove_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::DropItem as i16 => {
                forward_drop_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::MergeItem as i16 => {
                forward_merge_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RangeAttack as i16 => {
                forward_range_attack(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::Magic as i16 => {
                forward_magic(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::Harvest as i16 => {
                forward_harvest(&self.world_ref, msg.session_id, payload);
            }
            // NPC 商店
            x if x == ClientPacketIds::BuyItem as i16 => {
                forward_buy_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::SellItem as i16 => {
                forward_sell_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RepairItem as i16 => {
                forward_repair_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::SRepairItem as i16 => {
                forward_repair_item(&self.world_ref, msg.session_id, payload); // 特殊修理走相同逻辑
            }
            x if x == ClientPacketIds::CraftItem as i16 => {
                forward_craft_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::BuyItemBack as i16 => {
                forward_buy_item_back(&self.world_ref, msg.session_id, payload);
            }
            // 仓库操作
            x if x == ClientPacketIds::StoreItem as i16 => {
                handle_store_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::TakeBackItem as i16 => {
                handle_take_back_item(&self.world_ref, msg.session_id, payload);
            }
            // 金币
            x if x == ClientPacketIds::DropGold as i16 => {
                handle_drop_gold(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::Inspect as i16 => {
                handle_inspect(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::ChangeAMode as i16 => {
                forward_change_amode(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::ChangePMode as i16 => {
                forward_change_pmode(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::MagicKey as i16 => {
                handle_magic_key(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RemoveSlotItem as i16 => {
                handle_remove_slot_item(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::SplitItem as i16 => {
                forward_split_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::TeleportToNPC as i16 => {
                forward_teleport_to_npc(&self.world_ref, msg.session_id, payload);
            }
            // 死亡恢复
            x if x == ClientPacketIds::TownRevive as i16 => {
                forward_town_revive(&self.world_ref, msg.session_id);
            }
            x if x == ClientPacketIds::SpellToggle as i16 => {
                handle_spell_toggle(&gate_ref, msg.session_id, payload);
            }
            // 账号管理
            x if x == ClientPacketIds::NewCharacter as i16 => {
                forward_new_character(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::ChangePassword as i16 => {
                forward_change_password(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::DeleteCharacter as i16 => {
                forward_delete_character(&self.world_ref, msg.session_id, payload);
            }
            // 社交/组队
            x if x == ClientPacketIds::SwitchGroup as i16 => {
                forward_switch_group(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::AddMember as i16 => {
                forward_add_member(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::DellMember as i16 => {
                forward_dell_member(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::GroupInvite as i16 => {
                forward_group_invite(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::NewHero as i16 => {
                forward_new_hero(&self.world_ref, msg.session_id, payload);
            }
            // 交易
            x if x == ClientPacketIds::ChangeTrade as i16 => {
                forward_change_trade(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::TradeRequest as i16 => {
                forward_trade_request(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::TradeReply as i16 => {
                forward_trade_reply(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::TradeConfirm as i16 => {
                forward_trade_confirm(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::TradeCancel as i16 => {
                forward_trade_cancel(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::TradeGold as i16 => {
                forward_trade_gold(&self.world_ref, msg.session_id, payload);
            }
            // 好友
            x if x == ClientPacketIds::AddFriend as i16 => {
                forward_add_friend(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RemoveFriend as i16 => {
                forward_remove_friend(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RefreshFriends as i16 => {
                forward_refresh_friends(&self.world_ref, msg.session_id);
            }
            x if x == ClientPacketIds::AddMemo as i16 => {
                forward_add_memo(&self.world_ref, msg.session_id, payload);
            }
            // 邮件
            x if x == ClientPacketIds::SendMail as i16 => {
                handle_send_mail(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::ReadMail as i16 => {
                handle_read_mail(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::CollectParcel as i16 => {
                handle_collect_parcel(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::DeleteMail as i16 => {
                handle_delete_mail(&self.world_ref, msg.session_id, payload);
            }
            // 行会
            x if x == ClientPacketIds::GuildInvite as i16 => {
                handle_guild_invite(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RequestGuildInfo as i16 => {
                handle_request_guild_info(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::EditGuildMember as i16 => {
                handle_edit_guild_member(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::EditGuildNotice as i16 => {
                handle_edit_guild_notice(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::GuildNameReturn as i16 => {
                handle_guild_name_return(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::GuildStorageGoldChange as i16 => {
                handle_guild_storage_gold(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::GuildStorageItemChange as i16 => {
                handle_guild_storage_item(&self.world_ref, msg.session_id, payload);
            }
            // 婚姻
            x if x == ClientPacketIds::MarriageRequest as i16 => {
                handle_marriage_request(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::MarriageReply as i16 => {
                handle_marriage_reply(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::ChangeMarriage as i16 => {
                handle_change_marriage(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::DivorceRequest as i16 => {
                handle_divorce_request(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::DivorceReply as i16 => {
                handle_divorce_reply(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::AddMentor as i16 => {
                handle_add_mentor(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::MentorReply as i16 => {
                handle_mentor_reply(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::AllowMentor as i16 => {
                handle_allow_mentor(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::CancelMentor as i16 => {
                handle_cancel_mentor(&self.world_ref, msg.session_id, payload);
            }
            // 任务
            x if x == ClientPacketIds::AcceptQuest as i16 => {
                handle_accept_quest(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::FinishQuest as i16 => {
                handle_finish_quest(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::AbandonQuest as i16 => {
                handle_abandon_quest(&self.world_ref, msg.session_id, payload);
            }
            //  精炼
            x if x == ClientPacketIds::DepositRefineItem as i16 => {
                handle_deposit_refine_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RetrieveRefineItem as i16 => {
                handle_retrieve_refine_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RefineCancel as i16 => {
                handle_refine_cancel(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RefineItem as i16 => {
                handle_refine_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::CheckRefine as i16 => {
                handle_check_refine(&self.world_ref, msg.session_id, payload);
            }
            // 传送/地图
            x if x == ClientPacketIds::RequestMapInfo as i16 => {
                forward_request_map_info(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::SearchMap as i16 => {
                forward_search_map(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::Observe as i16 => {
                handle_observe(&gate_ref, msg.session_id, payload);
            }
            // 其他
            x if x == ClientPacketIds::RequestUserName as i16 => {
                handle_request_user_name(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RequestChatItem as i16 => {
                handle_request_chat_item(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::SetAutoPotValue as i16 => {
                handle_set_autopot_value(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::SetAutoPotItem as i16 => {
                handle_set_autopot_item(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::SetHeroBehaviour as i16 => {
                handle_set_hero_behaviour(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::ChangeHero as i16 => {
                handle_change_hero(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::TakeBackHeroItem as i16 => {
                handle_take_back_hero_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::TransferHeroItem as i16 => {
                handle_transfer_hero_item(&self.world_ref, msg.session_id, payload);
            }
            // 宠物
            x if x == ClientPacketIds::UpdateIntelligentCreature as i16 => {
                handle_update_intelligent_creature(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::IntelligentCreaturePickup as i16 => {
                handle_intelligent_creature_pickup(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RequestIntelligentCreatureUpdates as i16 => {
                handle_request_intelligent_creature_updates(&self.world_ref, msg.session_id, payload);
            }
            // 装备槽
            x if x == ClientPacketIds::EquipSlotItem as i16 => {
                handle_equip_slot_item(&self.world_ref, msg.session_id, payload);
            }
            // 市场/寄售
            x if x == ClientPacketIds::ConsignItem as i16 => {
                forward_consign_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::MarketSearch as i16 => {
                forward_market_search(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::MarketRefresh as i16 => {
                forward_market_refresh(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::MarketPage as i16 => {
                forward_market_page(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::MarketBuy as i16 => {
                forward_market_buy(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::MarketGetBack as i16 => {
                forward_market_get_back(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::MarketSellNow as i16 => {
                forward_market_sell_now(&self.world_ref, msg.session_id, payload);
            }
            // 钓鱼
            x if x == ClientPacketIds::FishingCast as i16 => {
                forward_fishing_cast(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::FishingChangeAutocast as i16 => {
                forward_fishing_change_autocast(&self.world_ref, msg.session_id, payload);
            }
            // 觉醒/分解
            x if x == ClientPacketIds::CombineItem as i16 => {
                forward_combine_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::AwakeningNeedMaterials as i16 => {
                handle_awakening_need_materials(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::AwakeningLockedItem as i16 => {
                handle_awakening_locked_item(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::Awakening as i16 => {
                handle_awakening(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::DisassembleItem as i16 => {
                forward_disassemble_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::DowngradeAwakening as i16 => {
                handle_downgrade_awakening(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::ResetAddedItem as i16 => {
                forward_reset_added_item(&self.world_ref, msg.session_id, payload);
            }
            // 交易子操作
            x if x == ClientPacketIds::DepositTradeItem as i16 => {
                forward_deposit_trade_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RetrieveTradeItem as i16 => {
                forward_retrieve_trade_item(&self.world_ref, msg.session_id, payload);
            }
            // 行会扩展
            x if x == ClientPacketIds::GuildWarReturn as i16 => {
                forward_guild_war_return(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::GuildBuffUpdate as i16 => {
                forward_guild_buff_update(&self.world_ref, msg.session_id, payload);
            }
            // 婚姻/师徒扩展
            x if x == ClientPacketIds::ReplaceWedRing as i16 => {
                handle_replace_wed_ring(&self.world_ref, msg.session_id, payload);
            }
            // 邮件扩展
            x if x == ClientPacketIds::LockMail as i16 => {
                forward_lock_mail(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::MailLockedItem as i16 => {
                forward_mail_locked_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::MailCost as i16 => {
                handle_mail_cost(&gate_ref, msg.session_id, payload);
            }
            // 轮回
            x if x == ClientPacketIds::ShareQuest as i16 => {
                forward_share_quest(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::AcceptReincarnation as i16 => {
                forward_accept_reincarnation(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::CancelReincarnation as i16 => {
                forward_cancel_reincarnation(&self.world_ref, msg.session_id, payload);
            }
            // 租赁系统
            x if x == ClientPacketIds::GetRentedItems as i16 => {
                handle_get_rented_items(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::ItemRentalRequest as i16 => {
                forward_item_rental_request(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::ItemRentalFee as i16 => {
                forward_item_rental_fee(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::ItemRentalPeriod as i16 => {
                forward_item_rental_period(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::DepositRentalItem as i16 => {
                forward_deposit_rental_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RetrieveRentalItem as i16 => {
                forward_retrieve_rental_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::CancelItemRental as i16 => {
                forward_cancel_item_rental(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::ItemRentalLockFee as i16 => {
                forward_item_rental_lock_fee(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::ItemRentalLockItem as i16 => {
                forward_item_rental_lock_item(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::ConfirmItemRental as i16 => {
                forward_confirm_item_rental(&self.world_ref, msg.session_id, payload);
            }
            // 其他
            x if x == ClientPacketIds::NPCConfirmInput as i16 => {
                forward_npc_confirm_input(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::GameshopBuy as i16 => {
                forward_gameshop_buy(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::ReportIssue as i16 => {
                forward_report_issue(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::GetRanking as i16 => {
                forward_get_ranking(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::Opendoor as i16 => {
                forward_opendoor(&self.world_ref, msg.session_id, payload);
            }
            // 行会领地 (auto-value enums)
            x if x == ClientPacketIds::GuildTerritoryPage as i16 => {
                forward_guild_territory_page(&self.world_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::PurchaseGuildTerritory as i16 => {
                forward_purchase_guild_territory(&self.world_ref, msg.session_id, payload);
            }
            _ => {
                debug!("Unknown opcode {} from session {}", opcode, msg.session_id);
            }
        }
    }
}

impl Message<SendToClient> for GateActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SendToClient,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if let Some(tx) = self.sessions.get(&msg.session_id) {
            let _ = tx.send(msg.data);
        } else {
            warn!(
                "Attempted to send to non-existent session {}",
                msg.session_id
            );
        }
    }
}

impl Message<ClientDisconnected> for GateActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: ClientDisconnected,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.sessions.remove(&msg.session_id);
        debug!("Session {} disconnected", msg.session_id);

        // 通知 WorldActor 清理玩家状态
        if let Some(world_ref) = &self.world_ref {
            let _ = world_ref.ask(crate::actors::world::PlayerDisconnected {
                session_id: msg.session_id,
            }).await;
        }
    }
}

impl Message<LoginResult> for GateActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: LoginResult,
        ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let response_data = if msg.success {
            // LoginSuccess: 空角色列表
            let mut body = Vec::new();
            body.extend_from_slice(&0i32.to_le_bytes()); // count = 0
            build_packet_bytes(ServerPacketIds::LoginSuccess as i16, &body)
        } else {
            // Login failure (result=4: Wrong Password)
            build_packet_bytes(ServerPacketIds::Login as i16, &[4u8])
        };

        let gate_ref = ctx.actor_ref().clone();
        let _ = gate_ref.ask(SendToClient {
            session_id: msg.session_id,
            data: response_data,
        }).await;
    }
}

impl Message<SetAccountRef> for GateActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SetAccountRef,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.account_ref = Some(msg.account_ref);
        info!("GateActor linked to AccountActor");
    }
}

impl Message<SetWorldRef> for GateActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: SetWorldRef,
        _ctx: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.world_ref = Some(msg.world_ref);
        info!("GateActor linked to WorldActor");
    }
}

// ============================================================
// 辅助函数
// ============================================================

/// 处理客户端版本：验证 payload 后回复 accepted
fn handle_client_version(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    use std::io::Cursor;
    use byteorder::{LittleEndian, ReadBytesExt};

    // ClientVersion payload: [version_hash_length: i32 LE][version_hash: bytes]
    if payload.len() < 4 {
        warn!("Session {} ClientVersion payload too short", session_id);
        return;
    }

    let mut cursor = Cursor::new(payload);
    if let Ok(hash_len) = ReadBytesExt::read_i32::<LittleEndian>(&mut cursor) {
        if !(0..=256).contains(&hash_len) || payload.len() < 4 + hash_len as usize {
            warn!("Session {} ClientVersion invalid hash length: {}", session_id, hash_len);
            return;
        }
    }

    debug!("ClientVersion from session {}", session_id);
    let response = build_packet_bytes(ServerPacketIds::ClientVersion as i16, &[1u8]); // accepted
    let _ = gate_ref.ask(SendToClient {
        session_id,
        data: response,
    });
}

/// 处理新账号注册
fn handle_new_account(gate_ref: &ActorRef<GateActor>, session_id: SessionId) {
    debug!("NewAccount request from session {}", session_id);
    // Phase 1: auto-register, respond success (result=8)
    let response = build_packet_bytes(ServerPacketIds::NewAccount as i16, &[8u8]);
    let _ = gate_ref.ask(SendToClient {
        session_id,
        data: response,
    });
}

/// 解析登录包：account_id (DotNetString) + password (DotNetString)
fn parse_login_payload(payload: &[u8]) -> Option<(String, String)> {
    use std::io::Cursor;
    use mir2_shared::binary::read_dotnet_string;

    let mut cursor = Cursor::new(payload);
    match (read_dotnet_string(&mut cursor), read_dotnet_string(&mut cursor)) {
        (Ok(username), Ok(password)) => Some((username, password)),
        _ => None,
    }
}

/// 发送系统消息（ChatType::System），用于提示/通知。
fn send_system_message(gate_ref: &ActorRef<GateActor>, session_id: SessionId, msg: &str) {
    let mut body = Vec::new();
    write_dotnet_string(&mut body, msg);
    body.push(2u8); // ChatType::System
    let _ = gate_ref.ask(SendToClient {
        session_id,
        data: build_packet_bytes(ServerPacketIds::Chat as i16, &body),
    });
}

/// 处理心跳：回复 KeepAlive
fn handle_keep_alive(gate_ref: &ActorRef<GateActor>, session_id: SessionId) {
    let response = build_packet_bytes(ServerPacketIds::KeepAlive as i16, &[]);
    let _ = gate_ref.ask(SendToClient {
        session_id,
        data: response,
    });
}

/// 解析 DotNetString: [length: i32 LE][bytes...]
fn parse_dotnet_string(data: &[u8]) -> String {
    use std::io::Cursor;
    use mir2_shared::binary::read_dotnet_string;
    let mut cursor = Cursor::new(data);
    read_dotnet_string(&mut cursor).unwrap_or_default()
}

/// 解析聊天包：DotNetString message + i32 linked_items_count
fn parse_chat_payload(payload: &[u8]) -> Option<String> {
    use std::io::Cursor;
    use mir2_shared::binary::read_dotnet_string;

    let mut cursor = Cursor::new(payload);
    read_dotnet_string(&mut cursor).ok()
}

/// 解析 CallNPC 包：[object_id: u32 LE][key: DotNetString]
fn parse_call_npc_payload(payload: &[u8]) -> Option<(u32, String)> {
    use std::io::Cursor;
    use byteorder::{LittleEndian, ReadBytesExt};
    use mir2_shared::binary::read_dotnet_string;

    if payload.len() < 4 {
        return None;
    }
    let mut cursor = Cursor::new(payload);
    let object_id = ReadBytesExt::read_u32::<LittleEndian>(&mut cursor).ok()?;
    let key = read_dotnet_string(&mut cursor).ok()?;
    Some((object_id, key))
}

// ============================================================================
// 物品操作 forward helpers（转发到 WorldActor）
// ============================================================================

fn forward_move_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 9 { return; }
    let world_ref = match world_ref { Some(w) => w, None => return };
    let grid = payload[0];
    let from = i32::from_le_bytes(payload[1..5].try_into().unwrap_or([0; 4]));
    let to = i32::from_le_bytes(payload[5..9].try_into().unwrap_or([0; 4]));
    let _ = world_ref.ask(crate::actors::world::MoveItemRequest {
        session_id, grid, from, to,
    });
}

fn forward_use_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 8 { return; }
    let world_ref = match world_ref { Some(w) => w, None => return };
    let uid = u64::from_le_bytes(payload[..8].try_into().unwrap_or([0; 8]));
    let _ = world_ref.ask(crate::actors::world::UseItemRequest {
        session_id, unique_id: uid,
    });
}

fn forward_equip_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 10 { return; }
    let world_ref = match world_ref { Some(w) => w, None => return };
    let grid = payload[0];
    let uid = u64::from_le_bytes(payload[1..9].try_into().unwrap_or([0; 8]));
    let slot = payload[9] as i32;
    let _ = world_ref.ask(crate::actors::world::EquipItemRequest {
        session_id, grid, unique_id: uid, slot,
    });
}

fn forward_remove_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 10 { return; }
    let world_ref = match world_ref { Some(w) => w, None => return };
    let grid = payload[0];
    let uid = u64::from_le_bytes(payload[1..9].try_into().unwrap_or([0; 8]));
    let _ = world_ref.ask(crate::actors::world::RemoveItemRequest {
        session_id, grid, unique_id: uid,
    });
}

fn forward_drop_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 11 { return; }
    let world_ref = match world_ref { Some(w) => w, None => return };
    let uid = u64::from_le_bytes(payload[..8].try_into().unwrap_or([0; 8]));
    let count = u16::from_le_bytes(payload[8..10].try_into().unwrap_or([0; 2]));
    let _hero_inv = payload[10] != 0;
    let _ = world_ref.ask(crate::actors::world::DropItemRequest {
        session_id, unique_id: uid, count,
    });
}

fn forward_merge_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 18 { return; }
    let world_ref = match world_ref { Some(w) => w, None => return };
    let grid_from = payload[0];
    let grid_to = payload[1];
    let from_uid = u64::from_le_bytes(payload[2..10].try_into().unwrap_or([0; 8]));
    let to_uid = u64::from_le_bytes(payload[10..18].try_into().unwrap_or([0; 8]));
    let _ = world_ref.ask(crate::actors::world::MergeItemRequest {
        session_id, grid_from, grid_to, from_uid, to_uid,
    });
}

fn forward_split_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 13 { return; }
    let world_ref = match world_ref { Some(w) => w, None => return };
    let grid = payload[0];
    let uid = u64::from_le_bytes(payload[1..9].try_into().unwrap_or([0; 8]));
    let count = u32::from_le_bytes(payload[9..13].try_into().unwrap_or([0; 4]));
    let _ = world_ref.ask(crate::actors::world::SplitItemRequest {
        session_id, grid, unique_id: uid, count,
    });
}

fn forward_buy_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 12 { return; }
    let world_ref = match world_ref { Some(w) => w, None => return };
    let npc_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    let item_index = u32::from_le_bytes(payload[4..8].try_into().unwrap_or([0; 4]));
    let count = u32::from_le_bytes(payload[8..12].try_into().unwrap_or([0; 4]));
    let _ = world_ref.ask(crate::actors::world::BuyItemRequest {
        session_id, npc_id, item_index, count,
    });
}

fn forward_sell_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 13 { return; }
    let world_ref = match world_ref { Some(w) => w, None => return };
    let grid = payload[0];
    let uid = u64::from_le_bytes(payload[1..9].try_into().unwrap_or([0; 8]));
    let count = u32::from_le_bytes(payload[9..13].try_into().unwrap_or([0; 4]));
    let _ = world_ref.ask(crate::actors::world::SellItemRequest {
        session_id, grid, unique_id: uid, count,
    });
}

fn forward_repair_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 8 { return; }
    let world_ref = match world_ref { Some(w) => w, None => return };
    let uid = u64::from_le_bytes(payload[..8].try_into().unwrap_or([0; 8]));
    let _ = world_ref.ask(crate::actors::world::RepairItemRequest { session_id, unique_id: uid });
}

/// RangeAttack: [dir: u8][x: i32][y: i32][target_id: u32][tx: i32][ty: i32]
fn forward_range_attack(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 21 { return; }
    let world_ref = match world_ref { Some(w) => w, None => { return; } };
    let dir = payload[0];
    let target_id = u32::from_le_bytes(payload[9..13].try_into().unwrap_or([0; 4]));
    let target_x = i32::from_le_bytes(payload[13..17].try_into().unwrap_or([0; 4]));
    let target_y = i32::from_le_bytes(payload[17..21].try_into().unwrap_or([0; 4]));
    debug!("RangeAttack: session={} dir={} target={} pos=({}, {})", session_id, dir, target_id, target_x, target_y);
    let _ = world_ref.ask(crate::actors::world::RangeAttackRequest { session_id, direction: dir, target_id, target_x, target_y });
}

/// Magic: [spell: u8][dir: u8][target_id: u32][x: i32][y: i32]
fn forward_magic(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 12 { return; }
    let world_ref = match world_ref { Some(w) => w, None => { return; } };
    let spell = payload[0];
    let dir = payload[1];
    let target_id = u32::from_le_bytes(payload[2..6].try_into().unwrap_or([0; 4]));
    let target_x = i32::from_le_bytes(payload[6..10].try_into().unwrap_or([0; 4]));
    let target_y = i32::from_le_bytes(payload[10..14].try_into().unwrap_or([0; 4]));
    debug!("Magic: session={} spell={} dir={} target={} pos=({}, {})", session_id, spell, dir, target_id, target_x, target_y);
    let _ = world_ref.ask(crate::actors::world::MagicRequest { session_id, direction: dir, spell, target_id, target_x, target_y });
}

/// Harvest: [dir: u8] — 采集/挖矿请求
fn forward_harvest(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.is_empty() { return; }
    let world_ref = match world_ref { Some(w) => w, None => return };
    let dir = payload[0];
    debug!("Harvest: session={} dir={}", session_id, dir);
    let _ = world_ref.ask(crate::actors::world::HarvestRequest { session_id, direction: dir });
}

// ============================================================================
// NPC 商店 / 合成 handlers
// ============================================================================

/// CraftItem: [recipe_id: u32][materials_count: u32]
fn forward_craft_item(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 8 { return; }
    let world_ref = match world_ref { Some(w) => w, None => { return; } };
    let recipe_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!("CraftItem: session={} recipe={}", session_id, recipe_id);
    let _ = world_ref.ask(crate::actors::world::CraftItemRequest { session_id, recipe_id });
}

/// BuyItemBack (回购): [item_index: u32]
fn forward_buy_item_back(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 4 { return; }
    let world_ref = match world_ref { Some(w) => w, None => { return; } };
    let item_index = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!("BuyItemBack: session={} item_index={}", session_id, item_index);
    let _ = world_ref.ask(crate::actors::world::BuyItemBackRequest { session_id, item_index });
}

// ============================================================================
// 仓库 handlers
// ============================================================================

/// StoreItem (存入仓库): [grid: u8][unique_id: u64][count: u32]
fn handle_store_item(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 13 {
        debug!("StoreItem: session={} payload too short", session_id);
        return;
    }
    let grid = payload[0];
    let uid = u64::from_le_bytes(payload[1..9].try_into().unwrap_or([0; 8]));
    let count = u32::from_le_bytes(payload[9..13].try_into().unwrap_or([0; 4]));
    debug!("StoreItem: session={} grid={} uid={} count={}", session_id, grid, uid, count);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::StoreItemRequest {
        session_id,
        grid,
        uid,
        count,
    });
}

/// TakeBackItem (从仓库取出): [grid: u8][unique_id: u64][count: u32]
fn handle_take_back_item(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 13 {
        debug!("TakeBackItem: session={} payload too short", session_id);
        return;
    }
    let grid = payload[0];
    let uid = u64::from_le_bytes(payload[1..9].try_into().unwrap_or([0; 8]));
    let count = u32::from_le_bytes(payload[9..13].try_into().unwrap_or([0; 4]));
    debug!("TakeBackItem: session={} grid={} uid={} count={}", session_id, grid, uid, count);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::TakeBackItemRequest {
        session_id,
        grid,
        uid,
        count,
    });
}

// ============================================================================
// 其他常用 handlers
// ============================================================================

/// DropGold (丢弃/设置金币): [amount: u32]
fn handle_drop_gold(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 4 {
        debug!("DropGold: session={} payload too short", session_id);
        return;
    }
    let amount = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!("DropGold: session={} amount={}", session_id, amount);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::DropGoldRequest {
        session_id,
        amount,
    });
}

/// Inspect (查看玩家): [target_id: u32]
fn handle_inspect(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 4 {
        debug!("Inspect: session={} payload too short", session_id);
        return;
    }
    let target_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!("Inspect: session={} target={}", session_id, target_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::InspectPlayerRequest { session_id, target_id });
}

/// ChangeAMode (切换攻击模式): [mode: u8]
fn forward_change_amode(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.is_empty() { return; }
    let world_ref = match world_ref { Some(w) => w, None => return };
    let mode = payload[0];
    debug!("ChangeAMode: session={} mode={}", session_id, mode);
    let mode = mir2_shared::enums::AttackMode::try_from(mode).unwrap_or(mir2_shared::enums::AttackMode::Peace);
    let _ = world_ref.ask(crate::actors::world::ChangeAModeRequest { session_id, mode });
}

/// ChangePMode (切换宠物模式): [mode: u8]
fn forward_change_pmode(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.is_empty() { return; }
    let world_ref = match world_ref { Some(w) => w, None => return };
    let mode = payload[0];
    debug!("ChangePMode: session={} mode={}", session_id, mode);
    let mode = mir2_shared::enums::PetMode::try_from(mode).unwrap_or(mir2_shared::enums::PetMode::Both);
    let _ = world_ref.ask(crate::actors::world::ChangePModeRequest { session_id, mode });
}

/// MagicKey (设置快捷键): [slot: u8][spell_id: u16]
fn handle_magic_key(_gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 3 {
        let slot = payload[0];
        let spell_id = u16::from_le_bytes(payload[1..3].try_into().unwrap_or([0; 2]));
        debug!("MagicKey: session={} slot={} spell={}", session_id, slot, spell_id);
    }
    // 快捷键设置由客户端本地处理
}

/// RemoveSlotItem (快捷栏移除): [slot: u8]
fn handle_remove_slot_item(_gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if !payload.is_empty() {
        let slot = payload[0];
        debug!("RemoveSlotItem: session={} slot={}", session_id, slot);
    }
    // 快捷栏移除由客户端本地处理
}

/// TeleportToNPC: [npc_id: u32]
fn forward_teleport_to_npc(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 4 { return; }
    let world_ref = match world_ref { Some(w) => w, None => { return; } };
    let npc_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!("TeleportToNPC: session={} npc={}", session_id, npc_id);
    let _ = world_ref.ask(crate::actors::world::TeleportToNPCRequest { session_id, npc_id });
}

fn forward_town_revive(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
) {
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::TownReviveRequest { session_id });
}

// ============================================================================
// 死亡恢复 / 技能切换
// ============================================================================

/// SpellToggle: [spell_id: u8][on: bool]
fn handle_spell_toggle(_gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 2 {
        let spell_id = payload[0];
        let on = payload[1] != 0;
        debug!("SpellToggle: session={} spell={} on={}", session_id, spell_id, on);
    }
    // 技能开关由客户端本地处理
}

// ============================================================================
// 账号管理
// ============================================================================

/// ChangePassword: [old_password: DotNetString][new_password: DotNetString]
fn forward_change_password(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 2 { return; }
    let world_ref = match world_ref { Some(w) => w, None => { return; } };
    let old_len = u16::from_le_bytes(payload[0..2].try_into().unwrap_or([0; 2])) as usize;
    if payload.len() < 2 + old_len + 2 { return; }
    let new_len = u16::from_le_bytes(payload[2 + old_len..4 + old_len].try_into().unwrap_or([0; 2])) as usize;
    if payload.len() < 4 + old_len + new_len { return; }
    let new_password = String::from_utf8_lossy(&payload[4 + old_len..4 + old_len + new_len]).to_string();
    debug!("ChangePassword: session={}", session_id);
    let _ = world_ref.ask(crate::actors::world::ChangePasswordRequest { session_id, new_password });
}

/// NewCharacter: [name: DotNetString][class: u8][gender: u8][hair: u16]
fn forward_new_character(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 6 { return; }
    let world_ref = match world_ref { Some(w) => w, None => { return; } };
    let name_len = u16::from_le_bytes(payload[0..2].try_into().unwrap_or([0; 2])) as usize;
    if payload.len() < 2 + name_len + 4 { return; }
    let name = String::from_utf8_lossy(&payload[2..2 + name_len]).to_string();
    let class = payload[2 + name_len];
    let gender = payload[2 + name_len + 1];
    let hair = u16::from_le_bytes(payload[2 + name_len + 2..2 + name_len + 4].try_into().unwrap_or([0; 2]));
    debug!("NewCharacter: session={} name={} class={} gender={} hair={}", session_id, name, class, gender, hair);
    let _ = world_ref.ask(crate::actors::world::NewCharacterRequest { session_id, name, class, gender, hair });
}

/// DeleteCharacter: [character_index: i32]
fn forward_delete_character(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 4 { return; }
    let world_ref = match world_ref { Some(w) => w, None => { return; } };
    let character_index = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!("DeleteCharacter: session={} index={}", session_id, character_index);
    let _ = world_ref.ask(crate::actors::world::DeleteCharacterRequest { session_id, character_index });
}

// ============================================================================
// 社交/组队
// ============================================================================

/// SwitchGroup: [allow_group: bool] (1 byte)
fn forward_switch_group(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.is_empty() { return; }
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _allow_group = payload[0] != 0;
    // Phase 3: SwitchGroup 切换组队模式，当前实现为离开/加入默认组队
    let _ = world_ref.ask(crate::actors::world::SwitchGroupRequest {
        session_id,
        target_id: 0, // 0 = leave current group
    });
}

/// AddMember: [name: string] (DotNet string format)
fn forward_add_member(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 2 { return; }
    let world_ref = match world_ref { Some(w) => w, None => return };
    // 解析 DotNet 字符串（长度前缀 + UTF8）
    let name_len = u16::from_le_bytes(payload[0..2].try_into().unwrap_or([0; 2])) as usize;
    if payload.len() < 2 + name_len { return; }
    let name = String::from_utf8_lossy(&payload[2..2 + name_len]).to_string();
    debug!("AddMember: session={} name={}", session_id, name);
    let _ = world_ref.ask(crate::actors::world::GroupInviteRequest {
        session_id,
        target_name: name,
    });
}

/// GroupInvite: [accept_invite: bool] (1 byte) - 邀请回复
fn forward_group_invite(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.is_empty() { return; }
    let world_ref = match world_ref { Some(w) => w, None => return };
    let accept = payload[0] != 0;
    debug!("GroupInvite reply: session={} accept={}", session_id, accept);
    let _ = world_ref.ask(crate::actors::world::GroupInviteReply {
        session_id,
        inviter_id: 0, // resolved from pending_invites in handler
        accept,
    });
}

/// DellMember: [name: string] (DotNet string format)
fn forward_dell_member(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 2 { return; }
    let world_ref = match world_ref { Some(w) => w, None => return };
    let name_len = u16::from_le_bytes(payload[0..2].try_into().unwrap_or([0; 2])) as usize;
    if payload.len() < 2 + name_len { return; }
    let name = String::from_utf8_lossy(&payload[2..2 + name_len]).to_string();
    debug!("DellMember: session={} name={}", session_id, name);
    let _ = world_ref.ask(crate::actors::world::DellMemberRequest {
        session_id,
        member_name: name,
    });
}

// ============================================================================
// Hero/宠物
// ============================================================================

/// NewHero: [hero_type: u8]
fn forward_new_hero(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.is_empty() { return; }
    let world_ref = match world_ref { Some(w) => w, None => { return; } };
    let hero_type = payload[0];
    debug!("NewHero: session={} type={}", session_id, hero_type);
    let _ = world_ref.ask(crate::actors::world::NewHeroRequest { session_id, hero_type });
}

/// SetHeroBehaviour: [behaviour: u8]
fn handle_set_hero_behaviour(_gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if !payload.is_empty() {
        let behaviour = payload[0];
        debug!("SetHeroBehaviour: session={} behaviour={}", session_id, behaviour);
    }
    // 英雄行为设置由客户端本地处理
}

/// SetAutoPotValue: [type: u8][value: i32]
fn handle_set_autopot_value(_gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 5 {
        let ptype = payload[0];
        let value = i32::from_le_bytes(payload[1..5].try_into().unwrap_or([0; 4]));
        debug!("SetAutoPotValue: session={} type={} value={}", session_id, ptype, value);
    }
    // 自动药水设置由客户端本地处理
}

/// SetAutoPotItem: [slot: u8][item_id: u32]
fn handle_set_autopot_item(_gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 5 {
        let slot = payload[0];
        let item_id = u32::from_le_bytes(payload[1..5].try_into().unwrap_or([0; 4]));
        debug!("SetAutoPotItem: session={} slot={} item={}", session_id, slot, item_id);
    }
    // 自动药水物品设置由客户端本地处理
}

/// ChangeHero: [hero_index: u8]
fn handle_change_hero(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.is_empty() { return; }
    let hero_index = payload[0];
    debug!("ChangeHero: session={} index={}", session_id, hero_index);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::ChangeHeroRequest { session_id, hero_index });
}

/// TakeBackHeroItem: [grid: u8][unique_id: u64]
fn handle_take_back_hero_item(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 9 { return; }
    let grid = payload[0];
    let uid = u64::from_le_bytes(payload[1..9].try_into().unwrap_or([0; 8]));
    debug!("TakeBackHeroItem: session={} grid={} uid={}", session_id, grid, uid);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::TakeBackHeroItemRequest { session_id, grid, unique_id: uid });
}

/// TransferHeroItem: [grid: u8][unique_id: u64]
fn handle_transfer_hero_item(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 9 { return; }
    let grid = payload[0];
    let uid = u64::from_le_bytes(payload[1..9].try_into().unwrap_or([0; 8]));
    debug!("TransferHeroItem: session={} grid={} uid={}", session_id, grid, uid);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::TransferHeroItemRequest { session_id, grid, unique_id: uid });
}

// ============================================================================
// 交易系统
// ============================================================================

// ============================================================================
// 交易系统
// ============================================================================

/// ChangeTrade: 添加/移除交易物品（客户端触发）
fn forward_change_trade(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 9 { return; }
    let world_ref = match world_ref { Some(w) => w, None => return };
    let is_add = payload[0] != 0;
    let uid = u64::from_le_bytes(payload[1..9].try_into().unwrap_or([0; 8]));
    let grid = if payload.len() >= 10 { payload[9] } else { 0 };
    let count = if payload.len() >= 12 { u16::from_le_bytes(payload[10..12].try_into().unwrap_or([0; 2])) } else { 1 };

    if is_add {
        let _ = world_ref.ask(crate::actors::world::TradeAddItem {
            session_id, unique_id: uid, grid, count,
        });
    } else {
        let _ = world_ref.ask(crate::actors::world::TradeRemoveItem {
            session_id, unique_id: uid,
        });
    }
}

/// TradeRequest: 发起交易
fn forward_trade_request(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    _payload: &[u8],
) {
    if world_ref.is_none() { return; }
    let _ = world_ref.as_ref().unwrap().ask(crate::actors::world::TradeStartRequest {
        session_id,
    });
}

/// TradeReply: [accept: bool]
fn forward_trade_reply(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.is_empty() { return; }
    let world_ref = match world_ref { Some(w) => w, None => return };
    let accept = payload[0] != 0;
    let _ = world_ref.ask(crate::actors::world::TradeStartReply {
        session_id, accept,
    });
}

/// TradeConfirm: [locked: bool]
fn forward_trade_confirm(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.is_empty() { return; }
    let world_ref = match world_ref { Some(w) => w, None => return };
    let locked = payload[0] != 0;
    let _ = world_ref.ask(crate::actors::world::TradeConfirmLock {
        session_id, locked,
    });
}

/// TradeCancel: 取消交易
fn forward_trade_cancel(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    _payload: &[u8],
) {
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::TradeCancel {
        session_id,
    });
}

/// TradeGold: [amount: u32]
fn forward_trade_gold(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 4 { return; }
    let world_ref = match world_ref { Some(w) => w, None => return };
    let amount = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    let _ = world_ref.ask(crate::actors::world::TradeAddGold {
        session_id, amount,
    });
}

// ============================================================================
// 好友系统
// ============================================================================

// ============================================================================
// 好友系统
// ============================================================================

/// AddFriend: [name: DotNetString][blocked: bool]
fn forward_add_friend(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 2 { return; }
    let world_ref = match world_ref { Some(w) => w, None => return };
    let name_len = u16::from_le_bytes(payload[0..2].try_into().unwrap_or([0; 2])) as usize;
    if payload.len() < 2 + name_len + 1 { return; }
    let name = String::from_utf8_lossy(&payload[2..2 + name_len]).to_string();
    let blocked = payload[2 + name_len] != 0;
    debug!("AddFriend: session={} name={} blocked={}", session_id, name, blocked);
    let _ = world_ref.ask(crate::actors::world::AddFriendRequest {
        session_id, friend_name: name, blocked,
    });
}

/// RemoveFriend: [character_index: i32]
fn forward_remove_friend(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 4 { return; }
    let world_ref = match world_ref { Some(w) => w, None => return };
    let character_index = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!("RemoveFriend: session={} char_idx={}", session_id, character_index);
    let _ = world_ref.ask(crate::actors::world::RemoveFriendRequest {
        session_id, friend_object_id: character_index as u32,
    });
}

/// RefreshFriends: no payload
fn forward_refresh_friends(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
) {
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::RefreshFriendsRequest {
        session_id,
    });
}

/// AddMemo: [character_index: i32][memo: DotNetString]
fn forward_add_memo(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 4 { return; }
    let world_ref = match world_ref { Some(w) => w, None => return };
    let character_index = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    let memo = if payload.len() > 4 {
        String::from_utf8_lossy(&payload[4..]).to_string()
    } else {
        String::new()
    };
    debug!("AddMemo: session={} char_idx={}", session_id, character_index);
    let _ = world_ref.ask(crate::actors::world::AddMemoRequest {
        session_id, friend_object_id: character_index as u32, memo,
    });
}

// ============================================================================
// 邮件系统
// ============================================================================

/// SendMail: [receiver_name: DotNetString][subject: DotNetString][message: DotNetString][gold: u32][items: 5*u64][stamped: bool]
fn handle_send_mail(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 4 { return; }
    let world_ref = match world_ref { Some(w) => w, None => return };
    let mut offset = 0;

    // Parse receiver name
    let name_len = u32::from_le_bytes(payload[offset..offset+4].try_into().unwrap_or([0;4])) as usize;
    offset += 4;
    let receiver_name = String::from_utf8_lossy(&payload[offset..offset+name_len.min(payload.len()-offset)]).to_string();
    offset += name_len;
    if offset + 4 > payload.len() { return; }

    // Parse subject
    let subj_len = u32::from_le_bytes(payload[offset..offset+4].try_into().unwrap_or([0;4])) as usize;
    offset += 4;
    let subject = String::from_utf8_lossy(&payload[offset..offset+subj_len.min(payload.len()-offset)]).to_string();
    offset += subj_len;
    if offset + 4 > payload.len() { return; }

    // Parse message
    let msg_len = u32::from_le_bytes(payload[offset..offset+4].try_into().unwrap_or([0;4])) as usize;
    offset += 4;
    let message = String::from_utf8_lossy(&payload[offset..offset+msg_len.min(payload.len()-offset)]).to_string();
    offset += msg_len;
    if offset + 4 > payload.len() { return; }

    // Parse gold
    let gold = u32::from_le_bytes(payload[offset..offset+4].try_into().unwrap_or([0;4]));
    offset += 4;

    // Parse 5 item UIDs
    let mut item_uids = Vec::new();
    for _ in 0..5 {
        if offset + 8 > payload.len() { break; }
        let uid = u64::from_le_bytes(payload[offset..offset+8].try_into().unwrap_or([0;8]));
        offset += 8;
        if uid != 0 { item_uids.push(uid); }
    }

    // Parse stamped
    let _stamped = if offset < payload.len() { payload[offset] != 0 } else { false };

    debug!("SendMail: session={} to={} gold={}", session_id, receiver_name, gold);
    let _ = world_ref.ask(crate::actors::world::SendMailRequest {
        session_id,
        receiver_name,
        subject,
        body: message,
        gold,
        item_uids,
    });
}

/// ReadMail: [mail_id: u64]
fn handle_read_mail(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let mail_id = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0;8]));
    debug!("ReadMail: session={} id={}", session_id, mail_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::ReadMailRequest { session_id, mail_id });
}

/// CollectParcel: [mail_id: u64]
fn handle_collect_parcel(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let mail_id = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0;8]));
    debug!("CollectParcel: session={} id={}", session_id, mail_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::CollectParcelRequest { session_id, mail_id });
}

/// DeleteMail: [mail_id: u64]
fn handle_delete_mail(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let mail_id = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0;8]));
    debug!("DeleteMail: session={} id={}", session_id, mail_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::DeleteMailRequest { session_id, mail_id });
}

// ============================================================================
// 行会系统
// ============================================================================

/// GuildInvite: [accept: bool] - 行会邀请回复
fn handle_guild_invite(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.is_empty() { return; }
    let accept = payload[0] != 0;
    debug!("GuildInvite: session={} accept={}", session_id, accept);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::GuildInviteReply { session_id, accept });
}

/// RequestGuildInfo: [info_type: u8]
fn handle_request_guild_info(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    let info_type = payload.first().copied().unwrap_or(0);
    debug!("RequestGuildInfo: session={} type={}", session_id, info_type);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::RequestGuildInfo { session_id, info_type });
}

/// EditGuildMember: [change_type: u8][rank_index: u8][name: DotNetString][rank_name: DotNetString]
fn handle_edit_guild_member(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 2 { return; }
    let change_type = payload[0];
    // Parse member name
    let name_start = 2;
    if name_start + 4 > payload.len() { return; }
    let name_len = u32::from_le_bytes(payload[name_start..name_start+4].try_into().unwrap_or([0;4])) as usize;
    let name_end = name_start + 4 + name_len;
    let member_name = if name_end <= payload.len() {
        String::from_utf8_lossy(&payload[name_start+4..name_end]).to_string()
    } else {
        return;
    };
    debug!("EditGuildMember: session={} type={} name={}", session_id, change_type, member_name);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::EditGuildMemberRequest { session_id, change_type, member_name });
}

/// EditGuildNotice: [count: i32][line1: DotNetString][line2: DotNetString]...
fn handle_edit_guild_notice(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 4 { return; }
    let count = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0;4])) as usize;
    let mut notice_lines = Vec::new();
    let mut offset = 4;
    for _ in 0..count {
        if offset + 4 > payload.len() { break; }
        let len = u32::from_le_bytes(payload[offset..offset+4].try_into().unwrap_or([0;4])) as usize;
        offset += 4;
        if offset + len > payload.len() { break; }
        notice_lines.push(String::from_utf8_lossy(&payload[offset..offset+len]).to_string());
        offset += len;
    }
    debug!("EditGuildNotice: session={} lines={}", session_id, notice_lines.len());
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::EditGuildNoticeRequest { session_id, notice: notice_lines });
}

/// GuildNameReturn: [name: DotNetString]
fn handle_guild_name_return(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 4 { return; }
    let name_len = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0;4])) as usize;
    if payload.len() < 4 + name_len { return; }
    let name = String::from_utf8_lossy(&payload[4..4+name_len]).to_string();
    debug!("GuildNameReturn: session={} name={}", session_id, name);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::CreateGuildRequest { session_id, guild_name: name });
}

/// GuildStorageGoldChange: [change_type: u8][amount: u32]
fn handle_guild_storage_gold(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 5 { return; }
    let change_type = payload[0];
    let amount = u32::from_le_bytes(payload[1..5].try_into().unwrap_or([0;4]));
    debug!("GuildStorageGoldChange: session={} type={} amount={}", session_id, change_type, amount);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::GuildStorageGoldChangeRequest { session_id, change_type, amount });
}

/// GuildStorageItemChange: [change_type: u8][grid: u8][unique_id: u64][count: u32]
fn handle_guild_storage_item(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 12 { return; }
    let change_type = payload[0];
    let grid = payload[1];
    let uid = u64::from_le_bytes(payload[2..10].try_into().unwrap_or([0; 8]));
    let count = u32::from_le_bytes(payload[10..14].try_into().unwrap_or([0; 4]));
    debug!("GuildStorageItemChange: session={} type={} grid={} uid={} count={}", session_id, change_type, grid, uid, count);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::GuildStorageItemChangeRequest { session_id, change_type, grid, unique_id: uid, count });
}

// ============================================================================
// 婚姻系统
// ============================================================================

/// MarriageRequest: [target_name: DotNetString]
fn handle_marriage_request(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 4 { return; }
    let name_len = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0;4])) as usize;
    let target_name = if payload.len() >= 4 + name_len {
        String::from_utf8_lossy(&payload[4..4+name_len]).to_string()
    } else { return; };
    debug!("MarriageRequest: session={} to={}", session_id, target_name);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::MarriageRequest { session_id, target_name });
}

/// MarriageReply: [accept: bool]
fn handle_marriage_reply(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.is_empty() { return; }
    let accept = payload[0] != 0;
    debug!("MarriageReply: session={} accept={}", session_id, accept);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::MarriageReply { session_id, accept });
}

/// ChangeMarriage: no payload or minimal
fn handle_change_marriage(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, _payload: &[u8]) {
    debug!("ChangeMarriage: session={}", session_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::ChangeMarriage { session_id });
}

/// DivorceRequest
fn handle_divorce_request(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 4 { return; }
    let name_len = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0;4])) as usize;
    let partner_name = if payload.len() >= 4 + name_len {
        String::from_utf8_lossy(&payload[4..4+name_len]).to_string()
    } else { return; };
    debug!("DivorceRequest: session={} partner={}", session_id, partner_name);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::DivorceRequest { session_id, partner_name });
}

/// DivorceReply: [accept: bool]
fn handle_divorce_reply(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.is_empty() { return; }
    let accept = payload[0] != 0;
    debug!("DivorceReply: session={} accept={}", session_id, accept);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::DivorceReply { session_id, accept });
}

/// AddMentor: [mentor_name: DotNetString]
fn handle_add_mentor(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 4 { return; }
    let name_len = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0;4])) as usize;
    let mentor_name = if payload.len() >= 4 + name_len {
        String::from_utf8_lossy(&payload[4..4+name_len]).to_string()
    } else { return; };
    debug!("AddMentor: session={} mentor={}", session_id, mentor_name);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::AddMentor { session_id, mentor_name });
}

/// MentorReply: [accept: bool]
fn handle_mentor_reply(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.is_empty() { return; }
    let accept = payload[0] != 0;
    debug!("MentorReply: session={} accept={}", session_id, accept);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::MentorReply { session_id, accept });
}

/// AllowMentor: [allow: bool]
fn handle_allow_mentor(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.is_empty() { return; }
    let allow = payload[0] != 0;
    debug!("AllowMentor: session={} allow={}", session_id, allow);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::AllowMentor { session_id, allow });
}

/// CancelMentor
fn handle_cancel_mentor(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, _payload: &[u8]) {
    debug!("CancelMentor: session={}", session_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::CancelMentor { session_id });
}

// ============================================================================
// 宠物系统
// ============================================================================

/// UpdateIntelligentCreature: [creature_type: u8][pickup_mode: u8][custom_name: DotNetString?]
fn handle_update_intelligent_creature(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 2 { return; }
    let creature_type = payload[0];
    let pickup_mode = payload[1];
    debug!("UpdateIntelligentCreature: session={} type={} mode={}", session_id, creature_type, pickup_mode);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::UpdateIntelligentCreature { session_id, creature_type, pickup_mode });
}

/// IntelligentCreaturePickup: [x: i32][y: i32]
fn handle_intelligent_creature_pickup(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let x = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0;4]));
    let y = i32::from_le_bytes(payload[4..8].try_into().unwrap_or([0;4]));
    debug!("IntelligentCreaturePickup: session={} x={} y={}", session_id, x, y);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::IntelligentCreaturePickup { session_id, x, y });
}

/// RequestIntelligentCreatureUpdates: [request_updates: bool]
fn handle_request_intelligent_creature_updates(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.is_empty() { return; }
    let request_updates = payload[0] != 0;
    debug!("RequestIntelligentCreatureUpdates: session={} updates={}", session_id, request_updates);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::RequestIntelligentCreatureUpdates { session_id, request_updates });
}

// ============================================================================
// 任务系统
// ============================================================================

/// AcceptQuest: [npc_index: i32][quest_index: i32]
fn handle_accept_quest(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let npc_index = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0;4]));
    let quest_index = i32::from_le_bytes(payload[4..8].try_into().unwrap_or([0;4]));
    debug!("AcceptQuest: session={} npc={} quest={}", session_id, npc_index, quest_index);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::AcceptQuestRequest { session_id, npc_index, quest_index });
}

/// FinishQuest: [quest_index: i32][selected_item_index: i32]
fn handle_finish_quest(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let quest_index = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0;4]));
    let selected_item_index = i32::from_le_bytes(payload[4..8].try_into().unwrap_or([0;4]));
    debug!("FinishQuest: session={} quest={}", session_id, quest_index);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::FinishQuestRequest { session_id, quest_index, selected_item_index });
}

/// AbandonQuest: [quest_index: i32]
fn handle_abandon_quest(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 4 { return; }
    let quest_index = i32::from_le_bytes(payload[..4].try_into().unwrap_or([0;4]));
    debug!("AbandonQuest: session={} quest={}", session_id, quest_index);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::AbandonQuestRequest { session_id, quest_index });
}

// ============================================================================
// 精炼系统
// ============================================================================

/// DepositRefineItem: [unique_id: u64]
fn handle_deposit_refine_item(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let uid = u64::from_le_bytes(payload[..8].try_into().unwrap_or([0; 8]));
    debug!("DepositRefineItem: session={} uid={}", session_id, uid);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::DepositRefineItemRequest {
        session_id, unique_id: uid,
    });
}

/// RetrieveRefineItem: [unique_id: u64]
fn handle_retrieve_refine_item(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let uid = u64::from_le_bytes(payload[..8].try_into().unwrap_or([0; 8]));
    debug!("RetrieveRefineItem: session={} uid={}", session_id, uid);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::RetrieveRefineItemRequest {
        session_id, unique_id: uid,
    });
}

/// RefineCancel: []
fn handle_refine_cancel(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, _payload: &[u8]) {
    debug!("RefineCancel: session={}", session_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::RefineCancelRequest { session_id });
}

/// RefineItem: [item_id: u32][materials: u32]
fn handle_refine_item(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let item_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    let materials = u32::from_le_bytes(payload[4..8].try_into().unwrap_or([0; 4]));
    debug!("RefineItem: session={} item={} materials={}", session_id, item_id, materials);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::RefineItemRequest {
        session_id, item_id, materials,
    });
}

/// CheckRefine: [unique_id: u64]
fn handle_check_refine(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let uid = u64::from_le_bytes(payload[..8].try_into().unwrap_or([0; 8]));
    debug!("CheckRefine: session={} uid={}", session_id, uid);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::CheckRefineRequest {
        session_id, unique_id: uid,
    });
}

// ============================================================================
// 传送/地图
// ============================================================================

/// RequestMapInfo: [map_id: u32]
fn forward_request_map_info(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 4 { return; }
    let world_ref = match world_ref { Some(w) => w, None => { return; } };
    let map_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!("RequestMapInfo: session={} map={}", session_id, map_id);
    let _ = world_ref.ask(crate::actors::world::RequestMapInfoRequest { session_id, map_id });
}

/// SearchMap: [keyword: DotNetString]
fn forward_search_map(
    world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>,
    session_id: SessionId,
    payload: &[u8],
) {
    if payload.len() < 2 { return; }
    let world_ref = match world_ref { Some(w) => w, None => { return; } };
    let name_len = u16::from_le_bytes(payload[0..2].try_into().unwrap_or([0; 2])) as usize;
    if payload.len() < 2 + name_len { return; }
    let keyword = String::from_utf8_lossy(&payload[2..2 + name_len]).to_string();
    debug!("SearchMap: session={} keyword={}", session_id, keyword);
    let _ = world_ref.ask(crate::actors::world::SearchMapRequest { session_id, keyword });
}

/// Observe: [target_id: u32]
fn handle_observe(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 4 { return; }
    let _target_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!("Observe: session={}", session_id);
    send_system_message(gate_ref, session_id, "观察模式已开启");
}

// ============================================================================
// 其他
// ============================================================================

/// ReplaceWedRing: [unique_id: u64] — 更换结婚戒指
fn handle_replace_wed_ring(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let uid = u64::from_le_bytes(payload[..8].try_into().unwrap_or([0; 8]));
    debug!("ReplaceWedRing: session={} uid={}", session_id, uid);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::ReplaceWedRingRequest { session_id, unique_id: uid });
}

/// RequestUserName: [target_id: u32]
fn handle_request_user_name(_gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 4 {
        let target_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
        debug!("RequestUserName: session={} target={}", session_id, target_id);
    }
    // 用户名请求由客户端本地处理
}

/// RequestChatItem: [item_index: u32]
fn handle_request_chat_item(_gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 4 {
        let item_index = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
        debug!("RequestChatItem: session={} item={}", session_id, item_index);
    }
    // 聊天链接物品请求由客户端本地处理
}

// ============================================================================
// 剩余 opcode stub handlers（Phase 15：覆盖所有未处理的 opcode）
// ============================================================================

/// EquipSlotItem: [slot: u8] — 快捷装备栏装备
fn handle_equip_slot_item(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.is_empty() { return; }
    let slot = payload[0];
    debug!("EquipSlotItem: session={} slot={}", session_id, slot);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::EquipSlotItemRequest { session_id, slot });
}

/// ConsignItem: [item_index: u32][price: u32][duration: u32] — 寄售
fn forward_consign_item(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 12 { return; }
    let unique_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    let price = u32::from_le_bytes(payload[4..8].try_into().unwrap_or([0; 4]));
    debug!("ConsignItem: session={} uid={} price={}", session_id, unique_id, price);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::ConsignItemRequest { session_id, unique_id: unique_id as u64, price: price as u64 });
}

/// MarketSearch: [keyword: DotNetString]
fn forward_market_search(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    let item_index = if payload.len() >= 4 { u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4])) } else { 0 };
    debug!("MarketSearch: session={} item={}", session_id, item_index);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::MarketSearchRequest { session_id, item_index });
}

/// MarketRefresh: []
fn forward_market_refresh(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, _payload: &[u8]) {
    debug!("MarketRefresh: session={}", session_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::MarketRefreshRequest { session_id });
}

/// MarketPage: [page: u32]
fn forward_market_page(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 4 { return; }
    let page = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!("MarketPage: session={} page={}", session_id, page);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::MarketPageRequest { session_id, page });
}

/// MarketBuy: [listing_id: u32]
fn forward_market_buy(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 4 { return; }
    let listing_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!("MarketBuy: session={} listing={}", session_id, listing_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::MarketBuyRequest { session_id, listing_id: listing_id as u64, count: 1 });
}

/// MarketGetBack: [listing_id: u32]
fn forward_market_get_back(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 4 { return; }
    let listing_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!("MarketGetBack: session={} listing={}", session_id, listing_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::MarketGetBackRequest { session_id, listing_id: listing_id as u64 });
}

/// MarketSellNow: [item_index: u32][price: u32]
fn forward_market_sell_now(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let unique_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    let price = u32::from_le_bytes(payload[4..8].try_into().unwrap_or([0; 4]));
    debug!("MarketSellNow: session={} uid={} price={}", session_id, unique_id, price);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::MarketSellNowRequest { session_id, unique_id: unique_id as u64, price: price as u64 });
}

/// FishingCast: [type: u8]
fn forward_fishing_cast(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    let fishing_type = payload.first().copied().unwrap_or(0);
    debug!("FishingCast: session={} type={}", session_id, fishing_type);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::FishingCastRequest { session_id, fishing_type });
}

/// FishingChangeAutocast: [enabled: bool]
fn forward_fishing_change_autocast(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    let enabled = payload.first().copied().unwrap_or(0) != 0;
    debug!("FishingChangeAutocast: session={} enabled={}", session_id, enabled);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::FishingChangeAutocastRequest { session_id, enabled });
}

/// CombineItem: [from: u32][to: u32]
fn forward_combine_item(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let from_grid = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    let to_grid = u32::from_le_bytes(payload[4..8].try_into().unwrap_or([0; 4]));
    debug!("CombineItem: session={} from={} to={}", session_id, from_grid, to_grid);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::CombineItemRequest { session_id, from_grid, to_grid });
}

/// AwakeningNeedMaterials: [item_index: u32]
fn handle_awakening_need_materials(gate_ref: &ActorRef<GateActor>, session_id: SessionId, _payload: &[u8]) {
    debug!("AwakeningNeedMaterials: session={}", session_id);
    // 发送空材料列表
    let mut body = Vec::new();
    body.extend_from_slice(&0i32.to_le_bytes());
    let _ = gate_ref.ask(SendToClient {
        session_id,
        data: build_packet_bytes(ServerPacketIds::AwakeningNeedMaterials as i16, &body),
    });
}

/// AwakeningLockedItem: [unique_id: u64]
fn handle_awakening_locked_item(gate_ref: &ActorRef<GateActor>, session_id: SessionId, _payload: &[u8]) {
    debug!("AwakeningLockedItem: session={}", session_id);
    send_system_message(gate_ref, session_id, "觉醒暂未开放");
}

/// Awakening: [unique_id: u64][material_slots: u32]
fn handle_awakening(gate_ref: &ActorRef<GateActor>, session_id: SessionId, _payload: &[u8]) {
    debug!("Awakening: session={}", session_id);
    send_system_message(gate_ref, session_id, "觉醒暂未开放");
}

/// DisassembleItem: [unique_id: u64]
fn forward_disassemble_item(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let uid = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0; 8]));
    debug!("DisassembleItem: session={} uid={}", session_id, uid);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::DisassembleItemRequest { session_id, unique_id: uid });
}

/// DowngradeAwakening: [unique_id: u64]
fn handle_downgrade_awakening(gate_ref: &ActorRef<GateActor>, session_id: SessionId, _payload: &[u8]) {
    debug!("DowngradeAwakening: session={}", session_id);
    send_system_message(gate_ref, session_id, "觉醒降级暂未开放");
}

/// ResetAddedItem: [unique_id: u64]
fn forward_reset_added_item(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let uid = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0; 8]));
    debug!("ResetAddedItem: session={} uid={}", session_id, uid);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::ResetAddedItemRequest { session_id, unique_id: uid });
}

/// DepositTradeItem: [unique_id: u64]
fn forward_deposit_trade_item(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let uid = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0; 8]));
    debug!("DepositTradeItem: session={} uid={}", session_id, uid);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::DepositTradeItemRequest { session_id, unique_id: uid });
}

/// RetrieveTradeItem: [unique_id: u64]
fn forward_retrieve_trade_item(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let uid = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0; 8]));
    debug!("RetrieveTradeItem: session={} uid={}", session_id, uid);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::RetrieveTradeItemRequest { session_id, unique_id: uid });
}

/// GuildWarReturn: [guild_name: DotNetString]
fn forward_guild_war_return(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    let guild_name = parse_dotnet_string(payload);
    debug!("GuildWarReturn: session={} guild={}", session_id, guild_name);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::GuildWarReturnRequest { session_id, guild_name });
}

/// GuildBuffUpdate: [buff_id: u32]
fn forward_guild_buff_update(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 4 { return; }
    let buff_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!("GuildBuffUpdate: session={} buff_id={}", session_id, buff_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::GuildBuffUpdateRequest { session_id, buff_id });
}

/// LockMail: [mail_id: u64][lock: bool]
fn forward_lock_mail(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 9 { return; }
    let mail_id = u64::from_le_bytes(payload[..8].try_into().unwrap_or([0; 8]));
    let lock = payload[8] != 0;
    debug!("LockMail: session={} mail_id={} lock={}", session_id, mail_id, lock);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::LockMailRequest { session_id, mail_id, lock });
}

/// MailLockedItem: [mail_id: u64][item_index: u32]
fn forward_mail_locked_item(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 12 { return; }
    let mail_id = u64::from_le_bytes(payload[..8].try_into().unwrap_or([0; 8]));
    let item_index = u32::from_le_bytes(payload[8..12].try_into().unwrap_or([0; 4]));
    debug!("MailLockedItem: session={} mail_id={} item_index={}", session_id, mail_id, item_index);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::MailLockedItemRequest { session_id, mail_id, item_index });
}

/// MailCost: [items_count: u32][gold: u32]
fn handle_mail_cost(gate_ref: &ActorRef<GateActor>, session_id: SessionId, _payload: &[u8]) {
    debug!("MailCost: session={}", session_id);
    // 返回计算结果（免费）
    let mut body = Vec::new();
    body.extend_from_slice(&0u32.to_le_bytes());
    let _ = gate_ref.ask(SendToClient {
        session_id,
        data: build_packet_bytes(ServerPacketIds::MailCost as i16, &body),
    });
}

/// ShareQuest: [quest_id: u32]
fn forward_share_quest(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 4 { return; }
    let quest_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!("ShareQuest: session={} quest_id={}", session_id, quest_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::ShareQuestRequest { session_id, quest_id });
}

/// AcceptReincarnation: []
fn forward_accept_reincarnation(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, _payload: &[u8]) {
    debug!("AcceptReincarnation: session={}", session_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::AcceptReincarnationRequest { session_id });
}

/// CancelReincarnation: []
fn forward_cancel_reincarnation(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, _payload: &[u8]) {
    debug!("CancelReincarnation: session={}", session_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::CancelReincarnationRequest { session_id });
}

/// GetRentedItems: [page: u32]
fn handle_get_rented_items(gate_ref: &ActorRef<GateActor>, session_id: SessionId, _payload: &[u8]) {
    debug!("GetRentedItems: session={}", session_id);
    // 发送空租赁物品列表
    let mut body = Vec::new();
    body.extend_from_slice(&0i32.to_le_bytes());
    let _ = gate_ref.ask(SendToClient {
        session_id,
        data: build_packet_bytes(ServerPacketIds::GetRentedItems as i16, &body),
    });
}

/// ItemRentalRequest: [target_name: DotNetString]
fn forward_item_rental_request(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    let _target_name = parse_dotnet_string(payload);
    debug!("ItemRentalRequest: session={} target={}", session_id, _target_name);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::ItemRentalRequestMsg { session_id, target_id: 0 });
}

/// ItemRentalFee: [amount: u32]
fn forward_item_rental_fee(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    let amount = if payload.len() >= 4 { u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4])) } else { 0 };
    debug!("ItemRentalFee: session={} amount={}", session_id, amount);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::ItemRentalFeeMsg { session_id, amount });
}

/// ItemRentalPeriod: [duration: u32]
fn forward_item_rental_period(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    let duration = if payload.len() >= 4 { u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4])) } else { 0 };
    debug!("ItemRentalPeriod: session={} duration={}", session_id, duration);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::ItemRentalPeriodMsg { session_id, duration });
}

/// DepositRentalItem: [unique_id: u64]
fn forward_deposit_rental_item(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let uid = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0; 8]));
    debug!("DepositRentalItem: session={} uid={}", session_id, uid);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::DepositRentalItemRequest { session_id, unique_id: uid });
}

/// RetrieveRentalItem: [unique_id: u64]
fn forward_retrieve_rental_item(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let uid = u64::from_le_bytes(payload[0..8].try_into().unwrap_or([0; 8]));
    debug!("RetrieveRentalItem: session={} uid={}", session_id, uid);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::RetrieveRentalItemRequest { session_id, unique_id: uid });
}

/// CancelItemRental: []
fn forward_cancel_item_rental(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, _payload: &[u8]) {
    debug!("CancelItemRental: session={}", session_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::CancelItemRentalRequest { session_id });
}

/// ItemRentalLockFee: []
fn forward_item_rental_lock_fee(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, _payload: &[u8]) {
    debug!("ItemRentalLockFee: session={}", session_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::ItemRentalLockFeeMsg { session_id });
}

/// ItemRentalLockItem: []
fn forward_item_rental_lock_item(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, _payload: &[u8]) {
    debug!("ItemRentalLockItem: session={}", session_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::ItemRentalLockItemMsg { session_id });
}

/// ConfirmItemRental: []
fn forward_confirm_item_rental(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, _payload: &[u8]) {
    debug!("ConfirmItemRental: session={}", session_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::ConfirmItemRentalMsg { session_id });
}

/// NPCConfirmInput: [npc_id: u32][input: DotNetString]
fn forward_npc_confirm_input(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 4 { return; }
    let npc_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    let input_text = parse_dotnet_string(&payload[4..]);
    debug!("NPCConfirmInput: session={} npc_id={} input={}", session_id, npc_id, input_text);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::NPCConfirmInputRequest { session_id, npc_id, input_text });
}

/// GameshopBuy: [item_id: u32][quantity: u32]
fn forward_gameshop_buy(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 8 { return; }
    let item_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    let count = u32::from_le_bytes(payload[4..8].try_into().unwrap_or([0; 4]));
    debug!("GameshopBuy: session={} item={} count={}", session_id, item_id, count);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::GameshopBuyRequest { session_id, item_id, count });
}

/// ReportIssue: [type: u32][description: DotNetString]
fn forward_report_issue(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    let issue_type = if payload.len() >= 4 { payload[3] } else { 0 };
    let description = if payload.len() >= 4 { parse_dotnet_string(&payload[4..]) } else { String::new() };
    debug!("ReportIssue: session={} type={}", session_id, issue_type);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::ReportIssueRequest { session_id, issue_type, description });
}

/// GetRanking: [type: u32][page: u32]
fn forward_get_ranking(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    let rank_type = if !payload.is_empty() { payload[0] } else { 0 };
    debug!("GetRanking: session={} type={}", session_id, rank_type);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::GetRankingRequest { session_id, rank_type });
}

/// Opendoor: [door_id: u32]
fn forward_opendoor(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    if payload.len() < 4 { return; }
    let door_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
    debug!("Opendoor: session={} door_id={}", session_id, door_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::OpendoorRequest { session_id, door_id });
}

/// GuildTerritoryPage: [page: u32]
fn forward_guild_territory_page(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    let page = if payload.len() >= 4 { u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4])) } else { 0 };
    debug!("GuildTerritoryPage: session={} page={}", session_id, page);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::GuildTerritoryPageRequest { session_id, page });
}

/// PurchaseGuildTerritory: [territory_id: u32]
fn forward_purchase_guild_territory(world_ref: &Option<ActorRef<crate::actors::world::WorldActor>>, session_id: SessionId, payload: &[u8]) {
    let territory_id = if payload.len() >= 4 { u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4])) } else { 0 };
    debug!("PurchaseGuildTerritory: session={} territory={}", session_id, territory_id);
    let world_ref = match world_ref { Some(w) => w, None => return };
    let _ = world_ref.ask(crate::actors::world::PurchaseGuildTerritoryRequest { session_id, territory_id });
}

