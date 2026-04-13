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
                // PickUp - 拾取物品（Phase 1：回复无物品）
                handle_pickup(&gate_ref, msg.session_id);
            }
            x if x == ClientPacketIds::MoveItem as i16 => {
                handle_move_item(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::UseItem as i16 => {
                handle_use_item(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::EquipItem as i16 => {
                handle_equip_item(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RemoveItem as i16 => {
                handle_remove_item(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::DropItem as i16 => {
                handle_drop_item(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::MergeItem as i16 => {
                handle_merge_item(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RangeAttack as i16 => {
                handle_range_attack(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::Magic as i16 => {
                handle_magic(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::Harvest as i16 => {
                handle_harvest(&gate_ref, msg.session_id, payload);
            }
            // NPC 商店
            x if x == ClientPacketIds::BuyItem as i16 => {
                handle_buy_item(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::SellItem as i16 => {
                handle_sell_item(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RepairItem as i16 => {
                handle_repair_item(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::SRepairItem as i16 => {
                handle_s_repair_item(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::CraftItem as i16 => {
                handle_craft_item(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::BuyItemBack as i16 => {
                handle_buy_item_back(&gate_ref, msg.session_id, payload);
            }
            // 仓库操作
            x if x == ClientPacketIds::StoreItem as i16 => {
                handle_store_item(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::TakeBackItem as i16 => {
                handle_take_back_item(&gate_ref, msg.session_id, payload);
            }
            // 其他常用操作
            x if x == ClientPacketIds::DropGold as i16 => {
                handle_drop_gold(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::Inspect as i16 => {
                handle_inspect(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::ChangeAMode as i16 => {
                handle_change_amode(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::ChangePMode as i16 => {
                handle_change_pmode(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::MagicKey as i16 => {
                handle_magic_key(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RemoveSlotItem as i16 => {
                handle_remove_slot_item(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::SplitItem as i16 => {
                handle_split_item(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::TeleportToNPC as i16 => {
                handle_teleport_to_npc(&gate_ref, msg.session_id, payload);
            }
            // 死亡恢复
            x if x == ClientPacketIds::TownRevive as i16 => {
                handle_town_revive(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::SpellToggle as i16 => {
                handle_spell_toggle(&gate_ref, msg.session_id, payload);
            }
            // 账号管理
            x if x == ClientPacketIds::ChangePassword as i16 => {
                handle_change_password(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::DeleteCharacter as i16 => {
                handle_delete_character(&gate_ref, msg.session_id, payload);
            }
            // 社交/组队
            x if x == ClientPacketIds::SwitchGroup as i16 => {
                handle_switch_group(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::AddMember as i16 => {
                handle_add_member(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::DellMember as i16 => {
                handle_dell_member(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::GroupInvite as i16 => {
                handle_group_invite(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::NewHero as i16 => {
                handle_new_hero(&gate_ref, msg.session_id, payload);
            }
            // 交易
            x if x == ClientPacketIds::ChangeTrade as i16 => {
                handle_change_trade(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::TradeRequest as i16 => {
                handle_trade_request(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::TradeReply as i16 => {
                handle_trade_reply(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::TradeConfirm as i16 => {
                handle_trade_confirm(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::TradeCancel as i16 => {
                handle_trade_cancel(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::TradeGold as i16 => {
                handle_trade_gold(&gate_ref, msg.session_id, payload);
            }
            // 好友
            x if x == ClientPacketIds::AddFriend as i16 => {
                handle_add_friend(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RemoveFriend as i16 => {
                handle_remove_friend(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RefreshFriends as i16 => {
                handle_refresh_friends(&gate_ref, msg.session_id);
            }
            // 邮件
            x if x == ClientPacketIds::SendMail as i16 => {
                handle_send_mail(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::ReadMail as i16 => {
                handle_read_mail(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::CollectParcel as i16 => {
                handle_collect_parcel(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::DeleteMail as i16 => {
                handle_delete_mail(&gate_ref, msg.session_id, payload);
            }
            // 行会
            x if x == ClientPacketIds::GuildInvite as i16 => {
                handle_guild_invite(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RequestGuildInfo as i16 => {
                handle_request_guild_info(&gate_ref, msg.session_id, payload);
            }
            // 婚姻
            x if x == ClientPacketIds::MarriageRequest as i16 => {
                handle_marriage_request(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::MarriageReply as i16 => {
                handle_marriage_reply(&gate_ref, msg.session_id, payload);
            }
            // 任务
            x if x == ClientPacketIds::AcceptQuest as i16 => {
                handle_accept_quest(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::FinishQuest as i16 => {
                handle_finish_quest(&gate_ref, msg.session_id, payload);
            }
            //  refining
            x if x == ClientPacketIds::DepositRefineItem as i16 => {
                handle_deposit_refine_item(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RetrieveRefineItem as i16 => {
                handle_retrieve_refine_item(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RefineCancel as i16 => {
                handle_refine_cancel(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::RefineItem as i16 => {
                handle_refine_item(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::CheckRefine as i16 => {
                handle_check_refine(&gate_ref, msg.session_id, payload);
            }
            // 传送/地图
            x if x == ClientPacketIds::RequestMapInfo as i16 => {
                handle_request_map_info(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::SearchMap as i16 => {
                handle_search_map(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::Observe as i16 => {
                handle_observe(&gate_ref, msg.session_id, payload);
            }
            // 其他
            x if x == ClientPacketIds::ReplaceWedRing as i16 => {
                handle_replace_wed_ring(&gate_ref, msg.session_id, payload);
            }
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
                handle_change_hero(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::TakeBackHeroItem as i16 => {
                handle_take_back_hero_item(&gate_ref, msg.session_id, payload);
            }
            x if x == ClientPacketIds::TransferHeroItem as i16 => {
                handle_transfer_hero_item(&gate_ref, msg.session_id, payload);
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

/// 处理 PickUp：回复 "附近没有物品"（Phase 1：无地面物品系统）
fn handle_pickup(gate_ref: &ActorRef<GateActor>, session_id: SessionId) {
    send_system_message(gate_ref, session_id, "附近没有可以拾取的物品。");
}

/// 处理心跳：回复 KeepAlive
fn handle_keep_alive(gate_ref: &ActorRef<GateActor>, session_id: SessionId) {
    let response = build_packet_bytes(ServerPacketIds::KeepAlive as i16, &[]);
    let _ = gate_ref.ask(SendToClient {
        session_id,
        data: response,
    });
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
// 物品操作 stub handler（Phase 1：回复操作失败/不支持）
// ============================================================================

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

/// 回复"操作失败"系统消息（别名，保持语义清晰）
fn reply_item_op_failed(gate_ref: &ActorRef<GateActor>, session_id: SessionId, msg: &str) {
    send_system_message(gate_ref, session_id, msg);
}

/// MoveItem: [grid: u8][from: i32][to: i32]
fn handle_move_item(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 9 {
        let grid = payload[0];
        let from = i32::from_le_bytes([payload[1], payload[2], payload[3], payload[4]]);
        let to = i32::from_le_bytes([payload[5], payload[6], payload[7], payload[8]]);
        debug!("MoveItem: session={} grid={} from={} to={}", session_id, grid, from, to);
    }
    reply_item_op_failed(gate_ref, session_id, "背包整理功能暂未开放。");
}

/// UseItem: [unique_id: u64]
fn handle_use_item(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 8 {
        let uid = u64::from_le_bytes(payload[..8].try_into().unwrap_or([0; 8]));
        debug!("UseItem: session={} uid={}", session_id, uid);
    }
    reply_item_op_failed(gate_ref, session_id, "物品使用功能暂未开放。");
}

/// EquipItem: [grid: u8][unique_id: u64][to: u8]
fn handle_equip_item(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 10 {
        let grid = payload[0];
        let uid = u64::from_le_bytes(payload[1..9].try_into().unwrap_or([0; 8]));
        let to = payload[9];
        debug!("EquipItem: session={} grid={} uid={} slot={}", session_id, grid, uid, to);
    }
    reply_item_op_failed(gate_ref, session_id, "装备功能暂未开放。");
}

/// RemoveItem: [grid: u8][unique_id: u64][to: u8]
fn handle_remove_item(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 10 {
        let grid = payload[0];
        let uid = u64::from_le_bytes(payload[1..9].try_into().unwrap_or([0; 8]));
        debug!("RemoveItem: session={} grid={} uid={}", session_id, grid, uid);
    }
    reply_item_op_failed(gate_ref, session_id, "卸下装备功能暂未开放。");
}

/// DropItem: [unique_id: u64][count: u16][hero_inventory: bool]
fn handle_drop_item(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 11 {
        let uid = u64::from_le_bytes(payload[..8].try_into().unwrap_or([0; 8]));
        let count = u16::from_le_bytes(payload[8..10].try_into().unwrap_or([0; 2]));
        let hero_inv = payload[10] != 0;
        debug!("DropItem: session={} uid={} count={} hero={}", session_id, uid, count, hero_inv);
    }
    reply_item_op_failed(gate_ref, session_id, "丢弃物品功能暂未开放。");
}

/// MergeItem: [grid_from: u8][grid_to: u8][id_from: u64][id_to: u64]
fn handle_merge_item(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 18 {
        let from = u64::from_le_bytes(payload[2..10].try_into().unwrap_or([0; 8]));
        let to = u64::from_le_bytes(payload[10..18].try_into().unwrap_or([0; 8]));
        debug!("MergeItem: session={} from={} to={}", session_id, from, to);
    }
    reply_item_op_failed(gate_ref, session_id, "物品堆叠功能暂未开放。");
}

/// RangeAttack: [dir: u8][x: i32][y: i32][target_id: u32][tx: i32][ty: i32]
fn handle_range_attack(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 21 {
        let dir = payload[0];
        let target_id = u32::from_le_bytes(payload[9..13].try_into().unwrap_or([0; 4]));
        debug!("RangeAttack: session={} dir={} target={}", session_id, dir, target_id);
    }
    // 远程攻击暂不支持，复用 Attack 的逻辑提示
    reply_item_op_failed(gate_ref, session_id, "远程攻击功能暂未开放。");
}

/// Magic: [spell: u8][dir: u8][target_id: u32][x: i32][y: i32]
fn handle_magic(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 12 {
        let spell = payload[0];
        let dir = payload[1];
        debug!("Magic: session={} spell={} dir={}", session_id, spell, dir);
    }
    reply_item_op_failed(gate_ref, session_id, "魔法技能功能暂未开放。");
}

/// Harvest: [dir: u8]
fn handle_harvest(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if !payload.is_empty() {
        let dir = payload[0];
        debug!("Harvest: session={} dir={}", session_id, dir);
    }
    reply_item_op_failed(gate_ref, session_id, "采集功能暂未开放。");
}

// ============================================================================
// NPC 商店 stub handlers
// ============================================================================

/// BuyItem: [npc_id: u32][item_index: u32][count: u32]
fn handle_buy_item(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 12 {
        let npc_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
        let item_index = u32::from_le_bytes(payload[4..8].try_into().unwrap_or([0; 4]));
        let count = u32::from_le_bytes(payload[8..12].try_into().unwrap_or([0; 4]));
        debug!("BuyItem: session={} npc={} item={} count={}", session_id, npc_id, item_index, count);
    }
    reply_item_op_failed(gate_ref, session_id, "购买功能暂未开放。");
}

/// SellItem: [grid: u8][unique_id: u64][count: u32]
fn handle_sell_item(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 13 {
        let grid = payload[0];
        let uid = u64::from_le_bytes(payload[1..9].try_into().unwrap_or([0; 8]));
        let count = u32::from_le_bytes(payload[9..13].try_into().unwrap_or([0; 4]));
        debug!("SellItem: session={} grid={} uid={} count={}", session_id, grid, uid, count);
    }
    reply_item_op_failed(gate_ref, session_id, "出售物品功能暂未开放。");
}

/// RepairItem: [unique_id: u64]
fn handle_repair_item(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 8 {
        let uid = u64::from_le_bytes(payload[..8].try_into().unwrap_or([0; 8]));
        debug!("RepairItem: session={} uid={}", session_id, uid);
    }
    reply_item_op_failed(gate_ref, session_id, "修理装备功能暂未开放。");
}

/// SRepairItem (特殊修理): [unique_id: u64]
fn handle_s_repair_item(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 8 {
        let uid = u64::from_le_bytes(payload[..8].try_into().unwrap_or([0; 8]));
        debug!("SRepairItem: session={} uid={}", session_id, uid);
    }
    reply_item_op_failed(gate_ref, session_id, "特殊修理功能暂未开放。");
}

/// CraftItem: [recipe_id: u32][materials_count: u32]
fn handle_craft_item(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 8 {
        let recipe_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
        debug!("CraftItem: session={} recipe={}", session_id, recipe_id);
    }
    reply_item_op_failed(gate_ref, session_id, "合成功能暂未开放。");
}

/// BuyItemBack (回购): [item_index: u32]
fn handle_buy_item_back(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 4 {
        let item_index = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
        debug!("BuyItemBack: session={} index={}", session_id, item_index);
    }
    reply_item_op_failed(gate_ref, session_id, "回购功能暂未开放。");
}

// ============================================================================
// 仓库 stub handlers
// ============================================================================

/// StoreItem (存入仓库): [grid: u8][unique_id: u64][count: u32]
fn handle_store_item(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 13 {
        let grid = payload[0];
        let uid = u64::from_le_bytes(payload[1..9].try_into().unwrap_or([0; 8]));
        let count = u32::from_le_bytes(payload[9..13].try_into().unwrap_or([0; 4]));
        debug!("StoreItem: session={} grid={} uid={} count={}", session_id, grid, uid, count);
    }
    reply_item_op_failed(gate_ref, session_id, "仓库存储功能暂未开放。");
}

/// TakeBackItem (从仓库取出): [grid: u8][unique_id: u64][count: u32]
fn handle_take_back_item(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 13 {
        let grid = payload[0];
        let uid = u64::from_le_bytes(payload[1..9].try_into().unwrap_or([0; 8]));
        let count = u32::from_le_bytes(payload[9..13].try_into().unwrap_or([0; 4]));
        debug!("TakeBackItem: session={} grid={} uid={} count={}", session_id, grid, uid, count);
    }
    reply_item_op_failed(gate_ref, session_id, "仓库取出功能暂未开放。");
}

// ============================================================================
// 其他常用 stub handlers
// ============================================================================

/// DropGold (丢弃金币): [amount: u32]
fn handle_drop_gold(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 4 {
        let amount = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
        debug!("DropGold: session={} amount={}", session_id, amount);
    }
    reply_item_op_failed(gate_ref, session_id, "丢弃金币功能暂未开放。");
}

/// Inspect (查看玩家): [target_id: u32]
fn handle_inspect(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 4 {
        let target_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
        debug!("Inspect: session={} target={}", session_id, target_id);
    }
    reply_item_op_failed(gate_ref, session_id, "查看玩家信息功能暂未开放。");
}

/// ChangeAMode (切换攻击模式): [mode: u8]
fn handle_change_amode(_gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if !payload.is_empty() {
        let mode = payload[0];
        debug!("ChangeAMode: session={} mode={}", session_id, mode);
    }
    // 切换攻击模式无需回复，客户端自行更新状态
}

/// ChangePMode (切换和平模式): [mode: u8]
fn handle_change_pmode(_gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if !payload.is_empty() {
        let mode = payload[0];
        debug!("ChangePMode: session={} mode={}", session_id, mode);
    }
    // 切换模式无需回复
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

/// SplitItem (物品拆分): [grid: u8][unique_id: u64][count: u32]
fn handle_split_item(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 13 {
        let grid = payload[0];
        let uid = u64::from_le_bytes(payload[1..9].try_into().unwrap_or([0; 8]));
        let count = u32::from_le_bytes(payload[9..13].try_into().unwrap_or([0; 4]));
        debug!("SplitItem: session={} grid={} uid={} count={}", session_id, grid, uid, count);
    }
    reply_item_op_failed(gate_ref, session_id, "拆分物品功能暂未开放。");
}

/// TeleportToNPC: [npc_id: u32]
fn handle_teleport_to_npc(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 4 {
        let npc_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
        debug!("TeleportToNPC: session={} npc={}", session_id, npc_id);
    }
    reply_item_op_failed(gate_ref, session_id, "传送功能暂未开放。");
}

// ============================================================================
// 死亡恢复 / 技能切换
// ============================================================================

/// TownRevive: 死亡后在城镇复活
fn handle_town_revive(gate_ref: &ActorRef<GateActor>, session_id: SessionId, _payload: &[u8]) {
    debug!("TownRevive: session={}", session_id);
    // Phase 1: 回复系统消息，不实际传送
    send_system_message(gate_ref, session_id, "复活功能暂未开放。");
}

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
fn handle_change_password(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 2 {
        debug!("ChangePassword: session={}", session_id);
    }
    send_system_message(gate_ref, session_id, "修改密码功能暂未开放。");
}

/// DeleteCharacter: [character_index: i32]
fn handle_delete_character(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 4 {
        let idx = i32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
        debug!("DeleteCharacter: session={} index={}", session_id, idx);
    }
    send_system_message(gate_ref, session_id, "删除角色功能暂未开放。");
}

// ============================================================================
// 社交/组队
// ============================================================================

/// SwitchGroup: [target_id: u32]
fn handle_switch_group(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 4 {
        let target_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
        debug!("SwitchGroup: session={} target={}", session_id, target_id);
    }
    send_system_message(gate_ref, session_id, "切换队伍功能暂未开放。");
}

/// AddMember: [target_id: u32]
fn handle_add_member(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 4 {
        let target_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
        debug!("AddMember: session={} target={}", session_id, target_id);
    }
    send_system_message(gate_ref, session_id, "添加队员功能暂未开放。");
}

/// DellMember: [member_id: u32]
fn handle_dell_member(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 4 {
        let member_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
        debug!("DellMember: session={} member={}", session_id, member_id);
    }
    send_system_message(gate_ref, session_id, "踢出队员功能暂未开放。");
}

/// GroupInvite: [target_id: u32]
fn handle_group_invite(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 4 {
        let target_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
        debug!("GroupInvite: session={} target={}", session_id, target_id);
    }
    send_system_message(gate_ref, session_id, "组队邀请功能暂未开放。");
}

// ============================================================================
// Hero/宠物
// ============================================================================

/// NewHero: [hero_type: u8]
fn handle_new_hero(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if !payload.is_empty() {
        let hero_type = payload[0];
        debug!("NewHero: session={} type={}", session_id, hero_type);
    }
    send_system_message(gate_ref, session_id, "英雄系统功能暂未开放。");
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

/// SetHeroBehaviour: [behaviour: u8]
fn handle_set_hero_behaviour(_gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if !payload.is_empty() {
        let behaviour = payload[0];
        debug!("SetHeroBehaviour: session={} behaviour={}", session_id, behaviour);
    }
    // 英雄行为设置由客户端本地处理
}

/// ChangeHero: [hero_index: u8]
fn handle_change_hero(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if !payload.is_empty() {
        let hero_index = payload[0];
        debug!("ChangeHero: session={} index={}", session_id, hero_index);
    }
    send_system_message(gate_ref, session_id, "切换英雄功能暂未开放。");
}

/// TakeBackHeroItem: [grid: u8][unique_id: u64]
fn handle_take_back_hero_item(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 9 {
        let grid = payload[0];
        let uid = u64::from_le_bytes(payload[1..9].try_into().unwrap_or([0; 8]));
        debug!("TakeBackHeroItem: session={} grid={} uid={}", session_id, grid, uid);
    }
    send_system_message(gate_ref, session_id, "取回英雄物品功能暂未开放。");
}

/// TransferHeroItem: [grid: u8][unique_id: u64]
fn handle_transfer_hero_item(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 9 {
        let grid = payload[0];
        let uid = u64::from_le_bytes(payload[1..9].try_into().unwrap_or([0; 8]));
        debug!("TransferHeroItem: session={} grid={} uid={}", session_id, grid, uid);
    }
    send_system_message(gate_ref, session_id, "转移英雄物品功能暂未开放。");
}

// ============================================================================
// 交易系统
// ============================================================================

/// ChangeTrade: [mode: u8]
fn handle_change_trade(_gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if !payload.is_empty() {
        let mode = payload[0];
        debug!("ChangeTrade: session={} mode={}", session_id, mode);
    }
    // 交易模式切换由客户端本地处理
}

/// TradeRequest: [target_id: u32]
fn handle_trade_request(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 4 {
        let target_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
        debug!("TradeRequest: session={} target={}", session_id, target_id);
    }
    send_system_message(gate_ref, session_id, "交易功能暂未开放。");
}

/// TradeReply: [accept: bool]
fn handle_trade_reply(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if !payload.is_empty() {
        let accept = payload[0] != 0;
        debug!("TradeReply: session={} accept={}", session_id, accept);
    }
    send_system_message(gate_ref, session_id, "交易功能暂未开放。");
}

/// TradeConfirm: []
fn handle_trade_confirm(gate_ref: &ActorRef<GateActor>, session_id: SessionId, _payload: &[u8]) {
    debug!("TradeConfirm: session={}", session_id);
    send_system_message(gate_ref, session_id, "交易功能暂未开放。");
}

/// TradeCancel: []
fn handle_trade_cancel(_gate_ref: &ActorRef<GateActor>, session_id: SessionId, _payload: &[u8]) {
    debug!("TradeCancel: session={}", session_id);
    // 取消交易无需回复，客户端本地清除UI
}

/// TradeGold: [amount: u32]
fn handle_trade_gold(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 4 {
        let amount = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
        debug!("TradeGold: session={} amount={}", session_id, amount);
    }
    send_system_message(gate_ref, session_id, "交易功能暂未开放。");
}

// ============================================================================
// 好友系统
// ============================================================================

/// AddFriend: [target_name: DotNetString]
fn handle_add_friend(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 4 {
        debug!("AddFriend: session={}", session_id);
    }
    send_system_message(gate_ref, session_id, "添加好友功能暂未开放。");
}

/// RemoveFriend: [friend_id: u32]
fn handle_remove_friend(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 4 {
        let friend_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
        debug!("RemoveFriend: session={} friend={}", session_id, friend_id);
    }
    send_system_message(gate_ref, session_id, "删除好友功能暂未开放。");
}

/// RefreshFriends: 刷新好友列表
fn handle_refresh_friends(_gate_ref: &ActorRef<GateActor>, session_id: SessionId) {
    debug!("RefreshFriends: session={}", session_id);
    // 好友列表刷新由客户端本地处理
}

// ============================================================================
// 邮件系统
// ============================================================================

/// SendMail: [target_name: DotNetString][subject: DotNetString][message: DotNetString][gold: u32]
fn handle_send_mail(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 4 {
        debug!("SendMail: session={}", session_id);
    }
    send_system_message(gate_ref, session_id, "发送邮件功能暂未开放。");
}

/// ReadMail: [mail_id: u32]
fn handle_read_mail(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 4 {
        let mail_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
        debug!("ReadMail: session={} id={}", session_id, mail_id);
    }
    send_system_message(gate_ref, session_id, "邮件系统功能暂未开放。");
}

/// CollectParcel: [mail_id: u32]
fn handle_collect_parcel(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 4 {
        let mail_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
        debug!("CollectParcel: session={} id={}", session_id, mail_id);
    }
    send_system_message(gate_ref, session_id, "收取邮件附件功能暂未开放。");
}

/// DeleteMail: [mail_id: u32]
fn handle_delete_mail(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 4 {
        let mail_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
        debug!("DeleteMail: session={} id={}", session_id, mail_id);
    }
    send_system_message(gate_ref, session_id, "删除邮件功能暂未开放。");
}

// ============================================================================
// 行会系统
// ============================================================================

/// GuildInvite: [target_name: DotNetString]
fn handle_guild_invite(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 4 {
        debug!("GuildInvite: session={}", session_id);
    }
    send_system_message(gate_ref, session_id, "行会邀请功能暂未开放。");
}

/// RequestGuildInfo: 请求行会信息
fn handle_request_guild_info(gate_ref: &ActorRef<GateActor>, session_id: SessionId, _payload: &[u8]) {
    debug!("RequestGuildInfo: session={}", session_id);
    send_system_message(gate_ref, session_id, "行会系统暂未开放。");
}

// ============================================================================
// 婚姻系统
// ============================================================================

/// MarriageRequest: [target_name: DotNetString]
fn handle_marriage_request(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 4 {
        debug!("MarriageRequest: session={}", session_id);
    }
    send_system_message(gate_ref, session_id, "求婚功能暂未开放。");
}

/// MarriageReply: [accept: bool]
fn handle_marriage_reply(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if !payload.is_empty() {
        let accept = payload[0] != 0;
        debug!("MarriageReply: session={} accept={}", session_id, accept);
    }
    send_system_message(gate_ref, session_id, "婚姻功能暂未开放。");
}

// ============================================================================
// 任务系统
// ============================================================================

/// AcceptQuest: [quest_id: u32]
fn handle_accept_quest(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 4 {
        let quest_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
        debug!("AcceptQuest: session={} quest={}", session_id, quest_id);
    }
    send_system_message(gate_ref, session_id, "任务系统暂未开放。");
}

/// FinishQuest: [quest_id: u32]
fn handle_finish_quest(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 4 {
        let quest_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
        debug!("FinishQuest: session={} quest={}", session_id, quest_id);
    }
    send_system_message(gate_ref, session_id, "任务系统暂未开放。");
}

// ============================================================================
// 精炼系统
// ============================================================================

/// DepositRefineItem: [unique_id: u64]
fn handle_deposit_refine_item(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 8 {
        let uid = u64::from_le_bytes(payload[..8].try_into().unwrap_or([0; 8]));
        debug!("DepositRefineItem: session={} uid={}", session_id, uid);
    }
    send_system_message(gate_ref, session_id, "精炼系统暂未开放。");
}

/// RetrieveRefineItem: [unique_id: u64]
fn handle_retrieve_refine_item(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 8 {
        let uid = u64::from_le_bytes(payload[..8].try_into().unwrap_or([0; 8]));
        debug!("RetrieveRefineItem: session={} uid={}", session_id, uid);
    }
    send_system_message(gate_ref, session_id, "精炼系统暂未开放。");
}

/// RefineCancel: []
fn handle_refine_cancel(gate_ref: &ActorRef<GateActor>, session_id: SessionId, _payload: &[u8]) {
    debug!("RefineCancel: session={}", session_id);
    send_system_message(gate_ref, session_id, "精炼系统暂未开放。");
}

/// RefineItem: [item_id: u32][materials: u32]
fn handle_refine_item(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 8 {
        let item_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
        debug!("RefineItem: session={} item={}", session_id, item_id);
    }
    send_system_message(gate_ref, session_id, "精炼系统暂未开放。");
}

/// CheckRefine: [unique_id: u64]
fn handle_check_refine(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 8 {
        let uid = u64::from_le_bytes(payload[..8].try_into().unwrap_or([0; 8]));
        debug!("CheckRefine: session={} uid={}", session_id, uid);
    }
    send_system_message(gate_ref, session_id, "精炼系统暂未开放。");
}

// ============================================================================
// 传送/地图
// ============================================================================

/// RequestMapInfo: [map_id: u32]
fn handle_request_map_info(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 4 {
        let map_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
        debug!("RequestMapInfo: session={} map={}", session_id, map_id);
    }
    send_system_message(gate_ref, session_id, "地图传送功能暂未开放。");
}

/// SearchMap: [keyword: DotNetString]
fn handle_search_map(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 4 {
        debug!("SearchMap: session={}", session_id);
    }
    send_system_message(gate_ref, session_id, "地图搜索功能暂未开放。");
}

/// Observe: [target_id: u32]
fn handle_observe(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 4 {
        let target_id = u32::from_le_bytes(payload[0..4].try_into().unwrap_or([0; 4]));
        debug!("Observe: session={} target={}", session_id, target_id);
    }
    send_system_message(gate_ref, session_id, "观察模式暂未开放。");
}

// ============================================================================
// 其他
// ============================================================================

/// ReplaceWedRing: [unique_id: u64]
fn handle_replace_wed_ring(gate_ref: &ActorRef<GateActor>, session_id: SessionId, payload: &[u8]) {
    if payload.len() >= 8 {
        let uid = u64::from_le_bytes(payload[..8].try_into().unwrap_or([0; 8]));
        debug!("ReplaceWedRing: session={} uid={}", session_id, uid);
    }
    send_system_message(gate_ref, session_id, "更换婚戒功能暂未开放。");
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
